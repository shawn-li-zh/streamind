use std::path::PathBuf;

use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartImageArgs,
    ChatCompletionRequestMessageContentPartText, ChatCompletionRequestMessageContentPartTextArgs,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, ImageUrl,
    ImageUrlArgs,
};

use base64::{Engine, engine::general_purpose};
use kernel::agent::{AgentContext, AgentInfo};

pub static GENERATE_RULE: &str = include_str!("../../../prompts/common/generate_rule.md");
pub static TOOL_CALL_RULE: &str = include_str!("../../../prompts/common/tool_call_rule.md");
pub static RENDER_RULE: &str = include_str!("../../../prompts/common/render_rule.md");

pub fn build_system_prompt(ctx: &AgentContext, agent_code: &str, agent_name: &str) -> String {
    let items: Vec<String> = ctx
        .tool_engine_context
        .tool_packages
        .iter()
        .map(|pkg_name| {
            let pkg = ctx.tool_engine_context.package_pool.get(pkg_name).unwrap();
            let tool_size = pkg.tools().len();
            format!("{}(tools:{tool_size}):{}", pkg_name, pkg.annotation)
        })
        .collect();

    format!(
        r#"
    # 基础信息:
    - 编号:{}
    - 名称:{}
    
    {}
    {}
    {}
    # 工具包列表(全部)
    {}
    "#,
        agent_code,
        agent_name,
        GENERATE_RULE,
        TOOL_CALL_RULE,
        RENDER_RULE,
        items.join("\n")
    )
}
pub fn assistant_message(content: String) -> ChatCompletionRequestMessage {
    ChatCompletionRequestAssistantMessageArgs::default()
        .content(content)
        .build()
        .unwrap()
        .into()
}
pub fn system_message(content: String) -> ChatCompletionRequestMessage {
    ChatCompletionRequestSystemMessageArgs::default()
        .content(content)
        .build()
        .unwrap()
        .into()
}

pub fn user_message(content: String, paths: Vec<PathBuf>) -> ChatCompletionRequestMessage {
    let (texts, urls) = to_parts(content, paths);
    let mut list = vec![];
    for ele in texts {
        list.push(ele.into());
    }
    for ele in urls {
        list.push(ele.into());
    }
    ChatCompletionRequestUserMessageArgs::default()
        .content(list)
        .build()
        .unwrap()
        .into()
}

pub fn tool_message(content: String, paths: Vec<PathBuf>) -> ChatCompletionRequestMessage {
    system_message(content)
}

fn to_parts(
    text: String,
    paths: Vec<PathBuf>,
) -> (
    Vec<ChatCompletionRequestMessageContentPartText>,
    Vec<ChatCompletionRequestMessageContentPartImage>,
) {
    let text = ChatCompletionRequestMessageContentPartTextArgs::default()
        .text(text)
        .build()
        .unwrap();
    let mut texts = vec![text.into()];
    let mut urls = vec![];
    paths
        .into_iter()
        .map(|path| -> anyhow::Result<ImageUrl> {
            let base = encode_image_to_base64(&path)?;
            let url = ImageUrlArgs::default().url(base).build()?;
            Ok(url)
        })
        .enumerate()
        .for_each(|(index, res)| match res {
            Ok(v) => {
                let v = ChatCompletionRequestMessageContentPartImageArgs::default()
                    .image_url(v)
                    .build()
                    .unwrap();
                urls.push(v);
            }
            Err(e) => {
                let msg = ChatCompletionRequestMessageContentPartTextArgs::default()
                    .text(format!("Index={index},图片读取失败:'{e}'"))
                    .build()
                    .unwrap();
                texts.push(msg.into());
            }
        });

    ((texts, urls))
}

fn encode_image_to_base64(path: &PathBuf) -> anyhow::Result<String> {
    let ext = path
        .extension()
        .ok_or_else(|| anyhow::anyhow!("无效的图片后缀名"))?;
    let file_items = std::fs::read(&path)?;
    let image_base64 = general_purpose::STANDARD.encode(file_items);

    Ok(format!(
        "data:image/{};base64,{}",
        ext.to_ascii_lowercase().to_str().unwrap(),
        image_base64
    ))
}
