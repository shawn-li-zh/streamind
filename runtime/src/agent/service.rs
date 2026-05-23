// use std::{backtrace::Backtrace, collections::HashMap, sync::Arc, time::Instant};

// use anyhow::Context as _;
// use axum::response::sse;
// use entity::event::EnvEventStatus;
// use entity_service::DB;
// use futures::StreamExt;
// use kernel::{
//     agent::{
//         AgentHandler,
//         context::{AgentContext, AgentEvent, AgentStatus},
//     },
//     context::stack::{Push, StackContext},
//     event::{
//         CallItem, Event, MindEvent, SectionEvent, ToolCallDetailItem, ToolCallEvent,
//         ToolCallStatus,
//     },
//     mind::{
//         MindInlet, MindOutlet,
//         stream::{EndReason, GenerateEvent},
//     },
//     parser,
//     render::RenderContext,
//     section::{Section, SectionTag},
//     tool::{
//         call::ToolCall,
//         core::ToolReturnValue,
//         engine::{ToolEngine, ToolEngineContext},
//     },
// };
// use serde_json::json;
// use tokio::sync::{RwLock, mpsc};
// use tracing::{info, warn};

// use crate::{agent::channel::AgentChannel, sse_event::WebEvent};

// pub struct Service {
//     agent: Arc<dyn AgentHandler>,
//     stack: Arc<RwLock<StackContext>>,
//     mind_inlet: Arc<MindInlet>,
// }
// impl Service {
//     async fn run(&self, inlet: MindInlet, mut outlet: MindOutlet) -> anyhow::Result<()> {
//         let ctx: Arc<AgentContext> = self.agent.context();
//         let tool_engine_context = ctx.get::<ToolEngineContext>().unwrap();
//         let mut generate_worker = GenerateWorker {
//             agent: self.agent.clone(),
//             start_time: Instant::now(),
//             last_tag_open: None,
//             stack: self.stack.clone(),
//             response_event: None,
//             response_event_id: 0,
//             response_sections: vec![],
//             web_sender: Arc::new(WebSender {
//                 inner: RwLock::new(None),
//             }),
//             tool_engine: Arc::new(ToolEngine::new(self.agent.clone(), tool_engine_context)),
//             mind_inlet: Arc::new(inlet),
//         };
//         while let Some(c) = outlet.next().await {
//             match c {
//                 MindEvent::Env(event, force) => {
//                     handle_env(self.agent.clone(), self.mind_inlet.clone(), event, force).await?;
//                 }
//                 MindEvent::Generate(event) => {
//                     generate_worker.handle(event).await?;
//                 }
//                 MindEvent::Status(agent_status) => todo!(),
//             }
//         }
//         Ok(())
//     }
// }
// async fn handle_env(
//     agent: Arc<dyn AgentHandler>,
//     mind_inlet: Arc<MindInlet>,
//     event: Event,
//     force: bool,
// ) -> anyhow::Result<()> {
//     //保存数据库
//     let db = DB.get().unwrap();
//     let (event, model) = {
//         let identity = agent.identity();
//         let stack = agent.stack();
//         let stack = stack.read().await;
//         let last = stack.last().unwrap();
//         entity_service::event::save(db, identity.id(), last.id, event, force).await?
//     };
//     //发送给代理
//     mind_inlet.send(event).await?;
//     Ok(())
// }
// struct WebSender {
//     inner: RwLock<Option<mpsc::UnboundedSender<sse::Event>>>,
// }
// impl WebSender {
//     pub async fn send(&self, event: WebEvent) {
//         let web_sender = self.inner.read().await;
//         if let Some(sender) = &*web_sender {
//             match sender.send(event.to_sse_event()) {
//                 Ok(_) => {}
//                 Err(e) => {
//                     //断开
//                     info!("sse连接断开：{}", e);
//                     drop(web_sender);
//                     let mut web_sender = self.inner.write().await;
//                     *web_sender = None;
//                 }
//             };
//             return;
//         }
//         warn!("断连中...");
//     }
// }
// struct GenerateWorker {
//     agent: Arc<dyn AgentHandler>,
//     start_time: Instant,
//     last_tag_open: Option<(&'static SectionTag, String)>,

//     response_event: Option<AgentEvent>,
//     response_event_id: i64,
//     response_sections: Vec<Section>,
//     stack: Arc<RwLock<StackContext>>,
//     web_sender: Arc<WebSender>,
//     tool_engine: Arc<ToolEngine>,
//     mind_inlet: Arc<MindInlet>,
// }
// impl GenerateWorker {
//     async fn handle(&mut self, event: GenerateEvent) -> anyhow::Result<()> {
//         let db = DB.get().unwrap();
//         let ctx = self.agent.context();
//         let agent_id = self.agent.identity().id();

//         match event {
//             GenerateEvent::Start(event_ids) => {
//                 println!("start...");
//                 self.agent.send_status(AgentStatus::Processing).await?;
//                 entity_service::event::update_event_status(
//                     db,
//                     event_ids,
//                     EnvEventStatus::Processing,
//                 )
//                 .await
//                 .unwrap();
//                 {
//                     let state_id = {
//                         let stack = self.stack.read().await;
//                         let last = stack.last().unwrap();
//                         last.id
//                     };
//                     //
//                     //保存一个空事件
//                     let (event, model) = entity_service::event::save(
//                         db,
//                         agent_id,
//                         state_id,
//                         Event::AgentResponse(vec![]),
//                         false,
//                     )
//                     .await
//                     .expect("保存agent回复失败");
//                     self.response_event = Some(event);
//                     self.response_event_id = model.id;
//                 };
//                 let web_event = WebEvent::Section(SectionEvent::Start, self.response_event_id);
//                 self.web_sender.send(web_event).await;
//             }
//             GenerateEvent::Open(tag, header) => {
//                 self.last_tag_open = Some((tag, header));
//                 self.start_time = Instant::now();
//                 let event_id = self.response_event_id;
//                 let web_event = WebEvent::Section(SectionEvent::Open(tag), event_id);
//                 self.web_sender.send(web_event).await;
//             }
//             GenerateEvent::Chunk(tag, chunk) => {
//                 let event_id = self.response_event_id;
//                 // section::Event
//                 match tag.name {
//                     "reply" | "think" => {
//                         let web_event =
//                             WebEvent::Section(SectionEvent::Chunk(tag, chunk), event_id);
//                         self.web_sender.send(web_event).await;
//                     }
//                     _ => {}
//                 }
//             }
//             GenerateEvent::Close(tag, content) => {
//                 let mut section = {
//                     let (tag, header) = self.last_tag_open.take().unwrap();
//                     let elapsed = self.start_time.elapsed();
//                     let elapsed = humantime::format_duration(elapsed).to_string();
//                     Section {
//                         id: 0,
//                         header,
//                         name: tag.name.to_string(),
//                         content,
//                         elapsed,
//                     }
//                 };
//                 //保存section
//                 {
//                     let model = entity_service::section::save(
//                         db,
//                         agent_id,
//                         self.response_event_id,
//                         section.clone(),
//                         serde_json::Value::Null,
//                     )
//                     .await?;
//                     section.id = model.id;
//                     self.response_sections.push(section.clone());
//                 };

//                 //发送到前端
//                 info!("SECTION:{}", &tag.name);
//                 let web_event =
//                     WebEvent::Section(SectionEvent::Close(section), self.response_event_id);
//                 self.web_sender.send(web_event).await;
//             }
//             GenerateEvent::End(event_ids, resaon) => {
//                 {
//                     //修改事件状态为完成
//                     entity_service::event::update_event_status(db, event_ids, EnvEventStatus::Done)
//                         .await
//                         .unwrap();
//                     let web_event = WebEvent::Section(SectionEvent::End, self.response_event_id);
//                     self.web_sender.send(web_event).await;
//                 }
//                 //加入临时阻塞事件接收机制，防止期间有事件触发执行其他任务去了
//                 println!("call start...");
//                 self.agent.send_status(AgentStatus::Waiting).await?;
//                 //解析响应的section
//                 let mut errors = vec![];
//                 let mut tool_call_infos: Vec<(ToolCallDetailItem, ToolCall)> = vec![];
//                 let mut pushs: Vec<Push> = vec![];
//                 {
//                     let mut counter: HashMap<&String, u32> = HashMap::new();
//                     for section in &self.response_sections {
//                         let value = counter.entry(&section.name).or_default();
//                         *value += 1;
//                         match parse_section(
//                             &self.tool_engine,
//                             section,
//                             &mut tool_call_infos,
//                             &mut pushs,
//                         )
//                         .await
//                         {
//                             Ok(payload) => {
//                                 entity_service::section::update_payload(db, section.id, payload)
//                                     .await?;
//                             }
//                             Err(e) => {
//                                 errors
//                                     .push(format!("第{}个<{}>解析失败:{}", value, section.name, e));
//                             }
//                         }
//                     }
//                 }
//                 if !errors.is_empty() {
//                     handle_env(
//                         self.agent.clone(),
//                         self.mind_inlet.clone(),
//                         Event::Error(errors.join("\n")),
//                         false,
//                     )
//                     .await?;

//                     return Ok(());
//                 }
//                 //非正常结束则不处理任何功能调用
//                 match resaon {
//                     EndReason::Error | EndReason::Canceled => return Ok(()),
//                     _ => {}
//                 }

//                 //函数调用
//                 if !tool_call_infos.is_empty() {
//                     if let Err(e) = tool_call(
//                         self.response_event_id,
//                         tool_call_infos,
//                         self.agent.clone(),
//                         &self.tool_engine,
//                         self.mind_inlet.clone(),
//                         self.web_sender.clone(),
//                     )
//                     .await
//                     {
//                         let bt = Backtrace::capture();
//                         eprintln!("{:?}", bt);
//                         //发送事件处理
//                         handle_env(
//                             self.agent.clone(),
//                             self.mind_inlet.clone(),
//                             Event::Error(e.to_string()),
//                             true,
//                         )
//                         .await
//                         .expect("send tool call result error");
//                     }
//                 }
//                 //处理压栈
//                 if !pushs.is_empty() {
//                     push_state(self.agent.clone(), pushs).await?;
//                 }
//             }
//         }
//         Ok(())
//     }
// }

// async fn parse_section(
//     tool_engine: &ToolEngine,
//     section: &Section,
//     tool_call_infos: &mut Vec<(ToolCallDetailItem, ToolCall)>,
//     pushs: &mut Vec<Push>,
// ) -> anyhow::Result<serde_json::Value> {
//     let content = section.content.trim();
//     let payload = match section.name.as_str() {
//         "render" => {
//             let mut ctx = RenderContext::from_xml(content)?;
//             let comp = entity_service::component::get_by_name(&ctx.name)
//                 .await?
//                 .ok_or_else(|| anyhow::anyhow!("组件不存在"))?;
//             ctx.path = Some(comp.path);
//             serde_json::to_value(ctx)?
//         }
//         "tool_call" => {
//             let inner = section
//                 .header
//                 .trim_start_matches("<tool_call")
//                 .trim_end_matches(">")
//                 .trim();
//             let mut attrs = parser::parse_attrs(inner);
//             let call_id = match attrs
//                 .remove("id")
//                 .map(|v| v.as_str().unwrap_or_default().to_string())
//             {
//                 Some(v) => v,
//                 None => {
//                     anyhow::bail!("tool_call id is required")
//                 }
//             };
//             let mut call = tool_engine.preprocess(call_id.clone(), content)?;
//             call.id = section.id;
//             let detail = ToolCallDetailItem {
//                 id: section.id,
//                 call_id,
//                 package: call.package.name.to_string(),
//                 tool: call.tool.name.to_string(),
//                 values: vec![],
//                 status: ToolCallStatus::Undo,
//                 args: call.args.clone(),
//                 elapsed: "".to_string(),
//             };

//             let p = json!(&detail);
//             tool_call_infos.push((detail, call));
//             p
//         }
//         "push" => {
//             let mut push = parse_push(&section.header, content)?;
//             push.id = section.id;
//             let p = json!(&push);
//             pushs.push(push);
//             p
//         }
//         _ => serde_json::Value::Null,
//     };
//     Ok(payload)
// }
// fn parse_push(header: &str, content: &str) -> anyhow::Result<Push> {
//     let inner = header
//         .trim_start_matches("<push")
//         .trim_end_matches(">")
//         .trim();
//     let mut attrs = parser::parse_attrs(inner);
//     let pos = match attrs.remove("pos") {
//         Some(v) => v
//             .as_u64()
//             .ok_or_else(|| anyhow::anyhow!("push pos is invalid"))?,
//         None => {
//             anyhow::bail!("push pos is required")
//         }
//     };

//     let (title, content) = content
//         .split_once("\n")
//         .with_context(|| anyhow::anyhow!("栈格式异常:{content}"))?;

//     Ok(Push {
//         id: 0,
//         pos: pos as _,
//         title: title.trim().to_string(),
//         content: content.trim().to_string(),
//     })
// }

// async fn tool_call(
//     response_event_id: i64,
//     tool_call_infos: Vec<(ToolCallDetailItem, ToolCall)>,
//     agent: Arc<dyn AgentHandler>,
//     tool_engine: &ToolEngine,
//     mind_inlet: Arc<MindInlet>,
//     web_sender: Arc<WebSender>,
// ) -> anyhow::Result<()> {
//     let db = DB.get().unwrap();
//     //
//     let mut tool_call_details: HashMap<i64, ToolCallDetailItem> = HashMap::new();
//     let mut tool_calls = vec![];
//     let mut items = vec![];
//     let mut call_ids = vec![];
//     for (detail, call) in tool_call_infos {
//         //
//         tool_call_details.insert(detail.id, detail);
//         call_ids.push(call.id);
//         //
//         let item = CallItem {
//             id: call.id,
//             package: call.package.name,
//             tool: call.tool.name,
//             args: call.args.clone(),
//         };
//         items.push(item);
//         tool_calls.push(call);
//     }
//     let (sender, mut receiver) = mpsc::channel::<ToolCallEvent>(tool_calls.len());
//     let web = web_sender.clone();
//     tokio::spawn(async move {
//         //异步处理函数调用结果
//         while let Some(event) = receiver.recv().await {
//             web.send(WebEvent::ToolCall(event.clone(), response_event_id))
//                 .await;

//             let item = match event {
//                 ToolCallEvent::Success(item) => item,
//                 ToolCallEvent::Failure(item) => item,
//                 _ => {
//                     continue;
//                 }
//             };
//             let detial = tool_call_details.get_mut(&item.id).unwrap();
//             detial.values = item.values;
//             detial.status = item.status;
//             detial.elapsed = item.elapsed;
//             //保存
//             entity_service::section::update_payload(db, detial.id, detial.clone())
//                 .await
//                 .unwrap();
//             let is_empty = detial
//                 .values
//                 .iter()
//                 .filter(|v| {
//                     if let ToolReturnValue::Ignore = v {
//                         return false;
//                     }
//                     return true;
//                 })
//                 .count()
//                 == 0;
//             if is_empty {
//                 return;
//             }
//             //发送完整事件
//             handle_env(
//                 agent.clone(),
//                 mind_inlet.clone(),
//                 Event::ToolResponse(detial.clone()),
//                 false,
//             )
//             .await
//             .unwrap();
//         }
//     });
//     //通知前端 工具调用开始执行
//     web_sender
//         .send(WebEvent::ToolCall(
//             ToolCallEvent::Start(items),
//             response_event_id,
//         ))
//         .await;
//     //批执行
//     tool_engine.batch_call(tool_calls, sender).await?;
//     //结束
//     web_sender
//         .send(WebEvent::ToolCall(ToolCallEvent::End, response_event_id))
//         .await;
//     Ok(())
// }

// async fn push_state(agent: Arc<dyn AgentHandler>, pushs: Vec<Push>) -> anyhow::Result<()> {
//     let agent_id = agent.identity().id();
//     let stack = agent.stack();
//     for mut push in pushs {
//         if push.pos == 0 {
//             anyhow::bail!("核心栈状态不能重置")
//         }
//         let stack = stack.clone();

//         entity_service::transaction::<_, _>(move |txn| {
//             Box::pin(async move {
//                 let model = entity_service::state::push(
//                     txn,
//                     agent_id,
//                     push.pos as _,
//                     push.title.clone(),
//                     push.content.clone(),
//                 )
//                 .await?;
//                 push.id = model.id;
//                 let mut stack = stack.write().await;
//                 stack.push(push);
//                 Ok(())
//             })
//         })
//         .await?;
//     }
//     Ok(())
// }
