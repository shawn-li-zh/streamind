use std::{collections::HashMap, sync::Arc};

use entity_service::DB;
use futures::{FutureExt, StreamExt};
use kernel::{
    parser,
    tool::{
        call::ToolCall,
        engine::ToolEngine,
        event::{ToolCallDetailItem, ToolCallEvent, ToolCallStatus},
    },
};
use runtime::agent::event::AppEnvEventData;

use crate::{agent::common::WebSender, sse_event::WebEvent};
pub async fn invoke(
    response_event_id: i64,
    tool_call_infos: Vec<(ToolCallDetailItem, ToolCall)>,
    tool_engine: &ToolEngine,
    web_sender: Arc<WebSender>,
    events: &mut Vec<(AppEnvEventData, bool)>,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    //
    let mut tool_call_details: HashMap<i64, ToolCallDetailItem> = HashMap::new();
    let mut tool_calls = vec![];

    let mut call_ids = vec![];
    for (detail, call) in tool_call_infos {
        //
        tool_call_details.insert(detail.id, detail);
        call_ids.push(call.id);
        tool_calls.push(call);
    }

    //批执行
    let mut stream = tool_engine.batch_call(tool_calls).await.boxed();
    while let Some(event) = stream.next().await {
        //通知前端
        web_sender.send(WebEvent::ToolCall(event.clone(), response_event_id));
        let item = match event {
            ToolCallEvent::Success(item) => item,
            ToolCallEvent::Failure(item) => item,
            _ => {
                continue;
            }
        };
        let mut detial = tool_call_details.remove(&item.id).unwrap();
        detial.values = item.values;
        detial.status = item.status;
        detial.elapsed = item.elapsed;
        //保存
        entity_service::section::update_payload(db, detial.id, detial.clone()).await?;
        let is_empty = detial.values.iter().filter(|v| !v.is_empty()).count() == 0;
        if is_empty {
            continue;
        }
        //发送完整事件
        events.push((AppEnvEventData::ToolResponse(detial), false));
    }
    println!("call invoke done...");
    //结束
    Ok(())
}

pub fn parse(
    header: &str,
    content: &str,
    tool_engine: &ToolEngine,
) -> anyhow::Result<(ToolCallDetailItem, ToolCall)> {
    let inner = header
        .trim_start_matches("<tool_call")
        .trim_end_matches(">")
        .trim();
    let mut attrs = parser::parse_attrs(inner);
    let call_id = match attrs
        .remove("id")
        .map(|v| v.as_str().unwrap_or_default().to_string())
    {
        Some(v) => v,
        None => {
            anyhow::bail!("tool_call id is required")
        }
    };
    let call = tool_engine.preprocess(call_id.clone(), content)?;
    let detail = ToolCallDetailItem {
        id: 0,
        call_id,
        package: call.package.name.to_string(),
        tool: call.tool.name.to_string(),
        values: vec![],
        status: ToolCallStatus::Undo,
        args: call.args.clone(),
        elapsed: "".to_string(),
    };
    Ok((detail, call))
}
