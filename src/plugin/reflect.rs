use std::{collections::HashMap, sync::Arc};

use reflection::{
    engine::{
        core::{Rule, RuleGroup},
        engine::RuleEngine,
    },
    operator::Similar,
};
use runtime::agent::reflect::ReflectData;

pub async fn load_reflection_engine(
    agent: &entity::agent::Model,
) -> anyhow::Result<RuleEngine<ReflectData>> {
    let groups = &agent.rule_groups.0;

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
