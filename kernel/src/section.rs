use std::{sync::LazyLock, time::Duration};

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub id: i64,
    pub tag_name: String,
    pub content: String,
    pub time: String,
}

impl Section {
    pub fn new(tag: &'static SectionTag, content: String, time: Duration) -> Section {
        let time = humantime::format_duration(time).to_string();
        Section {
            id: 0,
            tag_name: tag.name.to_string(),
            content,
            time,
        }
    }
    pub fn to_string(&self) -> String {
        format!(
            "<{}>\n{}\n</{}>",
            self.tag_name, self.content, self.tag_name
        )
    }
}

#[derive(Debug, Clone)]
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
        SectionTag::new("think", "<think>", "</think>"),
        SectionTag::new("tool_call", "<tool_call>", "</tool_call>"),
        SectionTag::new("reply", "<reply>", "</reply>"),
        SectionTag::new("render", "<render>", "</render>"),
        SectionTag::new("error", "<error>", "</error>"),
        SectionTag::new("push", "<push>", "</push>"),
        SectionTag::new("pop", "<pop>", "</pop>"),
        SectionTag::new("interrupt", "<interrupt>", "</interrupt>"),
    ]
});
