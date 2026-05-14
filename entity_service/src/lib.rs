use std::{pin::Pin, sync::OnceLock, time::Duration};

use sea_orm::{
    ConnectOptions, Database, DatabaseConnection, DatabaseTransaction, TransactionTrait,
};

pub mod agent;
pub mod agent_permission;
pub mod component;
pub mod dependence;
pub mod event;
pub mod model;
pub mod section;
pub mod state;

pub static DB: OnceLock<DatabaseConnection> = OnceLock::new();

pub async fn init() -> anyhow::Result<()> {
    //初始化数据库连接
    let url = std::env::var("DATABASE_URL")?;
    let mut opt = ConnectOptions::new(url);
    opt.sqlx_logging(false);
    opt.max_connections(500);
    opt.connect_timeout(Duration::from_mins(1));
    opt.acquire_timeout(Duration::from_mins(3));

    let db = Database::connect(opt).await.expect("数据库连接失败");
    db.get_schema_registry("entity::*").sync(&db).await?;
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
