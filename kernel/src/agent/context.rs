use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatus {
    #[default]
    Idle, //空闲
    Sleep,      //睡眠
    Processing, //处理任务中
    Waiting,    //等待工具调用返回结果
    Paused,     //暂停 需要用户手动恢复到空闲
}

pub struct AgentContext {
    // 注意：这里存的是 Arc<dyn Any>，而里面的具体类型是 RwLock<T>
    // 必须加上 Send + Sync，否则无法在多线程环境使用
    inner: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl AgentContext {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> Arc<T> {
        let type_id = TypeId::of::<T>();
        // 将整个 RwLock 包装成 dyn Any
        let boxed = Arc::new(value);

        self.inner
            .insert(type_id, (boxed.clone() as Arc<dyn Any + Send + Sync>));
        boxed
    }
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();
        let entry = self.inner.get(&type_id)?;
        let value = entry
            .clone()
            .downcast::<T>()
            .expect("Type mismatch in Context");
        Some(value)
    }
}
