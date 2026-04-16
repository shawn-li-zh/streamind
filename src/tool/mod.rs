use std::{
    collections::{HashMap, HashSet},
    fmt::format,
    sync::{Arc, LazyLock},
};

use futures::{
    future::{join_all, BoxFuture},
    FutureExt,
};
use regex::Regex;
use serde_json::Value;
use tokio::task::JoinHandle;

mod package;
#[derive(Debug)]
pub enum ToolCallError {
    ParseError(String),
    NotFound(String),
    ExecutionError(anyhow::Error),
    Undo,
}

type ToolHandler =
    Box<dyn Fn(Vec<Value>) -> BoxFuture<'static, Result<String, anyhow::Error>> + Send + Sync>;
pub struct Tool {
    pub name: String,
    //ts 函数签名
    pub signature: String,
    //函数描述 (从注释提取)
    pub description: String,
    handler: ToolHandler,
}

impl Tool {
    /// 注册异步工具
    pub fn new_async<F, Fut>(name: &str, sig: &str, desc: &str, f: F) -> Self
    where
        F: Fn(Vec<Value>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String, anyhow::Error>> + Send + 'static,
    {
        Self {
            name: name.to_string(),
            signature: sig.to_string(),
            description: desc.to_string(),
            handler: Box::new(move |args| f(args).boxed()), // 使用 .boxed() 转换为 BoxFuture
        }
    }

    /// 注册同步工具
    pub fn new_sync<F>(name: &str, sig: &str, desc: &str, f: F) -> Self
    where
        F: Fn(Vec<Value>) -> Result<String, anyhow::Error> + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            signature: sig.to_string(),
            description: desc.to_string(),
            handler: Box::new(move |args| {
                let res = f(args);
                async move { res }.boxed() // 将同步结果包装在 async 块中并 boxed
            }),
        }
    }
}
pub struct TypeInfo {
    pub name: String,
    //ts类型定义
    pub definition: String,
}
//工具包

pub struct ToolPackage {
    pub name: String,
    pub description: String,
    pub doc: String,
    pub types: HashMap<String, TypeInfo>,
    pub tools: HashMap<String, Tool>,
}
impl ToolPackage {
    //code格式 fn(..args)
    pub async fn call(&self, code: &str) -> Result<String, ToolCallError> {
        let (func_name, args) = parse_call(code)?;
        let tool = self.tools.get(&func_name).ok_or_else(|| {
            ToolCallError::NotFound(format!(
                "Tool '{}' not found in package '{}'",
                func_name, self.name
            ))
        })?;
        let result = (tool.handler)(args).await;
        result.map_err(ToolCallError::ExecutionError)
    }
}

fn parse_call(code: &str) -> Result<(String, Vec<Value>), ToolCallError> {
    // 1. 使用正则表达式匹配函数名和参数部分
    // 匹配格式：函数名(参数)
    let start = code.find("(").ok_or_else(|| {
        ToolCallError::ParseError(
            "Invalid function call format. Expected 'func_name(args)'".to_string(),
        )
    })?;
    let end = code.rfind(")").ok_or_else(|| {
        ToolCallError::ParseError(
            "Invalid function call format. Expected 'func_name(args)'".to_string(),
        )
    })?;
    let func_name = code[..start].trim();
    let args_str = code[start + 1..end].trim();

    // 3. 将参数字符串解析为 JSON 数组
    // 技巧：在外面包一层 []，使其符合 JSON 数组格式，直接利用 serde_json 解析复杂嵌套结构
    let json_args = if args_str.is_empty() {
        Vec::new()
    } else {
        let wrapped_args = format!("[{}]", args_str);
        serde_json::from_str::<Vec<Value>>(&wrapped_args)
            .map_err(|e| ToolCallError::ParseError(format!("Failed to parse arguments: {}", e)))?
    };
    Ok((func_name.to_string(), json_args))
}

pub fn package_from_ts(
    package_name: &str,
    doc: String,
    mut handlers: HashMap<String, ToolHandler>,
) -> Result<ToolPackage, String> {
    let mut types = HashMap::new();
    let mut tools = HashMap::new();

    // 1. 预处理：正则匹配所有的接口块 (interface Name { ... })
    // 捕获：注释 (group 1), 名称 (group 2), 内容 (group 3)
    let interface_re =
        Regex::new(r"(?s)(/\*\*.*?\*/)?\s*interface\s+(\w+)\s*\{([\s\S]*?)\}").unwrap();

    // 2. 匹配方法签名的正则 (针对 Package 内部)
    // 捕获：注释 (group 1), 方法名 (group 2), 参数 (group 3), 返回值 (group 4)
    let method_re = Regex::new(r"(?s)(/\*\*.*?\*/)?\s*(\w+)\s*\((.*?)\)\s*:\s*([^;]+);").unwrap();
    let mut packege_anno = "";
    for cap in interface_re.captures_iter(&doc) {
        let anno = cap.get(1).unwrap().as_str();
        let name = cap.get(2).unwrap().as_str();
        let body = cap.get(3).unwrap().as_str();

        if name == "Package" {
            packege_anno = anno;
            // 解析工具方法
            for m_cap in method_re.captures_iter(body) {
                let m_comment = m_cap.get(1).map_or("", |m| m.as_str()).trim();
                let m_name = m_cap.get(2).unwrap().as_str();
                let m_params = m_cap.get(3).unwrap().as_str();
                let m_return = m_cap.get(4).unwrap().as_str();

                // 提取描述（去除注释符号）
                let description = clean_jsdoc(m_comment);
                let signature = format!("{}({}): {}", m_name, m_params, m_return);

                // 绑定 Handler
                if let Some(handler) = handlers.remove(m_name) {
                    tools.insert(
                        m_name.to_string(),
                        Tool {
                            name: m_name.to_string(),
                            signature,
                            description,
                            handler,
                        },
                    );
                    continue;
                }
                return Err(format!(
                    "No handler found for method '{}'. Please provide a handler in the handlers map.",
                    m_name
                ));
            }
            continue;
        }
        // 作为普通类型存储
        types.insert(
            name.to_string(),
            TypeInfo {
                name: name.to_string(),
                definition: cap.get(0).unwrap().as_str().trim().to_string(),
            },
        );
    }

    Ok(ToolPackage {
        name: package_name.to_string(),
        description: packege_anno.to_string(),
        doc,
        types,
        tools,
    })
}

/// 辅助函数：清洗 JSDoc 注释
fn clean_jsdoc(comment: &str) -> String {
    comment
        .lines()
        .map(|line| {
            let mut line = line.trim();
            if line.starts_with("* ") {
                line = &line[2..];
            }
            line
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub static TOOL_PACKAGE_POOL: LazyLock<HashMap<String, Arc<ToolPackage>>> =
    LazyLock::new(|| HashMap::new());
pub struct ToolExcutor {
    pub packages: HashSet<String>,
}
impl ToolExcutor {
    //code格式 package.fn(..args)
    pub async fn call(&self, code: &str) -> Result<String, ToolCallError> {
        let (p, f) = code
            .split_once(".")
            .ok_or_else(|| ToolCallError::ParseError(format!("Invalid code format: '{}'", code)))?;
        let op = TOOL_PACKAGE_POOL.get(p);
        if !self.packages.contains(p) || op.is_none() {
            return Err(ToolCallError::NotFound(format!(
                "package '{}' not found!",
                p
            )));
        }
        let package = op.unwrap();
        package.call(f).await
    }
    //code格式 package.fn(..args) 或者 await package.fn(..args)
    pub async fn batch_call(&self, codes: Vec<String>) -> Vec<Result<String, ToolCallError>> {
        let mut results = vec![];
        for _ in 0..codes.len() {
            results.push(Err(ToolCallError::Undo));
        }
        let mut has_parse_error = false;
        //语法解析
        let mut valid_tasks = Vec::with_capacity(codes.len());
        for i in 0..codes.len() {
            let code = &codes[i];
            let mut expr = code.as_str();
            let is_await = code.starts_with("await ");
            if is_await {
                expr = &code[6..]
            }
            let (p, f) = match expr.split_once(".") {
                Some(p) => p,
                None => {
                    has_parse_error = true;
                    let err = ToolCallError::ParseError(format!("Invalid code format: '{}'", code));
                    results[i] = Err(err);
                    continue;
                }
            };
            let op = TOOL_PACKAGE_POOL.get(p);
            if !self.packages.contains(p) || op.is_none() {
                has_parse_error = true;
                let err = ToolCallError::NotFound(format!("package '{}' not found!", p));
                results[i] = Err(err);
                continue;
            }
            valid_tasks.push((i, op.unwrap(), f, is_await));
        }
        if has_parse_error {
            return results;
        }
        // 执行阶段
        let mut concurrent_tasks: Vec<(usize, JoinHandle<Result<String, ToolCallError>>)> =
            Vec::new();

        for (i, package, fn_expr, is_await) in valid_tasks {
            if is_await {
                // 串行：立即执行并等待结果，存入对应的索引位置
                results[i] = package.call(fn_expr).await;
            } else {
                let package = package.clone();
                let fn_expr = fn_expr.to_string();
                let handle = tokio::spawn(async move { package.call(&fn_expr).await });
                concurrent_tasks.push((i, handle));
            }
        }
        // 回收并发结果
        for (index, handle) in concurrent_tasks {
            // handle.await 可能会返回 join 错误（例如 panic 了）
            results[index] = handle.await.unwrap_or_else(|_| Err(ToolCallError::Undo));
        }
        results
    }
}
