use async_trait::async_trait;

use crate::{
 
    operator::EventElement,
};
use rule_script::*;
pub struct EventTypeIs;
#[async_trait]
impl Predicate<EventElement> for EventTypeIs {
    fn name(&self) -> &'static str {
        "event_type_is"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let ty = args[0].clone().must_string()?;

        return Ok(TestValue::Bool(element.event_type == ty));
    }
}
