pub mod evaluate;
pub mod simplify;
pub mod engine;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

use async_trait::async_trait;

    use crate::{core::engine::ScriptEngine, expr::{ArgValue, BinOperator, Expr}, pred::{Predicate, TestValue}};

    use super::*;
    struct Element {
        name: String,
        age: i32,
    }
    struct Contain;
    #[async_trait]
    impl Predicate<Element> for Contain {
        fn name(&self) -> &'static str {
            "contains"
        }
        async fn test(&self, element: &Element, args: &[ArgValue]) -> anyhow::Result<TestValue> {
            let name = args[0].clone().must_string()?;
            if element.name.contains(&name) {
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
                Expr::Func(name, other_args) => {
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
    impl Contain {
        fn handle(
            &self,
            args: &[ArgValue],
            this: Expr,
            op: BinOperator,
            other: Expr,
        ) -> anyhow::Result<Option<Expr>> {
            match &other {
                Expr::Func(name, other_args) => {
                    if name == self.name() {
                        let l = args[0].clone().must_string()?;
                        let r = other_args[0].clone().must_string()?;

                        match &op {
                            BinOperator::Logic(logic) => match logic {
                                crate::expr::Logic::And => {
                                    if l.contains(&r) {
                                        return Ok(Some(this));
                                    }
                                    if r.contains(&l) {
                                        return Ok(Some(other));
                                    }
                                }
                                crate::expr::Logic::Or => {
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
                Expr::Func(name, other_args) => {
                    if name == self.name() {
                        let l = args[0].clone().must_string()?;
                        let r = other_args[0].clone().must_string()?;

                        match &op {
                            BinOperator::Logic(logic) => match logic {
                                crate::expr::Logic::And => {
                                    if l.contains(&r) {
                                        return Ok(Some(Expr::Bool(false)));
                                    }
                                }
                                crate::expr::Logic::Or => {
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

    struct Age;
    #[async_trait]
    impl Predicate<Element> for Age {
        fn name(&self) -> &'static str {
            "age"
        }
        async fn test(&self, element: &Element, args: &[ArgValue]) -> anyhow::Result<TestValue> {
            Ok(TestValue::Number(element.age.into()))
        }
    }
    fn init_element() -> Element {
        Element {
            name: "foo".to_string(),
            age: 18,
        }
    }
    fn init_engine() -> ScriptEngine<Element> {
        let mut engine = ScriptEngine::<Element>::new();
        engine.register_predicate(Contain);
        engine.register_predicate(Age);
        engine
    }
    #[tokio::test]
    async fn test_parse_rule() {
        let engine = Arc::new(init_engine());
        let element = Arc::new(init_element());
        {
            let expr = r#"contains("f")"#;
            let res = engine.clone().evaluate(&vec![expr], element.clone()).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap()[0], TestValue::Bool(true));
        }

        {
            let expr = r#"contains("f") & contains("o") | !contains("bar")"#;
            let res = engine.clone().evaluate(&vec![expr],  element.clone()).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap()[0], TestValue::Bool(true));
        }
        {
            let expr = r#"age()<10 & contains("o")"#;
            let res = engine.clone().evaluate(&vec![expr],  element.clone()).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap()[0], TestValue::Bool(false));
        }
    }
    #[test]
    fn test_simplify() {
        let engine = init_engine();
        let expr = r#"contains("foo") & contains("o")"#;
        let res = engine.simplify(expr).unwrap();
        assert_eq!(&res, r#"contains("foo")"#);
        //
        let expr = r#"contains("foo") & !contains("o")"#;
        let res = engine.simplify(expr).unwrap();
        assert_eq!(&res, r#"false"#);
        //
        let expr = r#"contains("foo") | !contains("o")"#;
        let res = engine.simplify(expr).unwrap();
        assert_eq!(&res, r#"true"#);
    }
}
