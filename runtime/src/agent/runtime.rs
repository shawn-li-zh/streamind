use std::{backtrace::Backtrace, collections::HashMap, sync::Arc, time::Instant};

use anyhow::Context as _;

use entity::event::{EnvEventStatus, EnvEventType};
use entity_service::DB;
// use sea_orm::{DbErr, TransactionTrait};
use kernel::{
    agent::context::{AgentContext, AgentStatus},
    event::{CallItem, EnvEvent, SectionEvent, ToolCallDetailItem, ToolCallEvent, ToolCallStatus},
    render::RenderContext,
    section::{Section, SectionTag},
    tool::{call::ToolCall, core::ToolReturnValue, engine::ToolEngine},
};

use serde_json::json;
use tokio::sync::{RwLock, mpsc, watch};
use tracing::info;

use crate::{
    agent::channel::AgentChannel,
    context::{
        mind::{self, StreamMind},
        section,
        stack::{self, MindStack, Push},
    },
    sse_event::WebEvent,
};

pub struct AgentRuntime {
    // pub agent: Arc<RwLock<AgentInstance>>,
    context: Arc<AgentContext>,
    channel: Arc<AgentChannel>,
    stack: Arc<RwLock<MindStack>>,
    tool_engine: ToolEngine,
    section_receiver: mpsc::Receiver<mind::Event>,
    call_watcher: watch::Sender<bool>,
}

impl AgentRuntime {
    pub(crate) fn new(
        context: Arc<AgentContext>,
        channel: Arc<AgentChannel>,
        stack: Arc<RwLock<MindStack>>,
        tool_engine: ToolEngine,
        section_receiver: mpsc::Receiver<mind::Event>,
        call_watcher: watch::Sender<bool>,
    ) -> Self {
        Self {
            context,
            channel,
            stack,
            tool_engine,
            section_receiver,
            call_watcher,
        }
    }
    pub fn start(mut self, mind: StreamMind) {
        //运行输出流
        let ch = self.channel.clone();
        tokio::spawn(async move {
            mind::run_mind(mind, ch).await;
        });
        //运行时
        tokio::spawn(async move {
            entity_service::agent::startup(DB.get().unwrap(), self.context.agent_id)
                .await
                .unwrap();
            info!("智能体【{}】运行时已启动", self.context.agent_code);
            self.react_loop().await.unwrap()
        });
    }
    async fn react_loop(&mut self) -> anyhow::Result<()> {
        loop {
            {
                let mut info = self.context.info.write().await;
                info.status = AgentStatus::Idle;
            }
            let mut worker = ReactWorker {
                stack: self.stack.clone(),
                response_event_id: 0,
                response_sections: vec![],
            };
            let res = self.react(&mut worker).await;
            worker.end().await.expect("end failed");
            if let Err(e) = res {
                self.channel
                    .send_event(EnvEvent::Error(format!("{}", e)), true)
                    .await
                    .expect("send_event failed");
            };
        }
    }

    async fn react(&mut self, worker: &mut ReactWorker) -> anyhow::Result<()> {
        let ctx: Arc<AgentContext> = self.context.clone();
        let (agent_id, state_id) = {
            let info = ctx.info.read().await;
            (info.id, info.state_id)
        };
        let db = DB.get().unwrap();
        let mut tool_call_infos: Vec<(ToolCallDetailItem, ToolCall)> = vec![];
        let mut pushs: Vec<Push> = vec![];
        //
        let mut start_time = Instant::now();
        while let Some(section_event) = self.section_receiver.recv().await {
            match section_event {
                mind::Event::Start(event_ids) => {
                    entity_service::event::update_event_status(
                        db,
                        event_ids,
                        EnvEventStatus::Processing,
                    )
                    .await
                    .unwrap();
                    {
                        let mut info = ctx.info.write().await;
                        info.status = AgentStatus::Processing;
                        //
                        //保存一个空事件
                        let (event, model) = entity_service::event::save(
                            db,
                            agent_id,
                            state_id,
                            EnvEvent::AgentResponse(vec![]),
                            false,
                        )
                        .await
                        .expect("保存agent回复失败");
                        worker.response_event_id = model.id;
                    };
                    let web_event =
                        WebEvent::Section(SectionEvent::Start, worker.response_event_id);
                    self.channel.send_web_event(web_event).await;
                }
                mind::Event::Open(tag) => {
                    start_time = Instant::now();
                    let event_id = worker.response_event_id;
                    let web_event = WebEvent::Section(SectionEvent::Open(tag), event_id);
                    self.channel.send_web_event(web_event).await;
                }
                mind::Event::Chunk(tag, chunk) => {
                    let event_id = worker.response_event_id;
                    // section::Event
                    match tag.name {
                        "reply" | "think" => {
                            let web_event =
                                WebEvent::Section(SectionEvent::Chunk(tag, chunk), event_id);
                            self.channel.send_web_event(web_event).await;
                        }
                        _ => {
                            continue;
                        }
                    }
                }
                mind::Event::Close(tag, content) => {
                    let (mut section, payload) = parse_close_content(
                        &self,
                        tag,
                        content,
                        start_time,
                        &mut tool_call_infos,
                        &mut pushs,
                    )?;
                    //保存section
                    {
                        let model = entity_service::section::save(
                            db,
                            agent_id,
                            worker.response_event_id,
                            section.clone(),
                            payload,
                        )
                        .await?;
                        section.id = model.id;
                        worker.response_sections.push(section.clone());
                    };
                    if let Some((detail, call)) = tool_call_infos.last_mut() {
                        if detail.id == 0 {
                            call.id = section.id;
                            detail.id = call.id;
                        }
                    }
                    if let Some(push) = pushs.last_mut() {
                        if push.id == 0 {
                            push.id = section.id;
                        }
                    }
                    //发送到前端
                    info!("SECTION:{}", &tag.name);
                    let web_event =
                        WebEvent::Section(SectionEvent::Close(section), worker.response_event_id);
                    self.channel.send_web_event(web_event).await;
                }
                mind::Event::End(event_ids) => {
                    entity_service::event::update_event_status(db, event_ids, EnvEventStatus::Done)
                        .await
                        .unwrap();
                    let web_event = WebEvent::Section(SectionEvent::End, worker.response_event_id);
                    self.channel.send_web_event(web_event).await;
                    //没有调用的话，则等待外部事件触发循环
                    if tool_call_infos.is_empty() {
                        return Ok(());
                    }
                    //加入临时阻塞事件接收机制，防止期间有事件触发
                    self.call_watcher.send(true).unwrap();
                    //函数调用
                    if !tool_call_infos.is_empty() {
                        if let Err(e) = tool_call(
                            worker.response_event_id,
                            tool_call_infos,
                            self.channel.clone(),
                            &self.tool_engine,
                        )
                        .await
                        {
                            let bt = Backtrace::capture();
                            eprintln!("{:?}", bt);
                            //发送事件处理
                            self.channel
                                .send_event_unblock(EnvEvent::Error(e.to_string()), true)
                                .await
                                .expect("send tool call result error");
                        }
                    }
                    //处理压栈
                    if !pushs.is_empty() {
                        push_state(agent_id, self.stack.clone(), pushs).await?;
                    }
                    //解锁
                    self.call_watcher.clone().send(false).unwrap();
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

struct ReactWorker {
    stack: Arc<RwLock<MindStack>>,
    response_event_id: i64,
    response_sections: Vec<Section>,
}
impl ReactWorker {
    async fn end(self) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        if self.response_event_id > 0 {
            entity_service::event::update_agent_response(
                db,
                self.response_event_id,
                EnvEventType::AgentResponse,
                self.response_sections.clone(),
            )
            .await?;
            let mut stack = self.stack.write().await;
            stack.push_response(self.response_sections);
        }
        info!("step end");
        Ok(())
    }
}

fn parse_close_content(
    rt: &AgentRuntime,
    tag: &'static SectionTag,
    content: String,
    start_time: Instant,
    tool_call_infos: &mut Vec<(ToolCallDetailItem, ToolCall)>,
    pushs: &mut Vec<Push>,
) -> anyhow::Result<(Section, serde_json::Value)> {
    let payload = match tag.name {
        "render" => {
            let ctx: RenderContext = serde_json::from_str(content.trim())?;
            serde_json::to_value(ctx)?
        }
        "tool_call" => {
            let call = rt.tool_engine.preprocess(content.trim())?;
            let detail = ToolCallDetailItem {
                id: 0,
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
            let push = parse_push(content.trim())?;
            let p = json!(&push);
            pushs.push(push);
            p
        }
        _ => serde_json::Value::Null,
    };

    let elapsed = start_time.elapsed();
    let elapsed = humantime::format_duration(elapsed).to_string();
    Ok((
        Section {
            id: 0,
            name: tag.name.to_string(),
            content: content,
            elapsed: elapsed,
        },
        payload,
    ))
}
fn parse_push(content: &str) -> anyhow::Result<Push> {
    let (head, content) = content
        .split_once("\n")
        .with_context(|| anyhow::anyhow!("栈格式异常:{content}"))?;
    let (pos, title) = head
        .split_once(":")
        .with_context(|| anyhow::anyhow!("栈头格式异常:{head}"))?;
    let pos =
        usize::from_str_radix(pos.trim(), 10).map_err(|e| anyhow::anyhow!("栈指针格式异常"))?;
    Ok(Push {
        id: 0,
        pos,
        title: title.trim().to_string(),
        content: content.trim().to_string(),
    })
}

async fn tool_call(
    response_event_id: i64,
    tool_call_infos: Vec<(ToolCallDetailItem, ToolCall)>,
    channel: Arc<AgentChannel>,
    tool_engine: &ToolEngine,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    //
    let mut tool_call_details: HashMap<i64, ToolCallDetailItem> = HashMap::new();
    let mut tool_calls = vec![];
    let mut items = vec![];
    let mut call_ids = vec![];
    for (detail, call) in tool_call_infos {
        //
        tool_call_details.insert(detail.id, detail);
        call_ids.push(call.id);
        //
        let item = CallItem {
            id: call.id,
            package: call.package.name,
            tool: call.tool.name,
            args: call.args.clone(),
        };
        items.push(item);
        tool_calls.push(call);
    }
    let (sender, mut receiver) = mpsc::channel::<ToolCallEvent>(tool_calls.len());

    let ch = channel.clone();
    tokio::spawn(async move {
        //异步处理函数调用结果
        while let Some(event) = receiver.recv().await {
            ch.send_web_event(WebEvent::ToolCall(event.clone(), response_event_id))
                .await;

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
        }
        //排序
        let details: Vec<ToolCallDetailItem> = call_ids
            .iter()
            .map(|id| tool_call_details.remove(id).unwrap())
            .filter(|d| {
                d.values
                    .iter()
                    .filter(|v| {
                        if let ToolReturnValue::Ignore = v {
                            return false;
                        }
                        return true;
                    })
                    .count()
                    != 0
            })
            .collect();
        if details.is_empty() {
            //没有有效的工具调用结果
            return;
        }
        //发送完整事件
        ch.send_event(EnvEvent::ToolCallResult(details), true)
            .await
            .unwrap();
    });
    //通知前端 工具调用开始执行
    channel
        .send_web_event(WebEvent::ToolCall(
            ToolCallEvent::Start(items),
            response_event_id,
        ))
        .await;
    //批执行
    tool_engine.batch_call(tool_calls, sender).await?;
    //结束
    channel
        .send_web_event(WebEvent::ToolCall(ToolCallEvent::End, response_event_id))
        .await;
    Ok(())
}

async fn push_state(
    agent_id: u32,
    stack: Arc<RwLock<MindStack>>,
    pushs: Vec<Push>,
) -> anyhow::Result<()> {
    for push in pushs {
        let stack = stack.clone();
        entity_service::transaction::<_, _>(move |txn| {
            Box::pin(async move {
                let model = entity_service::state::push(
                    txn,
                    agent_id,
                    push.pos as _,
                    push.title.clone(),
                    push.content.clone(),
                )
                .await?;
                let mut stack = stack.write().await;
                stack.push(push);
                Ok(())
            })
        })
        .await?;
    }
    Ok(())
}
