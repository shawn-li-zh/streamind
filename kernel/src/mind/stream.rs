use crate::{
    agent::event::EnvEvent,
    mind::{MindInlet, MindOutlet, context::EventContext},
    section::SectionTag,
};
use std::sync::Arc;
use tracing::{error, info};

use crate::context::section::{self, section_stream};
use futures::stream;
use tokio::sync::mpsc;

use futures::{Stream, StreamExt};

pub enum MindFlag {
    Closed,
    Overflow,
}
#[derive(Debug, Clone)]
pub enum GenerateEvent {
    Start(Vec<i64>),
    Open(&'static SectionTag, String),
    Chunk(&'static SectionTag, String),
    Close(&'static SectionTag, String),
    End(Vec<i64>, EndReason),
    Error(Arc<anyhow::Error>),
}
#[derive(Debug, Clone)]
pub enum EndReason {
    Normal,
    Error,
    Canceled,
}
pub struct StreamMind<E: EnvEvent, C: EventContext<E>> {
    context: Arc<C>,
    event_buffer: Vec<E>,
    env_receiver: mpsc::Receiver<E>,
    generate_sender: mpsc::Sender<GenerateEvent>,
}

impl<E: EnvEvent, C: EventContext<E>> StreamMind<E, C> {
    pub fn new(context: Arc<C>) -> (Self, MindInlet<E>, MindOutlet) {
        let event = mpsc::channel::<E>(100);
        let inlet = MindInlet { inner: event.0 };
        let mind_event = mpsc::channel::<GenerateEvent>(100);
        let outlet = MindOutlet::new(mind_event.1);
        let mind = StreamMind {
            context,
            event_buffer: vec![],
            env_receiver: event.1,
            generate_sender: mind_event.0,
        };
        (mind, inlet, outlet)
    }

    /// 核心方法：非阻塞地抽干(Drain)当前队列中的所有事件
    fn drain_events(&mut self) {
        while let Ok(event) = self.env_receiver.try_recv() {
            if event.is_force() {
                // 在 drain 阶段遇到打断信号可以直接忽略
                // 因为我们本身就是在打断后的重新整理阶段
                println!("[队列提取] 忽略多余的打断信号");
            } else {
                self.event_buffer.push(event);
            }
        }
    }
    pub async fn event_loop(&mut self) -> anyhow::Result<MindFlag> {
        // let len = std::env::var("CTX_LEN_PER_ROUND")
        //     .map(|v| usize::from_str_radix(&v, 10).unwrap())
        //     .unwrap_or(50000);

        loop {
            let mut worker = ListeningWorker {
                generate_sender: self.generate_sender.clone(),
                last_tag: None,
                chunk_buffer: String::new(),
                event_ids: vec![],
            };
            //抽干事件
            self.drain_events();
            // 1. 如果没有待处理任务，阻塞等待下一个事件到达
            if self.event_buffer.is_empty() {
                if let Some(agent_event) = self.env_receiver.recv().await {
                    self.event_buffer.push(agent_event);
                } else {
                    info!("通道已关闭，智能体即将退出");
                    return Ok(MindFlag::Closed);
                }
            }
            let resaon = self
                .event_stream(&mut worker)
                .await
                .unwrap_or(EndReason::Error);

            worker.end(resaon).await;

            // //检查上下文长度是否超过配置
            // let stack = self.stack.read().await;
            // if stack.content_len >= len {
            //     return Ok(MindFlag::Overflow);
            // }
        }
    }
    async fn event_stream(&mut self, worker: &mut ListeningWorker) -> anyhow::Result<EndReason> {
        worker.event_ids = self.event_buffer.iter().map(|e| e.id()).collect();
        let proxy: (
            mpsc::UnboundedSender<String>,
            mpsc::UnboundedReceiver<String>,
        ) = mpsc::unbounded_channel();
        // 3. 准备当前要处理的上下文
        // 提取出当前所有的任务，清空 pending 准备接收新的
        let mut chunk_stream = {
            {
                let events = std::mem::take(&mut self.event_buffer);
                self.context.push_events(events);
            }
            // 4. 创建生成流
            self.context.chunk_stream().await
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
                                // print!("{}",&val);
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
                            yield Ok(val);
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
                // // 中断注入
                // Some(injected)=self.inject_receiver.recv() =>{
                //     self.inject_section(&proxy.0, injected).await;
                // }
                // 分支 A：段落生成流
                Some(section_event) = section_stream.next() =>  {

                   let event= match section_event {
                        section::Event::Start => {
                            GenerateEvent::Start(worker.event_ids.clone())
                        }
                        section::Event::End => {
                            //必须由外部确认是否真的结束,所以不发送事件
                            break;
                        }
                        section::Event::Open(tag,head)=>{
                            worker.last_tag=Some(tag);
                            worker.chunk_buffer.clear();
                            GenerateEvent::Open(tag,head)

                        }
                        section::Event::Chunk(tag,chunk)=>{
                            worker.chunk_buffer.push_str(&chunk);
                            GenerateEvent::Chunk(tag,chunk)
                        }
                        section::Event::Close(tag,content) => {
                            worker.last_tag=None;
                            worker.chunk_buffer.clear();
                            GenerateEvent::Close(tag,content)
                        }
                          section::Event::Error(e) => {

                            GenerateEvent::Error(Arc::new(e))
                        }
                    };

                    self.generate_sender.send(event).await?;
                }
                // 分支 B：在生成过程中，收到了新事件
                Some(event) = self.env_receiver.recv() => {
                    let force= event.is_force();
                    //   println!("new event:{},{:?}", force,&event.data);
                    self.event_buffer.push(event);

                    if force{
                        // 跳出内部 loop,强制处理
                        return Ok(EndReason::Canceled)
                    }
                }
                else=>{
                     return Ok(EndReason::Error)
                }
            };
        }
        Ok(EndReason::Normal)
    }
}
struct ListeningWorker {
    generate_sender: mpsc::Sender<GenerateEvent>,
    last_tag: Option<&'static SectionTag>,
    chunk_buffer: String,
    event_ids: Vec<i64>,
}
impl ListeningWorker {
    async fn end(self, reson: EndReason) {
        if let Some(last_tag) = self.last_tag {
            let last = (last_tag, self.chunk_buffer);
            self.generate_sender
                .send(GenerateEvent::Close(last.0, last.1))
                .await
                .expect("section_sender");
        }
        self.generate_sender
            .send(GenerateEvent::End(self.event_ids, reson))
            .await
            .expect("section_sender"); //输出结束
    }
}

// #[cfg(test)]

// mod test {

//     use futures::StreamExt;
//     use std::time::Duration;

//     use tokio::sync::mpsc;

//     use crate::context::section::{Event, section_stream};

//     #[tokio::test]
//     async fn test() {
//         println!("=== 开始测试 Section Stream ===");

//         // 1. 模拟 LLM 异步流式输出的数据块 (Chunks)
//         // 故意将标签切碎，以测试解析器的 buffer 拼接能力
//         let outputs = {
//             if let Ok(test) = std::fs::read_to_string(r"D:\project\rust\streamind\test.xml") {
//                 test.split("<end/>")
//                     .map(|item| item.trim())
//                     .filter(|v| !v.is_empty())
//                     .map(|item| split_into_chunks(item))
//                     .collect()
//             } else {
//                 vec![]
//             }
//         };
//         let (tx, rx) = mpsc::unbounded_channel();
//         tokio::spawn(async move {
//             for _ in 0..1 {
//                 for chunks in &outputs {
//                     for chunk in chunks {
//                         tokio::time::sleep(Duration::from_millis(10)).await;
//                         tx.send(Ok(chunk.to_string())).unwrap();
//                     }
//                 }
//             }
//         });
//         let chunk_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).boxed();
//         // 3. 调用你的 section_stream 函数
//         let mut event_stream = Box::pin(section_stream(chunk_stream));

//         // 4. 消费解析出来的事件
//         while let Some(event) = event_stream.next().await {
//             match event {
//                 Event::Start => {
//                     println!("\n[事件] >>> 流开始 <<<");
//                 }
//                 Event::Open(tag, head) => {
//                     println!("\n[事件] 开启标签: {}", head);
//                 }
//                 Event::Chunk(tag, chunk) => {
//                     // print!("{}", chunk)
//                 }
//                 Event::Close(tag, content) => {
//                     println!("[事件] 关闭标签{}，完整段落内容为: {:?}", tag.end, content);
//                 }
//                 Event::End => {
//                     println!("\n[事件] >>> 流结束 <<<");
//                 }
//                 Event::Error(e) => {
//                     println!("\n[事件] >>> 异常 {e} <<<");
//                 }
//             }
//         }

//         // 给后台异步任务一点时间打印完毕
//         tokio::time::sleep(Duration::from_secs(10)).await;
//     }
// }
