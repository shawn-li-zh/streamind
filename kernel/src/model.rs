use serde::{Deserialize, Serialize};

pub enum ModelFamily {
    OpenAI,
    DeepSeek,
    Azure,
    Claude,
    Custom,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelModal {
    Text,
    Image,
    Audio,
    Video,
    Embedding,
    Other,
}
