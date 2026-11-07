use futures::{Stream, StreamExt};
use std::pin::Pin;

// ==========================================
// 1. 配置模块 (config)
// ==========================================
pub mod config {
    use std::sync::LazyLock;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SectionTag {
        pub name: &'static str,
        pub start: &'static str,
        pub ends: &'static [&'static str],
    }

    impl SectionTag {
        pub fn new(name: &'static str, start: &'static str, ends: &'static [&'static str]) -> Self {
            SectionTag { name, start, ends }
        }
    }

    pub static TAGS: LazyLock<Vec<SectionTag>> = LazyLock::new(|| {
        vec![
            SectionTag::new("reply", "", &["</reply>", "</response>"]),
            SectionTag::new(
                "reply",
                "<reply",
                &["</reply>", "</response>", "</reply>\n</reply>","</reply>\n</think>"],
            ),
            SectionTag::new(
                "think",
                "<think",
                &[
                    "</think>",
                    "</thought>",
                    "</thinking>",
                    "</thinking\n",
                    "</think\n</think>",
                    "</think></think>",
                    "</reply></think>",
                    "</think></reply>",
                    "</think>\n</think>",
                    "</think>\n</reply>",
                    "</reply>\n</think>",
                ],
            ),
            SectionTag::new("tool_call", "<tool_call", &["</tool_call>"]),
            SectionTag::new("recall", "<recall", &["</recall>"]),
            SectionTag::new("render", "<render", &["</render>"]),
            SectionTag::new("push", "<push", &["</push>"]),
            SectionTag::new("pop", "<pop", &["</pop>"]),
            SectionTag::new("reset", "<reset", &["</reset>"]),
        ]
    });

    // 遇到这些字符作为标签前缀时，忽略标签解析
    pub static ESCAPE_CHARS: LazyLock<Vec<char>> = LazyLock::new(|| {
        vec![
            '`', '\'', '"', '*', '\\', '(', '[', '{', '<', '_', '&', '|', '#', '@', '=', '%', '$',
            '^', '!', '~', '?',
        ]
    });
}

// ==========================================
// 2. 事件与结构模块 (events)
// ==========================================
pub mod events {
    use super::config::SectionTag;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Section {
        pub id: i64,
        pub name: String,
        pub header: String,
        pub content: String,
        pub elapsed: String,
    }

    impl Section {
        pub fn to_string(&self) -> String {
            format!(
                "{}\n{}\n</{}>",
                self.header.trim(),
                self.content.trim(),
                self.name
            )
        }
    }

    #[derive(Debug)]
    pub enum Event {
        Start,
        Open(&'static SectionTag, String),
        Chunk(&'static SectionTag, String),
        Close(&'static SectionTag, String),
        End,
        Error(anyhow::Error),
    }
}

// ==========================================
// 3. 缓冲区模块 (buffer)
// 专门负责处理文本提取、安全边界及转义判断
// ==========================================
pub mod buffer {
    pub struct ParseBuffer {
        buffer: String,
        /// 记录被消耗掉的最后单个字符，用于跨 Chunk 的转义字符判断
        last_char: Option<char>,
        safe_size: usize,
    }

    impl ParseBuffer {
        pub fn new(safe_size: usize) -> Self {
            Self {
                buffer: String::new(),
                last_char: None,
                safe_size,
            }
        }

        pub fn push_str(&mut self, chunk: &str) {
            self.buffer.push_str(chunk);
        }

        pub fn as_str(&self) -> &str {
            &self.buffer
        }

        pub fn len(&self) -> usize {
            self.buffer.len()
        }

        pub fn is_empty(&self) -> bool {
            self.buffer.is_empty()
        }
        // 【新增方法】寻找多个模式字符串中最早未被转义的那个
        pub fn find_earliest_unescaped<'a>(
            &self,
            patterns: &[&'a str],
            escape_chars: &[char],
        ) -> Option<(usize, &'a str)> {
            let mut earliest: Option<(usize, &'a str)> = None;

            for &pattern in patterns {
                if pattern.is_empty() {
                    continue;
                } // 防御性判断

                // 复用之前的忽略转义查找
                if let Some(pos) = self.find_unescaped(pattern, escape_chars) {
                    match earliest {
                        Some((p, prev_pat)) => {
                            // 如果出现得更早，或者位置相同但匹配项更长（优先最长匹配，避免前缀冲突）
                            if pos < p || (pos == p && pattern.len() > prev_pat.len()) {
                                earliest = Some((pos, pattern));
                            }
                        }
                        None => earliest = Some((pos, pattern)),
                    }
                }
            }
            earliest
        }
        /// 提取 `0..pos` 的内容，并自动记录提取结果的最后一个字符作为下一个阶段的 `last_char`
        pub fn drain_up_to(&mut self, pos: usize) -> String {
            if pos > 0 {
                // 仅在提取时记录，避免完整的字符串 O(N) 遍历
                if let Some(c) = self.buffer[..pos].chars().next_back() {
                    self.last_char = Some(c);
                }
            }
            self.buffer.drain(..pos).collect()
        }

        /// 寻找模式字符串，但是自动忽略被 escape_chars 转义的匹配项
        pub fn find_unescaped(&self, pattern: &str, escape_chars: &[char]) -> Option<usize> {
            let mut start = 0;
            while let Some(pos) = self.buffer[start..].find(pattern) {
                let actual_pos = start + pos;

                // 判断前一个字符是什么 (如果在当前 chunk 首部，则使用跨 chunk 记录的 last_char)
                let prev_char = if actual_pos > 0 {
                    self.buffer[..actual_pos].chars().next_back()
                } else {
                    self.last_char
                };

                // 如果前置字符存在并且是转义字符，跳过此匹配继续往后找
                if let Some(c) = prev_char {
                    if escape_chars.contains(&c) {
                        start = actual_pos + pattern.len(); // 步进整个 pattern，跳过伪边界
                        continue;
                    }
                }
                return Some(actual_pos); // 未被转义的正常匹配
            }
            None
        }

        /// 原生 find，用于查找不需要转义判断的符号（如头部标签闭合符 `>`）
        pub fn find(&self, pattern: char) -> Option<usize> {
            self.buffer.find(pattern)
        }

        /// 计算保留 safe_size 后，安全的 UTF-8 截断点
        pub fn get_safe_drain_pos(&self) -> Option<usize> {
            if self.buffer.len() <= self.safe_size {
                return None;
            }
            let mut pos = self.buffer.len() - self.safe_size;
            while pos > 0 && !self.buffer.is_char_boundary(pos) {
                pos -= 1;
            }
            if pos == 0 { None } else { Some(pos) }
        }

        pub fn clear_and_update_last_char(&mut self) {
            if !self.buffer.is_empty() {
                if let Some(c) = self.buffer.chars().next_back() {
                    self.last_char = Some(c);
                }
                self.buffer.clear();
            }
        }
    }
}

// ==========================================
// 4. 解析器状态机模块 (parser)
// 纯逻辑流转，解耦字符串操作
// ==========================================
pub mod parser {
    use super::buffer::ParseBuffer;
    use super::config::{ESCAPE_CHARS, SectionTag};
    use super::events::Event;
    use std::mem::take;

    #[derive(Debug)]
    enum State {
        Idle,
        ParsingTagHead(&'static SectionTag),
        InsideTag(&'static SectionTag),
    }

    pub struct SectionParser {
        tags: &'static [SectionTag],
        reply_tag: &'static SectionTag,
        buffer: ParseBuffer,
        content: String,
        state: State,
        is_in_reply: bool,
    }

    impl SectionParser {
        pub fn new(tags: &'static [SectionTag], safe_size: usize) -> Self {
            Self {
                buffer: ParseBuffer::new(safe_size),
                content: String::new(),
                state: State::Idle,
                is_in_reply: false,
                reply_tag: &tags[0], // 默认 Reply 标签
                tags,
            }
        }

        pub fn push(&mut self, chunk: &str) -> Vec<Event> {
            self.buffer.push_str(chunk);
            let mut events = Vec::new();

            loop {
                let state_changed = match self.state {
                    State::Idle => self.process_idle(&mut events),
                    State::ParsingTagHead(tag) => self.process_tag_head(tag, &mut events),
                    State::InsideTag(tag) => self.process_inside_tag(tag, &mut events),
                };
                if !state_changed {
                    break;
                }
            }

            self.flush_safe_chunks(&mut events);
            events
        }

        fn process_idle(&mut self, events: &mut Vec<Event>) -> bool {
            let mut earliest = None;

            for tag in self.tags {
                if tag.start.is_empty() {
                    continue;
                }
                // 使用封装好的 buffer 方法，忽略处于转义状态的匹配
                if let Some(pos) = self.buffer.find_unescaped(tag.start, &*ESCAPE_CHARS) {
                    if earliest.map_or(true, |(p, _)| pos < p) {
                        earliest = Some((pos, tag));
                    }
                }
            }

            if let Some((pos, tag)) = earliest {
                let pre_text = self.buffer.drain_up_to(pos);

                // 如果此时已经在 Reply 状态或发现有效前置文本，关闭上一个回复段落
                if self.is_in_reply || !pre_text.trim_start().is_empty() {
                    if !self.is_in_reply {
                        events.push(Event::Open(self.reply_tag, "".to_string()));
                    }
                    self.content.push_str(&pre_text);
                    events.push(Event::Close(self.reply_tag, take(&mut self.content)));
                }

                self.is_in_reply = false;
                self.buffer.drain_up_to(tag.start.len()); // 消耗 <tag
                self.state = State::ParsingTagHead(tag);
                true
            } else {
                // 不处于 Reply 且全是空白则丢弃
                if !self.is_in_reply && self.buffer.as_str().trim_start().is_empty() {
                    self.buffer.clear_and_update_last_char();
                }
                false
            }
        }

        fn process_tag_head(&mut self, tag: &'static SectionTag, events: &mut Vec<Event>) -> bool {
            // '>' 不受常规内容前置字符的转义影响，直接匹配
            if let Some(pos) = self.buffer.find('>') {
                let attrs = self.buffer.drain_up_to(pos);
                let full_head = format!("{}{}>", tag.start, attrs);

                events.push(Event::Open(tag, full_head));
                self.buffer.drain_up_to(1); // 消耗 '>'
                self.state = State::InsideTag(tag);
                true
            } else {
                false
            }
        }

        fn process_inside_tag(
            &mut self,
            tag: &'static SectionTag,
            events: &mut Vec<Event>,
        ) -> bool {
            // tag.ends 现在是一个数组，查找最先匹配到的闭合标签
            if let Some((pos, matched_end)) = self
                .buffer
                .find_earliest_unescaped(tag.ends, &*ESCAPE_CHARS)
            {
                let chunk = self.buffer.drain_up_to(pos);
                self.content.push_str(&chunk);

                events.push(Event::Chunk(tag, chunk));
                events.push(Event::Close(tag, take(&mut self.content)));

                // 根据真实匹配到的标签长度进行消耗
                self.buffer.drain_up_to(matched_end.len());
                self.state = State::Idle;
                true
            } else {
                false
            }
        }
        fn flush_safe_chunks(&mut self, events: &mut Vec<Event>) {
            let split_pos = match self.buffer.get_safe_drain_pos() {
                Some(p) => p,
                None => return,
            };

            match self.state {
                State::Idle => {
                    if !self.is_in_reply {
                        let trim_len =
                            self.buffer.as_str().len() - self.buffer.as_str().trim_start().len();
                        if trim_len > 0 {
                            self.buffer.drain_up_to(trim_len);
                        }
                        if self.buffer.is_empty() {
                            return;
                        }

                        self.is_in_reply = true;
                        events.push(Event::Open(self.reply_tag, "".to_string()));
                    }

                    // trim 后 buffer 长度可能变化，重新计算安全的截断点
                    if let Some(safe_pos) = self.buffer.get_safe_drain_pos() {
                        let chunk = self.buffer.drain_up_to(safe_pos);
                        self.content.push_str(&chunk);
                        events.push(Event::Chunk(self.reply_tag, chunk));
                    }
                }
                State::InsideTag(tag) => {
                    let chunk = self.buffer.drain_up_to(split_pos);
                    self.content.push_str(&chunk);
                    events.push(Event::Chunk(tag, chunk));
                }
                State::ParsingTagHead(_) => {}
            }
        }

        pub fn finish(&mut self) -> Vec<Event> {
            let mut events = Vec::new();
            let buf_len = self.buffer.len();

            match self.state {
                State::Idle => {
                    let remainder = if self.is_in_reply {
                        self.buffer.drain_up_to(buf_len)
                    } else {
                        self.buffer.drain_up_to(buf_len).trim_start().to_string()
                    };

                    if !remainder.is_empty() || self.is_in_reply {
                        if !self.is_in_reply {
                            events.push(Event::Open(self.reply_tag, "".to_string()));
                        }
                        self.content.push_str(&remainder);
                        events.push(Event::Close(self.reply_tag, take(&mut self.content)));
                    }
                }
                State::ParsingTagHead(tag) => {
                    let remainder = self.buffer.drain_up_to(buf_len);
                    let full_head = format!("{}{}", tag.start, remainder);
                    events.push(Event::Open(tag, full_head));
                    events.push(Event::Close(tag, take(&mut self.content)));
                }
                State::InsideTag(tag) => {
                    let remainder = self.buffer.drain_up_to(buf_len);
                    self.content.push_str(&remainder);
                    events.push(Event::Close(tag, take(&mut self.content)));
                }
            }

            self.buffer.clear_and_update_last_char();
            self.state = State::Idle;
            self.is_in_reply = false;
            events.push(Event::End);

            events
        }
    }
}

// ==========================================
// 5. Stream 调用接口 (stream)
// ==========================================
pub mod stream {
    use crate::mind::context::ChunkStream;

    use super::config::TAGS;
    use super::events::Event;
    use super::parser::SectionParser;
    use futures::{Stream, StreamExt};
    use std::pin::Pin;

    pub fn section_stream(mut chunk_stream: ChunkStream) -> impl Stream<Item = Event> + Send {
        let mut parser = SectionParser::new(&*TAGS, 20);

        async_stream::stream! {
            yield Event::Start;

            while let Some(chunk) = chunk_stream.next().await {
                match chunk {
                    Ok(chunk) => {
                        for event in parser.push(&chunk) {
                            yield event;
                        }
                    },
                    Err(err) => {
                        yield Event::Error(err);
                    }
                }
            }

            for event in parser.finish() {
                yield event;
            }
        }
    }
}

// 导出对外使用的主要结构，方便其他文件调用
pub use config::{ESCAPE_CHARS, SectionTag, TAGS};
pub use events::{Event, Section};
pub use parser::SectionParser;
pub use stream::section_stream;
#[cfg(test)]
mod tests {
    use super::{TAGS, events::Event, parser::SectionParser};

    /// 把事件序列压成便于断言的字符串
    fn trace(events: Vec<Event>) -> Vec<String> {
        events
            .into_iter()
            .map(|e| match e {
                Event::Start => "start".to_string(),
                Event::Open(tag, head) => format!("open:{}:{}", tag.name, head),
                Event::Chunk(tag, chunk) => format!("chunk:{}:{}", tag.name, chunk),
                Event::Close(tag, content) => format!("close:{}:{}", tag.name, content),
                Event::End => "end".to_string(),
                Event::Error(e) => format!("error:{e}"),
            })
            .collect()
    }

    fn feed(chunks: &[&str]) -> Vec<String> {
        let mut parser = SectionParser::new(&TAGS, 20);
        let mut out = Vec::new();
        for chunk in chunks {
            out.extend(trace(parser.push(chunk)));
        }
        out.extend(trace(parser.finish()));
        out
    }

    /// 只取所有 Close 事件（分节最终内容）
    fn closed(events: &[String]) -> Vec<String> {
        events
            .iter()
            .filter(|e| e.starts_with("close:"))
            .cloned()
            .collect()
    }

    #[test]
    fn plain_text_becomes_reply() {
        let events = feed(&["hello"]);
        assert_eq!(
            events,
            vec!["open:reply:", "close:reply:hello", "end"],
            "没有标签的纯文本应当作为 reply 分节输出"
        );
    }

    #[test]
    fn think_section_is_split_out() {
        let events = feed(&["<think>let me see</think>"]);
        assert_eq!(
            events,
            vec![
                "open:think:<think>",
                "chunk:think:let me see",
                "close:think:let me see",
                "end",
            ]
        );
    }

    #[test]
    fn tag_split_across_chunks() {
        // 标签头被切到两个 chunk 里也必须能识别
        let events = feed(&["<thi", "nk>abc</think>"]);
        assert_eq!(
            events,
            vec![
                "open:think:<think>",
                "chunk:think:abc",
                "close:think:abc",
                "end",
            ]
        );
    }

    #[test]
    fn tool_call_head_keeps_attributes() {
        let events = feed(&[r#"<tool_call id="a1">timer.sleep(time="1m")</tool_call>"#]);
        assert_eq!(events[0], r#"open:tool_call:<tool_call id="a1">"#);
        assert_eq!(events[1], r#"chunk:tool_call:timer.sleep(time="1m")"#);
        assert_eq!(events[2], r#"close:tool_call:timer.sleep(time="1m")"#);
    }

    #[test]
    fn escaped_tag_is_plain_text() {
        // 被反引号包裹的 <think> 只是在描述协议，不能真的开启分节
        let events = feed(&["use `<think>` in prompt"]);
        assert!(
            events.iter().all(|e| !e.starts_with("open:think")),
            "转义后的标签不应生效: {events:?}"
        );
        assert!(
            events.iter().any(|e| e.contains("`<think>`")),
            "标签应原样保留在文本中: {events:?}"
        );
    }

    #[test]
    fn text_before_tag_becomes_reply() {
        let events = feed(&["你好<reply>世界</reply>"]);
        assert_eq!(closed(&events), vec!["close:reply:你好", "close:reply:世界"]);
        assert_eq!(events.last().unwrap(), "end");
    }

    #[test]
    fn finish_closes_unterminated_section() {
        // 模型没写闭合标签就结束流，也要把内容吐出来
        let events = feed(&["<think>abc"]);
        assert_eq!(
            events,
            vec![
                "open:think:<think>",
                "close:think:abc",
                "end",
            ]
        );
    }

    #[test]
    fn parser_is_reusable_after_finish() {
        let mut parser = SectionParser::new(&TAGS, 20);

        let mut first = trace(parser.push("<think>a</think>"));
        first.extend(trace(parser.finish()));
        assert_eq!(
            first,
            vec!["open:think:<think>", "chunk:think:a", "close:think:a", "end"]
        );

        // finish 之后状态应完全重置，可以接着解析下一个流
        let mut second = trace(parser.push("<reply>b</reply>"));
        second.extend(trace(parser.finish()));
        assert_eq!(
            second,
            vec!["open:reply:<reply>", "chunk:reply:b", "close:reply:b", "end"]
        );
    }
}
