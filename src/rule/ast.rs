use rayon::prelude::*;
use regex::Regex;
use std::hash::Hash;
use std::io::ErrorKind;
use std::result::Result;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use crate::rule::parser::parse_expr;
#[derive(Debug)]
pub enum SyntaxError {
    MissingArgument(String),
    InvalidRegex(String, String), // (pattern, error)
}
impl std::error::Error for SyntaxError {}
impl Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyntaxError::MissingArgument(func) => {
                write!(f, "Missing argument for function: {}", func)
            }
            SyntaxError::InvalidRegex(pattern, err) => {
                write!(f, "Invalid regex pattern: {}, error: {}", pattern, err)
            }
        }
    }
}
/// 对应语法中的具体函数调用，如 include("xxx") 或 match("regex")
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Counter {
    Count(String), // 计数器，代表某个元素出现的次数
    
}
/// 对应语法中的具体函数调用，如 include("xxx") 或 match("regex")
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Predicate {
    Include(String), // 代表包含关系
    Match(String),   // 代表匹配关系
}

impl Display for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::Include(s) => write!(f, "include(\"{}\")", s),
            Predicate::Match(s) => write!(f, "match(\"{}\")", s),
        }
    }
}

impl Predicate {
    pub fn from(key: String, args: Vec<String>) -> Result<Self, SyntaxError> {
        // 这里简单示例，实际可以根据函数名区分不同类型的 Predicate
        if key == "include" {
            if let Some(arg) = args.first() {
                Ok(Predicate::Include(arg.clone()))
            } else {
                Err(SyntaxError::MissingArgument("include".into()))
            }
        } else if key == "match" {
            if let Some(arg) = args.first() {
                //校验正则表达式是否合法
                match regex::Regex::new(arg) {
                    Ok(r) => {
                        return Ok(Predicate::Match(arg.clone()));
                    }
                    Err(e) => {
                        return Err(SyntaxError::InvalidRegex(arg.clone(), e.to_string()));
                    }
                }
            } else {
                Err(SyntaxError::MissingArgument("match".into()))
            }
        } else {
            Err(SyntaxError::InvalidRegex(key, "Unknown predicate".into()))
        }
    }

    pub fn evaluate(&self, text: &str) -> bool {
        match self {
            Predicate::Include(arg) => text.contains(arg),
            Predicate::Match(regex) => {
                let re = regex::Regex::new(regex).unwrap();
                re.is_match(text)
            }
        }
    }
}

/// 逻辑表达式树
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Expr {
    Pred(Predicate),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    True,
    False,
}
impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Pred(p) => write!(f, "{}", p),
            Expr::Not(e) => write!(f, "!{}", e),
            Expr::And(a, b) => write!(f, "({} & {})", a, b),
            Expr::Or(a, b) => write!(f, "({} | {})", a, b),
            Expr::True => write!(f, "true"),
            Expr::False => write!(f, "false"),
        }
    }
}

impl Expr {
    // 逻辑表达式化简，应用基本的逻辑恒等律，尽可能地简化表达式树
    pub fn simplify(&self) -> Expr {
        match self {
            Expr::Not(inner) => match inner.simplify() {
                Expr::Not(e) => *e, // 双重否定
                Expr::True => Expr::False, // !True => False
                Expr::False => Expr::True, // !False => True
                e => Expr::Not(Box::new(e)),
            },
            Expr::And(a, b) => {
                let sa = a.simplify();
                let sb = b.simplify();
                if sa == sb {
                    return sa; // A & A => A
                }
                //排序
                let (sa, sb) = if sa < sb { (sa, sb) } else { (sb, sa) };
                match (&sa, &sb) {
                    (Expr::True, _) | (_, Expr::True) => sa, // A & True => A
                    (Expr::False, _) | (_, Expr::False) => Expr::False, // A & False => False
                    (l, Expr::Not(inner)) => {
                        if **inner == *l {
                            return Expr::False; // A & !A => False
                        }
                        Expr::And(Box::new(sa), Box::new(sb))
                    }
                    (Expr::Not(inner), r) => {
                        if **inner == *r {
                            return Expr::False; // !A & A => False
                        }
                        Expr::And(Box::new(sa), Box::new(sb))
                    }
                    _ => Expr::And(Box::new(sa), Box::new(sb)),
                }
            }
            Expr::Or(a, b) => {
                let sa = a.simplify();
                let sb = b.simplify();
                if sa == sb {
                    return sa; // A | A => A
                }
                //排序
                let (sa, sb) = if sa < sb { (sa, sb) } else { (sb, sa) };
                match (&sa, &sb) {
                    (Expr::False, e) | (e, Expr::False) => sa, // A | False => A
                    (Expr::True, _) | (_, Expr::True) => Expr::True, // A | True => True
                    (l, Expr::Not(inner)) => {
                        if **inner == *l {
                            return Expr::True; // A | !A => True
                        }
                        Expr::Or(Box::new(sa), Box::new(sb))
                    }
                    (Expr::Not(inner), r) => {
                        if **inner == *r {
                            return Expr::True; // !A | A => True
                        }
                        Expr::Or(Box::new(sa), Box::new(sb))
                    }
                    _ => Expr::Or(Box::new(sa), Box::new(sb)),
                }
            }
            e => e.clone(),
        }
    }
}
/// 从 AST 提取所有叶子函数，用于去重
fn extract_predicates(expr: &Expr, set: &mut HashSet<Predicate>) {
    match expr {
        Expr::Pred(p) => {
            set.insert(p.clone());
        }
        Expr::Not(e) => extract_predicates(e, set),
        Expr::And(a, b) | Expr::Or(a, b) => {
            extract_predicates(a, set);
            extract_predicates(b, set);
        }
        _ => {}
    }
}

/// 使用已计算出的基础真值上下文（context），求解整个逻辑表达式
fn solve_expr(expr: &Expr, context: &HashMap<Predicate, bool>) -> bool {
    match expr {
        Expr::Pred(p) => *context.get(p).unwrap_or(&false),
        Expr::Not(e) => !solve_expr(e, context),
        Expr::And(a, b) => solve_expr(a, context) && solve_expr(b, context),
        Expr::Or(a, b) => solve_expr(a, context) || solve_expr(b, context),
        Expr::True => true,
        Expr::False => false,
    }
}

pub fn eval(text: &str, exprs: &Vec<&str>) -> anyhow::Result<Vec<bool>> {
    let mut asts = Vec::new();

    // Step 1: 解析所有表达式为 AST 结构
    for raw in exprs {
        // 因为 main 是静默规则，next直接拿到里面的 or_expr
        let ast = parse_expr(raw)?;
        asts.push(ast);
    }

    // Step 2: 提取所有的基础函数元素去重
    let mut unique_preds = HashSet::new();
    for ast in &asts {
        extract_predicates(ast, &mut unique_preds);
    }

    let pred_results: HashMap<Predicate, bool> = unique_preds
        .into_par_iter() // 开启多线程并行迭代
        .map(|pred| {
            // 对 text 做具体判定逻辑
            let res = pred.evaluate(text);
            (pred, res)
        })
        .collect();

    // Step 4: 把真值表带入每棵树做纯内存逻辑求解
    let results = asts
        .iter()
        .map(|ast| solve_expr(ast, &pred_results))
        .collect();
    Ok(results)
}

#[test]
fn test() {
    let text = "hello beautiful rust world，123";

    // 一批需要验证的表达式
    let raw_exprs = vec![
        r#"include("rust") | match("\d+")"#,
        r#"!(include("hello") & include("world"))"#,
        r#"!(include("hello") & include("hello"))"#,
        r#"match("hello")"#,
        r#"include("python") | include("go")"#,
        r#"include("rust") & !!match("rust")"#, // 包含了重合的谓词 include("rust")
    ];
    let results = eval(text, &raw_exprs).unwrap();

    // 打印缓存结果
    for (i, res) in results.iter().enumerate() {
        println!(" > {} = {}", raw_exprs[i], res);
    }
    //打印化简后的表达式
    raw_exprs.iter().for_each(|raw| {
        let ast = parse_expr(raw).unwrap();
        let simplified = ast.simplify();
        println!("Original: {}, Simplified: {}", raw, simplified);
    });
   
}
