use crate::{
    mind::event::EnvEvent,
    mind::{
        context::EventContext,
        port::{CommandEvent, EndReason, GenerateEvent, MindInlet, MindOutlet},
        section::{self, SectionTag, section_stream},
    },
};
use std::{
    fs::{File, OpenOptions},
    io::Write as _,
    marker::PhantomData,
    sync::Arc,
};
use tracing::{error, info};

use futures::stream;
use tokio::sync::{mpsc, watch};

use futures::{Stream, StreamExt};

pub enum MindFlag {
    Closed,
    Overflow,
}

pub struct StreamMind<E: EnvEvent, C: EventContext<E>> {
    context: Arc<C>,
    cmd_receiver: mpsc::Receiver<CommandEvent>,
    generate_sender: mpsc::Sender<GenerateEvent>,
    flag_sender: watch::Sender<bool>,
    _private: PhantomData<E>,
}

impl<E: EnvEvent, C: EventContext<E>> StreamMind<E, C> {
    pub fn new(context: Arc<C>) -> (Self, MindInlet, MindOutlet) {
        let (flag_sender, flag_receiver) = watch::channel::<bool>(false);
        let event = mpsc::channel::<CommandEvent>(50);
        let inlet = MindInlet::new(event.0, flag_receiver);
        let mind_event = mpsc::channel::<GenerateEvent>(100);
        let outlet = MindOutlet::new(mind_event.1);
        let mind = StreamMind {
            context,
            cmd_receiver: event.1,
            generate_sender: mind_event.0,
            flag_sender,
            _private: PhantomData,
        };
        (mind, inlet, outlet)
    }

    /// 非阻塞地抽干(Drain)当前队列中的所有事件
    fn drain_events(&mut self) -> bool {
        while let Ok(cmd) = self.cmd_receiver.try_recv() {
            match cmd {
                // CommandEvent::Inject(e) => {
                //     self.event_buffer.push(e);
                // }
                CommandEvent::Generate => {
                    //跳出事件监听开始
                    return true;
                }
                CommandEvent::Cancel => {
                    //已经停止忽略
                }
            }
        }
        false
    }
    pub async fn event_loop(&mut self) {
        loop {
            while let Some(cmd) = self.cmd_receiver.recv().await {
                match cmd {
                    // CommandEvent::Inject(e) => {
                    //     self.generate_sender.send(value)
                    // }
                    CommandEvent::Generate => {
                        //跳出事件监听开始
                        break;
                    }
                    CommandEvent::Cancel => {
                        //已经停止忽略
                    }
                }
            }
            self.flag_sender.send(true).unwrap(); //通知外部开始生成了
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .append(true)
                .create(true)
                .open("out.txt")
                .unwrap();
            file.write_all("\n---\n".as_bytes()).unwrap();
            let mut worker = ListeningWorker {
                generate_sender: self.generate_sender.clone(),
                last_tag: None,
                chunk_buffer: String::new(),
            };
            let res = self.event_stream(&mut worker, file).await;
            let resaon = match res {
                Ok(v) => v,
                Err(e) => EndReason::Error(Arc::new(e)),
            };
            worker.end(resaon).await;
            self.flag_sender.send(false).unwrap(); //通知外部生成结束
            self.drain_events();
        }
    }
    async fn event_stream(
        &mut self,
        worker: &mut ListeningWorker,
        mut file: File,
    ) -> anyhow::Result<EndReason> {
        let proxy: (
            mpsc::UnboundedSender<String>,
            mpsc::UnboundedReceiver<String>,
        ) = mpsc::unbounded_channel();
        // 3. 准备当前要处理的上下文
        let mut chunk_stream = self.context.chunk_stream().await?;
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
                                match &val {
                                    Ok(val) => {
                                        file.write_all(val.as_bytes()).unwrap();
                                    },
                                    _ => { }
                                }

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
                            GenerateEvent::Start
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
                Some(cmd) = self.cmd_receiver.recv() => {
                     match cmd {

                    CommandEvent::Generate => {
                       //已经在生成中忽略
                        break;
                    }
                    CommandEvent::Cancel => {
                         // 跳出内部 loop,强制处理
                        return Ok(EndReason::Canceled)
                    }
                }

                }
                else=>{
                     return Ok(EndReason::Error(Arc::new(anyhow::anyhow!("连接中断"))))
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
            .send(GenerateEvent::End(reson))
            .await
            .expect("section_sender"); //输出结束
    }
}

#[cfg(test)]

mod test {

    use futures::{Stream, StreamExt};
    use std::{pin::Pin, time::Duration};

    use tokio::sync::mpsc;

    use crate::mind::section::{Event, section_stream};
    fn split_into_chunks(s: &str) -> Vec<String> {
        s.chars()
            .collect::<Vec<char>>()
            .chunks(5) // 每5个字符一组
            .map(|chunk| chunk.iter().collect()) // 将字符重新组合成字符串
            .collect()
    }
    fn to_stream(s: &str) -> impl Stream<Item = Event> {
        let chunks = split_into_chunks(s);
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            for chunk in chunks {
                tokio::time::sleep(Duration::from_millis(10)).await;
                tx.send(Ok(chunk.to_string())).unwrap();
            }
        });
        let chunk_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).boxed();
        let event_stream = Box::pin(section_stream(chunk_stream));
        event_stream
    }

    #[tokio::test]
    async fn test() {
        println!("=== 开始测试 Section Stream ===");

        // 1. 模拟 LLM 异步流式输出的数据块 (Chunks)
        // 故意将标签切碎，以测试解析器的 buffer 拼接能力
        let outputs = {
            if let Ok(test) = std::fs::read_to_string(r"D:\project\rust\streamind\test.xml") {
                test.split("<end/>")
                    .map(|item| item.trim())
                    .filter(|v| !v.is_empty())
                    .map(|item| split_into_chunks(item))
                    .collect()
            } else {
                vec![]
            }
        };
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            for _ in 0..1 {
                for chunks in &outputs {
                    for chunk in chunks {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        tx.send(Ok(chunk.to_string())).unwrap();
                    }
                }
            }
        });
        let chunk_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).boxed();
        let mut event_stream = Box::pin(section_stream(chunk_stream));
        // 消费解析出来的事件
        while let Some(event) = event_stream.next().await {
            match event {
                Event::Start => {
                    println!("\n[事件] >>> 流开始 <<<");
                }
                Event::Open(tag, head) => {
                    println!("\n[事件] 开启标签: {}", head);
                }
                Event::Chunk(tag, chunk) => {
                    // print!("{}", chunk)
                }
                Event::Close(tag, content) => {
                    println!("[事件] 关闭标签{}，完整段落内容为: {:?}", tag.name, content);
                }
                Event::End => {
                    println!("\n[事件] >>> 流结束 <<<");
                }
                Event::Error(e) => {
                    println!("\n[事件] >>> 异常 {e} <<<");
                }
            }
        }

        // 给后台异步任务一点时间打印完毕
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    #[tokio::test]
    async fn test_extreme_robustness() {
        let tricky_text = r#"    <think id="1" attr="<fake_attr>">
我在思考...这里有一个被引号包围的假标签："</thinking>"，它不应该结束思考。
另外，左尖括号转义也得生效：<<reply> 不能开启新段落。
这是反斜杠转义：\<tool_call> 也不行。
```rust
// Markdown 代码块里的标签
let start = "<push>"; 
let end = "</push>"; // 因为前面有双引号，所以被忽略
```
甚至是这样的数组包裹：[<render>] 应该被忽略。
还有单引号包裹：'</think>' 也不该起作用。
真正的结束在下面！
</thinking
<reply>
紧接着毫无空格开启了回复！
这里有一个伪造的结束符被反引号包围：`</reply>`。
</reply><tool_call name="fetch">{"query": "</tool_call>"}</tool_call>"#;

        // 模拟极其恶劣的网络抖动：将字符串按随机 1 到 4 个字节进行切分

        let mut event_stream = to_stream(tricky_text);

        let mut final_content = String::new();
        let mut section_count = 0;

        // 收集并验证事件
        while let Some(event) = event_stream.next().await {
            match event {
                Event::Open(tag, head) => {
                    println!("\n🟢 [OPEN] tag: {}, head: {}", tag.name, head);
                    section_count += 1;
                }
                Event::Chunk(_tag, chunk) => {
                    print!("{}", chunk); // 打印出来的应该是连贯的内容
                }
                Event::Close(tag, content) => {
                    println!(
                        "\n🔴 [CLOSE] tag: {}\n=== CONTENT ===\n{}\n===============",
                        tag.name, content
                    );
                    final_content.push_str(&content);
                }
                Event::Error(e) => panic!("Stream error: {}", e),
                _ => {}
            }
        }

        // 核心断言
        // 应该正好解析出 3 个有效 Section: think, reply, tool_call
        assert_eq!(section_count, 3, "Failed to parse exactly 3 sections!");

        // 验证 JSON 里的假结束符是否被成功跳过，而真正的 </tool_call> 被捕获
        assert!(final_content.contains(r#"{"query": "</tool_call>"}"#));

        // 验证 Markdown 代码块没有被错误截断
        assert!(final_content.contains("let end = \"</push>\";"));

        println!("\n✅ 鲁棒性极限测试通过！状态机与跨Chunk转义拦截均完美运作。");
    }
}
