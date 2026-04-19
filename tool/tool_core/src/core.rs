use std::{
    any::Any,
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::Duration,
};
use streamind_core::agent::AgentInfo;

use futures::{FutureExt, future::BoxFuture};

use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    context::ToolPackageContext,
    value_type::{ToolValueType, TypeInfo, TypeItemInfo},
};
pub type SafeAny = dyn Any + Send + Sync + 'static;
pub type ContextData = Arc<RwLock<SafeAny>>;
pub type ToolHandler = Box<
    dyn Fn(
            HashMap<String, serde_json::Value>,
            ContextData,
        ) -> BoxFuture<'static, anyhow::Result<ToolCallReturn>>
        + Send
        + Sync,
>;

pub type ActiveFn = Box<dyn Fn(HashMap<String, serde_json::Value>,ContextData) -> anyhow::Result<String> + Send + Sync>;
pub type ContextBuilder = Box<dyn Fn(&AgentInfo) -> Box<SafeAny> + Send + Sync>;

pub fn async_tool_handler<F, Fut>(f: F) -> ToolHandler
where
    F: Fn(HashMap<String, serde_json::Value>, ContextData) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = anyhow::Result<ToolCallReturn>> + Send + 'static,
{
    Box::new(move |args, ctx| f(args, ctx).boxed())
}
pub fn sync_tool_handler<F>(f: F) -> ToolHandler
where
    F: Fn(HashMap<String, serde_json::Value>, ContextData) -> Result<ToolCallReturn, anyhow::Error>
        + Send
        + Sync
        + 'static,
{
    Box::new(move |args, ctx| {
        let res = f(args, ctx);
        async move { res }.boxed() // 将同步结果包装在 async 块中并 boxed
    })
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
    pub fn active_tip(&self, args: HashMap<String, serde_json::Value>, ctx: &ToolPackageContext,) -> String {
       match (self.active)(args,ctx.data.clone()) {
           Ok(r) => {r},
           Err(e) => format!("参数异常:{e}"),
       }
    }

    pub async fn call(
        &self,
        args: HashMap<String, serde_json::Value>,
        ctx: &ToolPackageContext,
    ) -> anyhow::Result<ToolCallReturn> {
        if ctx.is_limited(self.name) {
            return Err(anyhow::anyhow!("调用频率过高"));
        };
        (self.handler)(args, ctx.data.clone()).await
    }

    pub fn to_doc(&self) -> String {
        let mut annos = vec![self.annotation.to_string()];
        let mut signature = String::new();
        signature.push_str(self.name);
        let mut args = vec![];
        for arg in &self.type_signature.0 {
            annos.push(format!("@param {} {}", arg.name, arg.annotation));
            args.push(arg.to_type_ident());
        }
        signature.push_str("(");
        signature.push_str(&args.join(", "));
        signature.push_str("): ");
        if let Some(return_type) = &self.type_signature.1 {
            signature.push_str(&return_type.to_doc());
        } else {
            signature.push_str("void");
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
        doc.push_str(signature.as_str());
        doc
    }
}

//工具包
pub struct ToolPackage {
    pub name: &'static str,
    pub annotation: &'static str,
    types: HashMap<&'static str, TypeInfo>,
    tools: HashMap<&'static str, Tool>,
    context_builder: ContextBuilder,
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
    pub fn new_context(&self, agent: &AgentInfo) -> Box<dyn Any + Send + Sync + 'static> {
        (self.context_builder)(agent)
    }
    pub fn tools(&self) -> &HashMap<&'static str, Tool> {
        &self.tools
    }
}

pub struct ToolCallReturn {
    pub data: serde_json::Value,
    pub info: HashMap<String, serde_json::Value>,
}
impl ToolCallReturn {
    pub fn new<T: Serialize>(data: T) -> Self {
        Self {
            data: json!(data),
            info: HashMap::new(),
        }
    }
    pub fn set_info<V: Serialize>(&mut self, key: String, value: V) {
        self.info.insert(key, json!(value));
    }
}

impl<T: serde::Serialize> From<T> for ToolCallReturn {
    fn from(data: T) -> Self {
        ToolCallReturn::new(data)
    }
}
