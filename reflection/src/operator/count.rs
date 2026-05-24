use async_trait::async_trait;
use regex::Regex;

use crate::operator::EventElement;
use rule_script::*;
pub struct Count;
#[async_trait]
impl Predicate<EventElement> for Count {
    fn name(&self) -> &'static str {
        "count"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let regex = args[0].clone().must_string()?;
        let re = Regex::new(&regex)?;

        let count = re.find_iter(&element.context).count();
        return Ok(TestValue::Number(count.into()));
    }
}
