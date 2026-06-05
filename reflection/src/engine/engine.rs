use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};

use dashmap::DashMap;
use rule_script::{Expr, ScriptEngine};

use crate::{
    engine::core::{Rule, RuleGroup},
    operator::EventElement,
};
static ORDERS: LazyLock<HashMap<&'static str, i32>> = LazyLock::new(|| {
    //按执行代价排序
    HashMap::from([
        ("includes", 0),
        ("index_of", 1),
        ("count", 2),
        ("includes_pattern", 10),
        ("index_of_pattern", 11),
        ("count_pattern", 12),
        ("event_type_is", 0),
        ("time_with", 0),
        ("similar", 100),
    ])
});
pub struct RuleEngine {
    pub(crate) script_engine: Arc<ScriptEngine<EventElement>>,
}
impl RuleEngine {
    pub fn new(sim: crate::operator::Similar) -> Self {
        let mut engine = ScriptEngine::new();
        engine.register_predicate(crate::operator::Count);
        engine.register_predicate(crate::operator::CountPattern);
        engine.register_predicate(crate::operator::Includes);
        engine.register_predicate(crate::operator::IncludesPattern);
        engine.register_predicate(crate::operator::IndexOf);
        engine.register_predicate(crate::operator::IndexOfPattern);
        //
        engine.register_predicate(crate::operator::EventTypeIs);
        engine.register_predicate(crate::operator::TimeWith);
        engine.register_predicate(sim);
        RuleEngine {
            script_engine: Arc::new(engine),
        }
    }
    pub fn parses(&self, exprs: &[&str]) -> anyhow::Result<Vec<Expr>> {
        let mut asts = vec![];
        for expr in exprs {
            asts.push(rule_script::parse_expr(expr, &*&ORDERS)?);
        }
        return Ok(asts);
    }
    pub fn parse(&self, expr: &str) -> anyhow::Result<Expr> {
        return Ok(rule_script::parse_expr(expr, &*&ORDERS)?);
    }
    pub fn simplify(&self, expr: &str) -> anyhow::Result<String> {
        self.script_engine.simplify(expr, &*&ORDERS)
    }

    pub async fn filter<R: Clone>(
        &self,
        rules: Vec<Rule<R>>,
        element: EventElement,
    ) -> anyhow::Result<Vec<R>> {
        let element = Arc::new(element);
        let exprs = rules
            .iter()
            .map(|rule| rule.expr.clone())
            .collect::<Vec<_>>();
        // 构建任务
        let script_engine = self.script_engine.clone();
        let mut tasks = Vec::new();
        tasks.push(async move {
            let res = script_engine.solves(&exprs, element).await;
            (res, rules)
        });

        // 并发执行
        let results = futures::future::join_all(tasks).await;

        // 收集结果
        let mut triggered_rules = vec![];
        for (res, rules) in results {
            let values = res?;
            for (i, ele) in values.into_iter().enumerate() {
                if let Ok(b) = ele.must_bool() {
                    if b {
                        triggered_rules.push(rules[i].data.clone());
                    }
                }
            }
        }
        Ok(triggered_rules)
    }
}
