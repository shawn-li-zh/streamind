use std::{
    any::Any,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use futures::{FutureExt, future::BoxFuture};

use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;

use crate::{
    agent::AgentContext,
    tool::{
        context::ToolPackageContext,
        value_type::{ToolValueType, TypeInfo, TypeItemInfo},
    },
};

pub type ToolHandler = Box<
    dyn Fn(
            HashMap<String, serde_json::Value>,
            ToolCallContext,
        ) -> BoxFuture<'static, anyhow::Result<serde_json::Value>>
        + Send
        + Sync,
>;

pub type ActiveFn = Box<
    dyn Fn(HashMap<String, serde_json::Value>, ToolCallContext) -> anyhow::Result<String>
        + Send
        + Sync,
>;
pub type SafeAny = dyn Any + Send + Sync + 'static;
pub type ContextBuilder = Box<dyn Fn(&AgentContext) -> Arc<RwLock<SafeAny>> + Send + Sync>;

pub fn async_tool_handler<F, Fut>(f: F) -> ToolHandler
where
    F: Fn(HashMap<String, serde_json::Value>, ToolCallContext) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = anyhow::Result<serde_json::Value>> + Send + 'static,
{
    Box::new(move |args, ctx| f(args, ctx).boxed())
}
pub fn sync_tool_handler<F>(f: F) -> ToolHandler
where
    F: Fn(
            HashMap<String, serde_json::Value>,
            ToolCallContext,
        ) -> Result<serde_json::Value, anyhow::Error>
        + Send
        + Sync
        + 'static,
{
    Box::new(move |args, ctx| {
        let res = f(args, ctx);
        async move { res }.boxed() // 将同步结果包装在 async 块中并 boxed
    })
}

pub type ToolPackagePool = HashMap<&'static str, ToolPackage>;
pub struct ToolCallContext {
    pub agent_context: Arc<RwLock<AgentContext>>,
    pub context_data: Arc<RwLock<SafeAny>>,
}
impl ToolCallContext {
    pub async fn get_safe_path(&self, path: &str) -> anyhow::Result<(PathBuf, PathBuf)> {
        let ctx = self.agent_context.read().await;
        ctx.get_safe_path(path)
    }
}

pub struct Tool {
    pub name: &'static str,
    //函数描述
    pub annotation: &'static str,
    pub example: &'static str,
    pub rate_limit: Option<(Duration, u32)>,
    pub type_signature: (Vec<TypeItemInfo>, Option<ToolValueType>),

    pub active: ActiveFn,
    pub handler: ToolHandler,
}

impl Tool {
    pub fn new(
        name: &'static str,
        annotation: &'static str,
        example: &'static str,
        rate_limit: Option<(Duration, u32)>,
        type_signature: (Vec<TypeItemInfo>, Option<ToolValueType>),
        active: ActiveFn,
        handler: ToolHandler,
    ) -> Self {
        Self {
            name,
            annotation,
            example,
            type_signature,
            rate_limit,
            active,
            handler,
        }
    }
    pub fn active_tip(
        &self,
        args: HashMap<String, serde_json::Value>,
        ctx: &ToolPackageContext,
        agent_ctx: Arc<RwLock<AgentContext>>,
    ) -> String {
        let ctx = ToolCallContext {
            context_data: ctx.data.clone(),
            agent_context: agent_ctx,
        };
        match (self.active)(args, ctx) {
            Ok(r) => r,
            Err(e) => format!("参数异常:{e}"),
        }
    }

    pub async fn call(
        &self,
        args: HashMap<String, serde_json::Value>,
        ctx: &ToolPackageContext,
        agent_ctx: Arc<RwLock<AgentContext>>,
    ) -> anyhow::Result<serde_json::Value> {
        if ctx.is_limited(self.name) {
            return Err(anyhow::anyhow!("调用频率过高"));
        };
        let ctx = ToolCallContext {
            context_data: ctx.data.clone(),
            agent_context: agent_ctx,
        };
        (self.handler)(args, ctx).await
    }

    pub fn to_doc(&self) -> String {
        //注释
        let mut annos = vec![self.annotation.to_string()];
        for arg in &self.type_signature.0 {
            annos.push(format!("@param {} {}", arg.name, arg.annotation));
        }
        if !self.example.is_empty() {
            annos.push("@example".to_string());
            annos.push(self.example.to_string());
        }
        //
        let mut doc = String::new();
        annos.retain_mut(|v| !v.trim().is_empty());
        if !annos.is_empty() {
            doc.push_str("/**\n");
            for anno in annos {
                let anno = anno.trim();
                if anno.is_empty() {
                    continue;
                }
                doc.push_str(" * ");
                doc.push_str(anno);
                doc.push_str("\n");
            }
            doc.push_str("*/\n");
        }
        doc.push_str(&self.ts_singature());
        doc
    }
    pub fn ts_singature(&self) -> String {
        let mut signature = String::new();
        signature.push_str(self.name);
        //参数
        let mut args = vec![];
        for arg in &self.type_signature.0 {
            args.push(arg.to_type_ident());
        }
        //
        signature.push_str("(");
        signature.push_str(&args.join(", "));
        signature.push_str("): ");
        //返回值
        if let Some(return_type) = &self.type_signature.1 {
            signature.push_str(&return_type.to_doc());
        } else {
            signature.push_str("void");
        }
        signature
    }
}

//工具包
pub struct ToolPackage {
    pub name: &'static str,
    pub annotation: &'static str,
    types: HashMap<&'static str, TypeInfo>,
    tools: HashMap<&'static str, Tool>,
    pub context_builder: ContextBuilder,
}
impl ToolPackage {
    pub fn new(
        name: &'static str,
        annotation: &'static str,
        types: HashMap<&'static str, TypeInfo>,
        tools: HashMap<&'static str, Tool>,
        context_builder: ContextBuilder,
    ) -> Self {
        Self {
            name,
            annotation,
            types,
            tools,
            context_builder,
        }
    }
    pub fn get_tool(&self, tool_name: &str) -> Option<&Tool> {
        self.tools.get(tool_name)
    }
    pub fn get_type(&self, type_name: &str) -> Option<&TypeInfo> {
        self.types.get(type_name)
    }
    pub fn new_context(&self, agent: &AgentContext) -> Arc<RwLock<SafeAny>> {
        let ctx = (self.context_builder)(&agent);
        return ctx;
    }
    pub fn tools(&self) -> &HashMap<&'static str, Tool> {
        &self.tools
    }
}

