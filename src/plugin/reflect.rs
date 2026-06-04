use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use entity_service::DB;
use kernel::{message::Information, mind::event::EnvEvent as _};
use reflection::{
    engine::{
        core::{Rule, RuleGroup},
        engine::RuleEngine,
    },
    operator::{EventElement, Similar},
};
use runtime::{
    agent::{
        event::{AppEnvEvent, AppEnvEventData},
        reflect::{ReflectData, ReflectEvent},
    },
    context::stack::StackContext,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::global::EMBEDDING_CLIENT;
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Recall {
    pub event_type: String,
    pub time: String,
    pub content: String,
}
pub struct CondReflectEngine {
    agent_id: u32,
    event_context: Arc<StackContext<AppEnvEvent>>,
    pub reflect_engine: Arc<RuleEngine<ReflectData>>,
}
impl CondReflectEngine {
    pub fn new(
        agent_id: u32,
        event_context: Arc<StackContext<AppEnvEvent>>,
        reflect_engine: RuleEngine<ReflectData>,
    ) -> Self {
        Self {
            agent_id,
            event_context,
            reflect_engine: Arc::new(reflect_engine),
        }
    }

    //内容反射
    pub async fn conent_reflect(&self, recall: Recall) -> anyhow::Result<Option<ReflectEvent>> {
        let content = recall.content;
        let embedding = if content.len() > 15 {
            let client = EMBEDDING_CLIENT.get().unwrap();
            let embedding = client
                .union_embed(vec![Information::Text(content.clone())])
                .await?;
            embedding
        } else {
            vec![]
        };
        let element = EventElement {
            event_type: recall.event_type.clone(),
            time: recall.time.clone(),
            context: content,
            embedding,
        };
        self.reflect(0, element).await
    }

    //事件反射
    pub async fn event_reflect(&self, event: &AppEnvEvent) -> anyhow::Result<Option<ReflectEvent>> {
        let mut source_event_id = event.id;
        let mut level = 1;
        match &event.data {
            AppEnvEventData::Reflection(ev) => {
                level = ev.level + 1;
                if level > 2 {
                    //最多反射两层
                    return Ok(None);
                }
                source_event_id = ev.source_event_id;
            }
            AppEnvEventData::Heartbeat
            | AppEnvEventData::Interrupt(_)
            | AppEnvEventData::ToolResponse(_)
            | AppEnvEventData::StackPop(_, _) => {
                return Ok(None);
            }
            _ => {}
        }
        //
        let (text, _paths) = event.to_message().to_tuple();
        let embedding = match &event.data {
            AppEnvEventData::AgentResponse(_)
            | AppEnvEventData::Timer(_)
            | AppEnvEventData::ToolResponse(_)
            | AppEnvEventData::Interaction(_)
            | AppEnvEventData::UserRequest(_) => {
                let client = EMBEDDING_CLIENT.get().unwrap();
                let embedding = client
                    .union_embed(vec![Information::Text(text.clone())])
                    .await?;
                embedding
            }
            _ => {
                //其他的都没必要嵌入
                vec![]
            }
        };
        let element = EventElement {
            event_type: event.data.code(),
            time: event.time.format("%Y-%m-%d %H:%M:%S").to_string(),
            context: text,
            embedding,
        };
        self.reflect(source_event_id, element).await
    }

    async fn reflect(
        &self,
        source_event_id: i64,
        element: EventElement,
    ) -> anyhow::Result<Option<ReflectEvent>> {
        let db = DB.get().unwrap();
        //按语义匹配对应的规则域

        let scopes =
            entity_service::rule::page_scopes(1, 5, None, Some(element.embedding.clone())).await?;
        if scopes.is_empty() {
            return Ok(None);
        }
        let scope_ids: Vec<i64> = scopes.into_iter().map(|s| s.id).collect();
        // 过滤掉已经触发过的
        let rule_models = entity_service::rule::list_not_active_rules(scope_ids).await?;
        if rule_models.is_empty() {
            return Ok(None);
        }
        let mut rules = vec![];
        for ele in rule_models {
            let expr = self.reflect_engine.parse(&ele.expr)?;
            let data: ReflectData = ReflectData {
                id: ele.id,
                name: ele.name,
                content: ele.content,
            };
            let rule = Rule {
                expr: expr,
                data: data,
            };
            rules.push(rule);
        }
        let datas = self.reflect_engine.filter(rules, element).await?;
        if datas.is_empty() {
            return Ok(None);
        }
        //激活规则
        let state_id = self.event_context.top_node().map(|v| v.id).unwrap_or(1);
        let rule_ids = datas.iter().map(|v| v.id).collect();

        entity_service::rule::activate_rules(db, state_id, rule_ids).await?;
        let event = ReflectEvent {
            source_event_id,
            level: 1,
            datas,
        };
        Ok(Some(event))
    }
}
pub async fn load_reflection_engine(
    agent: &entity::agent::Model,
) -> anyhow::Result<RuleEngine<ReflectData>> {
    let mut groups = agent.rule_groups.0.clone();
    groups.push("meta".into());
    info!("加载规则组:{:?}", groups);
    let sim = Similar(Box::new(move |name: &str| {
        let name = name.to_string();
        Box::pin(async move {
            let res = entity_service::concept::find_by_name(&name, true).await?;
            let vec = res.map(|v| v.embedding.unwrap_or_default().0);
            Ok(vec)
        })
    }));
    let engine: RuleEngine<ReflectData> = RuleEngine::new(sim);
    let groups = load_rule_groups(&engine, &groups).await?;

    for group in groups {
        engine.add_rule_group(group);
    }
    Ok(engine)
}

pub async fn load_rule_groups(
    engine: &RuleEngine<ReflectData>,
    group_names: &Vec<String>,
) -> anyhow::Result<Vec<RuleGroup<ReflectData>>> {
    let group_models = entity_service::rule::list_groups_by_names(group_names).await?;
    let mut map: HashMap<i64, Vec<entity::rule_item::Model>> = HashMap::new();
    {
        let rules = entity_service::rule::list_rules_by_group_names(group_names).await?;
        for ele in rules {
            map.entry(ele.scope_id).or_default().push(ele);
        }
    }

    let mut groups: Vec<RuleGroup<ReflectData>> = vec![];
    for group in group_models {
        let rules = map.remove(&group.id).unwrap_or_default();
        let exprs: Vec<&str> = rules.iter().map(|v| v.expr.as_str()).collect();
        let exprs = engine.parses(&exprs)?;
        let rules: Vec<Rule<_>> = rules
            .into_iter()
            .zip(exprs.clone())
            .map(|(raw, expr)| Rule {
                expr: expr,
                data: ReflectData {
                    id: raw.id,
                    name: raw.name,
                    content: raw.content,
                },
            })
            .collect();
        let group = RuleGroup {
            name: group.name,

            exprs: Arc::new(exprs),
            rules,
        };
        groups.push(group);
    }
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Datelike, Local};
    use dotenvy::dotenv;
    use serde::Deserialize;

    use super::*;
    use std::{collections::HashMap, fs};
    #[tokio::test]
    pub async fn init() -> anyhow::Result<()> {
        dotenv().ok();
        entity_service::init().await.unwrap();
        #[derive(Deserialize)]
        struct Scope {
            name: String,
            content: String,
            active_threshold: f32,
            rules: Vec<Rule>,
        }
        #[derive(Deserialize)]
        struct Rule {
            name: String,
            expr: String,
            content: String,
        }
        let json = fs::read_to_string(r"D:\project\rust\streamind\prompts\rule_scope.json")?;
        let scopes = serde_json::from_str::<Vec<Scope>>(&json)?;
        //
        let sim = Similar(Box::new(move |name: &str| {
            let name = name.to_string();
            Box::pin(async move {
                let res = entity_service::concept::find_by_name(&name, true).await?;
                let vec = res.map(|v| v.embedding.unwrap_or_default().0);
                Ok(vec)
            })
        }));
        let engine: RuleEngine<ReflectData> = RuleEngine::new(sim);
        //
        let db = DB.get().unwrap();
        for scope in scopes {
            let scope_id = entity_service::rule::new_scope(
                db,
                scope.name,
                scope.content,
                scope.active_threshold,
            )
            .await?;
            for rule in scope.rules {
                let expr = engine.simplify(&rule.expr).unwrap();
                println!("simplify: Raw:{}\n New:{}", &rule.expr, &expr);
                entity_service::rule::save_rule(
                    db,
                    scope_id,
                    rule.name,
                    expr,
                    rule.content,
                    Local::now().with_year(10000).unwrap(),
                )
                .await?;
            }
        }
        Ok(())
    }
    // #[tokio::test]
    // pub async fn init_concepts() {
    //     dotenv().ok();
    //     entity_service::init().await.unwrap();
    //     let json = fs::read_to_string(r"D:\project\rust\streamind\prompts\concepts.json").unwrap();
    //     let concepts = serde_json::from_str::<Vec<HashMap<String, String>>>(&json).unwrap();
    //     //
    //     let db = DB.get().unwrap();
    //     for mut concept in concepts {
    //         let name = concept.remove("name").unwrap_or_default();
    //         let content = concept.remove("content").unwrap_or_default();
    //         entity_service::concept::create(db, name, content)
    //             .await
    //             .unwrap();
    //     }
    // }
}
