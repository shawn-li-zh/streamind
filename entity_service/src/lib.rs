use std::{pin::Pin, str::FromStr, sync::OnceLock, time::Duration};

pub use entity::*;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend::Sqlite, DatabaseConnection,
    DatabaseTransaction, Statement, TransactionTrait, sqlx::sqlite::SqliteConnectOptions,
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
pub mod task;

pub static DB: OnceLock<DatabaseConnection> = OnceLock::new();
static SQL: &'static str = include_str!("../../sql/init.vec.sql");
pub async fn init() -> anyhow::Result<()> {
    use libsqlite3_sys::sqlite3_auto_extension;
    use sqlite_vec::sqlite3_vec_init;

    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }
    //初始化数据库连接
    let url = std::env::var("DATABASE_URL")?;
    let mut opt = ConnectOptions::new(url);
    opt.sqlx_logging(false);
    opt.max_connections(500);
    opt.connect_timeout(Duration::from_mins(1));
    opt.acquire_timeout(Duration::from_mins(3));

    let db = Database::connect(opt).await.expect("数据库连接失败");
    db.get_schema_registry("entity::*").sync(&db).await?;

    let stmt = Statement::from_string(Sqlite, SQL);
    db.execute_raw(stmt).await?;

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
