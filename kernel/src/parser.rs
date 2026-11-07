use std::collections::HashMap;

use anyhow::Context;
//格式 pkg.fn(...args)
pub fn parse_call(
    input: &str,
) -> anyhow::Result<(Option<&str>, &str, HashMap<String, serde_json::Value>)> {
    if input.is_empty() {
        return Err(anyhow::anyhow!("无效的调用表达式"));
    }
    let (pkg, call) = input
        .split_once(".")
        .map(|(p, f)| (Some(p), f))
        .unwrap_or((None, input));
    // 1. 获取函数名
    let index = call
        .find("(")
        .with_context(|| format!("解析 '{}' 失败", input))?;
    let func_name = &call[..index];
    let args_str = &call[index..];

    // 2. 获取参数
    // 1. 去除首尾的括号
    let trimmed = args_str.trim();
    if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return Err(anyhow::anyhow!("参数格式错误：必须以 ( 开头并以 ) 结尾"));
    }
    let content = &trimmed[1..trimmed.len() - 1];

    let args =
        parse_args(content, '=', ',').map_err(|e| anyhow::anyhow!("Invalid arguments:{e}"))?;

    Ok((pkg, func_name, args))
}
/// 格式 aaa=1,bbb={},ccc=[],ddd="str",eee=true
pub fn parse_args(
    input: &str,
    pair: char,
    delimiter: char,
) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    let mut result = HashMap::new();

    // 2. 将内容按顶层的逗号拆分 (忽略 {} , [] 和 "" 内部的逗号)
    let parts = split_top_level(input, delimiter)?;

    for part in parts {
        if part.trim().is_empty() {
            continue;
        }
        // 3. 拆分键值对
        let (key, value_str) = part
            .split_once(pair)
            .ok_or_else(|| anyhow::anyhow!("无效的键值对: {}", part))?;

        let key = key.trim().to_string();
        let value_str = value_str.trim();
        let value = match value_str {
            "None" | "null" | "undefined" => continue,
            "True" | "true" => serde_json::Value::Bool(true),
            "False" | "false" => serde_json::Value::Bool(false),
            _ => {
                if (value_str.starts_with("'") && value_str.ends_with("'"))
                    || (value_str.starts_with("`") && value_str.ends_with("`"))
                    || (value_str.starts_with("\"") && value_str.ends_with("\""))
                {
                    serde_json::Value::String(value_str[1..value_str.len() - 1].to_string())
                } else {
                    serde_json::from_str(value_str)
                        .with_context(|| format!("解析键 '{}' 的值时失败: {}", key, value_str))?
                }
            }
        };
        // 4. 利用 serde_json 解析值
        result.insert(key, value);
    }

    Ok(result)
}

pub fn parse_xml(xml: &str) -> anyhow::Result<(String, HashMap<String, serde_json::Value>)> {
    let xml = xml.trim();
    if !(xml.starts_with('<') && xml.ends_with("/>")) {
        anyhow::bail!("无效的组件标签语法:{xml}");
    }
    let inner = xml[1..xml.len() - 2].trim();
    // 标签名与属性之间可以是任意空白，也允许完全没有属性（<Foo/>）
    let (name, attr_str) = match inner.find(char::is_whitespace) {
        Some(idx) => (inner[..idx].trim(), inner[idx..].trim()),
        None => (inner, ""),
    };
    if name.is_empty() {
        anyhow::bail!("无效的组件标签语法:{xml}");
    }
    let name = name.to_string();
    let attrs = parse_attrs(attr_str);
    Ok((name, attrs))
}

/// 辅助函数：只在括号/引号层级为 0 时分割字符串
fn split_top_level(s: &str, delimiter: char) -> anyhow::Result<Vec<String>> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth_brace = 0; // {}
    let mut depth_bracket = 0; // []
    let mut in_quotes1 = false;
    let mut in_quotes2 = false;
    let chars: Vec<char> = s.chars().collect();

    for i in 0..chars.len() {
        match chars[i] {
            // 注意：只有当另一种引号未开启时才切换状态，
            // 否则 expr='a="b' 这种「单引号里含奇数个双引号」的参数会被判成引号不匹配
            '"' if !in_quotes2 && (i == 0 || chars[i - 1] != '\\') => in_quotes1 = !in_quotes1,
            '\'' if !in_quotes1 && (i == 0 || chars[i - 1] != '\\') => in_quotes2 = !in_quotes2,
            _ if in_quotes1 || in_quotes2 => continue, // 引号内忽略任何符号
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            c if c == delimiter && depth_brace == 0 && depth_bracket == 0 => {
                let part: String = chars[start..i].iter().collect();
                parts.push(part);
                start = i + 1;
            }
            _ => {}
        }
    }

    if in_quotes1 || in_quotes2 || depth_brace != 0 || depth_bracket != 0 {
        return Err(anyhow::anyhow!("括号或引号不匹配"));
    }
    let part: String = chars[start..].iter().collect();
    parts.push(part);
    Ok(parts)
}

/// 专门解析 "key=value key=value" 格式的字符串，输出 KV 键值对
pub fn parse_attrs(mut s: &str) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();

    while !s.is_empty() {
        s = s.trim_start();
        if s.is_empty() {
            break;
        }

        // 1. 提取 Key (遇到 '=' 或空格停止)
        let key_end = s
            .find(|c: char| c == '=' || c.is_whitespace())
            .unwrap_or(s.len());
        let key = &s[..key_end];
        s = s[key_end..].trim_start();

        // 2. 找等号 '='
        if !s.starts_with('=') {
            // 没有等号，当作布尔值标签（如 disabled）
            map.insert(key.to_string(), serde_json::Value::Bool(true));
            continue;
        }
        s = s[1..].trim_start(); // 跳过 '=' 和后续空格

        // 3. 提取裸文本 Value 和剩余的字符串
        let (raw_val, remainder) = extract_raw_value(s);

        // 4. 扔给 serde_json 解析！
        // 如果是标准的 {} [] "" true false 123，serde_json 都能完美解析
        // 如果是 class="flex-col" 这种没有引号的非标字符串，反序列化会失败，我们直接降级为普通 String
        let val = serde_json::from_str(raw_val).unwrap_or_else(|_| {
            // 兼容处理：去掉两边可能存在的单引号（serde_json不支持单引号）
            let clean_str = raw_val.trim_matches(|c| c == '\'' || c == '"');
            serde_json::Value::String(clean_str.to_string())
        });

        map.insert(key.to_string(), val);
        s = remainder; // 继续处理下一对 KV
    }

    map
}

/// 核心切割逻辑：根据开头字符，切出完整的 value 文本区块
fn extract_raw_value(s: &str) -> (&str, &str) {
    let mut chars = s.char_indices();
    let Some((_, first_char)) = chars.next() else {
        return ("", "");
    };

    match first_char {
        // 场景 1: 对象或数组 (处理括号嵌套，并防止被内部字符串里的括号干扰)
        '{' | '[' => {
            let open = first_char;
            let close = if open == '{' { '}' } else { ']' };
            let mut depth = 1;
            let mut in_quote = false;
            let mut escape = false;

            for (i, c) in chars {
                if escape {
                    escape = false;
                } else if c == '\\' {
                    escape = true;
                } else if c == '"' {
                    in_quote = !in_quote;
                } else if !in_quote {
                    if c == open {
                        depth += 1;
                    } else if c == close {
                        depth -= 1;
                        if depth == 0 {
                            let end = i + c.len_utf8();
                            return (&s[..end], &s[end..]); // 返回 (切割下的块, 剩余字符串)
                        }
                    }
                }
            }
            (s, "") // 兜底：括号没闭合，直接全吞
        }
        // 场景 2: 字符串 (寻找结尾引号)
        '"' | '\'' => {
            let quote = first_char;
            let mut escape = false;
            for (i, c) in chars {
                if escape {
                    escape = false;
                } else if c == '\\' {
                    escape = true;
                } else if c == quote {
                    let end = i + c.len_utf8();
                    return (&s[..end], &s[end..]);
                }
            }
            (s, "") // 兜底
        }
        // 场景 3: 裸字符 (如 boolean、数字、或者无引号字符串，遇到空格或标签结尾 `>` `/` 停止)
        _ => {
            // 裸值：遇到空白或 '>' 结束；'/' 只有构成自闭合 "/>" 时才算结束，
            // 否则 path=/public/a.png 会被截成空串
            let mut end = s.len();
            for (i, c) in s.char_indices() {
                if c.is_whitespace() || c == '>' {
                    end = i;
                    break;
                }
                if c == '/' && s[i + c.len_utf8()..].starts_with('>') {
                    end = i;
                    break;
                }
            }
            (&s[..end], &s[end..])
        }
    }
}

#[cfg(test)]
mod test {
    use crate::parser;

    #[test]
    fn test_call() -> anyhow::Result<()> {
        let raw = r#"pkg.tool(a=1,   b={"inner":[1,2,3]},c=[1,2,{"a":{"b":1}}],d="str\"",e=true,f=false,g=1.3)"#;

        // 这两个用例来自真实模型输出，必须能解析（原来只打印，失败也不会让测试挂掉）
        let (pkg, func, args) = parser::parse_call(raw).expect("真实工具调用应当解析成功");
        println!("解析成功: {:?} {func} {:?}", pkg, args);
        Ok(())
    }
    #[test]
    fn test_call2() -> anyhow::Result<()> {
        let raw = r#"reflect.save_rule(group_id=4, name="ImageList本地图片前缀提醒", expr='similar("ImageList本地图片渲染规则") > 0.7 & event_type_is("userRequest")', content="您正在使用 ImageList 组件渲染本地图片，请记得在 src 路径前加上 public/ 前缀，如 src='/public/photo.png'，否则生产环境将无法加载。")
"#;

        // 这两个用例来自真实模型输出，必须能解析（原来只打印，失败也不会让测试挂掉）
        let (pkg, func, args) = parser::parse_call(raw).expect("真实工具调用应当解析成功");
        println!("解析成功: {:?} {func} {:?}", pkg, args);
        Ok(())
    }

    #[test]
    fn test_render() {
        let raw_tag = r#"<AuthRequest title="获取/usr/info.json文件的创建权限" class="flex flex-col" style="with:100px;" attrs={  "code":"file_unsafe",
    "subCode" :"/usr/info.json",
    "purpose" : "需要保存您的个人基本信息以提供个性化服务。"} />"#;

        let (tag_name, attrs) = super::parse_xml(raw_tag).unwrap();

        // 验证标签名
        assert_eq!(tag_name, "AuthRequest");

        // 验证普通属性
        assert_eq!(
            attrs.get("title").unwrap().as_str().unwrap(),
            "获取/usr/info.json文件的创建权限"
        );
        assert_eq!(
            attrs.get("class").unwrap().as_str().unwrap(),
            "flex flex-col"
        );

        // 验证跨行的 JSON 属性
        let json_attr = attrs.get("attrs").unwrap();
        assert!(json_attr.is_object(), "attrs 应该被解析为一个 JSON 对象");

        let obj = json_attr.as_object().unwrap();
        assert_eq!(obj.get("code").unwrap().as_str().unwrap(), "file_unsafe");
        assert_eq!(
            obj.get("subCode").unwrap().as_str().unwrap(),
            "/usr/info.json"
        );
        assert_eq!(
            obj.get("purpose").unwrap().as_str().unwrap(),
            "需要保存您的个人基本信息以提供个性化服务。"
        );

        println!("解析成功: 标签 {}, 属性: {:#?}", tag_name, attrs);
    }

    #[test]
    fn test_parse_call_nested_values() {
        let raw = r#"pkg.tool(a=1, b={"inner":[1,2,3]}, c=[1,2,{"a":{"b":1}}], d="x, y", e=true, f=1.5)"#;
        let (pkg, func, args) = super::parse_call(raw).unwrap();
        assert_eq!(pkg, Some("pkg"));
        assert_eq!(func, "tool");
        assert_eq!(args["a"], serde_json::json!(1));
        assert_eq!(args["b"]["inner"][2], serde_json::json!(3));
        assert_eq!(args["c"][2]["a"]["b"], serde_json::json!(1));
        // 引号里的逗号不能当分隔符
        assert_eq!(args["d"], serde_json::json!("x, y"));
        assert_eq!(args["e"], serde_json::json!(true));
        assert_eq!(args["f"], serde_json::json!(1.5));
    }

    #[test]
    fn test_parse_call_without_package() {
        let (pkg, func, args) = super::parse_call("plain(a=1)").unwrap();
        assert_eq!(pkg, None);
        assert_eq!(func, "plain");
        assert_eq!(args["a"], serde_json::json!(1));
    }

    #[test]
    fn test_parse_call_odd_double_quote_in_single_quoted_arg() {
        // 单引号参数里出现奇数个双引号时，不能误判为「引号不匹配」
        let raw = r#"reflect.save_rule(expr='a="b', n=1)"#;
        let (_, func, args) = super::parse_call(raw).unwrap();
        assert_eq!(func, "save_rule");
        assert_eq!(args["expr"], serde_json::json!("a=\"b"));
        assert_eq!(args["n"], serde_json::json!(1));
    }

    #[test]
    fn test_parse_call_invalid_input() {
        assert!(super::parse_call("").is_err());
        assert!(super::parse_call("timer.sleep 1m").is_err());
        assert!(super::parse_call("timer.sleep(a=1").is_err()); // 括号不匹配
    }

    #[test]
    fn test_parse_attrs_mixed_types() {
        let mut attrs = super::parse_attrs(
            r#"id="a1" disabled title='中文 标题' code={"a":[1,2]} path=/public/a.png"#,
        );
        assert_eq!(attrs.remove("id").unwrap(), serde_json::json!("a1"));
        // 无等号的属性视为布尔标记
        assert_eq!(attrs.remove("disabled").unwrap(), serde_json::json!(true));
        assert_eq!(attrs.remove("title").unwrap(), serde_json::json!("中文 标题"));
        assert_eq!(attrs.remove("code").unwrap(), serde_json::json!({"a":[1,2]}));
        // 裸值里的 / 不能被当成标签结束符
        assert_eq!(attrs.remove("path").unwrap(), serde_json::json!("/public/a.png"));
        assert!(attrs.is_empty(), "出现了多余的键: {:?}", attrs);
    }

    #[test]
    fn test_parse_xml_variants() {
        // 无属性
        let (name, attrs) = super::parse_xml("<Foo/>").unwrap();
        assert_eq!(name, "Foo");
        assert!(attrs.is_empty());

        // 换行分隔的属性
        let (name, attrs) = super::parse_xml("<Bar\n a=\"1\" />").unwrap();
        assert_eq!(name, "Bar");
        assert_eq!(attrs["a"], serde_json::json!("1"));

        // 非法语法：未自闭合、没有尖括号
        assert!(super::parse_xml("<Foo>").is_err());
        assert!(super::parse_xml("Foo").is_err());
        assert!(super::parse_xml("</>").is_err());
    }
}
