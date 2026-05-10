use std::io::Write;
use std::path::PathBuf;

use axum::body::Bytes;
use axum::{extract::Path, Router};

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

pub fn inject_router(mut router: Router) -> Router {
    router = router.route("/upload/{*path}", axum::routing::post(upload));
    //映射本地资源
    let serve_dir = tower_http::services::ServeDir::new(PUBLIC_DIR)
        .precompressed_gzip()
        .append_index_html_on_directories(true);
    router = router.nest_service("/public", serve_dir);
    router
}
