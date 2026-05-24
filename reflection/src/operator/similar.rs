use async_trait::async_trait;


use crate::operator::EventElement;
use rule_script::*;
pub struct Similar;
#[async_trait]
impl Predicate<EventElement> for Similar {
    fn name(&self) -> &'static str {
        "similar"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let concept_key = args[0].clone().must_string()?;
        let res = entity_service::concept::find_by_name(&concept_key, true)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Concept not found"))?;
        let vec = res.embedding.unwrap_or_default();
        let cos=cosine_similarity(&vec.0, &element.embedding);
        Ok(TestValue::Number(rust_decimal::Decimal::from_f64_retain(cos).unwrap_or_default()))
    }
}

/// 辅助函数：计算两个向量的余弦相似度
fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f64 {
    if v1.len() != v2.len() || v1.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let magnitude_v1: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_v2: f32 = v2.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_v1 == 0.0 || magnitude_v2 == 0.0 {
        return 0.0;
    }

    (dot_product / (magnitude_v1 * magnitude_v2)) as f64
}
