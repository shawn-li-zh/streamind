use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use crate::agent::Agent;
//全局agent池
pub static AGENT_POOL: LazyLock<HashMap<u32, Arc<Agent>>> = LazyLock::new(|| HashMap::new());
