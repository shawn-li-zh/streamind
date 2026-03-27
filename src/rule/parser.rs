use pest::Parser;
use pest_derive::Parser;
use std::hash::Hash;
use std::result::Result;

use crate::rule::ast::{Expr, Predicate};

// --- 1. 定义 AST (抽象语法树) 的结构 ---

#[derive(Parser)]
#[grammar = "rule/parser.pest"]
pub struct LogicParser;

//语法树构建：将 pest 解析结果转化为 Rust 枚举 ---
pub fn parse_expr(input: &str) -> anyhow::Result<Expr> {
    let pairs = LogicParser::parse(Rule::main, input)?;
    let pair = pairs.into_iter().next().unwrap(); // 获取顶层 expr 规则
    parse_expr_pair(pair)
}

fn parse_expr_pair(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Expr> {
    match pair.as_rule() {
        Rule::or_expr => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr_pair(pairs.next().unwrap())?;
            for p in pairs {
                expr = Expr::Or(Box::new(expr), Box::new(parse_expr_pair(p)?));
            }
            Ok(expr)
        }
        Rule::and_expr => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr_pair(pairs.next().unwrap())?;
            for p in pairs {
                expr = Expr::And(Box::new(expr), Box::new(parse_expr_pair(p)?));
            }
            Ok(expr)
        }
        Rule::unary_expr => {
            let mut pairs = pair.into_inner();
            let mut not_count = 0;
            let mut primary = None;
            for p in pairs {
                if p.as_rule() == Rule::not_op {
                    not_count += 1;
                } else {
                    primary = Some(p); // 这里的 p 可能是 predicate 或 or_expr (括号里的)
                }
            }
            let mut expr = parse_expr_pair(primary.unwrap())?;
            // 如果 "!" 出现奇数次，则包裹一层 Not
            if not_count % 2 != 0 {
                expr = Expr::Not(Box::new(expr));
            }
            Ok(expr)
        }
        Rule::predicate => {
            let mut pairs = pair.into_inner();
            let keyword = pairs.next().unwrap().as_str().to_string();
            let string_pair = pairs.next().unwrap();
            // 进入 string 规则，再进入 inner 获取纯净无引号字符串
            let arg = string_pair
                .into_inner()
                .next()
                .unwrap()
                .as_str()
                .to_string();
            let predicate = Predicate::from(keyword, vec![arg])?;
            Ok(Expr::Pred(predicate))
        }
        Rule::boolean=>{
            let val = pair.as_str();
            match val {
                "true" => Ok(Expr::True),
                "false" => Ok(Expr::False),
                _ => unreachable!(),
            }
        }
        _ => unreachable!("Unexpected rule: {:?}", pair.as_rule()),
    }
}
