use std::{pin::Pin, sync::LazyLock, time::Duration};

use futures::{Stream, StreamExt};
use tokio::sync::mpsc;

pub enum Section {
    //思考:内容
    Think(String),
    //函数调用
    ToolCall(String),
    //回复
    Reply(String),
    //客户端UI渲染代码
    Render(String),
    //异常
    Error(String),
    //中断
    Interrupt(String),
    //其他
    Other(String),
}

impl Into<String> for &Section {
    fn into(self) -> String {
        match self {
            Section::Think(content) => format!("<think>{}</think>", content),
            Section::ToolCall(content) => format!("<function_call>{}</function_call>", content),
            Section::Reply(content) => format!("<reply>{}</reply>", content),
            Section::Render(content) => format!("<render>{}</render>", content),
            Section::Error(content) => format!("<error>{}</error>", content),
            Section::Interrupt(content) => format!("<interrupt>{}</interrupt>", content),
            Section::Other(content) => format!("<other>{}</other>", content),
        }
    }
}
impl Section {
    pub fn name(&self) -> &'static str {
        match &self {
            Section::Think(_) => "think",
            Section::ToolCall(_) => "tool_call",
            Section::Reply(_) => "reply",
            Section::Render(_) => "render",
            Section::Error(_) => "error",
            Section::Interrupt(_) => "interrupt",
            Section::Other(_) => "other",
        }
    }
    pub fn content(&self)->&String{
         match &self {
            Section::Think(c) =>c,
            Section::ToolCall(c) =>c,
            Section::Reply(c) =>c,
            Section::Render(c) =>c,
            Section::Error(c) =>c,
            Section::Interrupt(c) =>c,
            Section::Other(c) =>c,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SectionTag {
    pub name: &'static str,
    pub start: &'static str,
    pub end: &'static str,
}
impl SectionTag {
    pub fn new(name: &'static str, start: &'static str, end: &'static str) -> Self {
        SectionTag { name, start, end }
    }
}
pub static TAGS: LazyLock<Vec<SectionTag>> = LazyLock::new(|| {
    vec![
        SectionTag::new("think", "<think>", "</think>"),
        SectionTag::new("function_call", "<function_call>", "</function_call>"),
        SectionTag::new("reply", "<reply>", "</reply>"),
        SectionTag::new("render", "<render>", "</render>"),
        SectionTag::new("error", "<error>", "</error>"),
        SectionTag::new("interrupt", "<interrupt>", "</interrupt>"),
    ]
});

pub enum SectionEvent {
    Start,
    Open(SectionTag),
    Chunk(&'static str, String),
    Close(Section),
    End,
}

/// 在流式输出中，解析不同的段内容

pub fn section_stream(
    mut chunk_stream: Pin<Box<dyn Stream<Item = String> + Send>>,
) -> impl Stream<Item = SectionEvent> {
    // safe_size 必须大于等于最长闭合标签的长度 (</function_call> 是 16)
    let safe_size = 20;
    let mut buffer = String::new();
    let mut current_tag: Option<SectionTag> = None;
    let mut tag_seek: usize = 0;

    let mut is_in_reply_stream = false;
    let tags = &*TAGS;

    async_stream::stream! {
        yield SectionEvent::Start;

        while let Some(chunk) = chunk_stream.next().await {
            buffer.push_str(&chunk);

            // ========================================================
            // 阶段 A：安全的 Chunk 流式输出（滑动窗口）
            // ========================================================
            let safe_len = buffer.len().saturating_sub(tag_seek);
            if safe_len > safe_size {
                // 1. 找到安全的切割偏移量
                let mut offset = buffer.len() - safe_size;

                // 2. 关键修复：防止 UTF-8 字符被切断导致服务 Panic！
                while offset > tag_seek && !buffer.is_char_boundary(offset) {
                    offset -= 1; // 往回退，直到找到合法的字符边界
                }

                if offset > tag_seek {
                    let chunk_text = buffer[tag_seek..offset].to_string();

                    if let Some(tag) = &current_tag {
                        // 处于标签内
                        yield SectionEvent::Chunk(tag.name, chunk_text);
                    } else {
                        // 处于正常文本内，也需要流式输出！
                        if !is_in_reply_stream {
                            yield SectionEvent::Open(tags[2].clone()); // tags[2] 是 reply
                            is_in_reply_stream = true;
                        }
                        yield SectionEvent::Chunk(tags[2].name, chunk_text);
                    }
                    tag_seek = offset;
                }
            }

            // ========================================================
            // 阶段 B：解析与标签捕获
            // ========================================================
            loop {
                if let Some(tag) = &current_tag {
                    // 1. 寻找闭合标签
                    if let Some(pos) = buffer.find(tag.end) {
                        // 把没输出完的部分补齐输出
                        if tag_seek < pos {
                            yield SectionEvent::Chunk(tag.name, buffer[tag_seek..pos].to_string());
                        }
                        let content = buffer[..pos].to_string();
                        yield SectionEvent::Close(create_section(tag.name, content));

                        // 截断 buffer，关键修复：重置状态！
                        buffer = buffer[pos + tag.end.len()..].to_string();
                        tag_seek = 0;
                        current_tag = None;
                    } else {
                        break; // 等待更多 chunk
                    }
                } else {
                    // 2. 寻找起始标签
                    let mut earliest_pos = None;
                    let mut earliest_tag = None;

                    for tag in tags {
                        if let Some(pos) = buffer.find(tag.start) {
                            if earliest_pos.map_or(true, |p| pos < p) {
                                earliest_pos = Some(pos);
                                earliest_tag = Some(tag.clone());
                            }
                        }
                    }

                    if let Some(tag) = earliest_tag {
                        let pos = earliest_pos.unwrap();

                        // 起始标签前面的内容处理
                        if pos > 0 && !&buffer[..pos].trim_start().is_empty() {
                            if !is_in_reply_stream {
                                yield SectionEvent::Open(tags[2].clone());
                            }
                            if tag_seek < pos {
                                yield SectionEvent::Chunk(tags[2].name, buffer[tag_seek..pos].to_string());
                            }
                            let content = buffer[..pos].to_string();
                            yield SectionEvent::Close(Section::Reply(content));
                            is_in_reply_stream = false;
                        }

                        // 截断 buffer 进入新标签，关键修复：不盲目直接 yield buffer！
                        buffer = buffer[pos + tag.start.len()..].to_string();
                        yield SectionEvent::Open(tag.clone());
                        tag_seek = 0;
                        current_tag = Some(tag);
                    } else {
                        break; // 没找到起始标签，交给下一轮网络流
                    }
                }
            }
        }

        // ========================================================
        // 阶段 C：收尾处理剩余数据
        // ========================================================
        if !buffer.is_empty() && tag_seek < buffer.len() {
            let remain = buffer[tag_seek..].to_string();

            if let Some(tag) = current_tag {
                yield SectionEvent::Chunk(tag.name, remain);
                yield SectionEvent::Close(create_section(tag.name, buffer.clone()));
            } else if !&buffer.trim_start().is_empty(){
                if !is_in_reply_stream {
                    yield SectionEvent::Open(tags[2].clone());
                }
                yield SectionEvent::Chunk(tags[2].name, remain);
                yield SectionEvent::Close(Section::Reply(buffer.clone()));
            }
        }

        yield SectionEvent::End;
    }
}

/// 辅助方法：根据 tag 名称构建对于的 Section 枚举
fn create_section(tag: &str, content: String) -> Section {
    match tag {
        "think" => Section::Think(content),
        "function_call" => Section::ToolCall(content),
        "render" => Section::Render(content),
        "reply" => Section::Reply(content),
        "error" => Section::Error(content),
        "interrupt" => Section::Interrupt(content),
        _ => Section::Other(content),
    }
}

#[tokio::test]
async fn test() {
    println!("=== 开始测试 Section Stream ===");

    // 1. 模拟 LLM 异步流式输出的数据块 (Chunks)
    // 故意将标签切碎，以测试解析器的 buffer 拼接能力
    let chunks = vec![
        "你好！我来",
        "思考一下。",
        "<th", // 切碎的开始标签
        "ink>这",
        "是一个",
        "思考过程</", // 切碎的结束标签
        "think>现在",
        "我给你最终回复：",
        "<reply>这是",
        "正常回复内容</reply>\n",
        // "后面",
        // "还有点尾巴",
    ];
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        for chunk in chunks {
            tokio::time::sleep(Duration::from_millis(10)).await;
            tx.send(chunk.to_string()).unwrap();
        }
    });
    let chunk_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).boxed();
    // 3. 调用你的 section_stream 函数
    let mut event_stream = Box::pin(section_stream(chunk_stream));

    // 4. 消费解析出来的事件
    while let Some(event) = event_stream.next().await {
        match event {
            SectionEvent::Start => {
                println!("\n[事件] >>> 流开始 <<<");
            }
            SectionEvent::Open(mut tag) => {
                println!("\n[事件] 开启标签: <{}>", tag.name);
            }
            SectionEvent::Chunk(tag_name, chunk) => {
                print!("{}", chunk)
            }
            SectionEvent::Close(section) => {
                let s: String = (&section).into();
                println!("[事件] 关闭标签，完整段落内容为: {:?}", s);
            }
            SectionEvent::End => {
                println!("\n[事件] >>> 流结束 <<<");
            }
        }
    }

    // 给后台异步任务一点时间打印完毕
    tokio::time::sleep(Duration::from_millis(50)).await;
}
