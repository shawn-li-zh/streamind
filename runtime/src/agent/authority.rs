use async_trait::async_trait;

use kernel::agent::AgentIdentity;

// pub struct AgentAuthority {
//     pub identity: Arc<AgentIdentity>,
//     pub context: Arc<AgentAuthorityContext>,
// }
// // #[async_trait]
// impl AgentAuthority {
//     //授予权限
//     async fn grant(&self, permission: AgentPermission) -> anyhow::Result<i64> {
//         let db = DB.get().unwrap();
//         let agent_id = self.identity.id();
//         let id = entity_service::agent_permission::create(
//             db,
//             agent_id,
//             permission.code.clone(),
//             permission.sub_code.clone(),
//             permission.purpose.clone(),
//             permission.life.clone(),
//         )
//         .await?;
//         self.context.add(permission);
//         Ok(id)
//     }
//     //撤销权限
//     async fn revoke(&self, id: i64) -> anyhow::Result<()> {
//         let db = DB.get().unwrap();
//         let agent_id = self.identity.id();
//         let model: Option<entity::agent_permission::Model> =
//             entity_service::agent_permission::get(agent_id, id).await?;
//         if let Some(model) = model {
//             let auth_code = auth_code(&model.code, model.sub_code.as_deref());
//             entity_service::agent_permission::revoke(db, agent_id, id).await?;
//             self.context.remove(&auth_code);
//         }
//         Ok(())
//     }
//     //验证是否授权
//     fn has_permission(&self, code: &str, sub_code: Option<&str>) -> bool {
//         let auth_code = auth_code(code, sub_code);
//         self.context.contains_key(&auth_code)
//     }
// }

use std::sync::{Arc, atomic::AtomicU32};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

pub struct AgentPermission {
    pub id: i64,
    pub code: String,
    pub sub_code: Option<String>,
    pub life: PermissionLife,
    pub use_times: AtomicU32,
    pub purpose: String,
    pub create_time: chrono::DateTime<chrono::Local>,
}
impl AgentPermission {
    pub fn new(
        code: String,
        sub_code: Option<String>,
        purpose: String,
        life: PermissionLife,
    ) -> Self {
        Self {
            id: 0,
            code,
            sub_code,
            purpose,
            life,
            use_times: AtomicU32::new(0),
            create_time: chrono::Local::now(),
        }
    }
    pub fn auth_code(&self) -> String {
        if let Some(sub_code) = &self.sub_code {
            format!("{}[{}]", self.code, sub_code)
        } else {
            self.code.clone()
        }
    }
    pub fn is_valid(&self) -> bool {
        let use_times = self.use_times.load(std::sync::atomic::Ordering::SeqCst);
        match self.life {
            PermissionLife::None => false,
            PermissionLife::Once => use_times == 0,
            PermissionLife::Times(times) => use_times < times,
            PermissionLife::Duration(duration) => {
                let expire_time = self.create_time + chrono::Duration::from_std(duration).unwrap();
                chrono::Local::now() < expire_time
            }
            PermissionLife::Forever => true,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum PermissionLife {
    #[default]
    None,
    Once,
    Times(u32),
    Duration(std::time::Duration),
    Forever,
}

pub struct AgentAuthorityContext {
    permissions: DashMap<String, Arc<AgentPermission>>,
}
impl AgentAuthorityContext {
    pub fn new() -> Self {
        Self {
            permissions: DashMap::new(),
        }
    }
    pub fn add(&self, p: AgentPermission) {
        let code = auth_code(&p.code, p.sub_code.as_deref());
        self.permissions.insert(code, Arc::new(p));
    }
    pub fn remove(&self, code: &str) {
        self.permissions.remove(code);
    }
    pub fn contains_key(&self, code: &str) -> bool {
        self.permissions.contains_key(code)
    }
}

fn auth_code(code: &str, sub_code: Option<&str>) -> String {
    if let Some(sub_code) = sub_code {
        format!("{}[{}]", code, sub_code)
    } else {
        code.to_string()
    }
}

#[async_trait]
pub trait AgentAuthorityHandler: Send + Sync + 'static {
    //授予权限
    async fn grant(&self, permission: AgentPermission) -> anyhow::Result<i64>;
    //撤销权限
    async fn revoke(&self, id: i64) -> anyhow::Result<()>;
    //验证是否授权
    fn has_permission(&self, code: &str, sub_code: Option<&str>) -> bool;
}
