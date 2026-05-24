use async_trait::async_trait;

use crate::operator::EventElement;
use rule_script::*;

pub struct IndexOf;
#[async_trait]
impl Predicate<EventElement> for IndexOf {
    fn name(&self) -> &'static str {
        "index_of"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let s = args[0].clone().must_string()?;
        if let Some(idnex) = element.context.find(&s) {
            return Ok(TestValue::Number(idnex.into()));
        }
        return Ok(TestValue::Number((-1).into()));
    }
}
