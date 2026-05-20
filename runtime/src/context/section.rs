use futures::{Stream, StreamExt};
use kernel::section::{SectionTag, TAGS};
use std::{mem::take, pin::Pin};

#[derive(Debug, PartialEq)]
pub enum Event {
    Start,
    /// 包含 (标签配置, 完整的标签头文本，例如 `<thought attr="1">`)
    Open(&'static SectionTag, String),
    /// 包含 (标签配置, 块状内容)
    Chunk(&'static SectionTag, String),
    /// 包含 (标签配置, 最后的剩余内容)
    Close(&'static SectionTag, String),
    End,
}

/// 解析器当前所处的状态
#[derive(Debug)]
enum State {
    /// 状态 A: 不在明确标签内，正在处理回复文本或忽略空白
    Idle,
    /// 状态 B: 已经匹配到 `<tag`，正在寻找 `>` 以完成 Open 事件
    ParsingTagHead(&'static SectionTag),
    /// 状态 C: 正在标签内部，收集 Chunk，寻找结束符
    InsideTag(&'static SectionTag),
}

pub struct SectionParser {
    tags: &'static [SectionTag],
    reply_tag: &'static SectionTag,
    safe_size: usize,

    buffer: String,
    content: String,

    state: State,
    is_in_reply: bool,
}

impl SectionParser {
    pub fn new(tags: &'static [SectionTag], safe_size: usize) -> Self {
        Self {
            buffer: String::new(),
            content: String::new(),
            state: State::Idle,
            is_in_reply: false,
            reply_tag: &tags[0], // 默认 Reply 标签
            tags,
            safe_size,
        }
    }

    /// 推入新文本块，返回解析产生的一系列事件
    pub fn push(&mut self, chunk: &str) -> Vec<Event> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        // 状态机流转
        loop {
            let state_changed = match self.state {
                State::Idle => self.process_idle(&mut events),
                State::ParsingTagHead(tag) => self.process_tag_head(tag, &mut events),
                State::InsideTag(tag) => self.process_inside_tag(tag, &mut events),
            };

            // 如果当前 buffer 数据不足以推进状态，退出循环等待下一次 push
            if !state_changed {
                break;
            }
        }

        // 状态机处理完后，将安全窗口之外的文本发送出去
        self.flush_safe_chunks(&mut events);
        events
    }

    // ==========================================
    // 状态处理器 (模块化拆分)
    // ==========================================

    /// 寻找下一个起始标签，并处理中间的空白或 Reply 文本
    fn process_idle(&mut self, events: &mut Vec<Event>) -> bool {
        let mut earliest = None;

        // 寻找最早出现的有效标签起始符
        for tag in self.tags {
            if tag.start.is_empty() {
                continue;
            } // 跳过 reply 自身
            if let Some(pos) = self.buffer.find(tag.start) {
                if earliest.map_or(true, |(p, _)| pos < p) {
                    earliest = Some((pos, tag));
                }
            }
        }

        if let Some((pos, tag)) = earliest {
            let pre_text = &self.buffer[..pos];

            // 核心逻辑：如果非纯空白或已经在 Reply 中，则算作 Reply 闭合
            if self.is_in_reply || !pre_text.trim_start().is_empty() {
                if !self.is_in_reply {
                    events.push(Event::Open(self.reply_tag, "".to_string()));
                }
                take(&mut self.content);
                events.push(Event::Close(self.reply_tag, pre_text.to_string()));
            }

            self.is_in_reply = false;
            // 消耗掉前置文本和标签起始符 (例如 `<think`)
            self.buffer.drain(..pos + tag.start.len());
            self.state = State::ParsingTagHead(tag);
            true
        } else {
            // 没有找到标签。如果不处于 Reply 且全是空白，直接清空丢弃
            if !self.is_in_reply && self.buffer.trim_start().is_empty() {
                self.buffer.clear();
            }
            false
        }
    }

    /// 已经找到了 `<tag`，继续寻找 `>` 取出整个标签头
    fn process_tag_head(&mut self, tag: &'static SectionTag, events: &mut Vec<Event>) -> bool {
        if let Some(pos) = self.buffer.find('>') {
            let attrs = &self.buffer[..pos];
            let full_head = format!("{}{}>", tag.start, attrs); // 还原完整头部 <tag attr="1">

            events.push(Event::Open(tag, full_head));

            // 消耗到 `>` 之后
            self.buffer.drain(..pos + 1);
            self.state = State::InsideTag(tag);
            true
        } else {
            false
        }
    }

    /// 在标签内部，寻找闭合符
    fn process_inside_tag(&mut self, tag: &'static SectionTag, events: &mut Vec<Event>) -> bool {
        if let Some(pos) = self.buffer.find(tag.end) {
            let chunk = self.buffer[..pos].to_string();
            self.content.push_str(&chunk);
            events.push(Event::Chunk(tag, chunk));
            events.push(Event::Close(tag, take(&mut self.content)));

            // 消耗到闭合符之后
            self.buffer.drain(..pos + tag.end.len());
            self.state = State::Idle;
            true
        } else {
            false
        }
    }

    // ==========================================
    // 缓冲与清理逻辑
    // ==========================================

    /// 安全输出窗口外的 Chunk
    fn flush_safe_chunks(&mut self, events: &mut Vec<Event>) {
        if self.buffer.len() <= self.safe_size {
            return;
        }

        let split_pos = self.find_char_boundary(self.buffer.len() - self.safe_size);
        if split_pos == 0 {
            return;
        }

        match self.state {
            State::Idle => {
                if !self.is_in_reply {
                    let trim_len = self.buffer.len() - self.buffer.trim_start().len();
                    if trim_len > 0 {
                        self.buffer.drain(..trim_len);
                    }
                    if self.buffer.is_empty() {
                        return;
                    }

                    self.is_in_reply = true;
                    events.push(Event::Open(self.reply_tag, "".to_string()));
                }

                // 此时已经处于 Reply 状态，执行 Chunk 输出
                let safe_pos =
                    self.find_char_boundary(self.buffer.len().saturating_sub(self.safe_size));
                if safe_pos > 0 {
                    let chunk: String = self.buffer.drain(..safe_pos).collect();
                    self.content.push_str(&chunk);
                    events.push(Event::Chunk(self.reply_tag, chunk));
                }
            }
            State::InsideTag(tag) => {
                let chunk: String = self.buffer.drain(..split_pos).collect();
                self.content.push_str(&chunk);
                events.push(Event::Chunk(tag, chunk));
            }
            State::ParsingTagHead(_) => {
                // 解析标签头期间（寻找 '>'），不输出 Chunk 避免截断头部
            }
        }
    }

    /// 处理流结束时剩余的数据
    pub fn finish(&mut self) -> Vec<Event> {
        let mut events = Vec::new();

        match self.state {
            State::Idle => {
                let remainder = if self.is_in_reply {
                    self.buffer.clone()
                } else {
                    self.buffer.trim_start().to_string()
                };

                if !remainder.is_empty() || self.is_in_reply {
                    if !self.is_in_reply {
                        events.push(Event::Open(self.reply_tag, "".to_string()));
                    }
                    events.push(Event::Close(self.reply_tag, take(&mut self.content)));
                }
            }
            State::ParsingTagHead(tag) => {
                // 流异常中断在标签头中，降级为直接 Close
                let full_head = format!("{}{}", tag.start, self.buffer);
                events.push(Event::Open(tag, full_head));
                events.push(Event::Close(tag, take(&mut self.content)));
            }
            State::InsideTag(tag) => {
                events.push(Event::Close(tag, take(&mut self.content)));
            }
        }

        self.buffer.clear();
        self.state = State::Idle;
        self.is_in_reply = false;
        events.push(Event::End);

        events
    }

    /// 辅助方法：寻找安全的 UTF-8 切断点
    fn find_char_boundary(&self, mut pos: usize) -> usize {
        while pos > 0 && !self.buffer.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
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
            for _ in 0..1 {
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
                Event::Open(tag, head) => {
                    println!("\n[事件] 开启标签: {}", head);
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
