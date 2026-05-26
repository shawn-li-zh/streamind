use async_trait::async_trait;
use regex::Regex;

use crate::operator::EventElement;
use rule_script::*;
pub struct Matches;
#[async_trait]
impl Predicate<EventElement> for Matches {
    fn name(&self) -> &'static str {
        "matches"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let regex = args[0].clone().must_string()?;
        let re = Regex::new(&regex)?;
        Ok(TestValue::Bool(re.is_match(&element.context)))
    }
}
