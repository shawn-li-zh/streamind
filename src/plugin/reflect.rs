use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use dashmap::DashMap;
use entity_service::DB;
use kernel::{agent::event::EnvEvent as _, message::Information};
use reflection::{
    engine::{
        core::{Rule, RuleGroup},
        engine::RuleEngine,
    },
    operator::{EventElement, Similar},
};
use runtime::agent::{
    event::{AppEnvEvent, AppEnvEventData},
    event_context::StackContext,
    reflect::{ReflectData, ReflectEvent},
};
use tracing::info;

use crate::global::EMBEDDING_CLIENT;

pub struct CondReflection {
    agent_id: u32,
    event_context: Arc<StackContext<AppEnvEvent>>,
    pub reflect_engine: Arc<RuleEngine<ReflectData>>,
    triggered_rule_ids: parking_lot::Mutex<HashSet<i64>>,
}
impl CondReflection {
    pub fn new(
        agent_id: u32,
        event_context: Arc<StackContext<AppEnvEvent>>,
        reflect_engine: RuleEngine<ReflectData>,
    ) -> Self {
        Self {
            agent_id,
            event_context,
            reflect_engine: Arc::new(reflect_engine),
            triggered_rule_ids: parking_lot::Mutex::new(HashSet::new()),
        }
    }
    pub fn refresh_triggered_rules(&self) {
        for  node in self.event_context.nodes() {
            let events = node.events();
            for event in events {
                match &event.data {
                    AppEnvEventData::Reflection(reflect_event) => {
                        for data in &reflect_event.datas {
                            let mut rule_ids = self.triggered_rule_ids.lock();
                            rule_ids.insert(data.id);
                        }
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }
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
            _ => {}
        }
        //
        let (text, _paths) = event.to_message().to_tuple();
        // let embedding = match &event.data {
        //     // AppEnvEventData::AgentResponse(_)
        //     | AppEnvEventData::Timer(_)
        //     // | AppEnvEventData::ToolResponse(_)
        //     // | AppEnvEventData::Interaction(_)
        //     // | AppEnvEventData::UserRequest(_) => {
        //     //     let client = EMBEDDING_CLIENT.get().unwrap();
        //     //     let embedding = client
        //     //         .union_embed(vec![Information::Text(text.clone())])
        //     //         .await?;
        //     //     embedding
        //     // }
        //     // _ => {
        //     //     //其他的都没必要嵌入
        //     //     vec![]
        //     // }
        // };
        let embedding = vec![];
        let element = EventElement {
            event_type: event.data.code(),
            time: event.time.format("%Y-%m-%d %H:%M:%S").to_string(),
            force: event.force,
            context: text,
            embedding,
        };

        let mut datas = self.reflect_engine.trigger(element).await?;
        // 过滤掉已经触发过的
        let rule_ids = self.triggered_rule_ids.lock();
        datas.retain(|v| !rule_ids.contains(&v.id));
        if datas.is_empty() {
            return Ok(None);
        }

        let event = ReflectEvent {
            source_event_id,
            level,
            datas,
        };
        Ok(Some(event))
    }

    pub async fn load_groups(&self, group_names: &Vec<String>) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        let rule_engine = self.reflect_engine.clone();
        let groups = load_rule_groups(&rule_engine, group_names).await?;
        for group in groups {
            rule_engine.add_rule_group(group);
        }
        let names = rule_engine.group_names().into_iter().collect();
        entity_service::agent::update_rule_groups(db, self.agent_id, names).await?;
        Ok(())
    }
    pub async fn unload_groups(&self, group_names: &Vec<String>) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        let rule_engine: Arc<RuleEngine<ReflectData>> = self.reflect_engine.clone();
        for group in group_names {
            rule_engine.remove_rule_group(&group);
        }
        let names = rule_engine.group_names().into_iter().collect();
        entity_service::agent::update_rule_groups(db, self.agent_id, names).await?;
        Ok(())
    }
    pub fn group_names(&self) -> HashSet<String> {
        let rule_engine = self.reflect_engine.clone();
        rule_engine.group_names()
    }
}
pub async fn load_reflection_engine(
    agent: &entity::agent::Model,
) -> anyhow::Result<RuleEngine<ReflectData>> {
    let groups = &agent.rule_groups.0;
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
    let groups = load_rule_groups(&engine, groups).await?;

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
            map.entry(ele.group_id).or_default().push(ele);
        }
    }

    let mut groups: Vec<RuleGroup<ReflectData>> = vec![];
    for group in group_models {
        let rules = map.remove(&group.id).unwrap_or_default();
        let exprs: Vec<&str> = rules.iter().map(|v| v.expr.as_str()).collect();
        let (exprs, operators) = engine.extract_operators(&exprs)?;
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
            operators: Arc::new(operators),
            exprs: Arc::new(exprs),
            rules,
        };
        groups.push(group);
    }
    Ok(groups)
}
