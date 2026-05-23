use std::{collections::HashMap, io::Write, path, sync::Arc};

use async_openai::types::ChatCompletionRequestMessage;
use chrono::Local;
use futures::{Stream, StreamExt};
use kernel::item::ItemGroup;
use tracing::error;

use crate::{
    agent::{event::AgentEvent, kit::{entity_kit, message_kit, prompt_kit, string_kit}, llm_engine::LLMEngine},
    global::{DATE_FMT, PROMPT_CACHE, TIME_FMT},
};

pub struct MindContext {
    pub llm_engine: Arc<LLMEngine>,
    pub history: Vec<AgentEvent>,
    pub content: String,
    pub messages: Vec<ChatCompletionRequestMessage>,
}
impl MindContext {
    pub fn push_events(&mut self, event_buffer: &mut Vec<AgentEvent>) {
        let buffer = std::mem::take(event_buffer);
        let (msg, len) = message_kit::build_step_messages(&buffer);
        self.messages.push(msg);
        //
        for event in buffer {
            self.push_content(&event.to_message());
            self.history.push(event);
        }
    }
    pub fn push_response(&mut self, response: String) {
        self.push_content(&response);
        //
        let res = prompt_kit::assistant_message(response);
        self.messages.push(res);
    }
    fn push_content(&mut self, c: &str) {
        self.content.push_str(c);
        self.content.push_str("\n\n---");
    }
    pub fn reset(&mut self, messages: Vec<ChatCompletionRequestMessage>) {
        self.messages = messages;
        let dir = std::env::var("HISTORY_DIR").expect("请设置历史对话存储文件夹");
        self.record(&dir);
        self.content = String::new();
        self.history.clear();
    }
    //重组上下文条目
    pub async fn reorganize_items(
        &self,
        agent_id: u32,
        groups: Vec<ItemGroup>,
    ) -> anyhow::Result<(Vec<ItemGroup>, String, String)> {
        let mut handles = vec![];
        //重组条目
        for mut group in groups {
            let mut map = HashMap::new();
            map.insert("item_group_name".to_string(), group.name.to_string());
            map.insert("item_group_desc".to_string(), group.description.to_string());
            let rule = PROMPT_CACHE
                .get_with_params("common/reorganize_item_rule", &map)
                .unwrap();
            //
            let mut messages = self.messages.clone();
            messages.push(prompt_kit::system_message(rule));
            let llm_engine = self.llm_engine.clone();
            //多线程更新
            let h = tokio::spawn(async move {
                let res = llm_engine.content(messages).await.unwrap();
                if let Some(content) = &(&res.choices[0].message).content {
                    let tags = string_kit::extract_tags(content, &vec!["save", "remove"]);

                    let actions = to_actions(tags);
                    entity_kit::update_item_group(agent_id, &mut group, actions)
                        .await
                        .unwrap();
                }
                group
            });
            handles.push(h);
        }
        //总结标题
        let sum = {
            let rule = PROMPT_CACHE.get("common/summary_ctx_rule").unwrap();
            let mut messages = self.messages.clone();
            messages.push(prompt_kit::system_message(rule));
            let res = self.llm_engine.content(messages).await.unwrap();
            let mut title = "略".to_string();
            let mut summary = "略".to_string();
            if let Some(content) = &(&res.choices[0].message).content {
                let tags = string_kit::extract_tags(content, &vec!["title", "summary"]);
                for (name, content) in tags {
                    if name == "title" {
                        title = content.to_string();
                    } else {
                        summary = content.to_string();
                    }
                }
            }
            (title, summary)
        };
        //
        let mut groups = vec![];
        for handle in handles {
            let g = handle.await?;
            groups.push(g);
        }
        Ok((groups, sum.0, sum.1))
    }
    //归档
    fn record(&self, dir: &str) {
        let now = Local::now();
        let date = now.format(DATE_FMT).to_string();
        let time = now.format(TIME_FMT).to_string();
        let dir = std::path::Path::new(dir);
        let dir = dir.join(&date);
        std::fs::create_dir_all(&dir).unwrap();
        //
        let file = dir.join(format!("{time}.txt"));
        let mut file = std::fs::File::create(file).unwrap();
        file.write_all(self.content.as_bytes()).unwrap();
    }

    async fn chunk_stream(&self) -> std::pin::Pin<Box<dyn Stream<Item = String> + Send>> {
        let llm_stream = self.llm_engine.stream(self.messages.clone()).await.unwrap();
        llm_stream
            .map(|c| match c {
                // 优化：使用 first_mut 和 and_then 扁平化取值，避免繁杂的 if/else
                Ok(mut r) => r
                    .choices
                    .first_mut()
                    .and_then(|choice| choice.delta.content.take())
                    .unwrap_or_default(),
                Err(e) => {
                    let section = format!("<error>Stream error: {:?}</error>", &e);
                    error!("{}", e);
                    // &self.inject_section(&section);
                    // String::new()
                    section
                }
            })
            .boxed()
    }
}

fn to_actions(tags: Vec<(&str, &str)>) -> (Vec<(String, String)>, Vec<String>) {
    let mut saves = vec![];
    let mut removes = vec![];
    for (a, c) in tags {
        if a == "save" {
            if let Some((name, content)) = c.split_once(":") {
                saves.push((name.to_string(), content.to_string()));
            }
        }
        if a == "remove" {
            for name in c.split(",") {
                removes.push(name.to_string());
            }
        }
    }
    (saves, removes)
}
