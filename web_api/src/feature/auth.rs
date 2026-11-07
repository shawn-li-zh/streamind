use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    sync::{Arc, LazyLock},
};

use axum::{
    BoxError, Router, ServiceExt, body::Body, error_handling::HandleErrorLayer, extract::{ConnectInfo, Query, Request}, handler::Handler, http::HeaderMap, middleware::{self, Next}, response::{IntoResponse, Response}
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use reqwest::{header, StatusCode};
use serde::{Deserialize, Serialize};
use kernel::simple_id;

use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
// use tracing::info;

use crate::wrapper::ApiResponse;

static SECRET_KEY: LazyLock<(EncodingKey, DecodingKey)> = LazyLock::new(|| {
    // 不再回退到内置默认值：弱密钥/占位密钥等同于没有鉴权。
    // 由 inject_router 在启动时强制初始化，配置缺失会直接启动失败。
    let key = std::env::var("JWT_SECRET_KEY").unwrap_or_default();
    if key.len() < 16 || key == "your_jwt_secret_key" {
        panic!(
            "JWT_SECRET_KEY 未配置或强度不足（至少 16 个字符）。请在 .env 中设置随机密钥，参考 .env.example"
        );
    }
    let encoding_key = EncodingKey::from_secret(key.as_ref());
    let decoding_key = DecodingKey::from_secret(key.as_ref());
    (encoding_key, decoding_key)
});
static JWT_EXPIRATION_MINUTES: LazyLock<i64> = LazyLock::new(|| {
    std::env::var("JWT_EXPIRATION_MINUTES")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(600)
});
pub static LOGIN_CODE: LazyLock<String> = LazyLock::new(|| {
    // 生成一个随机的登录验证码
    let code = simple_id(8);

    code
});

fn verify_token(token: &str) -> anyhow::Result<LoginState> {
    // 配置验证项（如是否检查过期、目标受众等）
    let mut validation = Validation::default();
    validation.validate_exp = true; // 验证过期时间
    let state = decode::<LoginState>(token, &SECRET_KEY.1, &validation)
        .map(|data| data.claims)
        .map_err(anyhow::Error::from)?;
    Ok(state)
}
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginState {
    pub sub: String, // 用户标识 (Subject)
    pub exp: usize,  // 过期时间 (Expiration Time, Unix 时间戳)
    pub iat: usize,  // 签发时间 (Issued At)
    pub ip: IpAddr,
    pub user_agent: String,
}

fn get_ip(headers: HeaderMap, ip: IpAddr) -> Option<IpAddr> {
    // 1. 尝试从 X-Forwarded-For 获取真实 IP（如果有反向代理）
    if let Some(forwarded) = headers.get("X-Forwarded-For") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip_str) = forwarded_str.split(',').next() {
                if let Ok(ip) = ip_str.trim().parse() {
                    return Some(ip);
                }
            }
        }
    }
    // 2. 否则使用远程地址
    Some(ip)
}
#[derive(Debug, Deserialize)]
struct LoginReq {
    pub code: String,
}
async fn login(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(req): Query<LoginReq>,
    headers: HeaderMap,
) -> ApiResponse<String> {
    // info!("{} is trying to login with code: {}", addr.ip(), req.code);
    if &req.code != &*LOGIN_CODE {
        return ApiResponse::error("Invalid login code").code(401);
    }

    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_else(|| "Unknown")
        .to_string();
    let state = LoginState {
        ip: get_ip(headers, addr.ip()).unwrap_or_else(|| "0.0.0.0".parse().expect("无法解析IP地址")),
        user_agent: ua,
        sub: "user".to_string(),
        iat: chrono::Utc::now().timestamp() as usize,
        exp: (chrono::Utc::now() + chrono::Duration::minutes(*JWT_EXPIRATION_MINUTES)).timestamp()
            as usize, // 根据环境变量设置过期时间
    };
    let token = encode(&Header::default(), &state, &SECRET_KEY.0).expect("Token 生成失败");
    ApiResponse::success(token)
}

// 2. 中间件处理函数
async fn auth_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: Next,
    whitelist: Vec<&'static str>,
) -> Result<Response, ApiResponse<()>> {
    let path = req.uri().path();

    // A. 白名单检查
    for ele in whitelist {
        if path.starts_with(ele) {
            return Ok(next.run(req).await);
        }
    }
    // B. 身份验证逻辑
    let auth_value = match req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        Some(v) => v,
        None => return Err(ApiResponse::error("未登录").code(401)),
    };
    let res = match verify_token(auth_value) {
        Ok(v) => v,
        Err(_) => return Err(ApiResponse::error("登录无效").code(403)),
    };
    // C. 其他验证（如 IP 白名单、User-Agent 检查等）
    Ok(next.run(req).await)
}
pub fn inject_router(mut router: Router) -> Router {
    // 启动即校验 JWT 密钥配置，配置缺失时fail-fast，而不是等到调用登录接口才崩
    LazyLock::force(&SECRET_KEY);
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(3)
        .burst_size(1)
        .finish()
        .unwrap();

    router = router.route(
        "/api/login",
        axum::routing::get(login).layer(GovernorLayer::new(governor_conf)),
    );

    // 1. 定义白名单路径
    let whitelist: Vec<&'static str> = vec![
        "/api/login", "/public/", // 允许访问 /public/ 下的所有资源
    ];
    // router = router.layer(middleware::from_fn(
    //     move |req: axum::http::Request<axum::body::Body>, next| {
    //         auth_middleware(req, next, whitelist.clone())
    //     },
    // ));

    router
}
