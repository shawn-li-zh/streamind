use std::{pin::Pin, str::FromStr, sync::OnceLock, time::Duration};

pub use entity::*;
use sea_orm::{
    ConnectOptions, Database, DatabaseConnection, DatabaseTransaction, TransactionTrait,
    sqlx::sqlite::SqliteConnectOptions,
};
pub mod agent;
pub mod agent_permission;
pub mod agent_timer;
pub mod component;
pub mod concept;
pub mod dependence;
pub mod event;
pub mod model;
pub mod rule;
pub mod section;
pub mod state;

pub static DB: OnceLock<DatabaseConnection> = OnceLock::new();

pub async fn init() -> anyhow::Result<()> {
    //初始化数据库连接
    let url = std::env::var("DATABASE_URL")?;
    // let mut opt = SqliteConnectOptions::from_str(&url)?;
    // opt.extension("sqlite_vec0");
    let mut opt = ConnectOptions::new(url);
    opt.sqlx_logging(false);
    opt.max_connections(500);
    opt.connect_timeout(Duration::from_mins(1));
    opt.acquire_timeout(Duration::from_mins(3));
    // 设置连接钩子
    opt.after_connect(|conn| {
        
        Box::pin(async move {
            unsafe {
                // 使用 sqlite-vec 提供的注册函数
                // 需要在 Cargo.toml 中添加 sqlite-vec = "0.1.0"
                sqlite_vec::sqlite3_vec_init();
            }
            Ok(())
        })
    });
    let db = Database::connect(opt).await.expect("数据库连接失败");
    db.get_schema_registry("entity::*").sync(&db).await?;
    // sea_orm::ConnectionTrait::execute_unprepared(
    //     &db,
    //     "SELECT load_extension('sqlite_vec0');", // 确保你的环境下能找到该扩展
    // )
    // .await?;
    DB.set(db).unwrap();
    //重写前端导入映射
    dependence::rewrite_manifest().await?;
    Ok(())
}

pub async fn transaction<F, T>(callback: F) -> anyhow::Result<T>
where
    F: for<'c> FnOnce(
            &'c DatabaseTransaction,
        )
            -> Pin<Box<dyn Future<Output = Result<T, anyhow::Error>> + Send + 'c>>
        + Send,
    T: Send,
{
    let db = DB.get().unwrap();
    let data = match db.transaction(callback).await {
        Ok(v) => v,
        Err(e) => {
            return Err(anyhow::anyhow!("{}", e));
        }
    };
    Ok(data)
}
