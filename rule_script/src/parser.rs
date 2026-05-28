use std::str::FromStr as _;

use pest::Parser;
use pest_derive::Parser;

// 这里假设您之前定义的 AST (Expr, Operator, Compare, Arithmetic) 均在此可用
use crate::expr::{ArgValue, Arithmetic, BinOperator, Compare, Expr, Logic};

#[derive(Parser)]
#[grammar = "src/parser.pest"] // 确保这里指向你正确的 pest 文件路径
pub struct LogicParser;

/// 工具函数：解析浮点数字符串并保留 n 位小数，返回 (整数部分, 小数部分)
fn parse_number(s: &str) -> rust_decimal::Decimal {
    let n = rust_decimal::Decimal::from_str(s).expect("parse number error");
    return n.round_dp_with_strategy(
        DECIMAL_PLACES,
        rust_decimal::RoundingStrategy::MidpointAwayFromZero,
    );
}

pub fn parse_expr(input: &str) -> anyhow::Result<Expr> {
    let pairs = LogicParser::parse(Rule::main, input)?;
    let pair = pairs.into_iter().next().unwrap();
    parse_expr_pair(pair)
}
pub(crate) const DECIMAL_PLACES: u32 = 4;
fn parse_expr_pair(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Expr> {
    match pair.as_rule() {
        // 使用一个通用的匹配块处理所有左结合的二元运算，极大地精简代码
        Rule::or_expr
        | Rule::and_expr
        | Rule::cmp_expr
        | Rule::add_expr
        | Rule::mul_expr
        | Rule::pow_expr => {
            let mut pairs = pair.into_inner();
            let mut expr = parse_expr_pair(pairs.next().unwrap())?;

            while let Some(op_pair) = pairs.next() {
                let op = match op_pair.as_str() {
                    "|" => BinOperator::Logic(Logic::Or),
                    "&" => BinOperator::Logic(Logic::And),
                    "+" => BinOperator::Arith(Arithmetic::Add),
                    "-" => BinOperator::Arith(Arithmetic::Sub),
                    "*" => BinOperator::Arith(Arithmetic::Mul),
                    "/" => BinOperator::Arith(Arithmetic::Div),
                    "%" => BinOperator::Arith(Arithmetic::Mod),
                    "^" => BinOperator::Arith(Arithmetic::Pow),
                    ">" => BinOperator::Cmp(Compare::Gt),
                    "<" => BinOperator::Cmp(Compare::Lt),
                    ">=" => BinOperator::Cmp(Compare::Gte),
                    "<=" => BinOperator::Cmp(Compare::Lte),
                    "==" => BinOperator::Cmp(Compare::Eq),
                    "!=" => BinOperator::Cmp(Compare::Ne),
                    _ => unreachable!("Unknown operator: {}", op_pair.as_str()),
                };
                let right = parse_expr_pair(pairs.next().unwrap())?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            }
            Ok(expr)
        }

        Rule::unary_expr => {
            let pairs = pair.into_inner();
            let mut not_count = 0;
            let mut primary = None;
            for p in pairs {
                if p.as_rule() == Rule::not_op {
                    not_count += 1;
                } else {
                    primary = Some(p);
                }
            }
            let mut expr = parse_expr_pair(primary.unwrap())?;
            if not_count % 2 != 0 {
                expr = Expr::Not(Box::new(expr));
            }
            Ok(expr)
        }

        Rule::func => {
            let mut pairs = pair.into_inner();
            let ident = pairs.next().unwrap().as_str().to_string(); // 函数名
            let mut args = Vec::new();

            // 解析参数列表
            if let Some(args_pair) = pairs.next() {
                for arg_pair in args_pair.into_inner() {
                    match arg_pair.as_rule() {
                        Rule::string => {
                            // inner 不带双引号
                            let s = arg_pair.into_inner().next().unwrap().as_str().to_string();
                            args.push(ArgValue::Str(s));
                        }
                        Rule::number => {
                            let n = parse_number(arg_pair.as_str());
                            args.push(ArgValue::Number(n));
                        }
                        Rule::boolean => {
                            args.push(ArgValue::Bool(arg_pair.as_str() == "true"));
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Ok(Expr::Func(ident, args))
        }

        Rule::number => {
            let n = parse_number(pair.as_str());
            Ok(Expr::Number(n))
        }

        Rule::boolean => Ok(Expr::Bool(pair.as_str() == "true")),

        _ => unreachable!("Unexpected rule: {:?}", pair.as_rule()),
    }
}

// --- 3. 测试 ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_complex_expr() {
        // 测试复合运算、函数调用、小数补齐和逻辑组合
        let input = r#" aaa("bbb", 1.2, true) > 3.1415926 & (10 + 5 * 2 == 20) | !false &test('中文') |str(`sdsds`)"#;
        let expr = parse_expr(input).unwrap();
        println!("{:#?}", expr);
    }
}
