use std::{
    any::Any,
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub use self::http::build_package;
use futures::StreamExt;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    multipart,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use streamind_core::tool::ToolPackage;
use tokio::{fs::File, io::AsyncWriteExt};
use tool_macro::package;

// pub fn build() -> ToolPackage {
//     let mut pkg = http::build_package();
//     pkg.context_builder = Box::new(|ag| {
//         let ctx = http::new_context(ag);
//         Box::new(ctx)
//     });
//     pkg
// }
#[package("HTTP网络请求与测试工具")]
pub mod http {
    use anyhow::{Result, anyhow};
    use reqwest::{
        Client, Method,
        header::{HeaderMap, HeaderName, HeaderValue},
    };
    use serde::Serialize;
    use serde_json::{Value, json};
    use std::{any::Any, collections::HashMap, time::Duration};
    use std::{path::Path, sync::Arc, time::Instant};
    use streamind_core::{agent::AgentContext, tool::ToolCallContext};
    use tokio::sync::RwLock;

    use crate::http::{
        PreparedBody, ResponseEntry, assemble_body, assemble_headers, process_response,
    };

    pub struct Context {
        pub root_path: String,
        pub client: Client,
        pub config: HTTPConfig,
        pub response_cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ResponseEntry>>>,
    }

    #[derive(Serialize, Clone)]
    pub struct HTTPConfig {
        pub default_headers: HashMap<String, String>,
        pub timeout_secs: u64,
    }
    #[context]
    pub fn new_context(ctx: &AgentContext) -> Context {
        let mut default_headers = HashMap::new();
        default_headers.insert(
            "User-Agent".to_string(),
            "Agent-HTTP-Client/1.0".to_string(),
        );
        let client = Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create client");
        let config = HTTPConfig {
            default_headers,
            timeout_secs: 30,
        };
        Context {
            root_path: "/".to_string(),
            client,
            config,
            response_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    #[tool(
        anno = "查看默认请求头组及基础配置（如超时时间）",
        example = r#"show_config()"#,
        limit = "60s:20"
    )]
    pub async fn show_config(#[ctx] ctx: ToolCallContext) -> anyhow::Result<String> {
        let ctx = ctx.context_data.read().await;
        let ctx = ctx.downcast_ref::<Context>().unwrap();
        let json = serde_json::to_string(&ctx.config)?;
        Ok(json)
    }
    #[tool(
        anno = "查看和编辑默认请求头组及基础配置（如超时时间）",
        example = r#"update_config(timeout=60, headers={"Authorization": "Bearer token123"})"#,
        limit = "60s:20"
    )]
    pub async fn update_config(
        #[arg("新的默认请求头，会与现有请求头合并")] headers: Option<HashMap<String, String>>,
        #[arg("超时时间（秒）")] timeout: Option<u64>,
        #[ctx] ctx: ToolCallContext,
    ) -> Result<()> {
        let mut ctx = ctx.context_data.write().await;
        let ctx = ctx.downcast_mut::<Context>().unwrap();
        if let Some(h) = headers {
            ctx.config.default_headers.extend(h);
        }
        if let Some(t) = timeout {
            ctx.config.timeout_secs = t;
        }
        Ok(())
    }
    #[tool(
        anno = "发起网络请求。支持在 body 中嵌入 '<file:相对路径>' 语法自动提取文件进行 multipart/form-data 上传",
        example = r#"request(method="POST", url="http://api.com/upload", body={"name":"avatar", "file":"<file:./test.png>"})"#,
        limit = "30s:15"
    )]
    pub async fn request(
        #[arg("请求方法，如 GET, POST, PUT, DELETE")] method: String,
        #[arg("完整的请求URL")] url: String,
        #[arg("本次请求特有的Header，会覆盖默认Header")] headers: Option<HashMap<String, String>>,
        #[arg(
            "请求体。如果是表单上传，请传入JSON Object。值如果为 '<file:路径>'，则自动提取该文件上传"
        )]
        body: Option<Value>,
        #[ctx] ctx: ToolCallContext,
    ) -> Result<String> {
        let (root_path, store_path) = ctx.get_safe_path("/tool/http").await?;
        let start_time = Instant::now();
        // 1. 获取 Context
        let (client, config, cache_ptr) = {
            let data = ctx.context_data.read().await;
            let c = data
                .downcast_ref::<Context>()
                .ok_or_else(|| anyhow!("Context Error"))?;
            (c.client.clone(), c.config.clone(), c.response_cache.clone())
        };
        // 2. 组装请求
        let method_enum = Method::from_bytes(method.to_uppercase().as_bytes())?;
        let header_map = assemble_headers(&config.default_headers, headers)?;
        let prepared_body = assemble_body(&root_path, body)?;

        let mut rb = client
            .request(method_enum, &url)
            .headers(header_map)
            .timeout(Duration::from_secs(config.timeout_secs));

        rb = match prepared_body {
            PreparedBody::Json(v) => rb.json(&v),
            PreparedBody::Multipart(f) => rb.multipart(f),
            PreparedBody::Empty => rb,
        };
        // 3. 执行请求
        let response = rb.send().await?;
        // 4. 处理响应 Body (区分文本和二进制)
        let entry = process_response(response, start_time, &store_path).await?;
        let res = format!("requestId: {},status:{}", &entry.id, entry.status);
        // 5. 存入缓存
        let mut cache = cache_ptr.write().await;
        cache.insert(entry.id.clone(), entry);
        Ok(res)
    }

    #[tool(
        anno = "根据请求ID查看响应详情。codes 可选值: status, headers, body, time, size",
        example = r#"detail(id="a1b2c3d4", codes=["status", "body"])"#,
        limit = "60s:50"
    )]
    pub async fn detail(
        #[arg("请求ID")] id: String,
        #[arg("需要查看的字段列表")] codes: Vec<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> Result<Value> {
        let cache_ptr = {
            let data = ctx.context_data.read().await;
            let c = data
                .downcast_ref::<Context>()
                .ok_or_else(|| anyhow!("Context Error"))?;
            c.response_cache.clone()
        };

        let cache = cache_ptr.read().await;
        let entry = cache
            .get(&id)
            .ok_or_else(|| anyhow!("找不到 ID 为 {} 的记录", id))?;

        let mut result = serde_json::Map::new();

        for code in codes {
            match code.to_lowercase().as_str() {
                "status" => {
                    result.insert("status".into(), entry.status.into());
                }
                "headers" => {
                    result.insert("headers".into(), serde_json::to_value(&entry.headers)?);
                }
                "body" => {
                    result.insert("body".into(), entry.body.clone().into());
                }
                "time" => {
                    let time = humantime::format_duration(entry.time.clone());
                    result.insert("time".into(), time.to_string().into());
                }
                "size" => {
                    let size = bytesize::ByteSize::b(entry.size);
                    result.insert("size".into(), size.to_string().into());
                }
                _ => {
                    result.insert(code.clone(), "Unknown code".into());
                }
            }
        }

        Ok(Value::Object(result))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ResponseEntry {
    pub id: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String, // 可能是纯文本，也可能是 "<file:temp_xxx.bin>"
    pub time: Duration,
    pub size: u64,
    pub file_path: PathBuf,
}

/// 内部使用的 Body 类型，区分普通 JSON 和 带文件的多部分表单
enum PreparedBody {
    Json(Value),
    Multipart(multipart::Form),
    Empty,
}

// ==========================================
// 内部私有组件：拆分的逻辑单元
// ==========================================

/// 1. 组装 Header：合并默认配置与本次请求配置
fn assemble_headers(
    defaults: &HashMap<String, String>,
    customs: Option<HashMap<String, String>>,
) -> anyhow::Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    // 写入默认
    for (k, v) in defaults {
        headers.insert(
            HeaderName::from_bytes(k.as_bytes())?,
            HeaderValue::from_str(v)?,
        );
    }
    // 覆盖自定义
    if let Some(ch) = customs {
        for (k, v) in ch {
            headers.insert(
                HeaderName::from_bytes(k.as_bytes())?,
                HeaderValue::from_str(&v)?,
            );
        }
    }
    Ok(headers)
}

/// 2. 组装 Body：处理 <file:path> 逻辑并决定请求格式
fn assemble_body(root_path: &PathBuf, body: Option<Value>) -> anyhow::Result<PreparedBody> {
    let val = match body {
        Some(v) => v,
        None => return Ok(PreparedBody::Empty),
    };

    if let Value::Object(ref map) = val {
        // 扫描是否包含文件标记
        let has_file = map.values().any(|v| {
            v.as_str()
                .map_or(false, |s| s.starts_with("<file:") && s.ends_with(">"))
        });

        if has_file {
            let mut form = multipart::Form::new();
            for (k, v) in map {
                if let Some(s) = v.as_str() {
                    if s.starts_with("<file:") && s.ends_with(">") {
                        // 提取并读取文件
                        let rel_path = &s[6..s.len() - 1];
                        let full_path = Path::new(root_path).join(rel_path);

                        let file_name = full_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file")
                            .to_string();

                        let bytes = std::fs::read(&full_path)
                            .map_err(|e| anyhow::anyhow!("读取文件失败 {:?}: {}", full_path, e))?;

                        form = form.part(
                            k.clone(),
                            multipart::Part::bytes(bytes).file_name(file_name),
                        );
                        continue;
                    }
                }
                // 普通文本字段
                form = form.text(k.clone(), v.to_string());
            }
            return Ok(PreparedBody::Multipart(form));
        }
    }

    // 如果不是对象或不含文件，走普通 JSON
    Ok(PreparedBody::Json(val))
}

async fn process_response(
    response: reqwest::Response,
    start: Instant,
    store_path: &PathBuf,
) -> anyhow::Result<ResponseEntry> {
    // 处理响应 Body (区分文本和二进制)
    let status = response.status().as_u16();
    let mut resp_headers = HashMap::new();
    for (k, v) in response.headers() {
        resp_headers.insert(k.to_string(), v.to_str().unwrap_or("").to_string());
    }
    let content_type = resp_headers
        .get("content-type")
        .map(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();

    let (body, size, file_path) =
        process_response_body(response, &content_type, &store_path).await?;
    let time = start.elapsed();
    let request_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let entry = ResponseEntry {
        id: request_id.clone(),
        status,
        headers: resp_headers,
        body,
        time,
        size,
        file_path,
    };
    Ok(entry)
}
/// 核心封装：流式处理响应体，防止大文件 OOM
async fn process_response_body(
    response: reqwest::Response,
    content_type: &String,
    store_path: &PathBuf,
) -> anyhow::Result<(String, u64, PathBuf)> {
    // 设定内存阈值：2kb
    let max_memory_size = 2 * 1024;
    let mut total_size = 0u64;
    //初始化文件
    let file_id = uuid::Uuid::new_v4().simple().to_string();
    let file_name = format!("{}.bin", file_id);
    if !store_path.exists() {
        std::fs::create_dir_all(&store_path)?;
    }
    let full_path = store_path.join(&file_name);
    //写入临时文件
    {
        
        let ref mut f = File::create(&full_path)
            .await
            .map_err(|e| anyhow::anyhow!("创建临时文件失败:{} {}", full_path.to_str().unwrap(), e))?;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| anyhow::anyhow!("网络流读取中断: {}", e))?;
            total_size += chunk.len() as u64;
            // 写入当前 chunk
            f.write_all(&chunk).await?;
        }
        // --- 收尾处理 ---
        f.flush().await?;
    }
    //
    if total_size <= max_memory_size
        && (content_type.contains("text/")
            || content_type.contains("/json")
            || content_type.contains("/xml"))
    {
        //尝试读取为文本
        let bytes = std::fs::read(&full_path)?;
        if let Ok(text) = String::from_utf8(bytes) {
            return Ok((text, total_size, full_path));
        }
    }
    Ok((format!("<file:{}>", file_name), total_size, full_path))
}
