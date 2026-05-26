use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReflectData {
    pub id: i64,
    pub name: String,
    pub content: String,
}
impl ReflectData {
    pub fn to_statement(&self) -> String {
        format!(
            "<item id={} name=\"{}\">\n{}\n</item>",
            self.id, self.name, self.content
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReflectEvent {
    pub source_event_id: i64,
    pub level: u32,
    pub datas: Vec<ReflectData>,
}
