use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionResponseStream, CreateChatCompletionRequestArgs,
    },
    Client,
};
#[derive(Debug, Clone)]
pub struct LLMEngine {
    pub model: String,
    pub client: Client<OpenAIConfig>,
}
impl LLMEngine {
    pub async fn stream(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<ChatCompletionResponseStream, OpenAIError> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.clone())
            .messages(messages)
            .build()?;

        let stream = self.client.chat().create_stream(request).await;
        return stream;
    }
}
