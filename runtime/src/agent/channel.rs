use std::sync::Arc;

use axum::response::sse;

use entity_service::DB;
// use sea_orm::{DbErr, TransactionTrait};
use kernel::{
    agent::context::{AgentEvent, AgentInfo, AgentStatus},
    event::EnvEvent,
};

use tokio::sync::{RwLock, mpsc, watch};
use tracing::{info, warn};

use crate::sse_event::WebEvent;
// #[derive()]
pub struct AgentChannel {
    agent_info: Arc<RwLock<AgentInfo>>,
    event_sender: mpsc::Sender<AgentEvent>,
    pub(crate) status_sender: watch::Sender<AgentStatus>,
    //函数调用
    call_watcher: RwLock<watch::Receiver<bool>>,
    run_watcher: RwLock<watch::Receiver<bool>>,
    status_watcher: RwLock<watch::Receiver<AgentStatus>>,
    web_sender: RwLock<Option<mpsc::UnboundedSender<sse::Event>>>,
}

impl AgentChannel {
    pub fn new(
        agent_info: Arc<RwLock<AgentInfo>>,
        event_sender: mpsc::Sender<AgentEvent>,
        status_sender: watch::Sender<AgentStatus>,
        //
        call_watcher: RwLock<watch::Receiver<bool>>,
        run_watcher: RwLock<watch::Receiver<bool>>,
        status_watcher: RwLock<watch::Receiver<AgentStatus>>,
    ) -> Self {
        Self {
            agent_info,
            event_sender,
            status_sender,
            call_watcher,
            run_watcher,
            status_watcher,
            web_sender: RwLock::new(None),
        }
    }
    pub async fn set_web_sender(&self, sender: mpsc::UnboundedSender<sse::Event>) {
        self.web_sender.write().await.replace(sender);
    }
    pub async fn send_event(
        &self,
        event_data: EnvEvent,
        force: bool,
    ) -> anyhow::Result<(AgentEvent, entity::event::Model)> {
        //等待工具调用结束
        {
            let mut call_watcher = self.call_watcher.write().await;
            call_watcher.wait_for(|&is_paused| !is_paused).await?;
        }
        self.send_event_unblock(event_data, force).await
    }

    pub(crate) async fn send_event_unblock(
        &self,
        event_data: EnvEvent,
        force: bool,
    ) -> anyhow::Result<(AgentEvent, entity::event::Model)> {
        //保存数据库
        let db = DB.get().unwrap();
        let (event, model) = {
            let info = self.agent_info.read().await;

            entity_service::event::save(db, info.id, info.state_id, event_data, force).await?
        };
        //发送事件
        self.event_sender.send(event.clone()).await?;
        //发送到前端
        {
            match &event.data {
                EnvEvent::AgentResponse(_)
                | EnvEvent::UserRequest(_)
                | EnvEvent::Interaction(_)
                | EnvEvent::Interrupt(_)
                | EnvEvent::Heartbeat
                | EnvEvent::ToolCallResult(_) => {
                    //忽略
                }
                _ => {
                    info!("发送事件:{:?}", &event.data);
                    let web_event = WebEvent::Env(event.data.clone(), event.id);
                    self.send_web_event(web_event).await;
                }
            }
        }
        Ok((event, model))
    }
    pub async fn wait_run_is(&self, is: bool) -> anyhow::Result<()> {
        let mut run_watcher = self.run_watcher.write().await;
        run_watcher.wait_for(|v| *v == is).await?;
        Ok(())
    }

    pub async fn wait_call_is(&self, is: bool) -> anyhow::Result<()> {
        let mut call_watcher = self.call_watcher.write().await;
        call_watcher.wait_for(|v| *v == is).await?;
        Ok(())
    }
    pub async fn wait_status_is(&self, is: AgentStatus) -> anyhow::Result<()> {
        let mut status_watcher = self.status_watcher.write().await;
        status_watcher.wait_for(|v| *v == is).await?;
        Ok(())
    }

    pub fn send_status(&self, status: AgentStatus) -> anyhow::Result<()> {
        self.status_sender.send(status)?;
        Ok(())
    }

    pub(crate) async fn send_web_event(&self, e: WebEvent) {
        let web_sender = self.web_sender.read().await;
        if let Some(sender) = &*web_sender {
            match sender.send(e.to_sse_event()) {
                Ok(_) => {}
                Err(e) => {
                    //断开
                    info!("sse连接断开：{}", e);
                    drop(web_sender);
                    let mut web_sender = self.web_sender.write().await;
                    *web_sender = None;
                }
            };
            return;
        }
        warn!("断连中...");
    }
}
