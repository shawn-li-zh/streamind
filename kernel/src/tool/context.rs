use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::Duration,
};

use chrono::Local;

use crate::{
    agent::{AgentIdentity, context::AgentContext},
    tool::{
        ContextData, Tool, ToolPackage,
        plugin::{
            interceptor::{
                ContentSizeLimiter, PermissonChecker, RateLimiter, ToolAuthority,
                ToolCallnterceptor,
            },
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
    pub fn from(
        agent_context: Arc<AgentContext>,
        pkg: &'static ToolPackage,
        tool: &'static Tool,
    ) -> Self {
        let mut ctx = ToolContext {
            wrapper: TimeoutWrapper {
                time: Duration::from_secs(*TIME_OUT),
            },
            interceptors: vec![],
        };
        if let Some(build) = RateLimiter::build(agent_context.clone(), pkg, tool) {
            ctx.interceptors.push(Box::new(build));
        }
        if let Some(build) = ContentSizeLimiter::build(agent_context.clone(), pkg, tool) {
            ctx.interceptors.push(Box::new(build));
        }
        if let Some(build) = PermissonChecker::build(agent_context.clone(), pkg, tool) {
            ctx.interceptors.push(Box::new(build));
        }
        ctx
    }
}
pub struct ToolPackageContext {
    pkg: &'static ToolPackage,
    agent_context: Arc<AgentContext>,
    pub data: ContextData,
    pub tool_contexts: HashMap<&'static str, Arc<ToolContext>>,
}

impl ToolPackageContext {
    pub fn form(agent_context: Arc<AgentContext>, pkg: &'static ToolPackage) -> Self {
        let data = (pkg.context_builder)(agent_context.clone());

        let mut map: HashMap<&str, Arc<ToolContext>> = HashMap::new();
        for (name, tool) in pkg.tools() {
            let ctx = ToolContext::from(agent_context.clone(), pkg, tool);
            map.insert(*name, Arc::new(ctx));
        }
        Self {
            pkg,
            agent_context,
            data: data,
            tool_contexts: map,
        }
    }

    pub fn data(&self) -> ContextData {
        self.data.clone()
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        (self.pkg.recycler)(self.agent_context.clone(), self.data()).await?;
        Ok(())
    }
}

pub struct ToolCallContext {
    // pub agent_context: Arc<AgentContext>,
    pub agent_identity: Arc<AgentIdentity>,
    pub agent_context: Arc<AgentContext>,
    pub context_data: ContextData,
    pub workspace: PathBuf,
    pkg: &'static ToolPackage,
    tool: &'static Tool,
    pub call_id: String,
}

impl ToolCallContext {
    pub fn new(
        agent_identity: Arc<AgentIdentity>,
        agent_context: Arc<AgentContext>,
        context_data: ContextData,
        pkg: &'static ToolPackage,
        tool: &'static Tool,
        call_id: String,
    ) -> Self {
        let workspace = agent_identity.workspace.clone();
        Self {
            agent_identity,
            agent_context,
            context_data,
            workspace,
            pkg,
            tool,
            call_id,
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
    pub fn new_file(&self, filepath: &str) -> anyhow::Result<(PathBuf, File)> {
        let (real_path, is_safe) = self.get_real_path(filepath);
        if !is_safe {
            if !self.has_permission("FILE_UNSAFE", real_path.to_str()) {
                return Err(anyhow::anyhow!("无权限"));
            }
        }
        if let Some(parent) = real_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).unwrap();
            }
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true) // 允许写入
            .create(true) // 如果不存在则创建
            .truncate(true) // 如果存在，清空文件内容
            .open(&real_path)?;
        Ok((real_path, file))
    }
    pub fn new_temp_result_file(&self, ext: &str) -> anyhow::Result<(PathBuf, File)> {
        let pkg_dir = format!("tool/{}/temp_result", self.pkg.name,);
        let now = Local::now();
        let ymd = now.format("%Y%m%d").to_string();
        let file_name = format!("{}_{}.{}.{ext}", ymd, &self.call_id, self.tool.name);
        let path = format!("{}/{}", pkg_dir, file_name);
        self.new_file(&path)
    }
    pub fn has_permission(&self, code: &str, sub_code: Option<&str>) -> bool {
        let auth = self.agent_context.get::<ToolAuthority>().unwrap();
        auth.has_permission(code, sub_code)
    }
}
