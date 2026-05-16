use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Local;
use dashmap::DashMap;
use kernel::{
    agent::{AgentContext, AgentTimer},
    event::{EnvEvent, TimerData},
    message::Message,
};
use tokio::{sync::RwLock, time::sleep_until};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::agent::channel::AgentChannel;

pub struct AgentScheduler {
    pub context: Arc<AgentContext>,
    pub channel: Arc<AgentChannel>,
    pub timers: DashMap<i64, AgentTimer>,
    pub sched: Arc<JobScheduler>,
    map: Arc<DashMap<String, i64>>,
}
impl AgentScheduler {
    pub async fn push_scheduled(&self, payload: Message, time: Instant) {

        // agent.timers.insert(timer.id, timer);
    }
    async fn run_scheduled(&self) -> anyhow::Result<()> {
        for timer in &self.timers {
            if timer.is_disused() {
                continue;
            }
            let channel = self.channel.clone();
            let map = self.map.clone();
            let timers = self.timers.clone();
            let job = Job::new_async(&timer.corn, move |uuid, l| {
                let channel = channel.clone();
                let map = map.clone();
                let timers = timers.clone();
                Box::pin(async move {
                    if let Some(id) = map.get(&uuid.to_string()) {
                        if let Some(mut timer) = timers.get_mut(&id) {
                            timer.trigger_times += 1;
                            let event = EnvEvent::Timer(TimerData {
                                id: timer.id,
                                corn: timer.corn.clone(),
                                payload: timer.payload.clone(),
                                times: timer.times,
                                trigger_times: timer.trigger_times,
                            });
                            let _ = channel.send_event(event, false).await.unwrap();
                            if !timer.is_disused() {
                                return;
                            }
                        }
                    }
                    l.remove(&uuid).await.unwrap();
                })
            })?;
            let id = timer.id;
            self.map.insert(job.guid().to_string(), id);
            self.sched.add(job).await?;
        }
        Ok(())
    }
    async fn heartbeat(&self, interval: Duration) {
        let mut interval = tokio::time::interval(interval);

        loop {
            let info = self.context.info.read().await;
            if let Some(date_time) = &info.sleep_until {
                // 休眠中
                let deadline = *date_time - Local::now();
                let deadline = Instant::now() + deadline.to_std().unwrap();
                sleep_until(deadline.into()).await;
                drop(info);
                let mut info = self.context.info.write().await;
                info.sleep_until = None
            }
            interval.tick().await;

            self.channel
                .send_event(EnvEvent::Heartbeat, false)
                .await
                .unwrap();
        }
    }
}
