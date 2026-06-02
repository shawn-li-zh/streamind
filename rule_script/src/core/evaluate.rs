use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_recursion::async_recursion;
use dashmap::DashMap;
use futures::{StreamExt, stream};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator as _};
use rust_decimal::MathematicalOps;
use tokio::sync::OnceCell;

use crate::{
    core::engine::ScriptEngine,
    expr::{ArgValue, Arithmetic, BinOperator, Compare, Expr, Logic, to_func_key},
    pred::TestValue,
};

//把真值表带入每棵树做纯内存逻辑求解
pub async fn solve_exprs<E: Send + Sync + 'static>(
    engine: Arc<ScriptEngine<E>>,
    exprs: &Vec<Expr>,
    element: Arc<E>,
) -> anyhow::Result<Vec<TestValue>> {
    let cache = Arc::new(ValueCache {
        inner: DashMap::new(),
    });
    let max_concurrency = 30;
    let tasks_data: Vec<_> = exprs
        .iter()
        .map(|expr| (expr.clone(), engine.clone(), element.clone(), cache.clone()))
        .collect();

    let results = stream::iter(tasks_data)
        .map(|(expr, engine, element, cache)| async move {
            match solve_expr(engine, &expr, element, cache).await {
                Ok(v) => v,
                Err(e) => TestValue::Error(e.to_string()),
            }
        })
        .buffer_unordered(max_concurrency)
        .collect::<Vec<_>>()
        .await;
    Ok(results)
}

/// 使用已计算出的基础真值上下文（context），求解整个逻辑表达式

static FALSE: TestValue = TestValue::Bool(false);

pub struct ValueCache {
    inner: DashMap<String, Arc<OnceCell<TestValue>>>,
}
#[async_recursion]
async fn solve_expr<E: Send + Sync + 'static>(
    engine: Arc<ScriptEngine<E>>,
    expr: &Expr,
    element: Arc<E>,
    cache: Arc<ValueCache>,
) -> anyhow::Result<TestValue> {
    let b = match expr {
        Expr::Func(_, name, args) => {
            let key = to_func_key(name, args);
            let element = element.clone();
            let engine = engine.clone();
            let cell = cache.inner.entry(key).or_default().value().clone();
            let value = cell
                .get_or_init(|| async move {
                    // 2. get_predicate 得到的引用 pred 仅在 await 之前有效
                    // 由于 engine 是 Arc，它保证了 ScriptEngine 的生命周期
                    let res = match engine.get_predicate(&name) {
                        Ok(pred) => match pred.test(&element, &args).await {
                            Ok(v) => v,
                            Err(e) => TestValue::Error(e.to_string()),
                        },
                        Err(e) => TestValue::Error(e.to_string()),
                    };
                    res
                })
                .await;
            value.clone()
        }
        Expr::Binary { left, op, right } => {
            let left = solve_expr(engine.clone(), left, element.clone(), cache.clone()).await?;
            let right = solve_expr(engine.clone(), right, element.clone(), cache.clone()).await?;
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
            let b = solve_expr(engine, a, element, cache).await?.must_bool()?;
            TestValue::Bool(!b)
        }
        Expr::Bool(b) => TestValue::Bool(*b),

        Expr::Number(n) => TestValue::Number(*n),
    };
    return Ok(b);
}
