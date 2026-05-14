use async_openai::{
    Client,
    config::OpenAIConfig,
    error::OpenAIError,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionResponseStream,
        CreateChatCompletionRequestArgs, CreateChatCompletionResponse, ReasoningEffort,
    },
};
use async_trait::async_trait;
use futures::StreamExt;

use crate::{kit::message_kit, llm::family::ModelFamily};

pub type ChunkStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<String, OpenAIError>> + Send>>;

#[async_trait]
pub trait ChatClient: Send + Sync + 'static {
    fn model_info(&self) -> (ModelFamily, String);
    async fn stream(
        &self,
        messages: Vec<kernel::message::Message>,
    ) -> Result<ChunkStream, OpenAIError>;
}

#[derive(Debug, Clone)]

pub struct LLMClient {
    pub model: entity::model::Model,
    pub client: Client<OpenAIConfig>,
}
#[async_trait]
impl ChatClient for LLMClient {
    fn model_info(&self) -> (ModelFamily, String) {
        (ModelFamily::OpenAI, self.model.name.to_string())
    }
    async fn stream(
        &self,
        messages: Vec<kernel::message::Message>,
    ) -> Result<super::client::ChunkStream, OpenAIError> {
        let messages: Vec<ChatCompletionRequestMessage> = messages
            .into_iter()
            .map(|v| message_kit::to_sdk_message(v))
            .collect();
        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.name.clone())
            .reasoning_effort(ReasoningEffort::High)
            .messages(messages)
            .build()?;
        let stream = self.client.chat().create_stream(request).await?;
        let stream = stream.map(|r| {
            r.map(|mut v| {
                let text = v.choices[0].delta.content.take();
                let refusal = v.choices[0].delta.refusal.take();
                text.or(refusal).unwrap_or_default()
            })
        });
        return Ok(stream.boxed());
    }
}
