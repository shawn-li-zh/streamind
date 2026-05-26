use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use futures::{StreamExt, stream};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator as _};
use rust_decimal::MathematicalOps;

use crate::{
    core::engine::ScriptEngine,
    expr::{ArgValue, Arithmetic, BinOperator, Compare, Expr, Logic, to_func_key},
    pred::TestValue,
};
pub struct TruthValues {
    pub(crate) inner: HashMap<String, TestValue>,
}

pub struct Operators {
    pub(crate) inner: HashMap<String, (String, Vec<ArgValue>)>,
}
//提取去重算子
pub fn extract_operators(exprs: &[Expr]) -> Operators {
    // Step 2: 提取所有的基础函数元素去重
    let mut oerators = Operators {
        inner: HashMap::new(),
    };

    for expr in exprs {
        extract_operator(expr, &mut oerators);
    }
    oerators
}
pub fn extract_operator(expr: &Expr, operators: &mut Operators) {
    match expr {
        Expr::Func(name, args) => {
            let key = to_func_key(&name, &args);
            operators.inner.insert(key, (name.clone(), args.clone()));
        }
        Expr::Not(e) => extract_operator(e, operators),
        Expr::Binary { left, right, op: _ } => {
            extract_operator(left, operators);
            extract_operator(right, operators);
        }
        _ => {}
    }
}

//计算所有算子的真值表
pub async fn compute_truth_values<E: Send + Sync + 'static>(
    engine: Arc<ScriptEngine<E>>,
    operators: Arc<Operators>,
    element: Arc<E>,
) -> anyhow::Result<TruthValues> {
    let max_concurrency = 30;

    let tasks_data: Vec<_> = operators
        .inner
        .iter()
        .map(|(key, (name, args))| (key.clone(), name.clone(), args.clone()))
        .collect();

    let pred_results_list = stream::iter(tasks_data)
        .map(|(key, name, args)| {
            let element = element.clone();
            let engine = engine.clone();

            // 这是一个 Send + 'static 的 Future
            async move {
                // 2. get_predicate 得到的引用 pred 仅在 await 之前有效
                // 由于 engine 是 Arc，它保证了 ScriptEngine 的生命周期
                let res = match engine.get_predicate(&name) {
                    Ok(pred) => {
                        match pred.test(&element, &args).await {
                            Ok(v) => v,
                            Err(e) => TestValue::Error(e.to_string()),
                        }
                    }
                    Err(e) => TestValue::Error(e.to_string()),
                };
                (key, res)
            }
        })
        .buffer_unordered(max_concurrency)
        .collect::<Vec<_>>()
        .await;

    let pred_results: HashMap<_, _> = pred_results_list.into_iter().collect();
    Ok(TruthValues {
        inner: pred_results,
    })
}
//把真值表带入每棵树做纯内存逻辑求解
pub fn solve_exprs(
    exprs: Arc<Vec<Expr>>,
    truth_values: TruthValues,
) -> anyhow::Result<Vec<TestValue>> {
    let results = (exprs)
        .par_iter()
        .map(|ast| match solve_expr(&ast, &truth_values.inner) {
            Ok(v) => v,
            Err(e) => TestValue::Error(e.to_string()),
        })
        .collect();
    Ok(results)
}

/// 使用已计算出的基础真值上下文（context），求解整个逻辑表达式

static FALSE: TestValue = TestValue::Bool(false);

fn solve_expr(expr: &Expr, context: &HashMap<String, TestValue>) -> anyhow::Result<TestValue> {
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
