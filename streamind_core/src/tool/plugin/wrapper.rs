use std::time::Duration;

use crate::{
    agent::AgentContext,
    tool::{Tool, ToolPackage, call::ToolCall},
};

pub trait ToolCallWrapper {
    fn build(agent: &AgentContext, pkg: &'static ToolPackage, tool: &'static Tool) -> Option<Self>
    where
        Self: Sized;
    async fn wrap<F>(&self, call: &ToolCall, future: F) -> anyhow::Result<serde_json::Value>
    where
        F: Future<Output = anyhow::Result<serde_json::Value>>;
}

pub struct TimeoutWrapper {
    pub time: Duration,
}
impl TimeoutWrapper {
    async fn wrap<F>(&self, call: &ToolCall, future: F) -> anyhow::Result<serde_json::Value>
    where
        F: Future<Output = anyhow::Result<serde_json::Value>>,
    {
        match tokio::time::timeout(self.time, future).await {
            Ok(o) => o,
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
}
