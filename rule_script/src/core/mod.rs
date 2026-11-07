pub mod engine;
pub mod evaluate;
pub mod simplify;

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use async_trait::async_trait;

    use crate::{
        core::engine::ScriptEngine, expr::{ArgValue, BinOperator, Expr}, parser, pred::{Predicate, TestValue}
    };

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
                Expr::Func(_, name, other_args) => {
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
                Expr::Func(_, name, other_args) => {
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
                Expr::Func(_, name, other_args) => {
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
    fn to_expr(s: &str) -> Expr {
   
        parser::parse_expr(s, &HashMap::new()).unwrap()
    }
    #[tokio::test]
    async fn test_parse_rule() {
        let engine = Arc::new(init_engine());
        let element = Arc::new(init_element());
        {
            let expr = r#"contains("f")"#;
            let res = engine.clone().solves(&vec![to_expr(expr)], element.clone()).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap()[0], TestValue::Bool(true));
        }

        {
            let expr = r#"contains("f") & contains("o") | !contains("bar")"#;
            let res = engine.clone().solves(&vec![to_expr(expr)], element.clone()).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap()[0], TestValue::Bool(true));
        }
        {
            let expr = r#"age()<10 & contains("o")"#;
            let res = engine.clone().solves(&vec![to_expr(expr)], element.clone()).await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap()[0], TestValue::Bool(false));
        }
    }
    #[test]
    fn test_simplify() {
        let engine = init_engine();
        let expr = r#"contains("foo") & contains("o")"#;
        let res = engine.simplify(expr, &HashMap::new()).unwrap();
        assert_eq!(&res, r#"contains("foo")"#);
        //
        let expr = r#"contains("foo") & !contains("o")"#;
        let res = engine.simplify(expr, &HashMap::new()).unwrap();
        assert_eq!(&res, r#"false"#);
        //
        let expr = r#"contains("foo") | !contains("o")"#;
        let res = engine.simplify(expr, &HashMap::new()).unwrap();
        assert_eq!(&res, r#"true"#);
    }

    /// 带延迟的谓词：用于验证批量求解结果的顺序与输入顺序一致
    struct Echo;
    #[async_trait]
    impl Predicate<Element> for Echo {
        fn name(&self) -> &'static str {
            "echo"
        }
        async fn test(&self, _element: &Element, args: &[ArgValue]) -> anyhow::Result<TestValue> {
            let n = args[0].clone().must_number()?;
            let millis: u64 = n.to_string().parse().unwrap_or(0);
            tokio::time::sleep(std::time::Duration::from_millis(millis * 20)).await;
            Ok(TestValue::Number(n))
        }
    }

    #[tokio::test]
    async fn test_solve_order_follows_input() {
        let mut engine = init_engine();
        engine.register_predicate(Echo);
        let engine = Arc::new(engine);
        let element = Arc::new(init_element());
        // 耗时递减：如果按完成顺序返回，结果会变成 [1, 2, 3]
        let exprs = vec![to_expr("echo(3)"), to_expr("echo(1)"), to_expr("echo(2)")];
        let res = engine.solves(&exprs, element).await.unwrap();
        let nums: Vec<u64> = res
            .iter()
            .map(|v| match v {
                TestValue::Number(n) => n.to_string().parse().unwrap(),
                other => panic!("期望数字，实际 {other:?}"),
            })
            .collect();
        assert_eq!(nums, vec![3, 1, 2]);
    }

    #[tokio::test]
    async fn test_division_by_zero_does_not_panic() {
        let engine = Arc::new(init_engine());
        let element = Arc::new(init_element());
        for src in ["age() / 0", "age() % 0"] {
            let res = engine
                .clone()
                .solves(&vec![to_expr(src)], element.clone())
                .await
                .unwrap();
            assert!(
                matches!(res[0], TestValue::Error(_)),
                "{src} 应该返回错误而不是 panic"
            );
        }
    }

    #[tokio::test]
    async fn test_arithmetic_and_precedence() {
        let engine = Arc::new(init_engine());
        let element = Arc::new(init_element());
        let cases = [
            ("1 + 2 * 3 == 7", true),
            ("(1 + 2) * 3 == 9", true),
            ("10 / 4 == 2.5", true),
            ("!true | true", true),
            ("!true & true", false),
            ("age() >= 18 & age() < 60", true),
        ];
        for (src, expected) in cases {
            let res = engine
                .clone()
                .solves(&vec![to_expr(src)], element.clone())
                .await
                .unwrap();
            assert_eq!(res[0], TestValue::Bool(expected), "{src}");
        }
    }
}
