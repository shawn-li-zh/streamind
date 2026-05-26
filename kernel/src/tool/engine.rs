use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use futures::{Stream, StreamExt, stream::FuturesUnordered};
use tracing::warn;

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
        }
    }
    //code格式  package.tool(..args)
    pub async fn batch_call(&self, calls: Vec<ToolCall>) -> impl Stream<Item = ToolCallEvent> {
        async fn build(
            id: i64,
            res: anyhow::Result<ToolReturn>,
            elapsed: Duration,
        ) -> ToolCallEvent {
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
        // 执行阶段
        let items: Vec<CallItem> = calls
            .iter()
            .map(|v| CallItem {
                id: v.id,
                package: v.package.name,
                tool: v.tool.name,
                args: v.args.clone(),
            })
            .collect();
        async_stream::stream! {
            let mut workers = FuturesUnordered::new();
            yield ToolCallEvent::Start(items);
            for call in calls {
                let id = call.id;
                yield ToolCallEvent::Active(id);

                let agent_identity = self.agent_identity.clone();
                let agent_context = self.agent_context.clone();
                let start = Instant::now();

                // 将 Future 放入集合，但不立即 await
                workers.push(async move {
                    let res = call.exec(agent_identity, agent_context).await;
                    build(id, res, start.elapsed()).await
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
