use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{Context, Error, Result, anyhow};
use dashmap::DashMap;
use serde_json::Value;
use tokio::{
    sync::{RwLock, mpsc},
    task::JoinHandle,
};
use tracing::debug;

use crate::{
    agent::AgentContext,
    tool::{
        Tool, ToolPackagePool,
        context::ToolPackageContext,
        core::{ToolCallReturn, ToolPackage},
    },
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
pub struct ToolEngineContext {
    pub tool_packages: HashSet<&'static str>,
    pub package_pool: &'static ToolPackagePool,
    package_contexts: DashMap<&'static str, Arc<ToolPackageContext>>,
}
impl ToolEngineContext {
    pub fn get_package(&self, pkg_name: &str) -> Option<&'static ToolPackage> {
        if !self.tool_packages.contains(pkg_name) && !self.tool_packages.contains("*") {
            return None;
        }
        self.package_pool.get(pkg_name)
    }
    pub fn get_package_context(&self, pkg_name: &str) -> Option<Arc<ToolPackageContext>> {
        self.package_contexts
            .get(pkg_name)
            .map(|v| v.value().clone())
    }
    pub fn init(&self, agent_ctx: &AgentContext) {
        for pkg_name in &self.tool_packages {
            let pkg = self.package_pool.get(pkg_name).expect("工具包不存在");
            let ctx = pkg.new_context(agent_ctx);
            let pkg_ctx = ToolPackageContext::new(ctx, pkg);
            self.package_contexts.insert(pkg.name, Arc::new(pkg_ctx));
        }
    }
    pub fn build(
        tool_packages: HashSet<&'static str>,
        package_pool: &'static ToolPackagePool,
    ) -> Self {
        Self {
            tool_packages,
            package_pool,
            package_contexts: DashMap::new(),
        }
    }
}

pub struct ToolEngine {
    agent_context: Arc<RwLock<AgentContext>>,
    context: Arc<ToolEngineContext>,
}
impl ToolEngine {
    pub fn new(agent_context: Arc<RwLock<AgentContext>>, context: Arc<ToolEngineContext>) -> Self {
        Self {
            agent_context,
            context,
        }
    }
    //code格式 await package.tool(..args)
    pub async fn batch_call(
        &self,
        calls: Vec<ToolCall>,
        sender: mpsc::Sender<ToolCallEvent>,
    ) -> anyhow::Result<()> {
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

        for ToolCall {
            id,
            package,
            tool,
            args,
            context,
            active_tip,
            is_await,
        } in calls
        {
            debug!("{}.{}(...)", package.name, tool.name);
            sender
                .send(ToolCallEvent::Active(id, active_tip))
                .await
                .unwrap();
            if is_await {
                // 串行：立即执行并等待结果，存入对应的索引位置
                let res = tool.call(args, &context, self.agent_context.clone()).await;
                let is_error = res.is_err();
                send(sender.clone(), id, res).await;
                if is_error {
                    return Err(anyhow!("同步函数调用失败!"));
                }
            } else {
                let sender: mpsc::Sender<ToolCallEvent> = sender.clone();
                let agent_ctx = self.agent_context.clone();
                let handle = tokio::spawn(async move {
                    let res: std::result::Result<ToolCallReturn, anyhow::Error> =
                        tool.call(args, &context, agent_ctx).await;
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
    pub fn batch_preprocess(&self, codes: Vec<(i64, String)>) -> PreprocessResult {
        //
        //语法解析
        let mut valid_tasks = Vec::with_capacity(codes.len());
        let mut invalid = Vec::with_capacity(codes.len());
        for i in 0..codes.len() {
            let (id, code) = &codes[i];
            let id = *id;
            match self.preprocess(id, code) {
                Ok(d) => {
                    valid_tasks.push(d);
                }
                Err(e) => {
                    invalid.push((id, e));
                }
            }
        }
        if !invalid.is_empty() {
            return PreprocessResult::Invalid(invalid);
        }
        PreprocessResult::Valid(valid_tasks)
    }
    fn preprocess(&self, id: i64, code: &String) -> anyhow::Result<ToolCall> {
        let mut expr = code.as_str();
        let is_await = code.starts_with("await ");
        if is_await {
            expr = &code[6..]
        }
        let (pkg_name, expr) = match expr.split_once(".") {
            Some(p) => p,
            None => {
                return Err(anyhow::anyhow!("Invalid code format: '{}'", code));
            }
        };
        let package: &'static ToolPackage = match self.context.get_package(pkg_name) {
            Some(v) => v,
            None => return Err(anyhow::anyhow!("Package '{}' not found!", code)),
        };

        //
        let (tool_name, args) = match expr.find("(") {
            Some(i) => (&expr[..i], &expr[i..]),
            None => {
                return Err(anyhow::anyhow!("Invalid code format: '{}'", code));
            }
        };
        let tool: &'static Tool = match package.get_tool(tool_name) {
            Some(v) => v,
            None => {
                return Err(anyhow::anyhow!(
                    "Tool '{}' not found in package '{}'!",
                    tool_name,
                    pkg_name
                ));
            }
        };
        //
        let args = parse_args(args).map_err(|e| anyhow::anyhow!("Invalid arguments:{e}"))?;
        //
        let context = self
            .context
            .get_package_context(package.name)
            .ok_or_else(|| anyhow::anyhow!("上下文不存在"))?;
        let active_tip = tool.active_tip(args.clone(), &context, self.agent_context.clone());
        Ok(ToolCall {
            id,
            package,
            tool,
            args,
            context,
            active_tip,
            is_await,
        })
    }
}

pub struct ToolCall {
    pub id: i64,
    pub package: &'static ToolPackage,
    pub tool: &'static Tool,
    pub context: Arc<ToolPackageContext>,
    pub args: HashMap<String, Value>,
    pub active_tip: String,
    pub is_await: bool,
}

pub enum PreprocessResult {
    Valid(Vec<ToolCall>),
    Invalid(Vec<(i64, Error)>),
}

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
#[cfg(test)]
mod test {
    use crate::tool::engine::parse_args;

    #[test]
    fn test() -> anyhow::Result<()> {
        let raw =
            r#"(a=1,b={"inner":[1,2,3]},c=[1,2,{"a":{"b":1}}],d="str\"",e=true,f=false,g=1.3)"#;

        match parse_args(raw) {
            Ok(map) => {
                println!("解析成功: {:#?}", map);
            }
            Err(e) => eprintln!("错误: {}", e),
        }
        Ok(())
    }
}
