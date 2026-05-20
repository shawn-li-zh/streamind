use std::{collections::HashMap, sync::Arc};

use crate::{
    agent::AgentHandler, parser, tool::{
        Tool, ToolCallContext, ToolPackage,
        context::{ToolContext, ToolPackageContext},
        core::ToolReturn,
        engine::ToolEngineContext,
    }
};

pub struct ToolCall {
    pub id: i64,
    pub call_id: String,//模型生成的id
    pub package: &'static ToolPackage,
    pub tool: &'static Tool,
    pub package_context: Arc<ToolPackageContext>,
    pub tool_context: Arc<ToolContext>,
    pub args: HashMap<String, serde_json::Value>,
}
impl ToolCall {
    pub fn from(call_id:String,code: &str, engine_ctx: &ToolEngineContext) -> anyhow::Result<Self> {
        let (pkg_name, tool_name, args) = parser::parse_call(code.trim())?;
        //
        let pkg_name = pkg_name.unwrap_or_default();
        let package: &'static ToolPackage = match engine_ctx.get_package(pkg_name) {
            Some(v) => v,
            None => return Err(anyhow::anyhow!("Package '{}' not found!", code)),
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
            call_id,
            package,
            tool,
            args,
            package_context,
            tool_context,
        })
    }
    pub async fn exec(self, agent_handler: Arc<dyn AgentHandler>) -> anyhow::Result<ToolReturn> {
        let ctx_data = self.package_context.data.clone();
        let ctx = ToolCallContext::new(
            agent_handler,
            ctx_data,
            self.package,
            self.tool,
            format!("{}", self.id),
        );
        for interceptor in &self.tool_context.interceptors {
            interceptor.on_call(&ctx, &self)?;
        }
        (self.tool.handler)(self.args, ctx).await
    }
}
