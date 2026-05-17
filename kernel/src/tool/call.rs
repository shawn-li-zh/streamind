use std::{collections::HashMap, sync::Arc};

use crate::{
    agent::AgentContext,
    tool::{
        Tool, ToolCallContext, ToolPackage,
        context::{ToolContext, ToolPackageContext},
        core::ToolReturn,
        engine::ToolEngineContext,
        parser,
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
    pub fn from(code: &str, engine_ctx: &ToolEngineContext) -> anyhow::Result<Self> {
        let (pkg_name, expr) = match code.split_once(".") {
            Some(p) => p,
            None => {
                return Err(anyhow::anyhow!("Invalid tool_call format: '{}'", code));
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
                return Err(anyhow::anyhow!("Invalid tool_call format: '{}'", code));
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
            parser::parse_args(args).map_err(|e| anyhow::anyhow!("Invalid arguments:{e}"))?;
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
            id: 0,
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
