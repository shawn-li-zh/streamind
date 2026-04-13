#[derive(Debug, PartialEq)]
pub enum StreamEvent {
    OutsideText(String), // 标签外部的普通文本
    TagStart,            // 匹配到开始标签
    InsideText(String),  // 标签内部的内容文本
    TagEnd,              // 匹配到结束标签
}

pub struct StreamTagParser {
    tag_name: String,
    start_tag: String,
    end_tag: String,
    buffer: String,
    is_inside: bool,
}

impl StreamTagParser {
    pub fn new(tag_name: &str) -> Self {
        Self {
            tag_name: tag_name.to_string(),
            start_tag: format!("<{}>", tag_name),
            end_tag: format!("</{}>", tag_name),
            buffer: String::new(),
            is_inside: false,
        }
    }

    /// 推入新的流式数据块，返回解析出的事件列表
    pub fn push(&mut self, chunk: &str) -> Vec<StreamEvent> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        loop {
            if !self.is_inside {
                // 在标签外部，寻找开始标签
                if let Some(pos) = self.buffer.find(&self.start_tag) {
                    // 找到了完整的开始标签
                    if pos > 0 {
                        events.push(StreamEvent::OutsideText(self.buffer[..pos].to_string()));
                    }
                    events.push(StreamEvent::TagStart);
                    self.buffer = self.buffer[pos + self.start_tag.len()..].to_string();
                    self.is_inside = true;
                } else {
                    // 没找到完整的，检查末尾是否有被截断的潜在标签 (例如 "<thi")
                    if let Some(partial_idx) =
                        self.find_partial_match(&self.buffer, &self.start_tag)
                    {
                        if partial_idx > 0 {
                            events.push(StreamEvent::OutsideText(
                                self.buffer[..partial_idx].to_string(),
                            ));
                        }
                        self.buffer = self.buffer[partial_idx..].to_string(); // 保留截断部分等待下一个 chunk
                    } else {
                        // 完全没有标签的影子，全部作为普通文本输出
                        if !self.buffer.is_empty() {
                            events.push(StreamEvent::OutsideText(self.buffer.clone()));
                            self.buffer.clear();
                        }
                    }
                    break;
                }
            } else {
                // 在标签内部，寻找结束标签
                if let Some(pos) = self.buffer.find(&self.end_tag) {
                    if pos > 0 {
                        events.push(StreamEvent::InsideText(self.buffer[..pos].to_string()));
                    }
                    events.push(StreamEvent::TagEnd);
                    self.buffer = self.buffer[pos + self.end_tag.len()..].to_string();
                    self.is_inside = false;
                } else {
                    if let Some(partial_idx) = self.find_partial_match(&self.buffer, &self.end_tag)
                    {
                        if partial_idx > 0 {
                            events.push(StreamEvent::InsideText(
                                self.buffer[..partial_idx].to_string(),
                            ));
                        }
                        self.buffer = self.buffer[partial_idx..].to_string();
                    } else {
                        if !self.buffer.is_empty() {
                            events.push(StreamEvent::InsideText(self.buffer.clone()));
                            self.buffer.clear();
                        }
                    }
                    break;
                }
            }
        }
        events
    }

    /// 流结束时调用，清空并输出最后残余的缓冲区内容
    pub fn flush(&mut self) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        if !self.buffer.is_empty() {
            if self.is_inside {
                events.push(StreamEvent::InsideText(self.buffer.clone()));
            } else {
                events.push(StreamEvent::OutsideText(self.buffer.clone()));
            }
            self.buffer.clear();
        }
        events
    }

    /// 核心逻辑：检查 buffer 尾部是否有可能是一个未完成的 target 标签
    fn find_partial_match(&self, buf: &str, target: &str) -> Option<usize> {
        for (idx, _) in buf.char_indices() {
            if buf[idx..].starts_with('<') {
                let remainder = &buf[idx..];
                if target.starts_with(remainder) {
                    return Some(idx);
                }
            }
        }
        None
    }
}

// === 测试用例 ===
#[test]
fn test_stream_tag_parser() {
    let mut parser = StreamTagParser::new("think");
    let chunks = vec![
        "Hello ",
        "<th",
        "ink>This is ",
        "thought",
        "</th",
        "ink> World",
    ];

    for chunk in chunks {
        let events = parser.push(chunk);
        for event in events {
            println!("{:?}", event);
        }
    }
}

use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum AdvStreamEvent {
    Text(String),                                // 普通文本
    TagStart { attrs: HashMap<String, String> }, // 开始标签，包含解析出的属性
    TagContent(String),                          // 标签内部内容
    TagEnd,                                      // 结束标签
}

#[derive(PartialEq)]
enum State {
    Outside,
    InStartTag, // 遇到了 `<tag ` 但还没遇到 `>`
    Inside,
}

pub struct AdvStreamParser {
    tag_name: String,
    end_tag: String,
    state: State,
    buffer: String,
    attr_regex: Regex,
}

impl AdvStreamParser {
    pub fn new(tag_name: &str) -> Self {
        Self {
            tag_name: tag_name.to_string(),
            end_tag: format!("</{}>", tag_name),
            state: State::Outside,
            buffer: String::new(),
            // 用于匹配 key="value" 或 key='value' 的正则
            attr_regex: Regex::new(r#"([a-zA-Z0-9_-]+)=["']([^"']+)["']"#).unwrap(),
        }
    }

    pub fn push(&mut self, chunk: &str) -> Vec<AdvStreamEvent> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();

        loop {
            match self.state {
                State::Outside => {
                    let strict_start = format!("<{}>", self.tag_name);
                    let prefix_start = format!("<{} ", self.tag_name);

                    // 查找 "<tag>" 或 "<tag "
                    let find_strict = self.buffer.find(&strict_start);
                    let find_prefix = self.buffer.find(&prefix_start);

                    let (found_pos, is_strict) = match (find_strict, find_prefix) {
                        (Some(s), Some(p)) => if s < p { (Some(s), true) } else { (Some(p), false) },
                        (Some(s), None) => (Some(s), true),
                        (None, Some(p)) => (Some(p), false),
                        (None, None) => (None, false),
                    };

                    if let Some(pos) = found_pos {
                        if pos > 0 {
                            events.push(AdvStreamEvent::Text(self.buffer[..pos].to_string()));
                        }
                        if is_strict {
                            // 无属性标签：<tag>
                            events.push(AdvStreamEvent::TagStart { attrs: HashMap::new() });
                            self.buffer = self.buffer[pos + strict_start.len()..].to_string();
                            self.state = State::Inside;
                        } else {
                            // 带属性标签前缀：<tag ... 等待 >
                            self.buffer = self.buffer[pos..].to_string(); // 保留 "<tag " 开头
                            self.state = State::InStartTag;
                        }
                    } else {
                        // 检查 partial match (可能被截断的 "<" 或 "<tag")
                        if let Some(partial_idx) = self.find_partial_start(&self.buffer) {
                            if partial_idx > 0 {
                                events.push(AdvStreamEvent::Text(self.buffer[..partial_idx].to_string()));
                            }
                            self.buffer = self.buffer[partial_idx..].to_string();
                        } else {
                            if !self.buffer.is_empty() {
                                events.push(AdvStreamEvent::Text(self.buffer.clone()));
                                self.buffer.clear();
                            }
                        }
                        break;
                    }
                }
                State::InStartTag => {
                    // 我们现在在 "<tag " 中，寻找最近的 ">"
                    if let Some(pos) = self.buffer.find('>') {
                        let tag_str = self.buffer[..pos].to_string(); // 例如 "<tag attr=\"1\""
                        
                        // 解析属性
                        let mut attrs = HashMap::new();
                        for cap in self.attr_regex.captures_iter(&tag_str) {
                            attrs.insert(cap[1].to_string(), cap[2].to_string());
                        }

                        events.push(AdvStreamEvent::TagStart { attrs });
                        self.buffer = self.buffer[pos + 1..].to_string();
                        self.state = State::Inside;
                    } else {
                        // 还没找到 ">"，继续缓冲，不输出
                        break;
                    }
                }
                State::Inside => {
                    // 寻找 </tag>，与基础版相同
                    if let Some(pos) = self.buffer.find(&self.end_tag) {
                        if pos > 0 {
                            events.push(AdvStreamEvent::TagContent(self.buffer[..pos].to_string()));
                        }
                        events.push(AdvStreamEvent::TagEnd);
                        self.buffer = self.buffer[pos + self.end_tag.len()..].to_string();
                        self.state = State::Outside;
                    } else {
                        if let Some(partial_idx) = self.find_partial_end(&self.buffer) {
                            if partial_idx > 0 {
                                events.push(AdvStreamEvent::TagContent(self.buffer[..partial_idx].to_string()));
                            }
                            self.buffer = self.buffer[partial_idx..].to_string();
                        } else {
                            if !self.buffer.is_empty() {
                                events.push(AdvStreamEvent::TagContent(self.buffer.clone()));
                                self.buffer.clear();
                            }
                        }
                        break;
                    }
                }
            }
        }
        events
    }

    fn find_partial_start(&self, buf: &str) -> Option<usize> {
        let strict_start = format!("<{}>", self.tag_name);
        let prefix_start = format!("<{} ", self.tag_name);
        for (idx, _) in buf.char_indices() {
            if buf[idx..].starts_with('<') {
                let remainder = &buf[idx..];
                if strict_start.starts_with(remainder) || prefix_start.starts_with(remainder) {
                    return Some(idx);
                }
            }
        }
        None
    }

    fn find_partial_end(&self, buf: &str) -> Option<usize> {
        for (idx, _) in buf.char_indices() {
            if buf[idx..].starts_with('<') {
                let remainder = &buf[idx..];
                if self.end_tag.starts_with(remainder) {
                    return Some(idx);
                }
            }
        }
        None
    }
}

// === 测试用例 ===
#[test]
fn test_adv_stream_parser() {
    let mut parser = AdvStreamParser::new("tool");
    // 模拟非常极端的截断场景
    let chunks = vec![
        "Here is the tool: <to", 
        "ol name=\"calc\" ", 
        "type=\"ma",
        "th\">",
        " 1 + 1 ",
        "</to",
        "ol> Done."
    ];
    
    for chunk in chunks {
        println!("Pushing: {:?}", chunk);
        let events = parser.push(chunk);
        for event in events {
            println!("  -> {:?}", event);
        }
    }
}