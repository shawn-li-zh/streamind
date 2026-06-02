use async_trait::async_trait;

use crate::operator::EventElement;
use rule_script::*;
pub struct Count;
#[async_trait]
impl Predicate<EventElement> for Count {
    fn name(&self) -> &'static str {
        "count"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let sub = args[0].clone().must_string()?;
        let len = sub.len();
        let mut i = 0;
        let mut count = 0;
        loop {
            let end = i + len;
            if end > element.context.len() {
                break;
            }
            if &element.context[i..end] == sub {
                count += 1;
                i += len;
                continue;
            }
            i += 1;
        }
        return Ok(TestValue::Number(count.into()));
    }
}
