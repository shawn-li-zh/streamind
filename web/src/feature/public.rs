use std::io::Write;
use std::path::PathBuf;

use axum::body::Bytes;
use axum::{Router, extract::Path};

use crate::wrapper::ApiResponse;

pub static PUBLIC_DIR: &str = "./public";
async fn upload(
    Path(path): Path<String>, // 提取 URL 中的路径部分 (aa/bb.png)
    body: Bytes,              // 提取原始请求体
) -> ApiResponse<String> {
    // 1. 设定根目录
    let base_dir = PathBuf::from(PUBLIC_DIR);

    // 2. 安全性检查：防止路径穿越 (例如 ../../etc/passwd)

    // 3. 构建完整路径
    let full_path = base_dir.join(&path);
    if !full_path.starts_with(base_dir) {
        return ApiResponse::error("非法路径");
    }
    // 4. 确保父目录存在 (例如 aa/ 目录)
    if let Some(parent) = full_path.parent() {
        match std::fs::create_dir_all(parent) {
            Ok(_) => {}
            Err(e) => {
                return ApiResponse::error(&format!("创建目录失败: {}", e));
            }
        }
    }

    // 5. 将 Body 写入文件
    let mut file = match std::fs::File::create(&full_path) {
        Ok(file) => file,
        Err(e) => {
            return ApiResponse::error(&format!("创建文件失败: {}", e));
        }
    };
    if let Err(e) = file.write_all(&body) {
        return ApiResponse::error(&format!("创建文件失败: {}", e));
    }

    ApiResponse::success(format!("{}/{}", PUBLIC_DIR, path))
}

use axum::{
    extract::{ConnectInfo, Query},
    http::{Response, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use std::net::SocketAddr;

use tokio::fs::File;
use tokio_util::io::ReaderStream;

#[derive(Deserialize)]
pub struct ImageQuery {
    path: String, // 本地图片的绝对路径
}

async fn get_local_image(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(query): Query<ImageQuery>,
) -> impl IntoResponse {
    // 1. 安全校验：仅允许本地调用 (IPv4 127.0.0.1 或 IPv6 ::1)
    let ip = addr.ip();
    if !ip.is_loopback() {
        return (StatusCode::FORBIDDEN, "Access denied: Local only").into_response();
    }

    // 2. 检查文件是否存在
    let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = dir.join(&query.path);
    println!("Attempting to serve local image: {:?}", path);
    if !path.exists() || !path.is_file() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }
    if !path.exists() || !path.is_file() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    // 3. 打开文件并转换为流
    let file = match File::open(&path).await {
        Ok(f) => f,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to open file").into_response();
        }
    };

    // 4. 猜测 MIME 类型（简单起见根据后缀判断）
    let ext = path
        .extension()
        .and_then(|s| s.to_str().map(|v| v.to_lowercase()))
        .unwrap_or_default();
    let content_type = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unsupported image format",
            )
                .into_response();
        }
    };

    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .body(body)
        .unwrap()
        .into_response()
}
pub fn inject_router(mut router: Router) -> Router {
    router = router.route("/upload/{*path}", axum::routing::post(upload));
    // 新增：本地图片流获取接口
    // 注意：在 main.rs 中启动 server 时，必须使用 .into_make_service_with_connect_info::<SocketAddr>()
    router = router.route("/api/local-image", get(get_local_image));

    //映射本地资源
    let serve_dir = tower_http::services::ServeDir::new(PUBLIC_DIR)
        .precompressed_gzip()
        .append_index_html_on_directories(true);
    router = router.nest_service("/public", serve_dir);
    router
}
