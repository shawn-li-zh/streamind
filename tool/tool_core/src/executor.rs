use std::{
    collections::{HashMap, HashSet},
    fmt::format,
    sync::Arc,
};

use serde_json::Value;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    Tool,
    context::ToolPackageContext,
    core::{ToolCallReturn, ToolPackage},
};
#[derive(Debug, Clone)]
pub enum ToolCallEvent {
    //调用id组
    Start(Vec<i64>),
    // section id+工具活动描述
    Active(i64, String),
    // section id+调用结果
    Success(i64, String),
    Failure(i64, String),
    End,
}
impl ToolCallEvent {
    pub fn from(id: i64, res: anyhow::Result<String>) -> Self {
        match res {
            Ok(c) => Self::Success(id, c),
            Err(e) => Self::Failure(id, format!("{:?}", e)),
        }
    }
}
pub type ToolPackagePool = HashMap<&'static str, ToolPackage>;
pub struct ToolExcutor {
    package_pool: &'static ToolPackagePool,
    packages: HashSet<String>,
    contexts: HashMap<&'static str, Arc<ToolPackageContext>>,
}
impl ToolExcutor {
    pub fn new(
        package_pool: &'static ToolPackagePool,
        packages: HashSet<String>,
        contexts: HashMap<&'static str, Arc<ToolPackageContext>>,
    ) -> Self {
        Self {
            package_pool,
            packages,
            contexts,
        }
    }

    //code格式 await package.fn(..args)

    pub async fn batch_call(
        &self,
        codes: Vec<(i64, String)>,
        sender: mpsc::Sender<ToolCallEvent>,
    ) -> anyhow::Result<()> {
        //语法解析
        let valid_tasks = self.parse_call(codes)?;

        // 执行阶段
        let mut concurrent_tasks: Vec<JoinHandle<()>> = Vec::new();
        async fn send(
            sender: mpsc::Sender<ToolCallEvent>,
            id: i64,
            res: Result<ToolCallReturn, anyhow::Error>,
        ) {
            match res {
                Ok(res) => {
                    sender
                        .send(ToolCallEvent::Success(id, res.data.to_string()))
                        .await
                        .unwrap();
                }
                Err(e) => {
                    sender
                        .send(ToolCallEvent::Failure(id, format!("{e}")))
                        .await
                        .unwrap();
                }
            }
        }
        for (id, pkg, tool, args, is_await) in valid_tasks {
            let ctx = self.contexts.get(pkg.name).unwrap();
            sender
                .send(ToolCallEvent::Active(id, tool.active_tip(args.clone(),&ctx)))
                .await
                .unwrap();
            if is_await {
                let ctx = self.contexts.get(pkg.name).unwrap();
                // 串行：立即执行并等待结果，存入对应的索引位置
                let res = tool.call(args, ctx).await;
                let is_error = res.is_err();
                send(sender.clone(), id, res).await;
                if is_error {
                    return Err(anyhow!("同步函数调用失败!"));
                }
            } else {
                let sender: mpsc::Sender<ToolCallEvent> = sender.clone();
                let ctx = ctx.clone();
                let handle = tokio::spawn(async move {
                    let res: std::result::Result<crate::core::ToolCallReturn, anyhow::Error> =
                        tool.call(args, &ctx).await;
                    send(sender, id, res).await
                });
                concurrent_tasks.push(handle);
            }
        }
        // 回收并发结果
        for handle in concurrent_tasks {
            // handle.await 可能会返回 join 错误（例如 panic 了）
            handle.await?;
        }
        Ok(())
    }
    fn parse_call(
        &self,
        codes: Vec<(i64, String)>,
    ) -> anyhow::Result<
        Vec<(
            i64,
            &'static ToolPackage,
            &'static Tool,
            HashMap<String, Value>,
            bool,
        )>,
    > {
        let mut has_parse_error = false;
        let mut error_info = String::new();
        //语法解析
        let mut valid_tasks = Vec::with_capacity(codes.len());
        for i in 0..codes.len() {
            let (id, code) = &codes[i];
            let mut expr = code.as_str();
            let is_await = code.starts_with("await ");
            if is_await {
                expr = &code[6..]
            }
            let (pkg_name, expr) = match expr.split_once(".") {
                Some(p) => p,
                None => {
                    has_parse_error = true;
                    error_info.push_str(format!("Invalid code format: '{}'", code).as_str());
                    continue;
                }
            };
            let op = self.package_pool.get(pkg_name);
            if !self.packages.contains(pkg_name) || op.is_none() {
                has_parse_error = true;
                error_info.push_str(format!("Package '{}' not found!", pkg_name).as_str());
                continue;
            }
            let pkg = op.unwrap();
            //
            let (tool_name, args) = match expr.find("(") {
                Some(i) => (&expr[..i], &expr[i..]),
                None => {
                    has_parse_error = true;
                    error_info.push_str(format!("Invalid code format: '{}'", code).as_str());
                    continue;
                }
            };
            let opt = pkg.get_tool(tool_name);
            if opt.is_none() {
                has_parse_error = true;
                error_info.push_str(
                    format!("Tool '{}' not found in package '{}'!", tool_name, pkg_name).as_str(),
                );
                continue;
            }
            let tool: &'static super::core::Tool = opt.unwrap();
            //
            let ctx = self.contexts.get(pkg_name).unwrap();
            let args = parse_args(args)?;
            valid_tasks.push((*id, pkg, tool, args, is_await));
        }
        if has_parse_error {
            return Err(anyhow!(error_info))?;
        }
        Ok(valid_tasks)
    }
}

use anyhow::{Context, Result, anyhow};
/// 格式 (aaa=1,bbb={},ccc=[],ddd="str",eee=true)
fn parse_args(input: &str) -> anyhow::Result<HashMap<String, Value>> {
    // 1. 去除首尾的括号
    let trimmed = input.trim();
    if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return Err(anyhow!("参数格式错误：必须以 ( 开头并以 ) 结尾"));
    }
    let content = &trimmed[1..trimmed.len() - 1];

    let mut result = HashMap::new();

    // 2. 将内容按顶层的逗号拆分 (忽略 {} , [] 和 "" 内部的逗号)
    let parts = split_top_level(content, ',')?;

    for part in parts {
        if part.trim().is_empty() {
            continue;
        }

        // 3. 按第一个 '=' 拆分键值对
        let (key, value_str) = part
            .split_once('=')
            .ok_or_else(|| anyhow!("无效的键值对: {}", part))?;

        let key = key.trim().to_string();
        let value_str = value_str.trim();

        // 4. 利用 serde_json 解析值
        // 注意：serde_json 要求字符串必须有双引号，这与你的 ddd="str" 一致
        let value: Value = serde_json::from_str(value_str)
            .with_context(|| format!("解析键 '{}' 的值时失败: {}", key, value_str))?;

        result.insert(key, value);
    }

    Ok(result)
}

/// 辅助函数：只在括号/引号层级为 0 时分割字符串
fn split_top_level(s: &str, delimiter: char) -> Result<Vec<String>> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth_brace = 0; // {}
    let mut depth_bracket = 0; // []
    let mut in_quotes = false;
    let chars: Vec<char> = s.chars().collect();

    for i in 0..chars.len() {
        match chars[i] {
            '"' if i == 0 || chars[i - 1] != '\\' => in_quotes = !in_quotes,
            _ if in_quotes => continue, // 引号内忽略任何符号
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            c if c == delimiter && depth_brace == 0 && depth_bracket == 0 => {
                parts.push(s[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }

    if in_quotes || depth_brace != 0 || depth_bracket != 0 {
        return Err(anyhow!("括号或引号不匹配"));
    }

    parts.push(s[start..].trim().to_string());
    Ok(parts)
}
#[test]
fn test() -> Result<()> {
    let raw = r#"(a=1,b={"inner":[1,2,3]},c=[1,2,{"a":{"b":1}}],d="str\"",e=true,f=false,g=1.3)"#;

    match parse_args(raw) {
        Ok(map) => {
            println!("解析成功: {:#?}", map);
        }
        Err(e) => eprintln!("错误: {}", e),
    }
    Ok(())
}
