use anyhow::Context;
use std::sync::Arc;
use streamind_core::{
    event::{EnvEvent, SectionEvent},
    section::Section,
};
use tracing::{error, info};

use crate::agent::{
    context::section::section_stream,
    context::stack::{self, MindNode, MindStack},
    core::{Agent, AgentChannel},
    event::AgentEvent,
    kit::{entity_kit, message_kit, prompt_kit, string_kit},
    llm_engine::LLMEngine,
};
use futures::stream;
use tokio::sync::{mpsc, oneshot, watch, RwLock};

use futures::{Stream, StreamExt};

pub enum MindFlag {
    Closed,
    Overflow,
}

pub struct StreamMind {
    pub agent_id: u32,
    pub stack: MindStack,
    event_buffer: Vec<AgentEvent>,
    last_section_head: RwLock<String>,
    inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
    event_receiver: mpsc::Receiver<AgentEvent>,
    section_sender: mpsc::Sender<SectionEvent>,
    reorganize_watcher: watch::Sender<bool>,
    run_watcher: watch::Sender<bool>,
}

impl StreamMind {
    pub fn new(
        agent_id: u32,
        stack: MindStack,
        inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
        event_receiver: mpsc::Receiver<AgentEvent>,
        section_sender: mpsc::Sender<SectionEvent>,
        reorganize_watcher: watch::Sender<bool>,
        run_watcher: watch::Sender<bool>,
    ) -> Self {
        StreamMind {
            agent_id,
            stack,
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
                EnvEvent::Interrupt(_) => {
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
    pub async fn event_loop(&mut self, engine: Arc<LLMEngine>) -> anyhow::Result<MindFlag> {
        let len = std::env::var("CTX_LEN_PER_ROUND")
            .map(|v| usize::from_str_radix(&v, 10).unwrap())
            .unwrap_or(50000);

        loop {
            let is_close = self.event_listening(engine.clone()).await.unwrap();
            self.run_watcher.send(false)?;
            if is_close {
                return Ok(MindFlag::Closed);
            }
            //检查上下文长度是否超过配置
            if self.stack.content_len >= len {
                return Ok(MindFlag::Overflow);
            }
        }
    }
    async fn event_listening(&mut self, engine: Arc<LLMEngine>) -> anyhow::Result<bool> {
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
        self.stack.push_events(&mut self.event_buffer);
        // 4. 创建生成流（模拟大模型输出或耗时任务）
        let mut chunk_stream = self.stack.chunk_stream(engine).await;
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
                    let is_end= matches!(&section_event,SectionEvent::End(_, _));
                    if let SectionEvent::Close(section)=&section_event{
                        watch_section(self,&section).await?;
                        let s:String=section.to_string();
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
                        self.section_sender.send(SectionEvent::End(0, 0)).await.unwrap();
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
        let res = response.trim().to_string();
        let len = res.len();
        let res = prompt_kit::assistant_message(res);
        self.stack.push_msg(res, len);
        // 2. 瞬间抽干队列中同时到达的其他任务（Batching）
        self.drain_events();
        Ok(false)
    }
}

pub async fn run_mind(
    agent: Arc<RwLock<Agent>>,
    engine: Arc<LLMEngine>,
    inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
    event_receiver: mpsc::Receiver<AgentEvent>,
    section_sender: mpsc::Sender<SectionEvent>,
    reorganize_watcher: watch::Sender<bool>,
    run_watcher: watch::Sender<bool>,
) {
    // 加载系统提示词和历史消息

    let (agent_id, stack) = {
        let agent = agent.read().await;
        let ctx = agent.ctx.read().await;
        (
            ctx.info.id,
            message_kit::load_stack(ctx.info.id).await.unwrap(),
        )
    };
    let mut mind_stream = StreamMind::new(
        agent_id,
        stack,
        inject_receiver,
        event_receiver,
        section_sender,
        reorganize_watcher,
        run_watcher,
    );
    loop {
        match mind_stream.event_loop(engine.clone()).await.unwrap() {
            MindFlag::Closed => return,
            MindFlag::Overflow => {
                //让agent退栈
                let mut agent = agent.write().await;
                agent
                    .send_event(
                        EnvEvent::Error("上下文溢出告警,请执行出栈操作".to_string()),
                        true,
                    )
                    .await
                    .unwrap();
            }
        }
    }
}

async fn watch_section(mind: &mut StreamMind, section: &Section) -> anyhow::Result<()> {
    match section.tag_name.as_str() {
        "push" => push_state(mind.agent_id, &mut mind.stack, section).await?,
        "pop" => pop_state(mind.agent_id, &mut mind.stack, section).await?,
        _ => {}
    }
    Ok(())
}

fn parse_state(content: &str) -> anyhow::Result<(usize, String, String)> {
    let (head, content) = content
        .split_once("\n")
        .with_context(|| anyhow::anyhow!("栈格式异常:{content}"))?;
    let (pos, title) = head
        .split_once(":")
        .with_context(|| anyhow::anyhow!("栈头格式异常:{head}"))?;
    let pos =
        usize::from_str_radix(pos.trim(), 10).map_err(|e| anyhow::anyhow!("栈指针格式异常"))?;
    Ok((pos, title.trim().to_string(), content.trim().to_string()))
}

async fn push_state(agent_id: u32, stack: &mut MindStack, section: &Section) -> anyhow::Result<()> {
    let (pos, title, content) = parse_state(&section.content.trim())?;
    let model = entity_kit::push_state_stack(agent_id, pos as _, title, content).await?;
    let node = MindNode {
        id: model.id,
        title: model.title,
        content: model.content,
        messages: vec![],
        content_len: 0,
    };
    stack.push(node);
    Ok(())
}

//格式
//<back>
//pos:title
//content
//</back>
async fn pop_state(agent_id: u32, stack: &mut MindStack, section: &Section) -> anyhow::Result<()> {
    let (pos, title, content) = parse_state(&section.content.trim())?;
    //更新节点内容
    {
        let node = match stack.nodes.get_mut(pos) {
            Some(n) => n,
            None => {
                return Err(anyhow::anyhow!("指向的栈中状态不存在,pos:{}", pos));
            }
        };
        node.title = title;
        node.content = content;
    }
    let nodes = stack.back_to(pos);
    let state_ids = nodes.iter().map(|n| n.id).collect();
    entity_kit::dump_states(agent_id, state_ids).await?;
    Ok(())
}
