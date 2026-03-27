use chrono::Local;
use rusqlite::{Connection, Result};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: i64,
    pub name: String,
    pub memory: String,
    pub goal: String,
    pub create_time: String,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: i64,
    pub stream_id: i64,
    pub event_type: String,
    pub content: String,
    pub response: String,
    pub source_event_id: Option<i64>,
    pub time: String,
}

#[derive(Debug, Clone)]
pub struct RuleGroup {
    pub id: i64,
    pub name: String,
    pub create_time: String,
}

#[derive(Debug, Clone)]
pub struct TriggerRule {
    pub id: i64,
    pub group_id: i64,
    pub expr: String,
    pub hook: String,
    pub create_time: String,
    pub trigger_times: i32,
    pub expire_time: String,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let sql = include_str!("../../sql/init.sql");
        conn.execute_batch(sql)?;
        Ok(())
    }

    pub fn insert_agent(&self, name: &str, memory: &str, goal: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agents (name, memory, goal, create_time) VALUES (?1, ?2, ?3, ?4)",
            (name, memory, goal, &now),
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_agent(&self, id: i64) -> Result<Option<Agent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, name, memory, goal, create_time FROM agents WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                memory: row.get(2)?,
                goal: row.get(3)?,
                create_time: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_agent(&self, id: i64, memory: &str, goal: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE agents SET memory = ?1, goal = ?2 WHERE id = ?3",
            (memory, goal, id),
        )?;
        Ok(())
    }

    pub fn list_agents(&self) -> Result<Vec<Agent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, memory, goal, create_time FROM agents")?;
        let agents = stmt
            .query_map([], |row| {
                Ok(Agent {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    memory: row.get(2)?,
                    goal: row.get(3)?,
                    create_time: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(agents)
    }

    pub fn insert_event(
        &self,
        stream_id: i64,
        event_type: &str,
        content: &str,
        source_event_id: Option<i64>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().to_rfc3339();
        conn.execute(
            "INSERT INTO events (stream_id, event_type, content, response, source_event_id, time) VALUES (?1, ?2, ?3, '', ?4, ?5)",
            (stream_id, event_type, content, source_event_id, &now),
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_event_response(&self, id: i64, response: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE events SET response = ?1 WHERE id = ?2",
            (response, id),
        )?;
        Ok(())
    }

    pub fn get_events_by_stream(&self, stream_id: i64) -> Result<Vec<Event>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, stream_id, event_type, content, response, source_event_id, time FROM events WHERE stream_id = ?1 ORDER BY id"
        )?;
        let events = stmt
            .query_map([stream_id], |row| {
                Ok(Event {
                    id: row.get(0)?,
                    stream_id: row.get(1)?,
                    event_type: row.get(2)?,
                    content: row.get(3)?,
                    response: row.get(4)?,
                    source_event_id: row.get(5)?,
                    time: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(events)
    }

    pub fn insert_rule_group(&self, name: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().to_rfc3339();
        conn.execute(
            "INSERT INTO rule_groups (name, create_time) VALUES (?1, ?2)",
            (name, &now),
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_rule_group(&self, id: i64) -> Result<Option<RuleGroup>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, name, create_time FROM rule_groups WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(RuleGroup {
                id: row.get(0)?,
                name: row.get(1)?,
                create_time: row.get(2)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_rule_groups(&self) -> Result<Vec<RuleGroup>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, create_time FROM rule_groups")?;
        let groups = stmt
            .query_map([], |row| {
                Ok(RuleGroup {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    create_time: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(groups)
    }

    pub fn insert_trigger_rule(
        &self,
        group_id: i64,
        expr: &str,
        hook: &str,
        expire_seconds: i64,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now();
        let expire = now + chrono::Duration::seconds(expire_seconds);
        let now_str = now.to_rfc3339();
        let expire_str = expire.to_rfc3339();
        conn.execute(
            "INSERT INTO trigger_rules (group_id, expr, hook, create_time, trigger_times, expire_time) VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            (group_id, expr, hook, &now_str, &expire_str),
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_trigger_rule(&self, id: i64) -> Result<Option<TriggerRule>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, group_id, expr, hook, create_time, trigger_times, expire_time FROM trigger_rules WHERE id = ?1"
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(TriggerRule {
                id: row.get(0)?,
                group_id: row.get(1)?,
                expr: row.get(2)?,
                hook: row.get(3)?,
                create_time: row.get(4)?,
                trigger_times: row.get(5)?,
                expire_time: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_rules_by_group(&self, group_id: i64) -> Result<Vec<TriggerRule>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, group_id, expr, hook, create_time, trigger_times, expire_time FROM trigger_rules WHERE group_id = ?1"
        )?;
        let rules = stmt
            .query_map([group_id], |row| {
                Ok(TriggerRule {
                    id: row.get(0)?,
                    group_id: row.get(1)?,
                    expr: row.get(2)?,
                    hook: row.get(3)?,
                    create_time: row.get(4)?,
                    trigger_times: row.get(5)?,
                    expire_time: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(rules)
    }

    pub fn get_active_rules(&self) -> Result<Vec<TriggerRule>> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, group_id, expr, hook, create_time, trigger_times, expire_time FROM trigger_rules WHERE expire_time > ?1"
        )?;
        let rules = stmt
            .query_map([&now], |row| {
                Ok(TriggerRule {
                    id: row.get(0)?,
                    group_id: row.get(1)?,
                    expr: row.get(2)?,
                    hook: row.get(3)?,
                    create_time: row.get(4)?,
                    trigger_times: row.get(5)?,
                    expire_time: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(rules)
    }

    pub fn increment_trigger_times(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE trigger_rules SET trigger_times = trigger_times + 1 WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn delete_trigger_rule(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM trigger_rules WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn cleanup_expired_rules(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Local::now().to_rfc3339();
        let affected = conn.execute("DELETE FROM trigger_rules WHERE expire_time <= ?1", [&now])?;
        Ok(affected as i64)
    }
}
