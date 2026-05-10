use async_openai::{
    Client,
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionResponseStream,
        CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
    },
};
#[derive(Debug, Clone)]
pub struct LLMEngine {
    pub model: entity::model::Model,
    pub client: Client<OpenAIConfig>,
}
impl LLMEngine {
    pub async fn stream(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<ChatCompletionResponseStream, OpenAIError> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.name.clone())
            .messages(messages)
            .build()?;
        let stream = self.client.chat().create_stream(request).await;
        return stream;
    }

    pub async fn content(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<CreateChatCompletionResponse, OpenAIError> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.name.clone())
            .messages(messages)
            .build()?;
        let res = self.client.chat().create(request).await;

        return res;
    }
}
