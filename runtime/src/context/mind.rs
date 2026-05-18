use anyhow::Context;

use kernel::{
    agent::context::AgentEvent,
    event::EnvEvent,
    section::{Section, SectionTag},
};
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    agent::channel::AgentChannel,
    context::{
        section::{self, section_stream},
        stack::{self, MindNode, MindStack},
    },
    llm::client::ChatClient,
};
use futures::stream;
use tokio::sync::{RwLock, mpsc, watch};

use futures::{Stream, StreamExt};

pub enum MindFlag {
    Closed,
    Overflow,
}
pub enum Event {
    Start(Vec<i64>),
    Open(&'static SectionTag),
    Chunk(&'static SectionTag, String),
    Close(&'static SectionTag, String),
    End(Vec<i64>),
}
pub struct StreamMind {
    pub agent_id: u32,
    pub stack: Arc<RwLock<MindStack>>,
    client: Arc<dyn ChatClient>,
    event_buffer: Vec<AgentEvent>,
    last_section_head: RwLock<String>,
    inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
    event_receiver: mpsc::Receiver<AgentEvent>,
    section_sender: mpsc::Sender<Event>,
    reorganize_watcher: watch::Sender<bool>,
    run_watcher: watch::Sender<bool>,
}

impl StreamMind {
    pub fn new(
        agent_id: u32,
        client: Arc<dyn ChatClient>,
        stack: Arc<RwLock<MindStack>>,
        inject_receiver: mpsc::UnboundedReceiver<(String, bool)>,
        event_receiver: mpsc::Receiver<AgentEvent>,
        section_sender: mpsc::Sender<Event>,
        reorganize_watcher: watch::Sender<bool>,
        run_watcher: watch::Sender<bool>,
    ) -> Self {
        StreamMind {
            agent_id,
            client,
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
    pub async fn event_loop(&mut self) -> anyhow::Result<MindFlag> {
        let len = std::env::var("CTX_LEN_PER_ROUND")
            .map(|v| usize::from_str_radix(&v, 10).unwrap())
            .unwrap_or(50000);

        loop {
            let mut worker = ListeningWorker {
                section_sender: self.section_sender.clone(),
                last_tag: None,
                chunk_buffer: String::new(),
                event_ids:vec![]
            };

            let res = self.event_listening(&mut worker).await;
            worker.end().await;
            self.run_watcher.send(false)?;
            let is_close = res?;
            if is_close {
                return Ok(MindFlag::Closed);
            }
            //检查上下文长度是否超过配置
            let stack = self.stack.read().await;
            if stack.content_len >= len {
                return Ok(MindFlag::Overflow);
            }
        }
    }
    async fn event_listening(&mut self, worker: &mut ListeningWorker) -> anyhow::Result<bool> {
        //抽干事件
        self.drain_events();
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
        worker.event_ids = self.event_buffer.iter().map(|e| e.id).collect();
        let proxy = mpsc::unbounded_channel();
        // 3. 准备当前要处理的上下文
        // 提取出当前所有的任务，清空 pending 准备接收新的
        let mut chunk_stream = {
            let mut stack = self.stack.write().await;
            stack.push_events(&mut self.event_buffer);
            // 4. 创建生成流（模拟大模型输出或耗时任务）
            stack.chunk_stream(self.client.clone()).await
        };
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

        // 5. 边生成边监听队列
        loop {
            tokio::select! {
                // 中断注入
                Some(injected)=self.inject_receiver.recv() =>{
                    self.inject_section(&proxy.0, injected).await;
                }
                // 分支 A：段落生成流
                Some(section_event) = section_stream.next() =>  {
                    let mut is_end=false;
                   let event= match section_event {
                        section::Event::Start => {
                            Event::Start(worker.event_ids.clone())
                        }
                        section::Event::End => {
                            //必须由外部确认是否真的结束,所以不发送事件
                            break;
                        }
                        
                        section::Event::Open(tag)=>{
                            worker.last_tag=Some(tag);
                            Event::Open(tag)

                        }
                        section::Event::Chunk(tag,chunk)=>{
                            worker.chunk_buffer.push_str(&chunk);
                            Event::Chunk(tag,chunk)

                        }
                        section::Event::Close(tag,content) => {
                            worker.last_tag=None;
                             Event::Close(tag,content)
                        }
                    };

                    self.section_sender.send(event).await?;
                }
                // 分支 B：在生成过程中，收到了新事件
                Some(event) = self.event_receiver.recv() => {
                    let force= event.force;
                    self.event_buffer.push(event);
                    if force{
                        break; // 跳出内部 loop,强制处理
                    }
                }
                else=>{
                    break ;
                }
            };
        }
        Ok(false)
    }
}
struct ListeningWorker {
    section_sender: mpsc::Sender<Event>,
    last_tag: Option<&'static SectionTag>,
    chunk_buffer: String,
    event_ids: Vec<i64>,
}
impl ListeningWorker {
    async fn end(self) {
        if let Some(last_tag) = self.last_tag {
            let last = (last_tag, self.chunk_buffer);
            self.section_sender
                .send(Event::Close(last.0, last.1))
                .await
                .expect("section_sender");
        }
        self.section_sender
            .send(Event::End(self.event_ids))
            .await
            .expect("section_sender"); //输出结束
    }
}
pub async fn run_mind(mut mind: StreamMind, channel: Arc<AgentChannel>) {
    loop {
        match mind.event_loop().await.unwrap() {
            MindFlag::Closed => return,
            MindFlag::Overflow => {
                //让agent退栈
                channel
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
