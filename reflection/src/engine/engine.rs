use std::collections::HashMap;

use rule_script::{Predicate, ScriptEngine};

use crate::{
    engine::core::{Rule, RuleGroup},
    operator::EventElement,
};

pub struct RuleEngine {
    pub(crate) script_engine: ScriptEngine<EventElement>,
    pub(crate) rule_groups: HashMap<String, RuleGroup>,
}
impl RuleEngine {
    pub fn new() -> Self {
        RuleEngine {
            script_engine: ScriptEngine::new(),
            rule_groups: HashMap::new(),
        }
    }
    pub fn register_predicate<P: Predicate<EventElement> + 'static>(&mut self, pred: P) {
        self.script_engine.register_predicate(pred);
    }
    pub async fn trigger(&self, element: &EventElement) -> anyhow::Result<Vec<&Rule>> {
        let mut triggered_rules = vec![];
        for group in self.rule_groups.values() {
            //求解
            let values = self
                .script_engine
                .solves(element, &group.operators, &group.exprs)
                .await?;
            for (i, ele) in values.into_iter().enumerate() {
                if let Ok(b) = ele.must_bool() {
                    if b {
                        triggered_rules.push(&group.rules[i]);
                    }
                }
            }
        }
        return Ok(triggered_rules);
    }
}
