use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub struct RenderComponent {
    //组件地址 xxx/xxx
    pub path: String,
    pub annotation: String,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderContext {
    pub path: String,
    pub title: String,
    pub style: Option<String>,
    pub class: Option<String>,
    pub attrs: HashMap<String, serde_json::Value>,
}
