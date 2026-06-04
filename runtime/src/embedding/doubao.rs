use anyhow::Context as _;

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use kernel::{
    embedding::EmbeddingClient,
    message::{Binary, Information},
};
use serde::{Deserialize, Serialize};
use serde_json::from_value;
#[derive(Serialize)]
struct DoubaoEmbeddingRequest {
    model: String,
    input: Vec<EmbeddingInputItem>,
    dimensions: Option<u32>,
}
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum EmbeddingInputItem {
    Text {
        text: String,
    },
    #[serde(rename = "image_url")]
    Image {
        image_url: MultimodalUrl,
    },
    #[serde(rename = "audio_url")]
    Audio {
        audio_url: MultimodalUrl,
    },
    #[serde(rename = "video_url")]
    Video {
        video_url: MultimodalUrl,
    },
    // 豆包 API 通常将普通文件视作某种多模态输入，此处以 text/file 逻辑处理，具体参考文档
}
#[derive(Serialize)]
struct MultimodalUrl {
    url: String, // 这里存放 base64 数据: "data:image/jpeg;base64,..."
}
#[derive(Deserialize)]
struct DoubaoEmbeddingResponse {
    created: u32,
    data: EmbeddingData,
    object: String,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    object: String,
}
pub struct DoubaoEmbedding {
    api_key: String,
    api_url: String,
    model: String,
    dimensions: u32,
    client: reqwest::Client,
}
impl DoubaoEmbedding {
    pub fn new(api_key: String, api_url: String, model: String, dimensions: u32) -> Self {
        let client: reqwest::Client = reqwest::Client::new();
        DoubaoEmbedding {
            api_key,
            api_url,
            model,
            dimensions,
            client,
        }
    }
}
#[async_trait]
impl EmbeddingClient for DoubaoEmbedding {
    async fn union_embed(&self, infos: Vec<Information>) -> anyhow::Result<Vec<f32>> {
        let mut input_items = Vec::new();

        for info in infos {
            match info {
                Information::Function(_, _) => continue, // 忽略 Function
                Information::Text(t) => {
                    if !t.is_empty() {
                        input_items.push(EmbeddingInputItem::Text { text: t });
                    }
                }
                Information::Image(b) => {
                    let data_url = self.file_to_base64_url(b, "image").await?;
                    input_items.push(EmbeddingInputItem::Image {
                        image_url: MultimodalUrl { url: data_url },
                    });
                }
                Information::Audio(b) => {
                    let data_url = self.file_to_base64_url(b, "audio").await?;
                    input_items.push(EmbeddingInputItem::Audio {
                        audio_url: MultimodalUrl { url: data_url },
                    });
                }
                Information::Video(b) => {
                    let data_url = self.file_to_base64_url(b, "video").await?;
                    input_items.push(EmbeddingInputItem::Video {
                        video_url: MultimodalUrl { url: data_url },
                    });
                }
                Information::File(b) => {
                    // 一般多模态模型对 File 会转为 Text 或特殊处理，此处示例转为 base64
                    let data_url = self.file_to_base64_url(b, "application").await?;
                    input_items.push(EmbeddingInputItem::Text { text: data_url });
                }
            }
        }

        if input_items.is_empty() {
            return Ok(vec![]);
        }

        let request_body = DoubaoEmbeddingRequest {
            model: self.model.clone(),
            input: input_items,
            dimensions: Some(self.dimensions),
        };

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Doubao API")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }

        let res_data: DoubaoEmbeddingResponse = response.json().await?;
        Ok(res_data.data.embedding)
    }
}

impl DoubaoEmbedding {
    /// 辅助函数：读取本地文件并转换为 Data URL 格式
    async fn file_to_base64_url(
        &self,
        binary: Binary,
        default_type: &str,
    ) -> anyhow::Result<String> {
        let content = tokio::fs::read(&binary.path)
            .await
            .with_context(|| format!("Failed to read file at {:?}", binary.path))?;

        let mime = mime_guess::from_path(&binary.path)
            .first_raw()
            .unwrap_or_else(|| match default_type {
                "image" => "image/jpeg",
                "audio" => "audio/mpeg",
                "video" => "video/mp4",
                _ => "application/octet-stream",
            });

        let b64 = general_purpose::STANDARD.encode(content);
        Ok(format!("data:{};base64,{}", mime, b64))
    }
}
