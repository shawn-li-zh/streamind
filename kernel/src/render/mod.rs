use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::parser;

pub struct RenderComponent {
    //组件地址 xxx/xxx
    pub path: String,
    pub annotation: String,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderContext {
    pub name: String,
    pub title: String,
    pub path: Option<String>,
    pub style: Option<String>,
    pub class: Option<String>,
    pub attrs: HashMap<String, serde_json::Value>,
}
impl RenderContext {
    pub fn from_xml(xml: &str) -> anyhow::Result<Self> {
        let (name, mut args) = parser::parse_xml(xml)?;
        args.insert("name".to_string(), json!(name));
        let s = serde_json::to_value(args)?;
        let s = serde_json::from_value::<RenderContext>(s)?;
        Ok(s)
    }
}
