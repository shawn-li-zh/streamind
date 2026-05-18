pub mod context;
pub mod error;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Local;

use crate::{
    agent::context::{AgentContext, AgentEvent, AgentPermission, AgentStatus, AgentTimer},
    event::EnvEvent,
};

#[async_trait]
pub trait AgentHandler: Send + Sync + 'static {
    //发送环境事件
    async fn send_event(&self, event: EnvEvent, force: bool) -> anyhow::Result<AgentEvent>;
    //睡眠
    async fn sleep_until(&self, time: chrono::DateTime<Local>) -> anyhow::Result<()>;
    //唤醒
    async fn wakeup(&self) -> anyhow::Result<()>;
    //停止
    async fn stop(&self) -> anyhow::Result<()>;
    //获取状态
    async fn get_status(&self) -> AgentStatus;
    //
    fn context(&self) -> Arc<AgentContext>;
    fn authority(&self) -> Arc<dyn AgentAuthorityHandler>;
    fn timer_handler(&self) -> Arc<dyn AgentSchedulerHandler>;
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

#[async_trait]
pub trait AgentSchedulerHandler: Send + Sync + 'static {
    //添加定时任务
    async fn add(&self, timer: AgentTimer) -> anyhow::Result<i64>;
    //取消定时任务
    async fn cancel(&self, id: i64) -> anyhow::Result<()>;
}

#[async_trait]
pub trait AgentCloneHandler: Send + Sync + 'static {
    //创建一个新代理
    async fn new(
        &self,
        model_id: u32,
        personal: String,
        root_state: String,
    ) -> anyhow::Result<(i64, Arc<dyn AgentHandler>)>;
    //发送消息
    async fn send_message(&self, agent_id: i64, message: String) -> anyhow::Result<()>;
}
