use std::{backtrace::Backtrace, collections::HashMap, sync::Arc};

use entity::event::EnvEventType;
use entity_service::DB;
// use sea_orm::{DbErr, TransactionTrait};
use kernel::{
    agent::{AgentContext, AgentStatus},
    event::{CallItem, EnvEvent, SectionEvent, ToolCallDetailItem, ToolCallEvent, ToolCallStatus},
    render::RenderContext,
    section::Section,
    tool::{call::ToolCall, engine::ToolEngine},
};

use serde_json::json;
use tokio::sync::{mpsc, watch};
use tracing::info;

use crate::{
    agent::channel::AgentChannel,
    context::stream::{self, StreamMind},
    sse_event::WebEvent,
};

pub struct AgentRuntime {
    // pub agent: Arc<RwLock<AgentInstance>>,
    context: Arc<AgentContext>,
    channel: Arc<AgentChannel>,
    tool_engine: ToolEngine,
    section_receiver: mpsc::Receiver<SectionEvent>,
    call_watcher: watch::Sender<bool>,
}

impl AgentRuntime {
    pub(crate) fn new(
        context: Arc<AgentContext>,
        channel: Arc<AgentChannel>,
        tool_engine: ToolEngine,
        section_receiver: mpsc::Receiver<SectionEvent>,
        call_watcher: watch::Sender<bool>,
    ) -> Self {
        Self {
            context,
            channel,
            tool_engine,
            section_receiver,
            call_watcher,
        }
    }
    pub fn start(mut self, mind: StreamMind) {
        //运行输出流
        let ch = self.channel.clone();
        tokio::spawn(async move {
            stream::run_mind(mind, ch).await;
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
                reaction_event_id: 0,
                response_event_id: 0,
                response_sections: vec![],
                reaction_sections: vec![],
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
        let mut tool_calls: Vec<ToolCall> = vec![];
        let mut tool_call_details: HashMap<i64, ToolCallDetailItem> = HashMap::new();
        while let Some(section_event) = self.section_receiver.recv().await {
            match section_event {
                SectionEvent::Start(_, _) => {
                    {
                        let mut info = ctx.info.write().await;
                        info.status = AgentStatus::Processing;
                        //
                        let (event, model) = entity_service::event::save(
                            db,
                            agent_id,
                            state_id,
                            EnvEvent::AgentReaction(vec![]),
                            false,
                        )
                        .await
                        .expect("保存agent反应失败");
                        worker.reaction_event_id = model.id;
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
                    let web_event = WebEvent::Section(
                        SectionEvent::Start(worker.reaction_event_id, worker.response_event_id),
                        0,
                    );
                    self.channel.send_web_event(web_event).await;
                }
                SectionEvent::Open(tag) => {
                    let event_id = match tag.name {
                        "reply" | "render" => worker.response_event_id,
                        _ => worker.reaction_event_id,
                    };
                    let web_event = WebEvent::Section(section_event.clone(), event_id);
                    self.channel.send_web_event(web_event).await;
                }
                SectionEvent::Chunk(tag, _) => {
                    let event_id = match tag.name {
                        "reply" | "render" => worker.response_event_id,
                        _ => worker.reaction_event_id,
                    };
                    match tag.name {
                        "reply" | "think" => {
                            let web_event = WebEvent::Section(section_event.clone(), event_id);
                            self.channel.send_web_event(web_event).await;
                        }
                        _ => {
                            continue;
                        }
                    }
                }
                SectionEvent::Close(mut section) => {
                    let (event_id, sections) = match section.tag_name.as_str() {
                        "reply" | "render" => {
                            (worker.response_event_id, &mut worker.response_sections)
                        }
                        _ => (worker.reaction_event_id, &mut worker.reaction_sections),
                    };
                    //保存section
                    {
                        let model = entity_service::section::save(
                            db,
                            agent_id,
                            event_id,
                            section.clone(),
                            serde_json::Value::Null,
                        )
                        .await?;
                        section.id = model.id;
                        sections.push(section.clone());
                    };
                    let payload = match section.tag_name.as_str() {
                        "render" => {
                            let section_content = section.content.trim();
                            let ctx: RenderContext = serde_json::from_str(section_content)?;
                            serde_json::to_value(ctx)?
                        }
                        "tool_call" => {
                            let call = self
                                .tool_engine
                                .preprocess(section.id, section.content.trim())?;
                            let detail = ToolCallDetailItem {
                                id: call.id,
                                package: call.package.name.to_string(),
                                tool: call.tool.name.to_string(),
                                values: vec![],
                                status: ToolCallStatus::Undo,
                                args: call.args.clone(),
                                elapsed: "".to_string(),
                            };
                            tool_call_details.insert(call.id, detail.clone());
                            tool_calls.push(call);
                            json!(&detail)
                        }
                        _ => serde_json::Value::Null,
                    };
                    entity_service::section::update_payload(db, section.id, payload).await?;
                    //发送到前端
                    info!("SECTION:{}", &section.tag_name);
                    let web_event = WebEvent::Section(SectionEvent::Close(section), event_id);
                    self.channel.send_web_event(web_event).await;
                }
                SectionEvent::End(_, _) => {
                    let web_event = WebEvent::Section(
                        SectionEvent::End(worker.reaction_event_id, worker.response_event_id),
                        0,
                    );
                    self.channel.send_web_event(web_event).await;
                    //没有调用的话，则等待外部事件触发循环
                    if tool_calls.is_empty() {
                        return Ok(());
                    }
                    //加入临时阻塞事件接收机制，防止执行函数调用时，中间有事件触发
                    self.call_watcher.send(true).unwrap();

                    if let Err(e) = tool_call(
                        tool_calls,
                        tool_call_details,
                        self.context.clone(),
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
    response_event_id: i64,
    reaction_event_id: i64,
    response_sections: Vec<Section>,
    reaction_sections: Vec<Section>,
}
impl ReactWorker {
    async fn end(self) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        if self.reaction_event_id > 0 {
            entity_service::event::update_agent_response(
                db,
                self.reaction_event_id,
                EnvEventType::AgentReaction,
                self.reaction_sections,
            )
            .await?;
            entity_service::event::update_agent_response(
                db,
                self.response_event_id,
                EnvEventType::AgentResponse,
                self.response_sections,
            )
            .await?;
        }
        info!("step end");
        Ok(())
    }
}
async fn tool_call(
    tool_calls: Vec<ToolCall>,
    mut tool_call_details: HashMap<i64, ToolCallDetailItem>,
    agent_ctx: Arc<AgentContext>,
    channel: Arc<AgentChannel>,
    tool_engine: &ToolEngine,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    // //保存一个空响应事件
    let mut bind_event = {
        let info = agent_ctx.info.read().await;
        let (event, model) = entity_service::event::save(
            db,
            info.id,
            info.state_id,
            EnvEvent::ToolCallResult(vec![]),
            true,
        )
        .await
        .expect("创建工具调用结果事件失败");
        event
    };
    let bind_event_id = bind_event.id;
    let mut items = vec![];
    let mut call_ids = vec![];
    for call in &tool_calls {
        call_ids.push(call.id);
        //
        let item = CallItem {
            id: call.id,
            package: call.package.name,
            tool: call.tool.name,
            args: call.args.clone(),
        };
        items.push(item);
    }
    let (sender, mut receiver) = mpsc::channel::<ToolCallEvent>(tool_calls.len());
    //批执行
    let ch = channel.clone();
    tokio::spawn(async move {
        //异步处理函数调用结果
        while let Some(event) = receiver.recv().await {
            ch.send_web_event(WebEvent::ToolCall(event.clone(), bind_event_id))
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
        }
        //排序
        let details: Vec<ToolCallDetailItem> = call_ids
            .iter()
            .map(|id| tool_call_details.remove(id).unwrap())
            .collect();
        entity_service::event::update_tool_call_result(db, bind_event_id, details.clone())
            .await
            .unwrap();
        //发送完整事件
        bind_event.data = EnvEvent::ToolCallResult(details);
        ch.event_sender.send(bind_event).await.unwrap();
    });
    //通知前端 工具调用开始执行
    channel
        .send_web_event(WebEvent::ToolCall(
            ToolCallEvent::Start(items),
            bind_event_id,
        ))
        .await;

    tool_engine.batch_call(tool_calls, sender).await?;
    //结束
    channel
        .send_web_event(WebEvent::ToolCall(ToolCallEvent::End, bind_event_id))
        .await;
    Ok(())
}

// async fn send_web_event(agent: Arc<RwLock<AgentInstance>>, e: WebEvent) {
//     let mut agent = agent.write().await;
//     agent.channel.send_web_event(e).await
// }
