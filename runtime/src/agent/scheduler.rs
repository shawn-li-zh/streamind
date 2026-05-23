// use std::{
//     sync::{
//         Arc, OnceLock,
//         atomic::{AtomicBool, Ordering},
//     },
//     time::Duration,
// };

// use async_trait::async_trait;
// use dashmap::DashMap;

// use entity_service::DB;
// use futures::{
//     SinkExt,
//     channel::{mpsc, oneshot::Receiver},
// };
// use kernel::{
//     agent::{
//         AgentHandler, AgentIdentity, AgentSchedulerHandler,
//         context::{AgentContext, AgentStatus},
//     },
//     event::TimerData,
//     timer::{AgentSchedulerContext, AgentTimer},
// };

// use tokio::{runtime, sync::watch};
// use tokio_cron_scheduler::{Job, JobScheduler};
// use uuid::Uuid;

// use crate::agent::channel::{AgentChannel, AgentStatusHolder};

// pub(crate) static SCHEDULER: OnceLock<JobScheduler> = OnceLock::new();

// pub struct AgentScheduler {
//     identity: Arc<AgentIdentity>,
//     context: Arc<AgentSchedulerContext>,
//     sender: mpsc::Sender<(Arc<AgentTimer>, bool)>,
// }
// impl AgentScheduler {
//     pub fn new(
//         identity: Arc<AgentIdentity>,
//         context: Arc<AgentSchedulerContext>,
//     ) -> (Self, mpsc::Receiver<(Arc<AgentTimer>, bool)>) {
//         let (sx, rx) = mpsc::channel(100);
//         (
//             Self {
//                 identity,
//                 context,
//                 sender: sx,
//             },
//             rx,
//         )
//     }
// }
// #[async_trait]
// impl AgentSchedulerHandler for AgentScheduler {
//     async fn add(&self, mut timer: AgentTimer) -> anyhow::Result<i64> {
//         let db = DB.get().unwrap();
//         let agent_id = self.identity.id();
//         let id = entity_service::agent_timer::create(
//             db,
//             agent_id,
//             timer.corn.clone(),
//             timer.times,
//             timer.payload.clone(),
//         )
//         .await?;
//         timer.id = id;
//         self.context.add(timer);
//         Ok(id)
//     }

//     async fn cancel(&self, id: i64) -> anyhow::Result<()> {
//         let db = DB.get().unwrap();
//         entity_service::agent_timer::cancel(db, id).await?;
//         self.context.remove(id);
//         Ok(())
//     }
// }

// pub struct AgentSchedulerRunner {
//     agent: Arc<dyn AgentHandler>,
//     context: Arc<AgentSchedulerContext>,
//     id_uuids: DashMap<i64, Uuid>,
//     uuid_timers: DashMap<Uuid, Arc<AgentTimer>>,
//     receiver: mpsc::Receiver<(Arc<AgentTimer>, bool)>,
// }
// impl AgentSchedulerRunner {
//     pub fn new(
//         agent: Arc<dyn AgentHandler>,
//         receiver: mpsc::Receiver<(Arc<AgentTimer>, bool)>,
//     ) -> Self {
//         let context: Arc<AgentSchedulerContext> =
//             agent.context().get::<AgentSchedulerContext>().unwrap();
//         let runner = Self {
//             agent,
//             context,
//             id_uuids: DashMap::new(),
//             uuid_timers: DashMap::new(),
//             receiver,
//         };
//         runner
//     }
//     pub fn start(mut self) -> anyhow::Result<()> {
//         let var = std::env::var("HEARTBEAT_INTERVAL").unwrap_or("10".to_string());
//         let secs = var.parse::<u64>()?;

//         let interval = std::time::Duration::from_secs(secs);
//         tokio::spawn(async move {
//             let sched = SCHEDULER.get().unwrap();
//             while let Ok((timer, is_add)) = self.receiver.recv().await {
//                 //添加进调度器
//                 if is_add {
//                     let job = self.to_job(timer.clone()).unwrap();
//                     self.uuid_timers.insert(job.guid(), timer.clone());
//                     self.id_uuids.insert(timer.id, job.guid());
//                     sched.add(job).await.unwrap();
//                     continue;
//                 }
//                 //移出调度器
//                 if let Some((id, uuid)) = self.id_uuids.remove(&timer.id) {
//                     sched.remove(&uuid).await.unwrap();
//                     self.uuid_timers.remove(&uuid);
//                     self.context.remove(id);
//                 }
//             }
//         });
//         Ok(())
//     }

//     fn to_job(&self, timer: Arc<AgentTimer>) -> anyhow::Result<Job> {
//         let context = self.context.clone();
//         let agent = self.agent.clone();
//         let uuid_timers = self.uuid_timers.clone();
//         let timer_id = timer.id;

//         let job: Job = Job::new_async(&timer.corn, move |uuid, l| {
//             let context = context.clone();
//             let agent = agent.clone();
//             let uuid_timers = uuid_timers.clone();
//             Box::pin(async move {
//                 if let Some(timer) = uuid_timers.get(&uuid) {
//                     let times = timer
//                         .trigger_times
//                         .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

//                     let event = Event::Timer(TimerData {
//                         id: timer.id,
//                         corn: timer.corn.clone(),
//                         payload: timer.payload.clone(),
//                         times: timer.times,
//                         trigger_times: times + 1,
//                     });
//                     let _ = agent.send_event(event, false).await.unwrap();
//                     if !timer.is_disused() {
//                         return;
//                     }
//                 }
//                 // 删除定时器
//                 l.remove(&uuid).await.unwrap();
//                 uuid_timers.remove(&uuid);
//                 context.remove(timer_id);
//             })
//         })?;
//         Ok(job)
//     }
// }

// pub struct AgentHeartbeat {
//     agent: Arc<dyn AgentHandler>,
//     status: Arc<AgentStatusHolder>,
//     flag: Arc<AtomicBool>,
// }

// impl AgentHeartbeat {
//     pub fn new(agent: Arc<dyn AgentHandler>, status: Arc<AgentStatusHolder>) -> Self {
//         Self {
//             agent,
//             status,
//             flag: Arc::new(AtomicBool::new(false)),
//         }
//     }
//     pub fn reset(&self) {
//         self.flag.store(false, Ordering::SeqCst);
//     }
//     async fn heartbeat(&self, interval: Duration) {
//         let mut interval = tokio::time::interval(interval);
//         interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
//         interval.tick().await;
//         loop {
//             interval.tick().await;
//             //只有空闲时才发送心跳
//             let status = self.status.get_status().await;
//             if status != AgentStatus::Idle {
//                 continue;
//             }
//             if self
//                 .flag
//                 .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
//                 .is_err()
//             {
//                 println!("Previous heartbeat still pending, skipping...");
//                 continue;
//             }
//             self.agent
//                 .send_event(Event::Heartbeat, false)
//                 .await
//                 .unwrap();
//             println!("heartbeat...{:?}", status);
//         }
//     }
// }

use async_trait::async_trait;
use std::sync::{Arc, atomic::AtomicU32};

use dashmap::DashMap;

#[derive(Debug)]
pub struct AgentTimer {
    pub id: i64,
    pub corn: String,
    pub times: u32,
    pub payload: String,
    pub trigger_times: AtomicU32,
}
impl AgentTimer {
    pub fn new(corn: String, times: u32, payload: String) -> Self {
        Self {
            id: 0,
            corn,
            times,
            payload,
            trigger_times: AtomicU32::new(0),
        }
    }

    pub fn is_disused(&self) -> bool {
        if self.times == 0 {
            return false;
        }
        self.trigger_times
            .load(std::sync::atomic::Ordering::Relaxed)
            >= self.times
    }
}

pub struct AgentSchedulerContext {
    timers: DashMap<i64, Arc<AgentTimer>>,
}
impl AgentSchedulerContext {
    pub fn new() -> Self {
        Self {
            timers: DashMap::new(),
        }
    }
    pub fn add(&self, t: AgentTimer) -> Arc<AgentTimer> {
        let t = Arc::new(t);
        self.timers.insert(t.id, t.clone());
        t
    }
    pub fn remove(&self, id: i64) -> Option<Arc<AgentTimer>> {
        self.timers.remove(&id).map(|v| v.1)
    }
    pub fn contains_key(&self, id: i64) -> bool {
        self.timers.contains_key(&id)
    }
    pub fn iter(&self) -> dashmap::iter::Iter<'_, i64, Arc<AgentTimer>> {
        self.timers.iter()
    }
}

#[async_trait]
pub trait AgentSchedulerHandler: Send + Sync + 'static {
    //添加定时任务
    async fn add(&self, timer: AgentTimer) -> anyhow::Result<i64>;
    //取消定时任务
    async fn cancel(&self, id: i64) -> anyhow::Result<()>;
}
