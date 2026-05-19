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
            _ => serde_json::from_str(value_str)
                .with_context(|| format!("解析键 '{}' 的值时失败: {}", key, value_str))?,
        };
        // 4. 利用 serde_json 解析值
        result.insert(key, value);
    }

    Ok(result)
}

pub fn parse_xml(xml: &str) -> anyhow::Result<(String, HashMap<String, serde_json::Value>)> {
    if !(xml.starts_with("<") && xml.ends_with("/>")) {
        anyhow::bail!("无效的组件标签语法:{xml}");
    }
    let inner = xml[1..xml.len() - 2].trim();
    let (name, attr_str) = match inner.split_once(" ") {
        Some((n, last)) => (n.trim(), last.trim()),
        None => {
            anyhow::bail!("缺失title属性");
        }
    };
    let name = name.to_string();
    let attrs = parse_kv_pairs(attr_str);
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
            '"' if i == 0 || chars[i - 1] != '\\' => in_quotes1 = !in_quotes1,
            '\'' if i == 0 || chars[i - 1] != '\\' => in_quotes2 = !in_quotes2,
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

use serde_json::Value;

/// 专门解析 "key=value key=value" 格式的字符串，输出 KV 键值对
pub fn parse_kv_pairs(mut s: &str) -> HashMap<String, Value> {
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
            map.insert(key.to_string(), Value::Bool(true));
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
            Value::String(clean_str.to_string())
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
            let end = s
                .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .unwrap_or(s.len());
            (&s[..end], &s[end..])
        }
    }
}

#[cfg(test)]
mod test {
    use std::backtrace::Backtrace;

    use crate::parser;

    #[test]
    fn test_call() -> anyhow::Result<()> {
        let raw = r#"pkg.tool(a=1,   b={"inner":[1,2,3]},c=[1,2,{"a":{"b":1}}],d="str\"",e=true,f=false,g=1.3)"#;

        match parser::parse_call(raw) {
            Ok((pkg, func, map)) => {
                println!("解析成功: {} {func} {:?} ", pkg.unwrap_or_default(), map);
            }
            Err(e) => {
                let bt = Backtrace::capture();
                eprintln!("错误: {:?}\n{:?}", e, bt)
            }
        }
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
}
