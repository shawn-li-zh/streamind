use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use dashmap::DashMap;
use futures::{Stream, StreamExt, stream::FuturesUnordered};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::{
    agent::{AgentIdentity, context::AgentContext},
    tool::{
        ToolPackagePool,
        call::ToolCall,
        context::ToolPackageContext,
        core::{ToolPackage, ToolReturn, ToolReturnValue},
        event::{CallItem, ToolCallEvent, ToolCallResultItem, ToolCallStatus},
    },
};

pub struct ToolEngineContext {
    pub tool_packages: HashSet<&'static str>,
    pub package_pool: &'static ToolPackagePool,
    package_contexts: DashMap<&'static str, Arc<ToolPackageContext>>,
}
impl ToolEngineContext {
    pub fn new(
        tool_packages: HashSet<&'static str>,
        package_pool: &'static ToolPackagePool,
    ) -> Self {
        Self {
            tool_packages: tool_packages,
            package_pool: package_pool,
            package_contexts: DashMap::new(),
        }
    }
    pub fn get_package(&self, pkg_name: &str) -> Option<&'static ToolPackage> {
        if !self.tool_packages.contains(pkg_name) && !self.tool_packages.contains("*") {
            return None;
        }
        self.package_pool.get(pkg_name)
    }
    pub fn get_package_context(&self, pkg_name: &str) -> Option<Arc<ToolPackageContext>> {
        self.package_contexts.get(pkg_name).map(|v| v.clone())
    }

    //初始化
    pub fn init(&self, agent_context: Arc<AgentContext>) {
        for pkg_name in &self.tool_packages {
            let pkg = self.package_pool.get(pkg_name).expect("工具包不存在");
            let pkg_ctx = ToolPackageContext::form(agent_context.clone(), pkg);
            self.package_contexts.insert(pkg.name, Arc::new(pkg_ctx));
        }
    }
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        for pkg_name in &self.tool_packages {
            let pkg = self.package_pool.get(pkg_name).expect("工具包不存在");
            let pkg_ctx = self.package_contexts.get(pkg.name).expect("工具包不存在");
            pkg_ctx.shutdown().await?;
        }
        Ok(())
    }
}
pub struct ToolEngine {
    agent_identity: Arc<AgentIdentity>,
    agent_context: Arc<AgentContext>,
    context: Arc<ToolEngineContext>,

    // 用于中断当前正在运行的所有任务
    cancel_token: Mutex<Option<CancellationToken>>,
    // 追踪当前活跃的 batch_call 数量
    running_count: Arc<AtomicUsize>,
}

impl ToolEngine {
    pub fn new(
        agent_identity: Arc<AgentIdentity>,
        agent_context: Arc<AgentContext>,
        context: Arc<ToolEngineContext>,
    ) -> Self {
        Self {
            agent_identity,
            agent_context,
            context,
            cancel_token: Mutex::new(None),
            running_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 中断操作：如果有正在运行的任务，触发取消信号
    pub async fn stop(&self) {
        let mut guard = self.cancel_token.lock().await;
        if let Some(token) = guard.take() {
            token.cancel();
            info!("ToolEngine: Stop signal sent.");
        }
    }

    /// 是否正在运行
    pub fn is_running(&self) -> bool {
        self.running_count.load(Ordering::SeqCst) > 0
    }

    pub async fn batch_call(&self, calls: Vec<ToolCall>) -> impl Stream<Item = ToolCallEvent> {
        // 1. 准备取消令牌
        let token = {
            let mut guard = self.cancel_token.lock().await;
            let t = CancellationToken::new();
            *guard = Some(t.clone());
            t
        };

        // 2. 准备状态追踪
        let running_count = self.running_count.clone();
        running_count.fetch_add(1, Ordering::SeqCst);

        // 引用克隆用于闭包
        let items: Vec<CallItem> = calls
            .iter()
            .map(|v| CallItem {
                id: v.id,
                package: v.package.name,
                tool: v.tool.name,
                args: v.args.clone(),
            })
            .collect();

        let agent_identity = self.agent_identity.clone();
        let agent_context = self.agent_context.clone();

        async_stream::stream! {
            // 使用 ScopeGuard 思想确保无论 Stream 如何结束，计数器都会减少
            let _guard = RunningGuard(running_count);

            let mut workers = FuturesUnordered::new();
            yield ToolCallEvent::Start(items);

            for call in calls {
                // 在投放新任务前检查是否已取消
                if token.is_cancelled() { break; }

                let id = call.id;
                yield ToolCallEvent::Active(id);

                let identity = agent_identity.clone();
                let context = agent_context.clone();
                let start = Instant::now();
                let child_token = token.clone(); // 每个 worker 共享同一个根取消信号

                workers.push(async move {
                    // 使用 select! 监听取消信号，实现真正的立即中断
                    tokio::select! {
                        _ = child_token.cancelled() => {
                            // 如果任务被取消，这里可以返回一个特殊的状态或者空
                            // 这里返回 Err 代表被中断
                            let res = Err(anyhow::anyhow!("Interrupted by user"));
                            to_event(id, res, start.elapsed()).await
                        }
                        res = call.exec(identity, context) => {
                            to_event(id, res, start.elapsed()).await
                        }
                    }
                });
            }

             // 循环驱动所有 worker，谁先好谁先出来
            while let Some(event) = workers.next().await {
                yield event;
            }

            yield ToolCallEvent::End;
        }
    }
    pub fn preprocess(&self, call_id: String, code: &str) -> anyhow::Result<ToolCall> {
        ToolCall::from(call_id, code, self.context.as_ref())
    }
}

// 辅助结构体：确保运行状态在 Stream 结束（包括被 Drop）时正确重置
struct RunningGuard(Arc<AtomicUsize>);
impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

async fn to_event(id: i64, res: anyhow::Result<ToolReturn>, elapsed: Duration) -> ToolCallEvent {
    let mut item: ToolCallResultItem = ToolCallResultItem {
        id,
        values: vec![],
        elapsed: humantime::format_duration(elapsed).to_string(),
        status: ToolCallStatus::Undo,
    };
    let event = match res {
        Ok(res) => {
            item.values = res.values;
            item.status = ToolCallStatus::Success;
            ToolCallEvent::Success(item)
        }
        Err(e) => {
            warn!("Call failed: {}", &e);
            item.values.push(ToolReturnValue::Raw(e.to_string()));
            item.status = ToolCallStatus::Failure;
            ToolCallEvent::Failure(item)
        }
    };
    event
}
