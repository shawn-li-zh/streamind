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
use kernel::{
    client::{ChatClient, ChunkStream},
    model::ModelFamily,
};

use crate::kit::message_kit;

#[derive(Debug, Clone)]

pub struct LLMClient {
    pub model: String,
    pub client: Client<OpenAIConfig>,
}
#[async_trait]
impl ChatClient for LLMClient {
    fn model_info(&self) -> (ModelFamily, String) {
        (ModelFamily::OpenAI, self.model.to_string())
    }
    async fn stream(
        &self,
        messages: Vec<kernel::message::RoleMessage>,
    ) -> anyhow::Result<ChunkStream> {
        let messages: Vec<ChatCompletionRequestMessage> = messages
            .into_iter()
            .map(|v| message_kit::to_sdk_message(v))
            .collect();
        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.clone())
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
            .map_err(|e| anyhow::anyhow!(e))
        });
        return Ok(stream.boxed());
    }
}
