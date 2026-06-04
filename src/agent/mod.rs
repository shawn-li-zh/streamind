use crate::agent::{app::AppAgent, memory::MemoryAgent};

pub mod app;
pub mod common;
pub mod memory;

pub enum Agent {
    App(AppAgent),
    Memory(MemoryAgent),
}
