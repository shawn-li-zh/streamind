use std::{
    backtrace::Backtrace,
    collections::HashMap,
    mem::take,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Local};

use entity_service::DB;
use futures::StreamExt;
use kernel::{
    agent::context::AgentContext,
    mind::{
        context::EventContext,
        port::{CommandEvent, EndReason, GenerateEvent, MindOutlet},
        section::{Section, SectionTag},
        stream::StreamMind,
    },
    render::RenderContext,
    tool::{
        call::ToolCall,
        engine::{ToolEngine, ToolEngineContext},
        event::ToolCallDetailItem,
    },
};
use runtime::{
    agent::event::{AppEnvEvent, AppEnvEventData, SectionEvent},
    context::stack::{StackCommand, StackContext},
};
use serde_json::json;

use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::{
    agent::{app::AppAgent, common::WebSender},
    plugin::{heartbeat::HeartbeatRunner, reflect::Recall, scheduler::AgentSchedulerRunner},
    runtime::{memory::MemoryAgentRuntime, reflect_call, stack_call, tool_call},
    sse_event::{ServerEvent, WebEvent},
};

pub struct AppAgentRuntime {
    agent: Arc<AppAgent>,
    stream_mind: StreamMind<AppEnvEvent, StackContext<AppEnvEvent>>,
    mind_outlet: MindOutlet,
    heartbeat: HeartbeatRunner,
    scheduler_runner: AgentSchedulerRunner,
    gen_sender: watch::Sender<bool>,
    memory_agent_runtime: MemoryAgentRuntime,
}
impl AppAgentRuntime {
    pub fn new(
        agent: Arc<AppAgent>,
        stream_mind: StreamMind<AppEnvEvent, StackContext<AppEnvEvent>>,
        mind_outlet: MindOutlet,
        heartbeat: HeartbeatRunner,
        scheduler_runner: AgentSchedulerRunner,
        gen_sender: watch::Sender<bool>,
        memory_agent_runtime: MemoryAgentRuntime,
    ) -> Self {
        Self {
            agent,
            stream_mind,
            mind_outlet,
            heartbeat,
            scheduler_runner,
            gen_sender,
            memory_agent_runtime,
        }
    }
    pub fn start(mut self) {
        tokio::spawn(async move { self.stream_mind.event_loop().await });
        {
            let var = std::env::var("HEARTBEAT_INTERVAL").unwrap_or("10".to_string());
            let secs = var.parse::<u64>().unwrap();
            let interval = std::time::Duration::from_secs(secs);
            self.heartbeat.start(interval);
            //
            self.scheduler_runner.start();

            self.memory_agent_runtime.start();
        }
        {
            let agent = self.agent.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    agent
                        .web_sender
                        .send(WebEvent::Server(ServerEvent::Heartbeat));
                }
            });
        }
        //
        let mut generate_worker = GenerateWorker {
            agent_id: self.agent.identity().id(),
            agent: self.agent.clone(),
            start_time: Local::now(),
            last_tag_open: None,
            response_event_id: 0,
            response_sections: vec![],
            gen_sender: self.gen_sender,
            web_sender: self.agent.web_sender.clone(),
            tool_engine: self.agent.tool_engine.clone(),
        };

        tokio::spawn(async move {
            {
                let identity = self.agent.identity();
                let db = DB.get().unwrap();
                entity_service::agent::startup(db, identity.id)
                    .await
                    .unwrap();
            }
            while let Some(event) = self.mind_outlet.next().await {
                if let Err(e) = generate_worker.handle(event).await {
                    error!("GenerateWorker failed to handle event: {:?}", e);
                    //释放状态
                    generate_worker
                        .gen_sender
                        .send(false)
                        .expect("Failed to send generation status update");
                    generate_worker.agent.heartbeat.in_idle();
                }
            }
        });
    }
}

struct GenerateWorker {
    agent_id: u32,
    agent: Arc<AppAgent>,
    start_time: DateTime<Local>,
    last_tag_open: Option<(&'static SectionTag, String, Instant)>,
    response_event_id: i64,
    response_sections: Vec<Section>,
    web_sender: Arc<WebSender>,
    tool_engine: Arc<ToolEngine>,
    gen_sender: watch::Sender<bool>,
}
impl GenerateWorker {
    fn reset(&mut self) {
        self.start_time = Local::now();
        self.last_tag_open = None;
        self.response_event_id = 0;
        self.response_sections.clear();
    }
    async fn handle(&mut self, event: GenerateEvent) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        let agent_id = self.agent_id;
        match event {
            GenerateEvent::Start => {
                println!("start...");
                self.gen_sender.send(true)?;
                self.agent.heartbeat.in_progress();
                self.reset(); // 增加清理
                entity_service::event::processing(db, agent_id).await?;
                {
                    //
                    let event = AppEnvEventData::AgentResponse(vec![]);

                    let event_id = self.agent.send_event((event, false)).await?;
                    self.response_event_id = event_id;
                };
                let web_event = WebEvent::Section(SectionEvent::Start, self.response_event_id);
                self.web_sender.send(web_event);
            }
            GenerateEvent::Open(tag, header) => {
                self.last_tag_open = Some((tag, header, Instant::now()));

                let event_id = self.response_event_id;
                let web_event = WebEvent::Section(SectionEvent::Open(tag), event_id);
                self.web_sender.send(web_event);
            }
            GenerateEvent::Chunk(tag, chunk) => {
                let event_id = self.response_event_id;
                if matches!(tag.name, "reply" | "think") {
                    let web_event = WebEvent::Section(SectionEvent::Chunk(tag, chunk), event_id);
                    self.web_sender.send(web_event);
                }
            }
            GenerateEvent::Close(tag, content) => {
                let mut section = {
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
                        id: 0,
                        header,
                        name: tag.name.to_string(),
                        content,
                        elapsed,
                    }
                };
                //保存section
                {
                    let model = entity_service::section::save(
                        db,
                        agent_id,
                        self.response_event_id,
                        section.clone(),
                        serde_json::Value::Null,
                    )
                    .await?;
                    section.id = model.id;
                    self.response_sections.push(section.clone());
                };

                //发送到前端
                info!("SECTION:{}", &tag.name);
                let web_event =
                    WebEvent::Section(SectionEvent::Close(section), self.response_event_id);
                self.web_sender.send(web_event);
            }
            GenerateEvent::End(reason) => {
                //加入临时阻塞事件接收机制，防止期间有事件触发执行其他任务去了
                println!("call start...{:?}", &reason);
                let (mut events, continue_flag) = self.handle_end(reason).await;

                //处理响应
                {
                    let sections = take(&mut self.response_sections);
                    //保存到数据库
                    entity_service::event::update_agent_response(
                        db,
                        self.response_event_id,
                        sections.clone(),
                    )
                    .await?;

                    let event = {
                        let data = AppEnvEventData::AgentResponse(sections);
                        let size = data.size();
                        AppEnvEvent {
                            id: self.response_event_id,
                            data: data,
                            force: false,
                            time: self.start_time,
                            size,
                        }
                    };
                    let reflection = self.agent.reflect_engine.event_reflect(&event).await?;
                    self.agent
                        .inject_event_to_context(event, reflection)
                        .await?;
                }
                {
                    // 配置写错不应该 panic，退回默认值即可
                    let limit = std::env::var("CONTEXT_SIZE_LIMIT")
                        .ok()
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(1024 * 1024);
                    let size = self.agent.event_context().size();
                    if size > limit {
                        events.push((
                            AppEnvEventData::Interrupt("上下文溢出,请重编上下文".to_string()),
                            false,
                        ));
                    }
                }

                println!("call end...{}", events.len());
                for ele in events {
                    let event = self.agent.save_event(ele).await?;
                    let reflection = self.agent.reflect_engine.event_reflect(&event).await?;
                    self.agent
                        .inject_event_to_context(event, reflection)
                        .await?;
                }

                if continue_flag {
                    self.agent.send_cmd(CommandEvent::Generate).await?;
                } else {
                    //释放状态
                    self.gen_sender.send(false)?;
                    self.agent.heartbeat.in_idle();
                }
            }
            GenerateEvent::Error(e) => {
                error!("generate event error: {:?}", e);
                //释放状态
                self.gen_sender.send(false)?;
                self.agent.heartbeat.in_idle();
            }
        }
        Ok(())
    }
    async fn handle_end(&mut self, resaon: EndReason) -> (Vec<(AppEnvEventData, bool)>, bool) {
        let mut events: Vec<(AppEnvEventData, bool)> = vec![];
        let db = DB.get().unwrap();
        let agent_id = self.agent_id;
        {
            //修改事件状态为完成
            entity_service::event::done(db, agent_id).await.unwrap();
            let web_event: WebEvent = WebEvent::Section(SectionEvent::End, self.response_event_id);
            self.web_sender.send(web_event);
        }
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
        let mut stack_cmds: Vec<StackCommand> = vec![];
        let mut recalls: Vec<Recall> = vec![];
        {
            let mut counter: HashMap<&String, u32> = HashMap::new();
            for section in &self.response_sections {
                let value = counter.entry(&section.name).or_default();
                *value += 1;
                //
                match parse_section(
                    &self.tool_engine,
                    section,
                    &mut tool_call_infos,
                    &mut stack_cmds,
                    &mut recalls,
                )
                .await
                {
                    Ok(payload) => {
                        if let Err(e) =
                            entity_service::section::update_payload(db, section.id, payload).await
                        {
                            error!("update payload error:{}", e);
                        };
                    }
                    Err(e) => {
                        errors.push(format!("第{}个<{}>解析失败:{}", value, section.name, e));
                    }
                }
            }
        }

        if !errors.is_empty() {
            //发送事件处理
            events.push((AppEnvEventData::Error(errors.join("\n")), false));
            return (events, true);
        }
        let mut continue_flag = events.len() > 0;
        //函数调用
        if !tool_call_infos.is_empty() {
            let res = tool_call::invoke(
                self.response_event_id,
                tool_call_infos,
                &self.tool_engine,
                self.web_sender.clone(),
                &mut events,
            )
            .await;
            if let Err(e) = res {
                let bt = Backtrace::capture();
                eprintln!("{:?}", bt);
                //发送事件处理
                events.push((AppEnvEventData::Error(e.to_string()), false));
            }
            continue_flag = events.len() > 0;
        }
        //处理压栈
        if !stack_cmds.is_empty() {
            continue_flag = true;
            if let Err(e) = stack_call::handle_stack_commands(self.agent.clone(), stack_cmds).await
            {
                events.push((AppEnvEventData::Error(e.to_string()), false));
            };
        }
        //处理经验/记忆召回
        if !recalls.is_empty() {
            continue_flag = true;
            let state_id = self
                .agent
                .event_context()
                .top_node()
                .map(|v| v.id)
                .unwrap_or(1);
            if let Err(e) =
                reflect_call::invoke(recalls, &self.agent.reflect_engine, &mut events, state_id)
                    .await
            {
                events.push((AppEnvEventData::Error(e.to_string()), false));
            };
        }
        (events, continue_flag)
    }
}

async fn parse_section(
    tool_engine: &ToolEngine,
    section: &Section,
    tool_call_infos: &mut Vec<(ToolCallDetailItem, ToolCall)>,
    stack_cmds: &mut Vec<StackCommand>,
    recalls: &mut Vec<Recall>,
) -> anyhow::Result<serde_json::Value> {
    let content = section.content.trim();
    let payload = match section.name.as_str() {
        "render" => {
            let mut ctx = RenderContext::from_xml(content)?;
            let comp = entity_service::component::get_by_name(&ctx.name)
                .await?
                .ok_or_else(|| anyhow::anyhow!("组件不存在"))?;
            ctx.path = Some(comp.path);
            serde_json::to_value(ctx)?
        }
        "tool_call" => {
            let (mut detail, mut call) =
                tool_call::parse(&section.header, &section.content, tool_engine)?;
            detail.id = section.id;
            call.id = section.id;
            //
            let p = json!(&detail);
            tool_call_infos.push((detail, call));
            p
        }
        "recall" => {
            let recall = reflect_call::parse(&section.header, &section.content)?;
            //
            let p = json!(&recall);
            recalls.push(recall);
            p
        }
        "push" => {
            let mut push = stack_call::parse_push(&section.header, content)?;
            push.id = section.id;
            let p = json!(&push);
            stack_cmds.push(StackCommand::Push(push));
            p
        }
        "reset" => {
            stack_cmds.push(StackCommand::Reset(content.to_string()));
            serde_json::Value::Null
        }
        "pop" => {
            stack_cmds.push(StackCommand::Pop(content.to_string()));
            serde_json::Value::Null
        }
        _ => serde_json::Value::Null,
    };
    Ok(payload)
}
