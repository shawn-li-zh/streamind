use entity_service::DB;
use kernel::parser;

use runtime::agent::event::AppEnvEventData;

use crate::plugin::reflect::{CondReflectEngine, Recall};

pub fn parse(header: &str, content: &str) -> anyhow::Result<Recall> {
    let inner = header
        .trim_start_matches("<recall")
        .trim_end_matches(">")
        .trim();
    let mut attrs = parser::parse_attrs(inner);
    let time = attrs
        .remove("time")
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .unwrap_or_default();
    let event_type = attrs
        .remove("type")
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .unwrap_or_default();
    Ok(Recall {
        time,
        event_type,
        content: content.trim().to_string(),
    })
}

pub async fn invoke(
    recalls: Vec<Recall>,
    reflect_engine: &CondReflectEngine,
    events: &mut Vec<(AppEnvEventData, bool)>,
    state_id: i64,
) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    for recall in recalls {
        let v = reflect_engine.conent_reflect(recall).await?;
        if let Some(v) = v {
            let rule_ids: Vec<i64> = v.datas.iter().map(|v| v.id).collect();
            events.push((AppEnvEventData::Reflection(v), false));
            entity_service::rule::activate_rules(db, state_id, rule_ids).await?;
        }
    }
    Ok(())
}
