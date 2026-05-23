// use std::sync::{Arc, atomic::AtomicU32};

// use dashmap::DashMap;
// use serde::{Deserialize, Serialize};

// pub struct AgentPermission {
//     pub id: i64,
//     pub code: String,
//     pub sub_code: Option<String>,
//     pub life: PermissionLife,
//     pub use_times: AtomicU32,
//     pub purpose: String,
//     pub create_time: chrono::DateTime<chrono::Local>,
// }
// impl AgentPermission {
//     pub fn new(
//         code: String,
//         sub_code: Option<String>,
//         purpose: String,
//         life: PermissionLife,
//     ) -> Self {
//         Self {
//             id: 0,
//             code,
//             sub_code,
//             purpose,
//             life,
//             use_times: AtomicU32::new(0),
//             create_time: chrono::Local::now(),
//         }
//     }
//     pub fn auth_code(&self) -> String {
//         if let Some(sub_code) = &self.sub_code {
//             format!("{}[{}]", self.code, sub_code)
//         } else {
//             self.code.clone()
//         }
//     }
//     pub fn is_valid(&self) -> bool {
//         let use_times = self.use_times.load(std::sync::atomic::Ordering::SeqCst);
//         match self.life {
//             PermissionLife::None => false,
//             PermissionLife::Once => use_times == 0,
//             PermissionLife::Times(times) => use_times < times,
//             PermissionLife::Duration(duration) => {
//                 let expire_time = self.create_time + chrono::Duration::from_std(duration).unwrap();
//                 chrono::Local::now() < expire_time
//             }
//             PermissionLife::Forever => true,
//         }
//     }
// }
// #[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
// pub enum PermissionLife {
//     #[default]
//     None,
//     Once,
//     Times(u32),
//     Duration(std::time::Duration),
//     Forever,
// }

// pub struct AgentAuthorityContext {
//     permissions: DashMap<String, Arc<AgentPermission>>,
// }
// impl AgentAuthorityContext {
//     pub fn new() -> Self {
//         Self {
//             permissions: DashMap::new(),
//         }
//     }
//     pub fn add(&self, p: AgentPermission) {
//         let code = auth_code(&p.code, p.sub_code.as_deref());
//         self.permissions.insert(code, Arc::new(p));
//     }
//     pub fn remove(&self, code: &str) {
//         self.permissions.remove(code);
//     }
//     pub fn contains_key(&self, code: &str) -> bool {
//         self.permissions.contains_key(code)
//     }
// }

// fn auth_code(code: &str, sub_code: Option<&str>) -> String {
//     if let Some(sub_code) = sub_code {
//         format!("{}[{}]", code, sub_code)
//     } else {
//         code.to_string()
//     }
// }
