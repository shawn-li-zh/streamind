use rule_script::{Expr, core::evaluate::Operators};

pub struct Rule {
    pub id: i64,
    pub expr: Expr,
    //反射内容
    pub content: String,
    pub time: chrono::DateTime<chrono::Local>,
}
pub struct RuleGroup {
    pub id: i64,
    pub name: String,
    pub operators: Operators,
    pub rules: Vec<Rule>,
    pub exprs: Vec<Expr>,
}
