use async_trait::async_trait;

use crate::operator::EventElement;
use rule_script::*;

pub struct Includes;
#[async_trait]
impl Predicate<EventElement> for Includes {
    fn name(&self) -> &'static str {
        "includes"
    }

    async fn test(&self, element: &EventElement, args: &[ArgValue]) -> anyhow::Result<TestValue> {
        let context = args[0].clone().must_string()?;
        if element.context.contains(&context) {
            Ok(TestValue::Bool(true))
        } else {
            Ok(TestValue::Bool(false))
        }
    }
    fn simplify(
        &self,
        args: &[ArgValue],
        this: Expr,
        op: BinOperator,
        other: Expr,
    ) -> anyhow::Result<Expr> {
        match &other {
            Expr::Func(_,name, other_args) => {
                if let Some(e) = self.handle(args, this.clone(), op.clone(), other.clone())? {
                    return Ok(e);
                }
            }
            Expr::Not(e) => {
                if let Some(e) = self.handle_not(args, op.clone(), *e.clone())? {
                    return Ok(e);
                }
            }
            _ => {}
        }

        Ok(Expr::Binary {
            left: Box::new(this),
            op: op,
            right: Box::new(other.clone()),
        })
    }
}
impl Includes {
    fn handle(
        &self,
        args: &[ArgValue],
        this: Expr,
        op: BinOperator,
        other: Expr,
    ) -> anyhow::Result<Option<Expr>> {
        match &other {
            Expr::Func(_,name, other_args) => {
                if name == self.name() {
                    let l = args[0].clone().must_string()?;
                    let r = other_args[0].clone().must_string()?;

                    match &op {
                        BinOperator::Logic(logic) => match logic {
                            expr::Logic::And => {
                                if l.contains(&r) {
                                    return Ok(Some(this));
                                }
                                if r.contains(&l) {
                                    return Ok(Some(other));
                                }
                            }
                            expr::Logic::Or => {
                                if l.contains(&r) {
                                    return Ok(Some(other));
                                }
                                if r.contains(&l) {
                                    return Ok(Some(this));
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }
    fn handle_not(
        &self,
        args: &[ArgValue],
        op: BinOperator,
        other: Expr,
    ) -> anyhow::Result<Option<Expr>> {
        match &other {
            Expr::Func(_,name, other_args) => {
                if name == self.name() {
                    let l = args[0].clone().must_string()?;
                    let r = other_args[0].clone().must_string()?;

                    match &op {
                        BinOperator::Logic(logic) => match logic {
                            expr::Logic::And => {
                                if l.contains(&r) {
                                    return Ok(Some(Expr::Bool(false)));
                                }
                            }
                            expr::Logic::Or => {
                                if l.contains(&r) {
                                    return Ok(Some(Expr::Bool(true)));
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }
}
