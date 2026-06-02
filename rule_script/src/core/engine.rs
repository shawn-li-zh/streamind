use std::{collections::HashMap, sync::Arc};

use crate::{
    core::{
        evaluate::{self},
        simplify,
    },
    expr::Expr,
    parser,
    pred::{Predicate, TestValue},
};

pub struct ScriptEngine<E> {
    pub(crate) preds: HashMap<&'static str, Arc<dyn Predicate<E> + Send + Sync>>,
}
impl<E: Send + Sync + 'static> ScriptEngine<E> {
    pub fn new() -> Self {
        ScriptEngine {
            preds: HashMap::new(),
        }
    }
    pub fn register_predicate<P: Predicate<E> + Send + Sync + 'static>(&mut self, pred: P) {
        self.preds.insert(pred.name(), Arc::new(pred));
    }

    pub async fn solves(
        self: Arc<ScriptEngine<E>>,
        exprs: &Vec<Expr>,
        element: Arc<E>,
    ) -> anyhow::Result<Vec<TestValue>> {
        //代入求解
        let values = evaluate::solve_exprs(self.clone(), &exprs, element).await?;
        return Ok(values);
    }

    pub fn simplify(&self, expr: &str, orders: &HashMap<&str, i32>) -> anyhow::Result<String> {
        let expr =
            parser::parse_expr(expr, orders).map_err(|e| anyhow::anyhow!("表达式解析失败:{e}"))?;
        let expr = simplify::simplify(self, expr)?;
        Ok(format!("{}", expr))
    }

    pub(crate) fn get_predicate(
        &self,
        pred_name: &str,
    ) -> anyhow::Result<Arc<dyn Predicate<E> + Send + Sync>> {
        let pred = self
            .preds
            .get(pred_name)
            .ok_or_else(|| anyhow::anyhow!("Predicate not found: {}", pred_name))?;
        Ok(pred.clone())
    }
}
