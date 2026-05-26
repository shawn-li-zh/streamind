use entity_service::DB;
use kernel::message::Information;
use tokio_cron_scheduler::Job;
use tracing::error;

use crate::global::{EMBEDDING_CLIENT, SCHEDULER};
// 定时嵌入概念向量
pub async fn init() {
    let sched = SCHEDULER.get().unwrap();
    sched
        .add(
            Job::new_async("0 * * * * ?", |_uuid, _locked| {
                Box::pin(async move {
                    let concepts = entity_service::concept::list_not_embedded(30)
                        .await
                        .expect("list_not_embedded error");
                    let texts: Vec<_> = concepts
                        .iter()
                        .map(|c| format!("{}:{}", c.name, c.content))
                        .map(|s| Information::Text(s))
                        .collect();
                    let clinet = EMBEDDING_CLIENT.get().unwrap();
                    let db = DB.get().unwrap();
                    match clinet.batch_embed(texts).await {
                        Ok(v) => {
                            for (concept, embedding) in concepts.into_iter().zip(v) {
                                let id = concept.id;

                                if let Err(e) =
                                    entity_service::concept::update_vector(db, id, embedding).await
                                {
                                    error!("更新向量失败: {},{}", id, e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("{}", e)
                        }
                    }
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();
}
