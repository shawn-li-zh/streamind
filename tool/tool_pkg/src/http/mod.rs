use std::{
    any::Any,
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub use self::http::build_package;
use futures::StreamExt;
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    multipart,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use kernel::simple_id;
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
    use crate::http::{
        PreparedBody, ResponseEntry, assemble_body, assemble_headers, process_response,
    };
    use anyhow::{Result, anyhow};
    use kernel::agent::AgentInfo;
    use kernel::simple_id;
    use kernel::tool::core::ToolReturn;
    use kernel::{agent::AgentContext, tool::ToolCallContext};
    use reqwest::{
        Client, Method,
        header::{HeaderMap, HeaderName, HeaderValue},
    };
    use serde::Serialize;
    use serde_json::{Value, json};
    use std::{any::Any, collections::HashMap, time::Duration};
    use std::{path::Path, sync::Arc, time::Instant};
    use tokio::sync::RwLock;

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
    pub fn new_context(ctx: &AgentInfo) -> Context {
        let mut default_headers = HashMap::new();
        default_headers.insert(
            "User-Agent".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36 Edg/147.0.0.0".to_string(),
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
    pub async fn show_config(#[ctx] ctx: ToolCallContext) -> anyhow::Result<ToolReturn> {
        let ctx = ctx.context_data.read().await;
        let ctx = ctx.downcast_ref::<Context>().unwrap();
        ToolReturn::json(&ctx.config)
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
    ) -> anyhow::Result<()> {
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
        anno = "发起网络请求。支持在 body 中嵌入 '<file:相对路径>' 语法自动提取文件进行 multipart/form-data 上传。
        返回请求id,使用detail工具来查看详情。",
        example = r#"request(method="POST", url="http://api.com/upload", body={"name":"avatar", "file":"<file:./test.png>"})"#,
        limit = "30s:15"
        timeout="5m"
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
    ) -> anyhow::Result<ToolReturn> {
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
        let prepared_body = assemble_body(&ctx.workspace, body)?;

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
        let (filename, charset) =
            super::get_file_name_form_headers(&ctx.call_id, response.headers());
        // 4. 处理响应 Body (区分文本和二进制)
        let (path, file) = ctx.new_file(&format!("tool/http/{}", &filename))?;
        let entry = process_response(
            ctx.call_id.clone(),
            response,
            start_time,
            path,
            file,
            charset,
        )
        .await?;
        let res = format!(r#"requestId:"{}",status:{}"#, &entry.id, entry.status);
        // 5. 存入缓存
        let mut cache = cache_ptr.write().await;
        cache.insert(entry.id.clone(), entry);
        Ok(ToolReturn::raw(res))
    }

    #[tool(
        anno = "根据请求ID查看响应详情。codes 可选值: status, headers, body, elapsed, size。
        body有时候可能因为内容过长，而存入了临时文件，你需要使用editor.load_file和editor.read来进行阅读",
        example = r#"detail(id="abc123", codes=["status", "body"])"#,
        limit = "60s:30"
    )]
    pub async fn detail(
        #[arg("请求ID")] id: String,
        #[arg("需要查看的字段列表")] codes: Vec<String>,
        #[ctx] ctx: ToolCallContext,
    ) -> anyhow::Result<ToolReturn> {
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
                    let body = match &entry.body {
                        super::ProcessedBody::Text(c) => c.clone(),
                        super::ProcessedBody::File(path, _) => {
                            let path = ctx.to_short_path(&path);
                            let path = path.to_str().unwrap();
                            format!("<file:{path}>")
                        }
                    };
                    result.insert("body".into(), body.into());
                }
                "elapsed" => {
                    let elapsed = humantime::format_duration(entry.elapsed.clone());
                    result.insert("elapsed".into(), elapsed.to_string().into());
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

       ToolReturn::json(result)
    }
}

#[derive(Clone)]
pub struct ResponseEntry {
    pub id: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub elapsed: Duration,
    pub size: u64,
    pub body: ProcessedBody,
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
    request_id: String,
    response: reqwest::Response,
    start: Instant,
    store_path: PathBuf,
    store_file: File,
    charset: Option<String>,
) -> anyhow::Result<ResponseEntry> {
    // 处理响应 Body (区分文本和二进制)
    let status = response.status().as_u16();
    let mut resp_headers: HashMap<String, String> = HashMap::new();
    for (k, v) in response.headers() {
        resp_headers.insert(k.to_string(), v.to_str().unwrap_or("").to_string());
    }
    let body = process_response_body(response, store_path, store_file, charset).await?;
    let size = match &body {
        ProcessedBody::Text(t) => t.len() as _,
        ProcessedBody::File(_, size) => *size,
    };
    let time = start.elapsed();

    let entry = ResponseEntry {
        id: request_id,
        status,
        headers: resp_headers,
        elapsed: time,
        size,
        body,
    };
    Ok(entry)
}

fn get_file_name_form_headers(
    request_id: &String,
    headers: &HeaderMap,
) -> (String, Option<String>) {
    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_lowercase();

    let is_text = content_type.starts_with("text/")
        || content_type.starts_with("application/json")
        || content_type.starts_with("application/xml");
    if is_text {
        // 根据 MIME 类型猜测扩展名
        let ext = mime_guess::get_mime_extensions_str(&content_type)
            .and_then(|exts| exts.first())
            .unwrap_or(&"txt");
        let charset = content_type
            .split(';')
            .map(|p| p.trim())
            .find(|p| p.starts_with("charset="))
            .map(|p| p.replace("charset=", ""))
            .unwrap_or_else(|| "utf-8".to_string());
        (format!("{}.{}", request_id, ext), Some(charset))
    } else {
        fn extract_filename(disposition: &str) -> Option<String> {
            disposition
                .split(';')
                .find(|part| part.trim().starts_with("filename="))
                .map(|part| {
                    part.replace("filename=", "")
                        .trim()
                        .trim_matches('"')
                        .to_string()
                })
        }
        // --- 二进制流处理逻辑 ---
        // 2. 尝试从 Content-Disposition 提取文件名
        let filename = headers
            .get(reqwest::header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .and_then(extract_filename)
            .unwrap_or_else(|| format!("{}.bin", request_id));

        (filename, None)
    }
}
/// 核心封装：流式处理响应体，防止大文件 OOM
async fn process_response_body(
    response: reqwest::Response,
    store_path: PathBuf,
    ref mut store_file: File,
    charset: Option<String>,
) -> anyhow::Result<ProcessedBody> {
    // 设定内存阈值
    let max_memory_size = 2 * 1024;
    let mut total_size: u64 = 0u64;
    let mut raw_text = String::with_capacity(max_memory_size as _);
    //写入临时文件
    let mut stream = response.bytes_stream();
    if let Some(charset) = &charset {
        let encoding =
            encoding_rs::Encoding::for_label(charset.as_bytes()).unwrap_or(encoding_rs::UTF_8);
        let mut decoder = encoding.new_decoder();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| anyhow::anyhow!("网络流读取中断: {}", e))?;
            total_size += chunk.len() as u64;
            // 预分配足够空间的缓冲区（UTF-8 最多比原始数据多一点点）
            let mut utf8_buffer = String::with_capacity(chunk.len() * 2);
            // 解码核心逻辑：last 参数为 false 表示流未结束
            // decoder 会自动处理 chunk 末尾被截断的残缺字符，留到下次处理
            let (_res, _read, _replaced) =
                decoder.decode_to_string(&chunk, &mut utf8_buffer, false);
            store_file.write_all(utf8_buffer.as_bytes())?;
            if raw_text.len() < max_memory_size {
                raw_text.push_str(&utf8_buffer);
            }
        }
    } else {
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| anyhow::anyhow!("网络流读取中断: {}", e))?;
            total_size += chunk.len() as u64;
            store_file.write_all(&chunk)?;
        }
    }
    // --- 收尾处理 ---
    store_file.flush()?;
    //
    if total_size <= max_memory_size as u64 && charset.is_some() {
        //直接返回为文本
        return Ok(ProcessedBody::Text(raw_text));
    }
    Ok(ProcessedBody::File(store_path, total_size))
}
#[derive(Clone)]
enum ProcessedBody {
    Text(String),
    File(PathBuf, u64),
}
