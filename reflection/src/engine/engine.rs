use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};

use dashmap::DashMap;
use rule_script::{Expr, ScriptEngine};

use crate::{engine::core::RuleGroup, operator::EventElement};
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
pub struct RuleEngine<R: Clone> {
    pub(crate) script_engine: Arc<ScriptEngine<EventElement>>,
    pub(crate) rule_groups: DashMap<String, Arc<RuleGroup<R>>>,
}
impl<R: Clone + Send + Sync + 'static> RuleEngine<R> {
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
            rule_groups: DashMap::new(),
        }
    }
    pub fn group_names(&self) -> HashSet<String> {
        self.rule_groups.iter().map(|v| v.key().clone()).collect()
    }
    pub fn clear_groups(&self) {
        self.rule_groups.clear();
    }
    pub fn remove_rule_group(&self, name: &str) {
        self.rule_groups.remove(name);
    }
    pub fn add_rule_group(&self, group: RuleGroup<R>) {
        self.rule_groups.insert(group.name.clone(), Arc::new(group));
    }
    pub fn parse(&self, exprs: &[&str]) -> anyhow::Result<Vec<Expr>> {
        let mut asts = vec![];
        for expr in exprs {
            asts.push(rule_script::parse_expr(expr, &*&ORDERS)?);
        }
        return Ok(asts);
    }
    pub fn simplify(&self, expr: &str) -> anyhow::Result<String> {
        self.script_engine.simplify(expr, &*&ORDERS)
    }
    pub async fn trigger(&self, element: EventElement) -> anyhow::Result<Vec<R>> {
        let element = Arc::new(element);
        let groups_data: Vec<_> = {
            self.rule_groups
                .iter()
                .map(|entry| {
                    let group = entry.value();
                    // 这里只克隆计算逻辑需要的核心数据
                    (group.exprs.clone(), group.rules.clone())
                })
                .collect() // 迭代器在这里销毁，DashMap 锁在此处彻底释放
        };
        // let element = Arc::new(element);
        let mut tasks = Vec::new();

        // 2. 构建任务
        for (exprs, rules) in groups_data {
            let element_ref = element.clone();
            let script_engine = self.script_engine.clone();

            tasks.push(async move {
                // 注意：确保 solves 内部不持有任何对 AppAgent 的非 'static 引用
                // let res:Result<Vec<rule_script::TestValue>, anyhow::Error>=Err(anyhow::anyhow!("111"));
                let res = script_engine.solves(&exprs, element_ref).await;
                (res, rules)
            });
        }

        // 3. 并发执行
        let results = futures::future::join_all(tasks).await;

        // 4. 收集结果
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
