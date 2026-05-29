use std::{
    backtrace::Backtrace,
    collections::HashMap,
    mem::take,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context as _;
use chrono::{DateTime, Local};

use entity_service::DB;
use futures::{FutureExt, StreamExt};
use kernel::{
    agent::{context::AgentContext, AgentOutfit},
    mind::{
        context::EventContext,
        port::{CommandEvent, EndReason, GenerateEvent, MindOutlet},
        section::{Section, SectionTag},
    },
    parser,
    render::RenderContext,
    tool::{
        call::ToolCall,
        core::ToolReturnValue,
        engine::{ToolEngine, ToolEngineContext},
        event::{ToolCallDetailItem, ToolCallEvent, ToolCallStatus},
    },
};
use runtime::agent::{
    event::{AppEnvEvent, AppEnvEventData, SectionEvent},
    event_context::{Node, StackCommand},
    instance::EventLoop,
};
use serde_json::json;

use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::{
    agent::{AppAgent, WebSender},
    plugin::{heartbeat::HeartbeatRunner, scheduler::AgentSchedulerRunner},
    sse_event::{ServerEvent, WebEvent},
};

pub struct AgentRuntime {
    agent: Arc<AppAgent>,
    mind_outlet: MindOutlet,
    event_loop: EventLoop,
    heartbeat: HeartbeatRunner,
    scheduler_runner: AgentSchedulerRunner,
    gen_sender: watch::Sender<bool>,
}
impl AgentRuntime {
    pub fn new(
        agent: Arc<AppAgent>,
        event_loop: EventLoop,
        mind_outlet: MindOutlet,
        heartbeat: HeartbeatRunner,
        scheduler_runner: AgentSchedulerRunner,
        gen_sender: watch::Sender<bool>,
    ) -> Self {
        Self {
            agent,
            event_loop,
            mind_outlet,
            heartbeat,
            scheduler_runner,
            gen_sender,
        }
    }
    pub fn start(mut self) {
        {
            let var = std::env::var("HEARTBEAT_INTERVAL").unwrap_or("10".to_string());
            let secs = var.parse::<u64>().unwrap();
            let interval = std::time::Duration::from_secs(secs);
            self.heartbeat.start(interval);
            //
            self.scheduler_runner.start();
            self.event_loop.start();
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
        let ctx: Arc<AgentContext> = self.agent.context();
        let tool_engine_context = ctx.get::<ToolEngineContext>().unwrap();
        let mut generate_worker = GenerateWorker {
            agent_id: self.agent.identity().id(),
            agent: self.agent.clone(),
            start_time: Local::now(),
            last_tag_open: None,
            // response_event: None,
            response_event_id: 0,
            response_sections: vec![],
            gen_sender: self.gen_sender,
            web_sender: self.agent.web_sender.clone(),

            tool_engine: Arc::new(ToolEngine::new(
                self.agent.identity(),
                self.agent.context(),
                tool_engine_context,
            )),
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
                    // 可选：通知前端或改变 Agent 状态为 Error
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
                            return Ok(()); // 或者生成一个默认的占位符
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
                println!("call start...");
                let mut events = self.handle_end(reason).await;

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
                    let reflection = self.agent.cond_reflection.event_reflect(&event).await?;
                    self.agent
                        .inject_event_to_context(event, reflection)
                        .await?;
                }
                {
                    let limit = std::env::var("CONTEXT_SIZE_LIMIT")
                        .map(|v| usize::from_str_radix(&v, 10).unwrap())
                        .unwrap_or(1024 * 1024);
                    let size = self.agent.event_context().size();
                    if size > limit {
                        events.push((
                            AppEnvEventData::Interrupt("上下文溢出,请重编上下文".to_string()),
                            false,
                        ));
                    }
                }
                let has_events = events.len() > 0;
                println!("call end...{}", events.len());
                for ele in events {
                    let event = self.agent.save_event(ele).await?;
                    let reflection = self.agent.cond_reflection.event_reflect(&event).await?;
                    self.agent
                        .inject_event_to_context(event, reflection)
                        .await?;
                }
                self.agent.cond_reflection.refresh_triggered_rules();
                if has_events {
                    self.agent.send_cmd(CommandEvent::Generate).await?;
                } else {
                    //释放状态
                    self.agent.heartbeat.reset();
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
    async fn handle_end(&mut self, resaon: EndReason) -> Vec<(AppEnvEventData, bool)> {
        let mut events: Vec<(AppEnvEventData, bool)> = vec![];
        let db = DB.get().unwrap();
        let agent_id = self.agent_id;
        {
            //修改事件状态为完成
            entity_service::event::done(db, agent_id).await.unwrap();
            let web_event: WebEvent = WebEvent::Section(SectionEvent::End, self.response_event_id);
            self.web_sender.send(web_event);
        }
        //解析响应的section
        let mut errors = vec![];
        let mut tool_call_infos: Vec<(ToolCallDetailItem, ToolCall)> = vec![];
        let mut stack_cmds: Vec<StackCommand> = vec![];
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
            return events;
        }
        //非正常结束则不处理任何功能调用
        match resaon {
            EndReason::Error | EndReason::Canceled => {
                return events;
            }
            _ => {}
        }

        //函数调用
        if !tool_call_infos.is_empty() {
            if let Err(e) = tool_call(
                self.response_event_id,
                tool_call_infos,
                &self.tool_engine,
                self.web_sender.clone(),
                &mut events,
            )
            .await
            {
                let bt = Backtrace::capture();
                eprintln!("{:?}", bt);
                //发送事件处理
                events.push((AppEnvEventData::Error(e.to_string()), false));
            }
        }
        //处理压栈
        if !stack_cmds.is_empty() {
            if let Err(e) = handle_stack_commands(self.agent.clone(), stack_cmds).await {
                events.push((AppEnvEventData::Error(e.to_string()), false));
            };
        }
        events
    }
}

async fn parse_section(
    tool_engine: &ToolEngine,
    section: &Section,
    tool_call_infos: &mut Vec<(ToolCallDetailItem, ToolCall)>,
    stack_cmds: &mut Vec<StackCommand>,
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
            let inner = section
                .header
                .trim_start_matches("<tool_call")
                .trim_end_matches(">")
                .trim();
            let mut attrs = parser::parse_attrs(inner);
            let call_id = match attrs
                .remove("id")
                .map(|v| v.as_str().unwrap_or_default().to_string())
            {
                Some(v) => v,
                None => {
                    anyhow::bail!("tool_call id is required")
                }
            };
            let mut call = tool_engine.preprocess(call_id.clone(), content)?;
            call.id = section.id;
            let detail = ToolCallDetailItem {
                id: section.id,
                call_id,
                package: call.package.name.to_string(),
                tool: call.tool.name.to_string(),
                values: vec![],
                status: ToolCallStatus::Undo,
                args: call.args.clone(),
                elapsed: "".to_string(),
            };

            let p = json!(&detail);
            tool_call_infos.push((detail, call));
            p
        }
        "push" => {
            let mut push = parse_push(&section.header, content)?;
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
fn parse_push(header: &str, content: &str) -> anyhow::Result<Node> {
    let inner = header
        .trim_start_matches("<push")
        .trim_end_matches(">")
        .trim();
    let mut attrs = parser::parse_attrs(inner);
    let title = attrs
        .remove("title")
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .ok_or_else(|| anyhow::anyhow!("push标签缺少title属性"))?;
    Ok(Node {
        id: 0,
        pos: 0,
        title: title.trim().to_string(),
        content: content.trim().to_string(),
    })
}

async fn tool_call(
    response_event_id: i64,
    tool_call_infos: Vec<(ToolCallDetailItem, ToolCall)>,
    tool_engine: &ToolEngine,
    web_sender: Arc<WebSender>,
    events: &mut Vec<(AppEnvEventData, bool)>,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
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

    //批执行
    let mut stream = tool_engine.batch_call(tool_calls).await.boxed();
    while let Some(event) = stream.next().await {
        //通知前端
        web_sender.send(WebEvent::ToolCall(event.clone(), response_event_id));
        let item = match event {
            ToolCallEvent::Success(item) => item,
            ToolCallEvent::Failure(item) => item,
            _ => {
                continue;
            }
        };
        let detial = tool_call_details.get_mut(&item.id).unwrap();
        detial.values = item.values;
        detial.status = item.status;
        detial.elapsed = item.elapsed;
        //保存
        entity_service::section::update_payload(db, detial.id, detial.clone())
            .await
            .unwrap();
        let is_empty = detial
            .values
            .iter()
            .filter(|v| {
                if let ToolReturnValue::Ignore = v {
                    return false;
                }
                return true;
            })
            .count()
            == 0;
        if is_empty {
            continue;
        }
        //发送完整事件
        events.push((AppEnvEventData::ToolResponse(detial.clone()), false));
    }
    //结束
    Ok(())
}

async fn handle_stack_commands(
    agent: Arc<AppAgent>,
    stack_cmds: Vec<StackCommand>,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    let agent_id = agent.identity().id();
    let stack = agent.event_context();
    for cmd in stack_cmds {
        match cmd {
            StackCommand::Push(mut node) => {
                //设置节点的位置
                {
                    let pos = stack.top_node().map(|v| v.pos).unwrap_or_default();
                    node.pos = pos + 1;
                }
                let model = entity_service::state::push(
                    db,
                    agent_id,
                    node.pos as _,
                    node.title.clone(),
                    node.content.clone(),
                )
                .await?;
                node.id = model.id;
                stack.push_node(node);
                println!("push node to stack");
            }
            StackCommand::Reset(content) => {
                if let Some(top_node) = stack.top_node() {
                    let node_id = top_node.id;
                    let c = content.clone();
                    entity_service::transaction(|db| {
                        async move {
                            entity_service::event::unload_events(db, node_id).await?;
                            entity_service::state::reset(db, node_id, c).await?;
                            Ok(())
                        }
                        .boxed()
                    })
                    .await?;
                    top_node.reset(content);
                } else {
                    anyhow::bail!("栈顶不存在")
                }
                 println!("reset node");
            }
            StackCommand::Pop(result) => {
                if stack.node_len() <= 1 {
                    anyhow::bail!("核心栈状态不能弹出")
                }

                let last_top_node = stack.pop_node().unwrap();
                let title = &last_top_node.title;
                let node_id = last_top_node.id;
                entity_service::transaction(|db| {
                    async move {
                        entity_service::event::unload_events(db, node_id).await?;
                        entity_service::state::dump(db, node_id).await?;
                        Ok(())
                    }
                    .boxed()
                })
                .await?;

                if let Some(top_node) = stack.top_node() {
                    let mut content = top_node.content.write();
                    let new = format!(
                        "{}\n子节点[{}]周期结束,返回结果:\n{}",
                        content, title, result
                    );
                    *content = new;
                } else {
                    anyhow::bail!("栈顶不存在")
                }
                println!("pop node from stack");
            }
        }
    }
    Ok(())
}
