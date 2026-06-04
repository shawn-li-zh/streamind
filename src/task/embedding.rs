use entity_service::DB;
use kernel::message::Information;
use tokio_cron_scheduler::Job;
use tracing::{error, info};

use crate::global::{EMBEDDING_CLIENT, SCHEDULER};

// 定时嵌入概念向量
pub async fn init() {
    let sched = SCHEDULER.get().unwrap();
    let job = Job::new_async("0 * * * * ?", |_uuid, _locked| {
        Box::pin(async move {
            let concepts = entity_service::concept::list_not_embedded(30)
                .await
                .expect("list_not_embedded error");
            info!("embedding concepts: {}", concepts.len());
            let texts: Vec<_> = concepts
                .iter()
                .map(|c| format!("{}:{}", c.name, c.content))
                .map(|s| Information::Text(s))
                .collect();
            let clinet = EMBEDDING_CLIENT.get().unwrap();
            let db = DB.get().unwrap();
            for (text, concept) in texts.into_iter().zip(concepts) {
                let embedding = match clinet.union_embed(vec![text]).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("{}", e);
                        continue;
                    }
                };
                let id = concept.id;
                if let Err(e) = entity_service::concept::update_vector(db, id, embedding).await {
                    error!("更新向量失败: {},{}", id, e);
                }
            }
        })
    })
    .unwrap();

    sched.add(job).await.unwrap();
    let job = Job::new_async("0 1/3 * * * ?", |_uuid, _locked| {
        Box::pin(async move {
            let scopes = entity_service::rule::list_scopes_not_embedded(30)
                .await
                .expect("list_not_embedded error");
            info!("embedding scopes: {}", scopes.len());
            let texts: Vec<_> = scopes
                .iter()
                .map(|c| format!("{}:{}", c.name, c.content))
                .map(|s| Information::Text(s))
                .collect();
            let clinet = EMBEDDING_CLIENT.get().unwrap();
            let db = DB.get().unwrap();
            for (text, scope) in texts.into_iter().zip(scopes) {
                let embedding = match clinet.union_embed(vec![text]).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("{}", e);
                        continue;
                    }
                };
                let id = scope.id;
                if let Err(e) = entity_service::rule::update_scope_vector(db, id, embedding).await {
                    error!("更新向量失败: {},{}", id, e);
                }
            }
        })
    })
    .unwrap();

    sched.add(job).await.unwrap();
}
