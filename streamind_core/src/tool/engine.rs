use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Error, Result, anyhow};

use tokio::{
    sync::{RwLock, mpsc},
    task::JoinHandle,
};
use tracing::debug;

use crate::{
    agent::{AgentContext, AgentInfo},
    event::{ToolCallEvent, ToolCallResultItem, ToolCallStatus},
    tool::{ToolPackagePool, call::ToolCall, context::ToolPackageContext, core::ToolPackage},
};

pub struct ToolEngineContext {
    pub tool_packages: HashSet<&'static str>,
    pub package_pool: &'static ToolPackagePool,
    package_contexts: HashMap<&'static str, Arc<ToolPackageContext>>,
}
impl ToolEngineContext {
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

    pub fn build(
        agent_info: &AgentInfo,
        tool_packages: HashSet<&'static str>,
        package_pool: &'static ToolPackagePool,
    ) -> Self {
        let mut ctx = Self {
            tool_packages,
            package_pool,
            package_contexts: HashMap::new(),
        };
        ctx.init(agent_info);
        ctx
    }
    fn init(&mut self, agent_info: &AgentInfo) {
        for pkg_name in &self.tool_packages {
            let pkg = self.package_pool.get(pkg_name).expect("工具包不存在");
            let pkg_ctx = ToolPackageContext::form(agent_info, pkg);
            self.package_contexts.insert(pkg.name, Arc::new(pkg_ctx));
        }
    }
}

pub struct ToolEngine {
    agent_context: Arc<RwLock<AgentContext>>,
    context: Arc<ToolEngineContext>,
}
impl ToolEngine {
    pub fn new(agent_context: Arc<RwLock<AgentContext>>, context: Arc<ToolEngineContext>) -> Self {
        Self {
            agent_context,
            context,
        }
    }
    //code格式 await package.tool(..args)
    pub async fn batch_call(
        &self,
        calls: Vec<ToolCall>,
        sender: mpsc::Sender<ToolCallEvent>,
    ) -> anyhow::Result<()> {
        // 执行阶段
        let mut concurrent_tasks: Vec<JoinHandle<()>> = Vec::new();
        async fn send(
            sender: mpsc::Sender<ToolCallEvent>,
            id: i64,
            res: Result<serde_json::Value, anyhow::Error>,
            time: Duration,
        ) {
            let mut item = ToolCallResultItem {
                id,
                content: String::new(),
                time,
                status: ToolCallStatus::Undo,
            };
            let event = match res {
                Ok(res) => {
                    item.content = res.to_string();
                    item.status = ToolCallStatus::Success;
                    ToolCallEvent::Success(item)
                }
                Err(e) => {
                    item.content = format!("{e}");
                    item.status = ToolCallStatus::Failure;
                    ToolCallEvent::Failure(item)
                }
            };
            sender.send(event).await.unwrap();
        }

        for call in calls {
            let id = call.id;
            debug!("{}.{}(...)", call.package.name, call.tool.name);
            let start = Instant::now();
            sender.send(ToolCallEvent::Active(call.id)).await.unwrap();
            //
            let sender: mpsc::Sender<ToolCallEvent> = sender.clone();
            let agent_ctx = self.agent_context.clone();
            let handle = tokio::spawn(async move {
                let res = call.exec(agent_ctx).await;
                send(sender, id, res, start.elapsed()).await
            });
            concurrent_tasks.push(handle);
        }
        // 回收并发结果
        for handle in concurrent_tasks {
            // handle.await 可能会返回 join 错误（例如 panic 了）
            handle.await?;
        }
        Ok(())
    }
    pub fn batch_preprocess(&self, codes: Vec<String>) -> PreprocessResult {
        //
        //语法解析
        let mut valid_tasks: Vec<ToolCall> = Vec::with_capacity(codes.len());
        let mut invalid = Vec::with_capacity(codes.len());
        for i in 0..codes.len() {
            let code = &codes[i];
            let id = i as i64;
            match ToolCall::from(id, code, self.context.as_ref()) {
                Ok(d) => {
                    valid_tasks.push(d);
                }
                Err(e) => {
                    invalid.push((id, e));
                }
            }
        }
        if !invalid.is_empty() {
            return PreprocessResult::Invalid(invalid);
        }
        PreprocessResult::Valid(valid_tasks)
    }
}

pub enum PreprocessResult {
    Valid(Vec<ToolCall>),
    Invalid(Vec<(i64, Error)>),
}
