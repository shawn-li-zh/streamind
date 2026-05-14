use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::section::Section;
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
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    User,
    System,
    Assistant,
    Tool,
}
impl Role {
    pub fn message(self) -> Message {
        Message {
            role: self,
            items: vec![],
            size: 0,
            prefix: false,
        }
    }
    pub fn user() -> Message {
        Role::User.message()
    }
    pub fn system() -> Message {
        Role::System.message()
    }
    pub fn assistant() -> Message {
        Role::Assistant.message()
    }
    pub fn tool() -> Message {
        Role::Tool.message()
    }
}


#[derive(Debug, Clone)]
pub struct Message {
    role: Role,
    pub items: Vec<Information>,
    pub size: usize,
    pub prefix: bool,
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
