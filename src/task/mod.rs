pub mod embedding;
pub async fn init() {
    embedding::init().await;
}
