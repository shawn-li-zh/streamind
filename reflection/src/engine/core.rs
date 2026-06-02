use std::sync::Arc;

use rule_script::{
    Expr,
    evaluate::{self},
};
#[derive(Clone)]
pub struct Rule<T: Clone> {
    pub expr: Expr,
    //反射内容
    pub data: T,
}
pub struct RuleGroup<T: Clone> {
    pub name: String,
    pub rules: Vec<Rule<T>>,
    pub exprs: Arc<Vec<Expr>>,
}
