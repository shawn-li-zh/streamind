use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub elapsed: String,
}

impl Section {
    pub fn to_string(&self) -> String {
        format!("<{}>\n{}\n</{}>", self.name, self.content, self.name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionTag {
    pub name: &'static str,
    pub start: &'static str,
    pub end: &'static str,
}
impl SectionTag {
    pub fn new(name: &'static str, start: &'static str, end: &'static str) -> Self {
        SectionTag { name, start, end }
    }
}
pub static TAGS: LazyLock<Vec<SectionTag>> = LazyLock::new(|| {
    vec![
        SectionTag::new("reply", "<reply>", "</reply>"),
        SectionTag::new("think", "<think>", "</think>"),
        SectionTag::new("tool_call", "<tool_call id=", "</tool_call>"),
        SectionTag::new("render", "<render>", "</render>"),
        SectionTag::new("push", "<push>", "</push>"),
    ]
});
