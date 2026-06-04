use std::{collections::HashSet, path::PathBuf};

use axum::response::sse;
use kernel::{agent::AgentIdentity, model::ModelModal, tool::engine::ToolEngineContext};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::info;

use crate::{global::TOOL_PACKAGE_TOOL, sse_event::WebEvent};

pub struct WebSender {
    inner: RwLock<Option<mpsc::UnboundedSender<sse::Event>>>,
}
impl WebSender {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }
    pub fn send(&self, event: WebEvent) {
        let web_sender = self.inner.read();
        if let Some(sender) = &*web_sender {
            if let Err(e) = sender.send(event.to_sse_event()) {
                //断开
                info!("sse连接断开：{}", e);
                drop(web_sender);
                let mut web_sender = self.inner.write();
                *web_sender = None;
            }
            return;
        }
        // warn!("断连中...");
    }
    pub fn set_sender(&self, sender: mpsc::UnboundedSender<sse::Event>) {
        let mut web_sender = self.inner.write();
        *web_sender = Some(sender);
    }
}

pub fn build_agent_identity(agent: &entity::agent::Model, modas: Vec<ModelModal>) -> AgentIdentity {
    let identity = AgentIdentity {
        id: agent.id,
        code: agent.code.to_string(),
        name: agent.name.to_string(),
        workspace: PathBuf::from(&agent.workspace),
        input_modals: modas,
        tool_packages: agent.tool_packages.0.clone(),
    };
    identity
}

pub fn build_tool_engine_context(tool_packages: &Vec<String>) -> ToolEngineContext {
    let mut tool_pkgs = HashSet::new();
    for pkg_name in tool_packages {
        if pkg_name == "*" {
            tool_pkgs.extend(TOOL_PACKAGE_TOOL.keys());
            break;
        }
        tool_pkgs.insert(
            TOOL_PACKAGE_TOOL
                .get(pkg_name.as_str())
                .expect("包不存在")
                .name,
        );
    }
    ToolEngineContext::new(tool_pkgs, &*TOOL_PACKAGE_TOOL)
}
