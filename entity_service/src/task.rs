use chrono::Local;
use entity::task_queue::{self, TaskStatus};
use sea_orm::{
    ActiveModelTrait as _, ActiveValue::Set, ColumnTrait as _, EntityTrait as _, QueryFilter as _,
    QueryOrder as _, sea_query::Expr,
};

use crate::DB;

pub async fn new_task(task_type: &str, payload: serde_json::Value) -> anyhow::Result<i64> {
    let db = DB.get().unwrap();
    let task = task_queue::ActiveModel {
        task_type: Set(task_type.to_string()),
        payload: Set(payload),
        time: Set(Local::now()),
        status: Set(TaskStatus::Pending),
        ..Default::default()
    };
    let res=task.insert(db).await?;
    Ok(res.id)
}

pub async fn load_tasks(
    task_type: &str,
    status: TaskStatus,
) -> anyhow::Result<Vec<task_queue::Model>> {
    let db = DB.get().unwrap();
    let tasks = task_queue::Entity::find()
        .filter(task_queue::Column::TaskType.eq(task_type))
        .filter(task_queue::Column::Status.eq(status))
        .order_by_asc(task_queue::Column::Id)
        .all(db)
        .await?;
    Ok(tasks)
}

pub async fn update_tasks(task_ids: Vec<i64>, status: TaskStatus) -> anyhow::Result<()> {
    let db = DB.get().unwrap();
    task_queue::Entity::update_many()
        .col_expr(task_queue::Column::Status, Expr::value(status))
        .filter(task_queue::Column::Id.is_in(task_ids))
        .exec(db)
        .await?;

    Ok(())
}
