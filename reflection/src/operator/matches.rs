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
    // fn simplify(
    //     &self,
    //     args: &[ArgValue],
    //     this: Expr,
    //     op: BinOperator,
    //     other: Expr,
    // ) -> anyhow::Result<Expr> {
    //     match &other {
    //         Expr::Func(name, other_args) => {
    //             if let Some(e) = self.handle(args, this.clone(), op.clone(), other.clone())? {
    //                 return Ok(e);
    //             }
    //         }
    //         Expr::Not(e) => {
    //             if let Some(e) = self.handle_not(args, op.clone(), *e.clone())? {
    //                 return Ok(e);
    //             }
    //         }
    //         _ => {}
    //     }

    //     Ok(Expr::Binary {
    //         left: Box::new(this),
    //         op: op,
    //         right: Box::new(other.clone()),
    //     })
    // }
}
