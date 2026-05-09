use std::{
    collections::HashMap,
    sync::{LazyLock, OnceLock},
    time::Duration,
};

use crate::common::prompt_cache::PromptCache;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::level_filters::LevelFilter;

pub static PROMPT_CACHE: LazyLock<PromptCache> =
    LazyLock::new(|| PromptCache::new("./prompts".into()));

pub static DATE_TIME_FMT: &str = "%Y-%m-%d %H:%M:%S";
pub static DATE_FMT: &str = "%Y-%m-%d";
pub static TIME_FMT: &str = "%H:%M:%S";
