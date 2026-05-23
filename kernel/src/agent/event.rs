use crate::{agent::context::AgentStatus, message::RoleMessage, mind::stream::GenerateEvent};

pub trait EnvEvent: Send + Sync + 'static {
    fn id(&self) -> i64;
    fn is_force(&self) -> bool;
    fn to_message(&self) -> RoleMessage;
}
#[derive(Debug, Clone)]
pub enum AgentEvent<E> {
    Env(E),
    Generate(GenerateEvent),
    Status(AgentStatus),
    Overflow,
}
