use std::{
    any::{self, Any},
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use futures::{FutureExt, future::BoxFuture};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    agent::AgentInfo,
    message::Binary,
    tool::{
        context::ToolCallContext,
        value_type::{ToolValueType, TypeInfo, TypeItemInfo},
    },
};

pub type ToolHandler = Box<
    dyn Fn(
            HashMap<String, serde_json::Value>,
            ToolCallContext,
        ) -> BoxFuture<'static, anyhow::Result<ToolReturn>>
        + Send
        + Sync,
>;

pub type ActiveFn = Box<
    dyn Fn(HashMap<String, serde_json::Value>, ToolCallContext) -> anyhow::Result<String>
        + Send
        + Sync,
>;
pub type SafeAny = dyn Any + Send + Sync + 'static;
pub type ContextBuilder = Box<dyn Fn(&AgentInfo) -> Arc<RwLock<SafeAny>> + Send + Sync>;

pub fn async_tool_handler<F, Fut>(f: F) -> ToolHandler
where
    F: Fn(HashMap<String, serde_json::Value>, ToolCallContext) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = anyhow::Result<ToolReturn>> + Send + 'static,
{
    Box::new(move |args, ctx| f(args, ctx).boxed())
}
pub fn sync_tool_handler<F>(f: F) -> ToolHandler
where
    F: Fn(HashMap<String, serde_json::Value>, ToolCallContext) -> anyhow::Result<ToolReturn>
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
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", content = "payload")]
#[serde(rename_all = "camelCase")]
pub enum ToolReturnValue {
    Raw(String),
    Json(serde_json::Value),
    Image(Binary),
}
#[derive(Debug, Clone)]
pub struct ToolReturn {
    pub values: Vec<ToolReturnValue>,
}
impl ToolReturn {
    pub fn new() -> Self {
        Self { values: vec![] }
    }
    pub fn raw<S: std::fmt::Display>(s: S) -> Self {
        ToolReturnValue::Raw(s.to_string()).into()
    }
    pub fn raws<S: std::fmt::Display>(s: Vec<S>) -> Self {
        let values = s
            .into_iter()
            .map(|s| ToolReturnValue::Raw(s.to_string()))
            .collect();
        Self { values }
    }
    pub fn json<T: Serialize>(v: T) -> anyhow::Result<Self> {
        Ok(ToolReturnValue::Json(serde_json::to_value(v)?).into())
    }
}

impl<T: Serialize + 'static> From<T> for ToolReturn {
    fn from(value: T) -> Self {
        let id = any::TypeId::of::<T>();
        if id == any::TypeId::of::<ToolReturnValue>() {
            let v: Box<dyn Any> = Box::new(value);
            let v = v.downcast::<ToolReturnValue>().unwrap();
            return ToolReturn { values: vec![*v] };
        }
        ToolReturn::json(value).unwrap()
    }
}

pub struct Tool {
    pub name: &'static str,
    //函数描述
    pub annotation: &'static str,
    pub example: &'static str,
    pub type_signature: (Vec<TypeItemInfo>, Option<ToolValueType>),
    //
    pub handler: ToolHandler,
    //
    pub rate_limit: Option<(Duration, u32)>,
    pub timeout: Option<Duration>,
    //或
    pub permissions: HashSet<&'static str>,
}

impl Tool {
    pub fn new(
        name: &'static str,
        annotation: &'static str,
        example: &'static str,
        type_signature: (Vec<TypeItemInfo>, Option<ToolValueType>),
        handler: ToolHandler,
    ) -> Self {
        Self {
            name,
            annotation,
            example,
            type_signature,
            handler,
            rate_limit: None,
            timeout: None,
            permissions: HashSet::new(),
        }
    }
    //=====额外设置
    pub fn rate_limit(&mut self, window_time: Duration, times: u32) {
        self.rate_limit = Some((window_time, times))
    }
    pub fn timeout(&mut self, time: Duration) {
        self.timeout = Some(time);
    }
    pub fn permissions(&mut self, permissions: &'static str) {
        self.permissions = permissions
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
    }
    //
    pub fn to_doc(&self) -> String {
        //注释
        let mut annos = vec![];
        annos.extend(self.annotation.split("\n").map(|s| s.trim().to_string()));
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

    pub fn tools(&self) -> &HashMap<&'static str, Tool> {
        &self.tools
    }
}
