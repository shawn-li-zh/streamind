use std::{
    collections::HashMap,
    sync::{LazyLock, OnceLock},
};

use crate::common::prompt_cache::PromptCache;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::level_filters::LevelFilter;

pub static PROMPT_CACHE: LazyLock<PromptCache> =
    LazyLock::new(|| PromptCache::new("./prompts".into()));

pub static DB: OnceLock<DatabaseConnection> = OnceLock::new();
pub static DATE_TIME_FMT: &str = "%Y-%m-%d %H:%M:%S";
pub static DATE_FMT: &str = "%Y-%m-%d";
pub static TIME_FMT: &str = "%H:%M:%S";
pub async fn init() -> anyhow::Result<()> {
    //初始化数据库连接
    let url = std::env::var("DATABASE_URL")?;
    let mut opt = ConnectOptions::new(url);
    opt.sqlx_logging(false);
    opt.max_connections(500);

    let db = Database::connect(opt).await.expect("数据库连接失败");
    db.get_schema_registry("entity::*").sync(&db).await?;
    DB.set(db).unwrap();
    Ok(())
}
