use std::{f32::consts::E, path::PathBuf};

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binary {
    pub path: PathBuf,
    pub name: Option<String>,
    pub size: usize,
}
impl Binary {
    pub fn head(&self, index: usize) -> String {
        let mut head = format!("Index={},Path={}", index, self.path.to_str().unwrap());
        if let Some(name) = &self.name {
            head.push_str(&format!(",Name={}", name));
        }
        head
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Information {
    Text(String),
    Image(Binary),
    Audio(Binary),
    Video(Binary),
    File(Binary),
}
impl<S: std::fmt::Display> From<S> for Information {
    fn from(value: S) -> Self {
        Information::Text(value.to_string())
    }
}
impl Information {
    pub fn size(&self) -> usize {
        let b = match self {
            Information::Text(v) => return v.len(),
            Information::Image(v) => v,
            Information::Audio(v) => v,
            Information::Video(v) => v,
            Information::File(v) => v,
        };
        b.size
    }
    fn to_tuple(self, index: usize) -> (String, Option<PathBuf>) {
        let b = match self {
            Information::Text(v) => return (v, None),
            Information::Image(v) => v,
            Information::Audio(v) => v,
            Information::Video(v) => v,
            Information::File(v) => v,
        };
        (b.head(index), Some(b.path))
    }
}
#[derive(Debug,Clone)]
pub enum Role {
    User,
    System,
    Assistant,
    ToolCall,
}
impl Role {
    pub fn user() -> Message {
        Message {
            role: Role::User,
            items: vec![],
            size: 0,
        }
    }
    pub fn system() -> Message {
        Message {
            role: Role::System,
            items: vec![],
            size: 0,
        }
    }
    pub fn assistant() -> Message {
        Message {
            role: Role::Assistant,
            items: vec![],
            size: 0,
        }
    }
    pub fn tool_call() -> Message {
        Message {
            role: Role::ToolCall,
            items: vec![],
            size: 0,
        }
    }
}
#[derive(Debug,Clone)]
pub struct Message {
    role: Role,
    pub items: Vec<Information>,
    pub size: usize,
}
impl Message {
    pub fn get_role(&self) -> Role {
        self.role.clone()
    }
    pub fn infos(mut self, infos: Vec<Information>) -> Self {
        for ele in infos {
            self.size += ele.size();
            self.items.push(ele);
        }
        self
    }
    pub fn text<S: std::fmt::Display>(mut self, v: S) -> Self {
        let v = v.to_string();
        self.size += v.len();
        self.items.push(Information::Text(v));
        self
    }
    pub fn image(mut self, path: PathBuf, size: usize, name: Option<String>) -> Self {
        self.size += size;
        self.items
            .push(Information::Image(Binary { path, name, size }));
        self
    }

    pub fn to_tuple(self) -> (String, Vec<PathBuf>) {
        let mut text = String::new();
        let mut paths = Vec::new();
        for (index, info) in self.items.into_iter().enumerate() {
            let (chunk, path) = info.to_tuple(index);
            text.push_str(&chunk);
            text.push_str("\n\n");
            if let Some(path) = path {
                paths.push(path);
            }
        }
        (text, paths)
    }
}
