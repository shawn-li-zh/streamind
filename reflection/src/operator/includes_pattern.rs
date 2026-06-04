use async_trait::async_trait;
use regex::Regex;

use crate::operator::EventElement;
use rule_script::*;

pub struct IncludesPattern;
#[async_trait]
impl Predicate<EventElement> for IncludesPattern {
    fn name(&self) -> &'static str {
        "includes_pattern"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let regex = args[0].clone().must_string()?;
        let re = Regex::new(&regex)?;
        let opt = re.find(&element.context);
        Ok(TestValue::Bool(opt.is_some()))
    }
}
