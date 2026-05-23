use std::sync::Arc;

use async_trait::async_trait;
use entity_service::DB;
use kernel::agent::AgentIdentity;
use runtime::agent::authority::{AgentAuthorityContext, AgentAuthorityHandler, AgentPermission};


pub struct AgentAuthority {
    pub identity: Arc<AgentIdentity>,
    pub context: Arc<AgentAuthorityContext>,
}
#[async_trait]
impl AgentAuthorityHandler for AgentAuthority {
    //授予权限
    async fn grant(&self, permission: AgentPermission) -> anyhow::Result<i64> {
        let db = DB.get().unwrap();
        let agent_id = self.identity.id();
        let id = entity_service::agent_permission::create(
            db,
            agent_id,
            permission.code.clone(),
            permission.sub_code.clone(),
            permission.purpose.clone(),
            permission.life.clone(),
        )
        .await?;
        self.context.add(permission);
        Ok(id)
    }
    //撤销权限
    async fn revoke(&self, id: i64) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        let agent_id = self.identity.id();
        let model: Option<entity::agent_permission::Model> =
            entity_service::agent_permission::get(agent_id, id).await?;
        if let Some(model) = model {
            let auth_code = auth_code(&model.code, model.sub_code.as_deref());
            entity_service::agent_permission::revoke(db, agent_id, id).await?;
            self.context.remove(&auth_code);
        }
        Ok(())
    }
    //验证是否授权
    fn has_permission(&self, code: &str, sub_code: Option<&str>) -> bool {
        let auth_code = auth_code(code, sub_code);
        self.context.contains_key(&auth_code)
    }
}

fn auth_code(code: &str, sub_code: Option<&str>) -> String {
    if let Some(sub_code) = sub_code {
        format!("{}[{}]", code, sub_code)
    } else {
        code.to_string()
    }
}
