use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::expr::{ArgValue, BinOperator, Compare, Expr};
#[async_trait]
pub trait Predicate<E>: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    async fn test(&self, element: &E, args: &[ArgValue]) -> anyhow::Result<TestValue>;
    fn simplify(
        &self,
        args: &[ArgValue],
        this: Expr,
        op: BinOperator,
        other: Expr,
    ) -> anyhow::Result<Expr> {
        Ok(Expr::Binary {
            left: Box::new(this),
            op: op,
            right: Box::new(other),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestValue {
    Bool(bool),
    Number(Decimal),
    Error(String),
}

impl TestValue {
    pub fn must_bool(self) -> anyhow::Result<bool> {
        match self {
            TestValue::Bool(v) => Ok(v),
            TestValue::Error(e) => anyhow::bail!(e),
            _ => anyhow::bail!("Not a bool"),
        }
    }
    pub fn must_number(self) -> anyhow::Result<Decimal> {
        match self {
            TestValue::Number(v) => Ok(v),
            TestValue::Error(e) => anyhow::bail!(e),
            _ => anyhow::bail!("Not a number"),
        }
    }
}
