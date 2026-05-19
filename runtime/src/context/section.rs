use futures::{Stream, StreamExt};
use kernel::section::{SectionTag, TAGS};
use std::pin::Pin;

#[derive(Debug, PartialEq)]
pub enum Event {
    Start,
    Open(&'static SectionTag),
    Chunk(&'static SectionTag, String),
    Close(&'static SectionTag, String),
    End,
}

/// 流式解析状态机：彻底脱离 async_stream 宏，恢复 IDE 的语法提示
pub struct SectionParser {
    buffer: String,
    tag_seek: usize,
    current_tag: Option<&'static SectionTag>,
    is_in_reply: bool,
    tags: &'static [SectionTag],
    reply_tag: &'static SectionTag,
    safe_size: usize,
}

impl SectionParser {
    pub fn new(tags: &'static [SectionTag], safe_size: usize) -> Self {
        Self {
            buffer: String::new(),
            tag_seek: 0,
            current_tag: None,
            is_in_reply: false,
            reply_tag: &tags[0], // 假设第一个元素是默认的 Reply
            tags,
            safe_size,
        }
    }

    /// 推入新文本块，返回解析产生的一系列事件
    pub fn push(&mut self, chunk: &str) -> Vec<Event> {
        let mut events = Vec::new();
        self.buffer.push_str(chunk);

        loop {
            if let Some(tag) = self.current_tag {
                // ==========================================
                // 状态 A: 正在某个明确的标签内，寻找它的闭合符
                // ==========================================
                if let Some(pos) = self.buffer.rfind(tag.end) {
                    if self.tag_seek < pos {
                        events.push(Event::Chunk(
                            tag,
                            self.buffer[self.tag_seek..pos].to_string(),
                        ));
                    }
                    let content = self.buffer[..pos].to_string();
                    events.push(Event::Close(tag, content));

                    // 截断已处理部分，重置状态
                    self.buffer = self.buffer[pos + tag.end.len()..].to_string();
                    self.tag_seek = 0;
                    self.current_tag = None;
                    // 找到了闭合标签，继续循环处理剩余的 buffer
                    continue;
                } else {
                    // 没有找到闭合符，将安全的部分先 Chunk 出去
                    self.flush_safe_chunks(&mut events);
                    break;
                }
            } else {
                // ==========================================
                // 状态 B: 不在明确标签内（可能在空闲区或 Reply 区）
                // ==========================================
                let mut earliest_pos = None;
                let mut earliest_tag = None;

                // 寻找所有可用标签的最早起始符
                for tag in self.tags {
                    if tag.start.is_empty() {
                        continue;
                    } // 跳过无头标签（如 reply 自身）

                    if let Some(pos) = self.buffer.find(tag.start) {
                        if earliest_pos.map_or(true, |p| pos < p) {
                            earliest_pos = Some(pos);
                            earliest_tag = Some(tag);
                        }
                    }
                }

                if let Some(tag) = earliest_tag {
                    let pos = earliest_pos.unwrap();
                    let pre_text = &self.buffer[..pos];

                    // 关键修复：判断标签前的内容是否纯空白。如果是纯空白且不在 Reply 中，则直接丢弃（满足要求）
                    if !pre_text.trim_start().is_empty() || self.is_in_reply {
                        if !self.is_in_reply {
                            events.push(Event::Open(self.reply_tag));
                        }
                        if self.tag_seek < pos {
                            events.push(Event::Chunk(
                                self.reply_tag,
                                self.buffer[self.tag_seek..pos].to_string(),
                            ));
                        }
                        let content = self.buffer[..pos].to_string();
                        events.push(Event::Close(self.reply_tag, content));
                    }

                    // 重置 Reply 状态，开启新标签
                    self.is_in_reply = false;
                    self.buffer = self.buffer[pos + tag.start.len()..].to_string();
                    self.tag_seek = 0;
                    self.current_tag = Some(tag);
                    events.push(Event::Open(tag));
                    continue;
                } else {
                    // 如果没找到任何标签起始符：
                    // 关键修复：如果当前不在 Reply 里，则所有打头的换行、空格全部清空抛弃！
                    if !self.is_in_reply {
                        let trimmed = self.buffer.trim_start();
                        if trimmed.len() < self.buffer.len() {
                            self.buffer = trimmed.to_string();
                            self.tag_seek = 0;
                        }
                        if self.buffer.is_empty() {
                            break; // 全是空白且被清空了，直接等待下一个 Chunk
                        }
                    }

                    // buffer 中存在无法构成完整 tag 的文本，安全输出 Chunk
                    self.flush_safe_chunks(&mut events);
                    break;
                }
            }
        }
        events
    }

    /// 安全窗口流式输出
    fn flush_safe_chunks(&mut self, events: &mut Vec<Event>) {
        let safe_len = self.buffer.len().saturating_sub(self.tag_seek);
        if safe_len > self.safe_size {
            let mut offset = self.buffer.len() - self.safe_size;

            // 防止把 UTF-8 字符切断
            while offset > self.tag_seek && !self.buffer.is_char_boundary(offset) {
                offset -= 1;
            }

            if offset > self.tag_seek {
                let chunk_text = self.buffer[self.tag_seek..offset].to_string();

                if let Some(tag) = self.current_tag {
                    events.push(Event::Chunk(tag, chunk_text));
                } else {
                    if !self.is_in_reply {
                        events.push(Event::Open(self.reply_tag));
                        self.is_in_reply = true;
                    }
                    events.push(Event::Chunk(self.reply_tag, chunk_text));
                }
                self.tag_seek = offset;
            }
        }
    }

    /// 处理流结束时剩余的数据
    pub fn finish(&mut self) -> Vec<Event> {
        let mut events = Vec::new();

        if let Some(tag) = self.current_tag {
            if self.tag_seek < self.buffer.len() {
                events.push(Event::Chunk(tag, self.buffer[self.tag_seek..].to_string()));
            }
            events.push(Event::Close(tag, self.buffer.clone()));
        } else {
            let remain_trimmed = if self.is_in_reply {
                &self.buffer[..] // 如果已经在 reply 中了，结尾的空格就不吃掉了
            } else {
                self.buffer.trim_start()
            };

            if !remain_trimmed.is_empty() || self.is_in_reply {
                if !self.is_in_reply {
                    events.push(Event::Open(self.reply_tag));
                }
                if self.tag_seek < self.buffer.len() {
                    events.push(Event::Chunk(
                        self.reply_tag,
                        self.buffer[self.tag_seek..].to_string(),
                    ));
                }
                events.push(Event::Close(self.reply_tag, self.buffer.clone()));
            }
        }

        events.push(Event::End);
        events
    }
}

/// 极简的对外宏接口：仅负责做 await 转接
pub fn section_stream(
    mut chunk_stream: Pin<Box<dyn Stream<Item = String> + Send>>,
) -> impl Stream<Item = Event> {
    // safe_size 设置为 20（需大于等于最长闭合标签长度）
    let mut parser = SectionParser::new(&*TAGS, 20);

    async_stream::stream! {
        yield Event::Start;

        while let Some(chunk) = chunk_stream.next().await {
            // 调用纯同步的状态机进行推入，立刻得到产生的事件
            for event in parser.push(&chunk) {
                yield event;
            }
        }

        // 处理最后的收尾部分
        for event in parser.finish() {
            yield event;
        }
    }
}
#[cfg(test)]

mod test {
    use super::Event;
    use futures::StreamExt;
    use std::time::Duration;

    use tokio::sync::mpsc;

    use crate::context::{section::section_stream, stack::split_into_chunks};

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
            for _ in 0..100 {
                for chunks in &outputs {
                    for chunk in chunks {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        tx.send(chunk.to_string()).unwrap();
                    }
                }
            }
        });
        let chunk_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).boxed();
        // 3. 调用你的 section_stream 函数
        let mut event_stream = Box::pin(section_stream(chunk_stream));

        // 4. 消费解析出来的事件
        while let Some(event) = event_stream.next().await {
            match event {
                Event::Start => {
                    println!("\n[事件] >>> 流开始 <<<");
                }
                Event::Open(tag) => {
                    println!("\n[事件] 开启标签: {}", tag.start);
                }
                Event::Chunk(tag, chunk) => {
                    // print!("{}", chunk)
                }
                Event::Close(tag, content) => {
                    println!("[事件] 关闭标签{}，完整段落内容为: {:?}", tag.end, content);
                }
                Event::End => {
                    println!("\n[事件] >>> 流结束 <<<");
                }
            }
        }

        // 给后台异步任务一点时间打印完毕
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
