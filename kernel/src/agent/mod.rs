pub mod context;
pub mod error;

pub mod port;

use std::{marker::PhantomData, path::PathBuf, sync::Arc};

use crate::{
    mind::{
        context::EventContext,
        event::EnvEvent,
        port::{CommandEvent, MindInlet, MindOutlet},
        stream::StreamMind,
    },
    model::ModelModal,
};

use async_trait::async_trait;
use futures::StreamExt;

use crate::agent::context::AgentContext;
pub struct AgentIdentity {
    pub id: u32,
    pub code: String,
    pub name: String,
    pub workspace: PathBuf,
    pub tool_packages: Vec<String>,
    pub input_modals: Vec<ModelModal>,
}
impl AgentIdentity {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn code(&self) -> &str {
        self.code.as_str()
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn workspace(&self) -> PathBuf {
        self.workspace.clone()
    }
}

#[async_trait]
pub trait AgentOutfit<E, C>: Send + Sync + 'static {
    async fn send_cmd(&self, cmd: CommandEvent) -> anyhow::Result<()>;

    fn event_context(&self) -> Arc<C>;
    fn context(&self) -> Arc<AgentContext>;
    fn identity(&self) -> Arc<AgentIdentity>;
    async fn shutdown(&self) -> anyhow::Result<()>;
}
