use std::{pin::Pin, sync::LazyLock};

use futures::{Stream, StreamExt};

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
        SectionTag::new("interupt", "<interupt>", "</interupt>"),
    ]
});
pub trait SectionListener {
    fn on_start(&mut self, section: &str);
    fn on_end(&mut self, section: Section);
}

pub enum SectionEvent {
    Start,
    Open(SectionTag),
    Close(Section),
    End,
}
/// 在流式输出中，解析不同的段内容，并通过listener回调
///
/// 注：由于 trait 对象中的 `&str` 需要明确生命周期，这里增加了生命周期参数 `'a`。
/// （如果实际项目中 chunk 是 `String`，把 `&'a str` 替换为 `String` 即可）
pub fn section_stream(
    mut chunk_stream: Pin<Box<dyn Stream<Item = String> + Send>>,
) -> impl Stream<Item = SectionEvent> {
    let mut buffer = String::new();
    let mut current_tag: Option<SectionTag> = None;

    // 定义我们需要解析的标签映射
    let tags = &*TAGS;

    // 使用 async_stream::stream! 宏来生成一个异步流
    async_stream::stream! {
        yield SectionEvent::Start;
        while let Some(chunk) = chunk_stream.next().await {
            buffer.push_str(&chunk);

            // 循环处理 buffer，直到无法解析出完整的标签为止
            loop {
                if let Some(tag) = &current_tag {
                    // 1. 如果当前处于某个标签内部，寻找它的闭合标签

                    if let Some(pos) = buffer.find(tag.end) {
                        // 提取标签内部的内容
                        let content = buffer[..pos].to_string();
                        // 将已处理的内容移出缓冲区
                        buffer = buffer[pos + tag.end.len()..].to_string();

                        // yield 抛出到外部流
                        yield SectionEvent::Close(create_section(tag.name, content));
                        current_tag = None;
                    } else {
                        // 闭合标签还没到达，跳出当前处理，等待下一个 chunk
                        break;
                    }
                } else {
                    // 2. 如果不在任何标签内，寻找下一个出现的起始标签
                    let mut earliest_pos = None;
                    let mut earliest_tag = None;

                    for tag in tags {
                        if let Some(pos) = buffer.find(tag.start) {
                            // 找到在文本中最靠前的一个标签
                            if earliest_pos.map_or(true, |p| pos < p) {
                                earliest_pos = Some(pos);
                                earliest_tag = Some(tag.clone());
                            }
                        }
                    }

                    if let Some(tag) = earliest_tag {
                        let pos = earliest_pos.unwrap();
                        // 标签前面的内容作为普通的 Other 返回
                        if pos > 0 {
                            let content = buffer[..pos].to_string();
                            yield SectionEvent::Open(tags[2].clone());
                            yield SectionEvent::Close(Section::Reply(content));
                        }
                        yield SectionEvent::Open(tag.clone());
                        // 将当前标签从 buffer 中移除
                        buffer = buffer[pos + tag.start.len()..].to_string();
                        current_tag = Some(tag);
                    } else {
                        // 未找到任何完整的起始标签。
                        // 注意：这里可能存在 `<` 等截断的半个标签，因此我们安全的做法是先不返回 Reply，
                        // 将文本留在 buffer 中，等待后续流补全后再做判断。
                        break;
                    }
                }
            }
        }

        // 3. 流结束，处理 buffer 中剩余的尾部数据
        if !buffer.is_empty() {
            if let Some(tag) = current_tag {
                // 有未闭合的标签（遇到异常截断的流）
                let section = create_section(tag.name, buffer);
                yield SectionEvent::Close(section);
            } else {
                // 剩余内容当作普通
                yield SectionEvent::Open(tags[2].clone());
                yield SectionEvent::Close(Section::Reply(buffer));
            }
        }
        yield SectionEvent::End
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
        "interupt" => Section::Interrupt(content),
        _ => Section::Other(content),
    }
}
