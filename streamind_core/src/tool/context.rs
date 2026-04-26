use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::Duration,
};

use tokio::sync::RwLock;

use crate::{
    agent::{AgentContext, AgentInfo},
    tool::{
        SafeAny, Tool, ToolPackage, call,
        plugin::{
            interceptor::{ContentSizeLimiter, RateLimiter, ToolCallnterceptor},
            wrapper::TimeoutWrapper,
        },
    },
};
static TIME_OUT: LazyLock<u64> = LazyLock::new(|| {
    //默认5分钟
    let timeout = std::env::var("TOOL_CALL_TIME_OUT")
        .map(|v| u64::from_str_radix(&v, 10).expect("env TOOL_CALL_TIME_OUT"))
        .unwrap_or(60 * 5);
    timeout
});
pub struct ToolContext {
    pub wrapper: TimeoutWrapper,
    pub interceptors: Vec<Box<dyn ToolCallnterceptor + 'static + Send + Sync>>,
}
impl ToolContext {
    pub fn from(agent_info: &AgentInfo, pkg: &'static ToolPackage, tool: &'static Tool) -> Self {
        let mut ctx = ToolContext {
            wrapper: TimeoutWrapper {
                time: Duration::from_secs(*TIME_OUT),
            },
            interceptors: vec![],
        };
        if let Some(build) = RateLimiter::build(agent_info, pkg, tool) {
            ctx.interceptors.push(Box::new(build));
        }
        if let Some(build) = ContentSizeLimiter::build(agent_info, pkg, tool) {
            ctx.interceptors.push(Box::new(build));
        }
        ctx
    }
}
pub struct ToolPackageContext {
    pub data: Arc<RwLock<SafeAny>>,
    pub tool_contexts: HashMap<&'static str, Arc<ToolContext>>,
}

impl ToolPackageContext {
    pub fn form(agent_info: &AgentInfo, pkg: &'static ToolPackage) -> Self {
        let data = (pkg.context_builder)(agent_info);

        let mut map = HashMap::new();
        for (name, tool) in pkg.tools() {
            let ctx = ToolContext::from(agent_info, pkg, tool);
            map.insert(*name, Arc::new(ctx));
        }
        Self {
            data: data,
            tool_contexts: map,
        }
    }

    pub fn data(&self) -> Arc<RwLock<SafeAny>> {
        self.data.clone()
    }
}

pub struct ToolCallContext {
    pub agent_context: Arc<RwLock<AgentContext>>,
    pub context_data: Arc<RwLock<SafeAny>>,
    pub workspace: PathBuf,
    pkg: &'static ToolPackage,
    tool: &'static Tool,
}

impl ToolCallContext {
    pub async fn new(
        agent_context: Arc<RwLock<AgentContext>>,
        context_data: Arc<RwLock<SafeAny>>,
        pkg: &'static ToolPackage,
        tool: &'static Tool,
    ) -> Self {
        let workspace = {
            let ctx = agent_context.read().await;
            PathBuf::from(&ctx.info.workspace)
        };
        Self {
            agent_context,
            context_data,
            workspace,
            pkg,
            tool,
        }
    }
    pub fn get_real_path(&self, path: &str) -> (PathBuf, bool) {
        let real_path = self.workspace.join(path);
        let is_safe = real_path.starts_with(&self.workspace);
        (real_path, is_safe)
    }
    pub fn to_short_path(&self, path: &PathBuf) -> PathBuf {
        // 1. 获取工作区的绝对路径
        let root = Path::new(&self.workspace);

        let path = Path::new(path);
        if let Ok(path) = path.strip_prefix(root) {
            return path.to_path_buf();
        }
        path.to_path_buf()
    }

    pub fn new_temp_result_file(&self, ext: &str) -> (PathBuf, File) {
        call::new_temp_result_file(
            &PathBuf::from(&self.workspace),
            self.pkg.name,
            self.tool.name,
            ext,
        )
    }
}
