use async_trait::async_trait;
use regex::Regex;

use crate::operator::EventElement;
use rule_script::*;

pub struct IndexOfPattern;
#[async_trait]
impl Predicate<EventElement> for IndexOfPattern {
    fn name(&self) -> &'static str {
        "index_of_pattern"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let regex = args[0].clone().must_string()?;
        let re = Regex::new(&regex)?;

        if let Some(index) = re.find(&element.context) {
            return Ok(TestValue::Number(index.start().into()));
        }
        return Ok(TestValue::Number((-1).into()));
    }
}
