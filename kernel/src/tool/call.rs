use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
use chrono::Local;

use crate::{
    agent::AgentContext,
    tool::{
        Tool, ToolCallContext, ToolPackage,
        context::{ToolContext, ToolPackageContext},
        core::ToolReturn,
        engine::ToolEngineContext,
    },
};

pub struct ToolCall {
    pub id: i64,
    pub package: &'static ToolPackage,
    pub tool: &'static Tool,
    pub package_context: Arc<ToolPackageContext>,
    pub tool_context: Arc<ToolContext>,
    pub args: HashMap<String, serde_json::Value>,
}
impl ToolCall {
    pub fn from(id: i64, code: &str, engine_ctx: &ToolEngineContext) -> anyhow::Result<Self> {
        let (pkg_name, expr) = match code.split_once(".") {
            Some(p) => p,
            None => {
                return Err(anyhow::anyhow!("Invalid code format: '{}'", code));
            }
        };
        let package: &'static ToolPackage = match engine_ctx.get_package(pkg_name) {
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
        let args: HashMap<String, serde_json::Value> =
            parse_args(args).map_err(|e| anyhow::anyhow!("Invalid arguments:{e}"))?;
        //
        let package_context = engine_ctx
            .get_package_context(package.name)
            .ok_or_else(|| anyhow::anyhow!("上下文不存在"))?;
        let tool_context = package_context
            .tool_contexts
            .get(tool.name)
            .map(|v| v.clone())
            .ok_or_else(|| anyhow::anyhow!("上下文不存在"))?;
        Ok(ToolCall {
            id,
            package,
            tool,
            args,
            package_context,
            tool_context,
        })
    }
    pub async fn exec(self, agent_ctx: Arc<AgentContext>) -> anyhow::Result<ToolReturn> {
        let ctx_data = self.package_context.data.clone();
        let ctx = ToolCallContext::new(
            agent_ctx,
            ctx_data,
            self.package,
            self.tool,
            format!("{}", self.id),
        )
        .await;
        for interceptor in &self.tool_context.interceptors {
            interceptor.on_call(&ctx, &self)?;
        }
        (self.tool.handler)(self.args, ctx).await
    }
}

/// 格式 (aaa=1,bbb={},ccc=[],ddd="str",eee=true)
fn parse_args(input: &str) -> anyhow::Result<HashMap<String, serde_json::Value>> {
    // 1. 去除首尾的括号
    let trimmed = input.trim();
    if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return Err(anyhow::anyhow!("参数格式错误：必须以 ( 开头并以 ) 结尾"));
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

#[cfg(test)]
mod test {
    use std::backtrace::Backtrace;

    use crate::tool::call::parse_args;

    #[test]
    fn test() -> anyhow::Result<()> {
        let raw =
            r#"(a=1,b={"inner":[1,2,3]},c=[1,2,{"a":{"b":1}}],d="str\"",e=true,f=false,g=1.3)"#;

        match parse_args(raw) {
            Ok(map) => {
                println!("解析成功: {:#?}", map);
            }
            Err(e) => {
                let bt = Backtrace::capture();
                eprintln!("错误: {:?}\n{:?}", e, bt)
            }
        }
        Ok(())
    }
}
