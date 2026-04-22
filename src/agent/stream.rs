use std::{collections::HashMap, io::Write, sync::Arc};

use async_openai::types::ChatCompletionRequestMessage;

use chrono::Local;
use streamind_core::{
    agent::{AgentContext, AgentStatus},
    item::ItemGroup,
};
use tracing::{error, info};

use crate::{
    agent::{
        core::Agent,
        event::{AgentEvent, AgentEventData},
        kit::{entity_kit, message_kit, prompt_kit, string_kit},
        llm_engine::LLMEngine,
        section::{section_stream, Section, SectionEvent},
    },
    global::{DATE_FMT, PROMPT_CACHE, TIME_FMT},
};
use futures::stream;
use tokio::sync::{mpsc, oneshot, watch, RwLock};

use futures::{Stream, StreamExt};

fn to_message(event: &AgentEvent) -> ChatCompletionRequestMessage {
    match &event.data {
        crate::agent::event::AgentEventData::UserInput(_) => {
            prompt_kit::user_message(event.to_message())
        }
        _ => prompt_kit::system_message(event.to_message()),
    }
}
fn build_bundle_messages(events: &Vec<AgentEvent>) -> ChatCompletionRequestMessage {
    //
    let mut msgs = vec![];
    //事件转消息
    for ele in events {
        let msg = ele.to_message();
        msgs.push(msg);
    }
    let c = msgs.join("\n\n");
    prompt_kit::system_message(c)
}

pub enum MindFlag {
    Closed,
    Excess,
}

pub struct MindContext {
    pub llm_engine: Arc<LLMEngine>,
    pub history: Vec<AgentEvent>,
    pub content: String,
    pub messages: Vec<ChatCompletionRequestMessage>,
}
impl MindContext {
    pub fn push_events(&mut self, event_buffer: &mut Vec<AgentEvent>) {
        let buffer = std::mem::take(event_buffer);
        let msg = build_bundle_messages(&buffer);
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

pub struct MindStream {
    pub context: MindContext,
    event_buffer: Vec<AgentEvent>,
    last_section_head: RwLock<String>,
    inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
    event_receiver: mpsc::Receiver<AgentEvent>,
    section_sender: mpsc::Sender<SectionEvent>,
    reorganize_watcher: watch::Sender<bool>,
    run_watcher: watch::Sender<bool>,
}

impl MindStream {
    pub fn new(
        engine: Arc<LLMEngine>,
        messages: Vec<ChatCompletionRequestMessage>,
        inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
        event_receiver: mpsc::Receiver<AgentEvent>,
        section_sender: mpsc::Sender<SectionEvent>,
        reorganize_watcher: watch::Sender<bool>,
        run_watcher: watch::Sender<bool>,
    ) -> Self {
        let context = MindContext {
            history: vec![],
            messages,
            content: String::new(),
            llm_engine: engine,
        };
        MindStream {
            context,
            event_buffer: vec![],
            last_section_head: RwLock::new(String::new()),
            inject_receiver,
            event_receiver,
            section_sender,
            reorganize_watcher,
            run_watcher,
        }
    }

    async fn inject_section(
        &self,
        inject_sender: &mpsc::UnboundedSender<String>,
        injected: (String, bool),
    ) {
        let (inject, is_section) = injected;
        if is_section {
            let mut last_section_head = self.last_section_head.write().await;
            let head = std::mem::replace(&mut *last_section_head, String::new());
            let end = format!("</{}>", head);
            inject_sender.send(format!("{}{}", end, inject)).unwrap();
            return;
        }
        //普通内容
        inject_sender.send(inject).unwrap();
    }
    /// 核心方法：非阻塞地抽干(Drain)当前队列中的所有事件
    fn drain_events(&mut self) {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event.data {
                AgentEventData::Interrupt(_) => {
                    // 在 drain 阶段遇到打断信号可以直接忽略
                    // 因为我们本身就是在打断后的重新整理阶段
                    println!("[队列提取] 忽略多余的打断信号");
                }
                _ => {
                    self.event_buffer.push(event);
                }
            }
        }
    }
    pub async fn event_loop(&mut self) -> anyhow::Result<MindFlag> {
        let len = std::env::var("CTX_LEN_PER_ROUND")
            .map(|v| usize::from_str_radix(&v, 10).unwrap())
            .unwrap_or(50000);

        loop {
            let is_close = self.event_listening().await.unwrap();
            self.run_watcher.send(false)?;
            if is_close {
                return Ok(MindFlag::Closed);
            }
            //检查上下文长度是否超过配置
            if self.context.content.len() >= len {
                return Ok(MindFlag::Excess);
            }
        }
    }
    async fn event_listening(&mut self) -> anyhow::Result<bool> {
        // 1. 如果没有待处理任务，阻塞等待下一个事件到达
        if self.event_buffer.is_empty() {
            if let Some(agent_event) = self.event_receiver.recv().await {
                self.event_buffer.push(agent_event);
            } else {
                info!("通道已关闭，智能体即将退出");
                return Ok(true);
            }
        }
        self.run_watcher.send(true)?;

        let proxy = mpsc::unbounded_channel();
        // 3. 准备当前要处理的上下文
        // 提取出当前所有的任务，清空 pending 准备接收新的
        self.context.push_events(&mut self.event_buffer);
        info!("msg={}", self.context.messages.len());
        // 4. 创建生成流（模拟大模型输出或耗时任务）
        let mut chunk_stream = self.context.chunk_stream().await;
        // 3. 将 Rx 转换为 Stream 并与 LLM 流合并
        let mut inject_stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(proxy.1).boxed();
        let merged_stream = async_stream::stream! {
            loop {
                tokio::select! {
                    // 偏向于优先处理 chunk_stream
                    biased;
                    chunk_opt = chunk_stream.next() => {
                        match chunk_opt {
                            Some(val) => {
                                print!("{}",&val);
                                yield val
                            },
                            None => {
                                info!("end");
                                // 重点：chunk_stream 结束了，直接 break 退出循环，流也随之结束
                                break;
                            }
                        }
                    }
                    inject_opt = inject_stream.next() => {
                        if let Some(val) = inject_opt {
                            yield val;
                        }
                    }
                }
            }
        }
        .boxed();

        let mut section_stream = Box::pin(section_stream(merged_stream));
        let mut response = String::new();
        // 5. 边生成边监听队列
        loop {
            tokio::select! {
                // 中断注入
                Some(injected)=self.inject_receiver.recv() =>{
                    self.inject_section(&proxy.0, injected).await;
                }
                // 分支 A：段落生成流
                Some(section_event) = section_stream.next() =>  {
                    let is_end= matches!(&section_event,SectionEvent::End);
                    if let SectionEvent::Close(section)=&section_event{
                        let s:String=section.into();
                        response.push_str(&s);
                        response.push_str("\n");
                    }
                    self.section_sender.send(section_event).await.unwrap();
                    if is_end{
                        break ;
                    }
                }

                // 分支 B：在生成过程中，收到了新事件
                Some(event) = self.event_receiver.recv() => {
                    if event.force{
                        self.event_buffer.push(event);
                        self.section_sender.send(SectionEvent::End).await.unwrap();
                        break; // 跳出内部 loop,强制处理
                    }
                    // 如果是普通任务，先默默存起来，不打断当前生成
                    self.event_buffer.push(event)
                }
                else=>{
                    break ;
                }
            };
        }
        //放入agent的回复
        self.context.push_response(response.trim().to_string());
        // 2. 瞬间抽干队列中同时到达的其他任务（Batching）
        self.drain_events();
        Ok(false)
    }

    pub async fn reset(&mut self, agent_ctx: Arc<RwLock<AgentContext>>) -> anyhow::Result<()> {
        self.reorganize_watcher.send(true)?;
        //
        let mut ctx = agent_ctx.write().await;
        ctx.info.status = AgentStatus::Organizing;

        let item_groups = ctx.item_groups.clone();
        let (item_groups, title, summary) = self
            .context
            .reorganize_items(ctx.info.id, item_groups)
            .await?;
        ctx.item_groups = item_groups;
        //
        entity_kit::incr_agent_state_version(ctx.info.id, ctx.info.state_version, title, summary)
            .await?;
        ctx.info.state_version += 1;
        //重置
        let messages = message_kit::build_state_context(&ctx).await;
        self.context.reset(messages);
        //解锁
        self.reorganize_watcher.send(false)?;
        Ok(())
    }
}

pub async fn run_mind(
    agent_ctx: Arc<RwLock<AgentContext>>,
    engine: Arc<LLMEngine>,
    inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
    event_receiver: mpsc::Receiver<AgentEvent>,
    section_sender: mpsc::Sender<SectionEvent>,
    reorganize_watcher: watch::Sender<bool>,
    run_watcher: watch::Sender<bool>,
) {
    // 加载系统提示词和历史消息
    let messages = {
        let ctx = agent_ctx.read().await;
        message_kit::build_state_context(&ctx).await
    };
    let mut mind_stream = MindStream::new(
        engine,
        messages,
        inject_receiver,
        event_receiver,
        section_sender,
        reorganize_watcher,
        run_watcher,
    );
    loop {
        match mind_stream.event_loop().await.unwrap() {
            MindFlag::Closed => return,
            MindFlag::Excess => mind_stream
                .reset(agent_ctx.clone())
                .await
                .expect("重组上下文失败"),
        }
    }
}
