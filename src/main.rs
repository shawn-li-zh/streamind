use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs},
    Client,
};
use std::io::{self, Write};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod stream;
mod event;
mod agent;
mod llm_engine;
mod db;
mod rule;
mod service;
mod common;
mod global;

use db::Database;
use service::{AgentService, RuleService};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting StreamInd AI Client");

    dotenvy::dotenv()?;

    let client = Client::with_config(
        OpenAIConfig::new()
            .with_api_base(std::env::var("API_BASE")?)
            .with_api_key(std::env::var("API_KEY")?),
    );

    let db = Arc::new(Database::new("streamind.db")?);
    let agent_service = AgentService::new(db.clone());
    let rule_service = RuleService::new(db.clone());

    agent_service
        .init_agent("Assistant", "初始化记忆", "帮助用户完成任务")
        .await?;

    let group_id = rule_service.create_rule_group("default")?;
    rule_service.add_rule(
        group_id,
        r#""hello"&"world""#,
        "inject(你好世界)",
        3600,
    )?;
    rule_service.add_rule(
        group_id,
        r#"contains("help")"#,
        "context(用户需要帮助)",
        3600,
    )?;

    println!("\n=== StreamInd AI Chat ===");
    println!("Type 'exit' to quit\n");

    let mut conversation: Vec<ChatCompletionRequestMessage> = vec![];

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }
        if input.eq_ignore_ascii_case("exit") {
            println!("Goodbye!");
            break;
        }

        let injections = agent_service.add_event(input).await?;
        if !injections.is_empty() {
            println!("[Rule triggered: {}]", injections.join(", "));
        }

        let user_msg = ChatCompletionRequestUserMessageArgs::default()
            .content(input.to_string())
            .build()?;
        conversation.push(user_msg.into());

        info!("Sending message to OpenAI");

        let response = client
            .chat()
            .create(async_openai::types::CreateChatCompletionRequest {
                model: "gpt-4o-mini".to_string(),
                messages: conversation.clone(),
                ..Default::default()
            })
            .await?;

        if let Some(choice) = response.choices.first() {
            let reply = choice.message.content.clone().unwrap_or_default();
            println!("\nAssistant: {}\n", reply);

            let _ = agent_service.evaluate_rules(&reply).await;

            conversation.push(
                async_openai::types::ChatCompletionRequestMessage::Assistant(
                    async_openai::types::ChatCompletionRequestAssistantMessageArgs::default()
                        .content(reply)
                        .build()?,
                ),
            );
        }
    }

    rule_service.cleanup_expired()?;

    Ok(())
}