use std::sync::Arc;

use async_trait::async_trait;
use entity_service::DB;
use kernel::agent::{
    AgentAuthorityHandler,
    context::{AgentContext, AgentPermission},
};

pub struct AgentAuthority {
    pub(crate) context: Arc<AgentContext>,
}
#[async_trait]
impl AgentAuthorityHandler for AgentAuthority {
    //授予权限
    async fn grant(&self, permission: AgentPermission) -> anyhow::Result<i64> {
        let db = DB.get().unwrap();
        let agent_id = self.context.agent_id;
        let id = entity_service::agent_permission::create(
            db,
            agent_id,
            permission.code.clone(),
            permission.sub_code.clone(),
            permission.purpose.clone(),
            permission.life.clone(),
        )
        .await?;
        self.context
            .permissions
            .insert(permission.auth_code(), permission);
        Ok(id)
    }
    //撤销权限
    async fn revoke(&self, id: i64) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        let agent_id = self.context.agent_id;
        let model = entity_service::agent_permission::get(agent_id, id).await?;
        if let Some(model) = model {
            let auth_code = auth_code(&model.code, model.sub_code.as_deref());
            entity_service::agent_permission::revoke(db, agent_id, id).await?;
            self.context.permissions.remove(&auth_code);
        }
        Ok(())
    }
    //验证是否授权
    fn has_permission(&self, code: &str, sub_code: Option<&str>) -> bool {
        let auth_code = auth_code(code, sub_code);
        self.context.permissions.contains_key(&auth_code)
    }
}
fn auth_code(code: &str, sub_code: Option<&str>) -> String {
    if let Some(sub_code) = sub_code {
        format!("{}[{}]", code, sub_code)
    } else {
        code.to_string()
    }
}
