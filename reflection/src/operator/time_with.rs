use async_trait::async_trait;

use crate::operator::EventElement;
use rule_script::*;

pub struct TimeWith;
#[async_trait]
impl Predicate<EventElement> for TimeWith {
    fn name(&self) -> &'static str {
        "time_with"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let s = args[0].clone().must_string()?;

        return Ok(TestValue::Bool(element.time.contains(&s)));
    }
}
