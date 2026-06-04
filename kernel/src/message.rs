use std::path::PathBuf;

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
    //功能信息
    Function(String, String),
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
            Information::Function(_tag, v) => return v.len(),
            Information::Text(v) => return v.len(),
            Information::Image(v) => v,
            Information::Audio(v) => v,
            Information::Video(v) => v,
            Information::File(v) => v,
        };
        b.size
    }
    pub fn to_tuple(self, index: usize) -> (String, Option<PathBuf>) {
        let b = match self {
            Information::Function(_tag, v) => return (v, None),
            Information::Text(v) => return (v, None),
            Information::Image(v) => v,
            Information::Audio(v) => v,
            Information::Video(v) => v,
            Information::File(v) => v,
        };
        (b.head(index), Some(b.path))
    }
    pub fn join(infos: Vec<Information>) -> (String, Vec<PathBuf>) {
        let mut text = String::new();
        let mut paths = Vec::new();
        for (index, info) in infos.into_iter().enumerate() {
            let (chunk, path) = info.to_tuple(index);
            text.push_str(&chunk);
            text.push_str("\n");
            if let Some(path) = path {
                paths.push(path);
            }
        }
        text.pop();
        (text, paths)
    }
}

#[derive(Debug, Clone)]
pub enum RoleMessage {
    User(Vec<Information>),
    System(Vec<Information>),
    Assistant(Vec<Information>, bool),
    Tool(String, Vec<Information>),
}

impl RoleMessage {
    pub fn size(&self) -> usize {
        match self {
            RoleMessage::User(v)
            | RoleMessage::System(v)
            | RoleMessage::Tool(_, v)
            | RoleMessage::Assistant(v, _) => v.iter().map(|v| v.size()).sum(),
        }
    }
    pub fn role(&self) -> Role {
        match self {
            RoleMessage::User(_) => Role::User,
            RoleMessage::System(_) => Role::System,
            RoleMessage::Assistant(_, _) => Role::Assistant,
            RoleMessage::Tool(_, _) => Role::Tool,
        }
    }
    pub fn infos(role: Role, infos: Vec<Information>) -> Self{
        let s = match role {
            Role::User => RoleMessage::User(infos),
            Role::System => RoleMessage::System(infos),
            Role::Assistant => RoleMessage::Assistant(infos, false),
            Role::Tool => RoleMessage::Tool("ID".to_string(), infos),
        };
        s
    }
    pub fn text<S: std::fmt::Display>(role: Role, v: S) -> Self {
        let v = v.to_string();
        let v = Information::Text(v);
        RoleMessage::infos(role, vec![v])
    }
    pub fn image(
        role: Role,
        path: PathBuf,
        size: usize,
        name: Option<String>,
    ) -> Self {
        let v = Information::Image(Binary { path, name, size });
        RoleMessage::infos(role, vec![v])
    }

    pub fn to_tuple(self) -> (String, Vec<PathBuf>) {
        let informations = match self {
            RoleMessage::User(informations) => informations,
            RoleMessage::System(informations) => informations,
            RoleMessage::Assistant(informations, _) => informations,
            RoleMessage::Tool(_, informations) => informations,
        };
        Information::join(informations)
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
