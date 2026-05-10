use std::collections::HashMap;

use serde::{Deserialize, Serialize};
#[derive(Clone)]
pub struct ItemGroup {
    pub name: &'static str,
    pub description: &'static str,
    pub items: HashMap<String, Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    #[serde(default)]
    pub id: u32,
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub editable: bool,
    #[serde(default)]
    pub removable: bool,
}
