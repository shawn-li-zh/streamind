use std::sync::Arc;
use tokio::sync::RwLock;

use crate::db::{Database, TriggerRule};
use crate::rule::engine::RuleEngine;
use crate::rule::hooks::BuiltinHooks;

pub struct AgentService {
    db: Arc<Database>,
    rule_engine: RuleEngine,
    context: RwLock<AgentContext>,
}

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub agent_id: i64,
    pub name: String,
    pub memory: String,
    pub goal: String,
    pub recent_events: Vec<String>,
    pub triggered_rules: Vec<i64>,
}

impl AgentService {
    pub fn new(db: Arc<Database>) -> Self {
        let rule_engine = BuiltinHooks::create_engine();
        AgentService {
            db,
            rule_engine,
            context: RwLock::new(AgentContext {
                agent_id: 0,
                name: String::new(),
                memory: String::new(),
                goal: String::new(),
                recent_events: Vec::new(),
                triggered_rules: Vec::new(),
            }),
        }
    }

    pub async fn init_agent(&self, name: &str, memory: &str, goal: &str) -> anyhow::Result<i64> {
        let id = self.db.insert_agent(name, memory, goal)?;
        let mut ctx = self.context.write().await;
        ctx.agent_id = id;
        ctx.name = name.to_string();
        ctx.memory = memory.to_string();
        ctx.goal = goal.to_string();
        Ok(id)
    }

    pub async fn add_event(&self, content: &str) -> anyhow::Result<Vec<String>> {
        let ctx = self.context.read().await;
        let event_id = self.db.insert_event(
            ctx.agent_id,
            "UserInput",
            content,
            None,
        )?;
        drop(ctx);

        let mut ctx = self.context.write().await;
        let event_text = content.to_string();
        ctx.recent_events.push(event_text.clone());
        if ctx.recent_events.len() > 10 {
            ctx.recent_events.remove(0);
        }
        ctx.triggered_rules.clear();

        let results = self.evaluate_rules(&event_text).await;
        
        Ok(results)
    }

    pub async fn evaluate_rules(&self, text: &str) -> Vec<String> {
        let active_rules = match self.db.get_active_rules() {
            Ok(rules) => rules,
            Err(_) => Vec::new(),
        };
        
        let mut results = Vec::new();
        let mut ctx = self.context.write().await;
        
        for rule in active_rules {
            if ctx.triggered_rules.contains(&rule.id) {
                continue;
            }
            
            if let Some(injection) = self.rule_engine.evaluate_and_inject(&rule.expr, &rule.hook, text) {
                results.push(injection);
                let _ = self.db.increment_trigger_times(rule.id);
                ctx.triggered_rules.push(rule.id);
            }
        }
        
        results
    }

    pub async fn get_context_summary(&self) -> String {
        let ctx = self.context.read().await;
        let mut summary = format!(
            "Agent: {}\nGoal: {}\nMemory: {}\nRecent events:\n",
            ctx.name, ctx.goal, ctx.memory
        );
        for (i, event) in ctx.recent_events.iter().enumerate() {
            summary.push_str(&format!("  {}. {}\n", i + 1, event));
        }
        summary
    }

    pub async fn update_memory(&self, memory: &str) -> anyhow::Result<()> {
        let ctx = self.context.read().await;
        self.db.update_agent(ctx.agent_id, memory, &ctx.goal)?;
        let mut ctx_mut = self.context.write().await;
        ctx_mut.memory = memory.to_string();
        Ok(())
    }

    pub async fn update_goal(&self, goal: &str) -> anyhow::Result<()> {
        let ctx = self.context.read().await;
        self.db.update_agent(ctx.agent_id, &ctx.memory, goal)?;
        let mut ctx_mut = self.context.write().await;
        ctx_mut.goal = goal.to_string();
        Ok(())
    }
}

pub struct RuleService {
    db: Arc<Database>,
    rule_engine: RuleEngine,
}

impl RuleService {
    pub fn new(db: Arc<Database>) -> Self {
        let rule_engine = BuiltinHooks::create_engine();
        RuleService { db, rule_engine }
    }

    pub fn create_rule_group(&self, name: &str) -> anyhow::Result<i64> {
        Ok(self.db.insert_rule_group(name)?)
    }

    pub fn add_rule(
        &self,
        group_id: i64,
        expr: &str,
        hook: &str,
        expire_seconds: i64,
    ) -> anyhow::Result<i64> {
        Ok(self.db.insert_trigger_rule(group_id, expr, hook, expire_seconds)?)
    }

    pub fn evaluate(&self, expr: &str, text: &str) -> bool {
        self.rule_engine.evaluate_rule(expr, text)
    }

    pub fn list_groups(&self) -> anyhow::Result<Vec<crate::db::RuleGroup>> {
        Ok(self.db.list_rule_groups()?)
    }

    pub fn list_rules(&self, group_id: i64) -> anyhow::Result<Vec<TriggerRule>> {
        Ok(self.db.get_rules_by_group(group_id)?)
    }

    pub fn cleanup_expired(&self) -> anyhow::Result<i64> {
        Ok(self.db.cleanup_expired_rules()?)
    }
}