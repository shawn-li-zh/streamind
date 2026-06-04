use std::{backtrace::Backtrace, collections::HashMap, mem::take, sync::Arc, time::Instant};

use chrono::{DateTime, Local};

use futures::StreamExt;
use kernel::{
    agent::context::AgentContext,
    mind::{
        context::EventContext,
        port::{CommandEvent, EndReason, GenerateEvent, MindOutlet},
        section::{Section, SectionTag},
        stream::StreamMind,
    },
    tool::{
        call::ToolCall,
        engine::{ToolEngine, ToolEngineContext},
        event::{ToolCallDetailItem, ToolCallEvent},
    },
};
use runtime::{
    agent::event::{AppEnvEvent, AppEnvEventData},
    context::linear::LinearContext,
};
use serde_json::json;

use tokio::sync::watch;
use tracing::{error, warn};

use crate::{
    agent::memory::{MemoryAgent, MemoryMessage},
    plugin::memory::MemoryTaskRunner,
    runtime::tool_call,
};

pub struct MemoryAgentRuntime {
    agent: Arc<MemoryAgent>,
    mind: StreamMind<MemoryMessage, LinearContext<MemoryMessage>>,
    mind_outlet: MindOutlet,
    gen_sender: watch::Sender<bool>,
    memory_task: MemoryTaskRunner,
}
impl MemoryAgentRuntime {
    pub fn new(
        agent: Arc<MemoryAgent>,
        mind: StreamMind<MemoryMessage, LinearContext<MemoryMessage>>,
        mind_outlet: MindOutlet,
        gen_sender: watch::Sender<bool>,
        memory_task: MemoryTaskRunner,
    ) -> Self {
        Self {
            agent,
            mind,
            mind_outlet,
            gen_sender,
            memory_task,
        }
    }
    pub fn start(mut self) {
        {
            tokio::spawn(async move { self.mind.event_loop().await });

            tokio::spawn(async move {
                if let Err(e) = self.memory_task.start().await {
                    error!("MemoryTaskRunner: {:?}", e);
                }
            });
        }

        //
        let ctx: Arc<AgentContext> = self.agent.context.clone();
        let tool_engine_context = ctx.get::<ToolEngineContext>().unwrap();
        let mut generate_worker = GenerateWorker {
            section_id: 0,
            agent: self.agent.clone(),
            start_time: Local::now(),
            last_tag_open: None,
            response_sections: vec![],
            gen_sender: self.gen_sender,

            tool_engine: Arc::new(ToolEngine::new(
                self.agent.identity.clone(),
                self.agent.context.clone(),
                tool_engine_context,
            )),
        };

        tokio::spawn(async move {
            while let Some(event) = self.mind_outlet.next().await {
                if let Err(e) = generate_worker.handle(event).await {
                    error!("GenerateWorker failed to handle event: {:?}", e);
                }
            }
        });
    }
}

struct GenerateWorker {
    section_id: i64,
    agent: Arc<MemoryAgent>,
    start_time: DateTime<Local>,
    last_tag_open: Option<(&'static SectionTag, String, Instant)>,
    response_sections: Vec<Section>,
    tool_engine: Arc<ToolEngine>,
    gen_sender: watch::Sender<bool>,
}
impl GenerateWorker {
    fn reset(&mut self) {
        self.start_time = Local::now();
        self.last_tag_open = None;
        self.response_sections.clear();
    }
    async fn handle(&mut self, event: GenerateEvent) -> anyhow::Result<()> {
        match event {
            GenerateEvent::Start => {
                println!("start...");
                self.gen_sender.send(true)?;
                self.reset(); // 增加清理
            }
            GenerateEvent::Open(tag, header) => {
                self.last_tag_open = Some((tag, header, Instant::now()));
            }
            GenerateEvent::Chunk(_tag, _chunk) => {}
            GenerateEvent::Close(tag, content) => {
                let section = {
                    let (tag, header, open_time) = match self.last_tag_open.take() {
                        Some(data) => data,
                        None => {
                            warn!("Received Close event without Open for tag: {}", tag.name);
                            return Ok(());
                        }
                    };
                    let elapsed = open_time.elapsed();
                    let elapsed = humantime::format_duration(elapsed).to_string();
                    Section {
                        id: self.section_id,
                        header,
                        name: tag.name.to_string(),
                        content,
                        elapsed,
                    }
                };
                self.section_id += 1;
                self.response_sections.push(section.clone());
            }
            GenerateEvent::End(reason) => {
                //加入临时阻塞事件接收机制，防止期间有事件触发执行其他任务去了
                println!("call start...{:?}", &reason);
                let (events, continue_flag) = self.handle_end(reason).await;
                println!("call end...{}", events.len());
                //处理响应
                let sections = take(&mut self.response_sections);
                self.agent.push_message(sections).await?;
                for ele in events {
                    let size = ele.size();
                    let app_event = AppEnvEvent {
                        id: 0,
                        data: ele,
                        force: false,
                        size: size,
                        time: Local::now(),
                    };
                    self.agent.push_message(app_event).await?;
                }
                if continue_flag {
                    self.agent.send_cmd(CommandEvent::Generate).await?;
                } else {
                    //释放状态
                    self.gen_sender.send(false)?;
                }
            }
            GenerateEvent::Error(e) => {
                error!("generate event error: {:?}", e);
                self.gen_sender.send(false)?;
            }
        }
        Ok(())
    }
    async fn handle_end(&mut self, resaon: EndReason) -> (Vec<AppEnvEventData>, bool) {
        let mut events: Vec<AppEnvEventData> = vec![];

        //非正常结束则不处理任何功能调用
        match resaon {
            EndReason::Error(e) => {
                error!("{}", e);
                return (events, false);
            }
            EndReason::Canceled => {
                return (events, false);
            }
            _ => {}
        }
        //解析响应的section
        let mut errors = vec![];
        let mut tool_call_infos: Vec<(ToolCallDetailItem, ToolCall)> = vec![];
        {
            let mut counter: HashMap<&String, u32> = HashMap::new();
            for section in &self.response_sections {
                let value = counter.entry(&section.name).or_default();
                *value += 1;
                //
                match parse_section(&self.tool_engine, section, &mut tool_call_infos).await {
                    Ok(_payload) => {}
                    Err(e) => {
                        errors.push(format!("第{}个<{}>解析失败:{}", value, section.name, e));
                    }
                }
            }
        }

        if !errors.is_empty() {
            //发送事件处理
            events.push(AppEnvEventData::Error(errors.join("\n")));
            return (events, true);
        }
        let mut continue_flag = events.len() > 0;
        //函数调用
        if !tool_call_infos.is_empty() {
            let res = tool_invoke(tool_call_infos, &self.tool_engine).await;
            match res {
                Ok(details) => {
                    for ele in details {
                        events.push(AppEnvEventData::ToolResponse(ele));
                    }
                }
                Err(e) => {
                    let bt = Backtrace::capture();
                    eprintln!("{:?}", bt);
                    //发送事件处理
                    events.push(AppEnvEventData::Error(e.to_string()));
                }
            }
            continue_flag = events.len() > 0;
        }

        (events, continue_flag)
    }
}

async fn parse_section(
    tool_engine: &ToolEngine,
    section: &Section,
    tool_call_infos: &mut Vec<(ToolCallDetailItem, ToolCall)>,
) -> anyhow::Result<serde_json::Value> {
    let content = section.content.trim();
    let payload = match section.name.as_str() {
        "tool_call" => {
            let (mut detail, mut call) = tool_call::parse(&section.header, &content, tool_engine)?;
            detail.id = section.id;
            call.id = section.id;
            //
            let p = json!(&detail);
            tool_call_infos.push((detail, call));
            p
        }

        _ => serde_json::Value::Null,
    };
    Ok(payload)
}

pub async fn tool_invoke(
    tool_call_infos: Vec<(ToolCallDetailItem, ToolCall)>,
    tool_engine: &ToolEngine,
) -> anyhow::Result<Vec<ToolCallDetailItem>> {
    //
    let mut tool_call_details: HashMap<i64, ToolCallDetailItem> = HashMap::new();
    let mut tool_calls = vec![];

    let mut call_ids = vec![];
    for (detail, call) in tool_call_infos {
        //
        tool_call_details.insert(detail.id, detail);
        call_ids.push(call.id);
        tool_calls.push(call);
    }
    let mut details = vec![];
    //批执行
    let mut stream = tool_engine.batch_call(tool_calls).await.boxed();
    while let Some(event) = stream.next().await {
        let item = match event {
            ToolCallEvent::Success(item) => item,
            ToolCallEvent::Failure(item) => item,
            _ => {
                continue;
            }
        };
        let mut detial: ToolCallDetailItem = tool_call_details.remove(&item.id).unwrap();
        detial.values = item.values;
        detial.status = item.status;
        detial.elapsed = item.elapsed;
        //保存
        let is_empty = detial.values.iter().filter(|v| !v.is_empty()).count() == 0;
        if is_empty {
            continue;
        }
        //发送完整事件
        details.push(detial);
    }
    //结束
    Ok(details)
}
