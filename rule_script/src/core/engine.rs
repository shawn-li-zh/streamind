use std::{collections::HashMap, sync::Arc};

use crate::{
    TruthValues,
    core::{
        evaluate::{self, Operators},
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

    pub async fn evaluate(
        self: Arc<ScriptEngine<E>>,
        exprs: &[&str],
        element: Arc<E>,
    ) -> anyhow::Result<Vec<TestValue>> {
        //解析表达式
        let mut asts = vec![];
        for expr in exprs {
            asts.push(parser::parse_expr(expr)?);
        }
        //算子去重
        let operators = evaluate::extract_operators(&asts);
        let operators = Arc::new(operators);
        let asts = Arc::new(asts);
        let values = self.solves(element, operators, asts).await?;
        return Ok(values);
    }

    pub async fn solves(
        self: Arc<ScriptEngine<E>>,
        element: Arc<E>,
        operators: Arc<Operators>,
        exprs: Arc<Vec<Expr>>,
    ) -> anyhow::Result<Vec<TestValue>> {
        //计算真值
        let truth_values = evaluate::compute_truth_values(self.clone(), operators, element).await?;
        //代入求解
        let values = evaluate::solve_exprs(exprs, truth_values)?;
        return Ok(values);
    }

    pub fn simplify(&self, expr: &str) -> anyhow::Result<String> {
        let expr = parser::parse_expr(expr)?;
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
