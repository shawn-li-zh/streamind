use crate::{agent::context::AgentStatus, message::RoleMessage, mind::port::GenerateEvent};

pub trait EnvEvent: Send + Sync + 'static {
    fn id(&self) -> i64;
    fn to_message(&self) -> RoleMessage;
    fn size(&self) -> usize;
}
#[derive(Debug, Clone)]
pub enum AgentEvent<E> {
    Env(E),
    Generate(GenerateEvent),
    Status(AgentStatus),
    Overflow,
}
