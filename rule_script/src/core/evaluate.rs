use std::collections::{HashMap, HashSet};

use futures::{StreamExt, stream};
use rayon::iter::{IntoParallelIterator, ParallelIterator as _};
use rust_decimal::MathematicalOps;

use crate::{
    core::engine::ScriptEngine,
    expr::{ArgValue, Arithmetic, BinOperator, Compare, Expr, Logic, to_func_key},
    pred::TestValue,
};
pub struct TruthValues<'a> {
    inner: HashMap<&'a String, TestValue>,
}

pub struct Operators {
    inner: HashMap<String, (String, Vec<ArgValue>)>,
}
//提取去重算子
pub fn extract_operators(exprs: &[Expr]) -> anyhow::Result<Operators> {
    // Step 2: 提取所有的基础函数元素去重
    let mut unique_preds = HashMap::new();
    for expr in exprs {
        extract_predicates(expr.clone(), &mut unique_preds);
    }
    return Ok(Operators {
        inner: unique_preds,
    });
}
//计算所有算子的真值表
pub async fn compute_truth_values<'a, E: Send + Sync + 'static>(
    engine: &ScriptEngine<E>,
    operators: &'a Operators,
    element: &E,
) -> anyhow::Result<TruthValues<'a>> {
    let max_concurrency = 30;

    // 直接在流中处理，由于没有 spawn，可以安全地引用 &self 和 &element
    let pred_results_list = stream::iter(&operators.inner)
        .map(|(key, (name, args))| async move {
            if let Ok(pred) = engine.get_predicate(&name) {
                let res = match pred.test(element, &args).await {
                    Ok(v) => v,
                    Err(e) => TestValue::Error(e.to_string()),
                };
                (key, res)
            } else {
                (key, TestValue::Error("Invalid predicate".to_string()))
            }
        })
        .buffer_unordered(max_concurrency) // 这里控制并发数
        .collect::<Vec<_>>()
        .await;

    let pred_results: HashMap<_, _> = pred_results_list.into_iter().collect();
    return Ok(TruthValues {
        inner: pred_results,
    });
}
//把真值表带入每棵树做纯内存逻辑求解
pub fn solve_exprs<'a>(
    exprs: &[Expr],
    truth_values: &TruthValues<'a>,
) -> anyhow::Result<Vec<TestValue>> {
    let results = exprs
        .into_par_iter()
        .map(|ast| match solve_expr(&ast, &truth_values.inner) {
            Ok(v) => v,
            Err(e) => TestValue::Error(e.to_string()),
        })
        .collect();
    Ok(results)
}

fn extract_predicates(expr: Expr, map: &mut HashMap<String, (String, Vec<ArgValue>)>) {
    match expr {
        Expr::Func(name, args) => {
            let key = to_func_key(&name, &args);
            map.insert(key, (name, args));
        }
        Expr::Not(e) => extract_predicates(*e, map),
        Expr::Binary { left, right, op: _ } => {
            extract_predicates(*left, map);
            extract_predicates(*right, map);
        }
        _ => {}
    }
}

/// 使用已计算出的基础真值上下文（context），求解整个逻辑表达式

static FALSE: TestValue = TestValue::Bool(false);

fn solve_expr(expr: &Expr, context: &HashMap<&String, TestValue>) -> anyhow::Result<TestValue> {
    let b = match expr {
        Expr::Func(name, args) => {
            let key = to_func_key(name, args);
            context
                .get(&key)
                .map(|v| v.clone())
                .unwrap_or(FALSE.clone())
        }
        Expr::Binary { left, op, right } => {
            let left = solve_expr(left, context)?;
            let right = solve_expr(right, context)?;
            match op {
                BinOperator::Cmp(cmp) => {
                    let left = left.must_number()?;
                    let right = right.must_number()?;
                    let b = match cmp {
                        Compare::Gt => left > right,
                        Compare::Lt => left < right,
                        Compare::Gte => left >= right,
                        Compare::Lte => left <= right,
                        Compare::Eq => left == right,
                        Compare::Ne => left != right,
                    };
                    TestValue::Bool(b)
                }
                BinOperator::Arith(arith) => {
                    let left = left.must_number()?;
                    let right = right.must_number()?;
                    let num = match arith {
                        Arithmetic::Add => left + right,
                        Arithmetic::Sub => left - right,
                        Arithmetic::Mul => left * right,
                        Arithmetic::Div => left / right,
                        Arithmetic::Mod => left % right,
                        Arithmetic::Pow => left.powf(right.as_f64()),
                    };
                    TestValue::Number(num)
                }
                BinOperator::Logic(logic) => {
                    let left = left.must_bool()?;
                    let right = right.must_bool()?;
                    let b = match logic {
                        Logic::And => left && right,
                        Logic::Or => left || right,
                    };
                    TestValue::Bool(b)
                }
            }
        }
        Expr::Not(a) => {
            let b = solve_expr(a, context)?.must_bool()?;
            TestValue::Bool(!b)
        }
        Expr::Bool(b) => TestValue::Bool(*b),

        Expr::Number(n) => TestValue::Number(*n),
    };
    return Ok(b);
}
