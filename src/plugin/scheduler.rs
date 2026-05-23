use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
    time::Duration,
};

use async_trait::async_trait;
use dashmap::DashMap;

use entity_service::DB;
use kernel::agent::{AgentIdentity, AgentOutfit};

use ::runtime::agent::{
    event::{AppEnvEventData, TimerData},
    scheduler::{AgentSchedulerContext, AgentSchedulerHandler, AgentTimer},
};

use tokio::sync::mpsc;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::{agent::AppAgent, global::SCHEDULER};

pub struct AgentScheduler {
    identity: Arc<AgentIdentity>,
    context: Arc<AgentSchedulerContext>,
    sender: mpsc::Sender<(Arc<AgentTimer>, bool)>,
}
impl AgentScheduler {
    pub fn new(
        identity: Arc<AgentIdentity>,
        context: Arc<AgentSchedulerContext>,
    ) -> (Self, mpsc::Receiver<(Arc<AgentTimer>, bool)>) {
        let (sx, rx) = mpsc::channel(100);
        (
            Self {
                identity,
                context,
                sender: sx,
            },
            rx,
        )
    }
}
#[async_trait]
impl AgentSchedulerHandler for AgentScheduler {
    async fn add(&self, mut timer: AgentTimer) -> anyhow::Result<i64> {
        let db = DB.get().unwrap();
        let agent_id = self.identity.id();
        let id = entity_service::agent_timer::create(
            db,
            agent_id,
            timer.corn.clone(),
            timer.times,
            timer.payload.clone(),
        )
        .await?;
        timer.id = id;
        let timer = self.context.add(timer);
        self.sender.send((timer, true)).await?;
        Ok(id)
    }

    async fn cancel(&self, id: i64) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        if let Some(timer) = self.context.remove(id) {
            entity_service::agent_timer::cancel(db, id).await?;
            self.sender.send((timer, false)).await?;
        }
        Ok(())
    }
}

pub struct AgentSchedulerRunner {
    agent: Arc<AppAgent>,
    context: Arc<AgentSchedulerContext>,
    id_uuids: DashMap<i64, Uuid>,
    uuid_timers: DashMap<Uuid, Arc<AgentTimer>>,
    receiver: mpsc::Receiver<(Arc<AgentTimer>, bool)>,
}
impl AgentSchedulerRunner {
    pub fn new(agent: Arc<AppAgent>, receiver: mpsc::Receiver<(Arc<AgentTimer>, bool)>) -> Self {
        let context = agent.context();
        let context: Arc<AgentSchedulerContext> = context.get::<AgentSchedulerContext>().unwrap();
        let runner = Self {
            agent,
            context,
            id_uuids: DashMap::new(),
            uuid_timers: DashMap::new(),
            receiver,
        };
        runner
    }
    pub fn start(mut self) {
        let sched = SCHEDULER.get().unwrap();
        tokio::spawn(async move {
            while let Some((timer, is_add)) = self.receiver.recv().await {
                //添加进调度器
                if is_add {
                    let job = self.to_job(timer.clone()).unwrap();
                    self.uuid_timers.insert(job.guid(), timer.clone());
                    self.id_uuids.insert(timer.id, job.guid());
                    sched.add(job).await.unwrap();
                    continue;
                }
                //移出调度器
                if let Some((id, uuid)) = self.id_uuids.remove(&timer.id) {
                    sched.remove(&uuid).await.unwrap();
                    self.uuid_timers.remove(&uuid);
                    self.context.remove(id);
                }
            }
        });
    }

    fn to_job(&self, timer: Arc<AgentTimer>) -> anyhow::Result<Job> {
        let context = self.context.clone();
        let agent = self.agent.clone();
        let uuid_timers = self.uuid_timers.clone();
        let timer_id = timer.id;

        let job: Job = Job::new_async(&timer.corn, move |uuid, l| {
            let context = context.clone();
            let agent = agent.clone();
            let uuid_timers = uuid_timers.clone();
            Box::pin(async move {
                if let Some(timer) = uuid_timers.get(&uuid) {
                    let times = timer
                        .trigger_times
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    let event = AppEnvEventData::Timer(TimerData {
                        id: timer.id,
                        corn: timer.corn.clone(),
                        payload: timer.payload.clone(),
                        times: timer.times,
                        trigger_times: times + 1,
                    });
                    let _ = agent.send_event((event, false)).await.unwrap();
                    if !timer.is_disused() {
                        return;
                    }
                }
                // 删除定时器
                l.remove(&uuid).await.unwrap();
                uuid_timers.remove(&uuid);
                context.remove(timer_id);
            })
        })?;
        Ok(job)
    }
}
