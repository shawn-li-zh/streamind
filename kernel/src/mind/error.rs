#[derive(Debug, thiserror::Error)]
pub enum MindError {
  #[error("http error: {0}")]
    Reqwest(#[from] anyhow::Error),
}