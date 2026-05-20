use std::{
    sync::{Arc, OnceLock, atomic::AtomicBool},
    time::Duration,
};

use async_trait::async_trait;
use dashmap::DashMap;

use entity_service::DB;
use futures::{
    SinkExt,
    channel::{mpsc, oneshot::Receiver},
};
use kernel::{
    agent::{
        AgentSchedulerHandler,
        context::{AgentContext, AgentStatus, AgentTimer},
    },
    event::{EnvEvent, TimerData},
};

use tokio::sync::watch;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::agent::channel::AgentChannel;

pub(crate) static SCHEDULER: OnceLock<JobScheduler> = OnceLock::new();

pub struct AgentScheduler {
    pub context: Arc<AgentContext>,
    pub channel: Arc<AgentChannel>,
    timers: DashMap<i64, (Arc<AgentTimer>, Uuid)>,
    uuid_timers: DashMap<Uuid, Arc<AgentTimer>>,
    is_running: AtomicBool,
    watch_dog: Arc<AtomicBool>,
}
impl AgentScheduler {
    pub fn new(context: Arc<AgentContext>, channel: Arc<AgentChannel>) -> Self {
        Self {
            context,
            channel,
            timers: DashMap::new(),
            uuid_timers: DashMap::new(),
            is_running: AtomicBool::new(false),
            watch_dog: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn start(self: Arc<Self>) -> anyhow::Result<()> {
        if self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            anyhow::bail!("AgentScheduler is already running");
        }
        self.is_running
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let var = std::env::var("HEARTBEAT_INTERVAL").unwrap_or("10".to_string());
        let secs = var.parse::<u64>()?;

        let interval = std::time::Duration::from_secs(secs);

        let s = self.clone();
        tokio::spawn(async move {
            //添加进调度器
            for timer in self.context.timers.iter() {
                self.add_timer(timer.clone()).await.unwrap();
            }
            //启动心跳
            s.heartbeat(interval).await;
        });
        Ok(())
    }
    pub async fn add_timer(&self, timer: Arc<AgentTimer>) -> anyhow::Result<()> {
        if timer.is_disused() {
            return Ok(());
        }
        let id = timer.id;
        let job = self.to_job(timer.clone())?;
        self.uuid_timers.insert(job.guid(), timer.clone());
        self.timers.insert(id, (timer, job.guid()));
        let sched = SCHEDULER.get().unwrap();
        sched.add(job).await?;
        Ok(())
    }
    pub async fn remove_timer(&self, id: i64) -> anyhow::Result<()> {
        if let Some((_, (_, uuid))) = self.timers.remove(&id) {
            let sched = SCHEDULER.get().unwrap();
            sched.remove(&uuid).await?;
            self.uuid_timers.remove(&uuid);
        }
        Ok(())
    }

    async fn heartbeat(&self, interval: Duration) {
        let (mut sender, mut receiver) = mpsc::channel::<()>(1);
        let watch_dog = Arc::new(AtomicBool::new(false));
        tokio::spawn(async move {
            while let Ok(_) = receiver.recv().await {
                watch_dog.store(false, std::sync::atomic::Ordering::SeqCst);
                println!("dog...");
            }
        });
        let mut interval = tokio::time::interval(interval);
        interval.tick().await;
        loop {
            interval.tick().await;
            let is_send = self
                .watch_dog
                .fetch_or(true, std::sync::atomic::Ordering::SeqCst);
            if is_send {
                continue;
            }
            //只有空闲时才发送心跳
            let status = self.channel.get_status().await;
            if status != AgentStatus::Idle {
                continue;
            }
            self.channel
                .send_event(EnvEvent::Heartbeat, false)
                .await
                .unwrap();
            sender.send(()).await.unwrap();
            println!("heartbeat...{:?}", status);
        }
    }

    fn to_job(&self, timer: Arc<AgentTimer>) -> anyhow::Result<Job> {
        let channel = self.channel.clone();
        let timers = self.timers.clone();
        let uuid_timers = self.uuid_timers.clone();
        let timer_id = timer.id;
        let job: Job = Job::new_async(&timer.corn, move |uuid, l| {
            let channel = channel.clone();

            let timers = timers.clone();
            let uuid_timers = uuid_timers.clone();
            Box::pin(async move {
                if let Some(timer) = uuid_timers.get(&uuid) {
                    let times = timer
                        .trigger_times
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    let event = EnvEvent::Timer(TimerData {
                        id: timer.id,
                        corn: timer.corn.clone(),
                        payload: timer.payload.clone(),
                        times: timer.times,
                        trigger_times: times + 1,
                    });
                    let _ = channel.send_event(event, false).await.unwrap();
                    if !timer.is_disused() {
                        return;
                    }
                }
                // 删除定时器
                l.remove(&uuid).await.unwrap();
                uuid_timers.remove(&uuid);
                timers.remove(&timer_id);
            })
        })?;
        Ok(job)
    }
}
#[async_trait]
impl AgentSchedulerHandler for AgentScheduler {
    async fn add(&self, mut timer: AgentTimer) -> anyhow::Result<i64> {
        let db = DB.get().unwrap();
        let agent_id = self.context.agent_id;
        let id = entity_service::agent_timer::create(
            db,
            agent_id,
            timer.corn.clone(),
            timer.times,
            timer.payload.clone(),
        )
        .await?;
        timer.id = id;
        self.add_timer(Arc::new(timer)).await?;
        Ok(id)
    }

    async fn cancel(&self, id: i64) -> anyhow::Result<()> {
        let db = DB.get().unwrap();
        entity_service::agent_timer::cancel(db, id).await?;
        self.remove_timer(id).await?;
        Ok(())
    }
}
