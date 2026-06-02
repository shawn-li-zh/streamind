// use anyhow::Ok;

use crate::{
    core::engine::ScriptEngine,
    expr::{BinOperator, Compare, Expr, Logic},
};

// 逻辑表达式化简，应用基本的逻辑恒等律，尽可能地简化表达式树
pub fn simplify<E: Send + Sync + 'static>(
    engine: &ScriptEngine<E>,
    expr: Expr,
) -> anyhow::Result<Expr> {
    let e = match expr {
        Expr::Not(inner) => {
            match simplify(engine, *inner)? {
                Expr::Not(e) => *e, // 双重否定
                Expr::Bool(b) => Expr::Bool(!b),
                e => Expr::Not(Box::new(e)),
            }
        }
        Expr::Binary { left, op, right } => {
            let sl = simplify(engine, *left)?;
            let sr = simplify(engine, *right)?;
            //排序
            let (sl, sr) = if sl < sr { (sl, sr) } else { (sr, sl) };
            match (&sl, &sr) {
                (Expr::Func(_,name, args), e) => {
                    let pred = engine.get_predicate(name)?;
                    return Ok(pred.simplify(args, sl.clone(), op, sr)?);
                }
                (e, Expr::Func(_, name, args)) => {
                    let pred = engine.get_predicate(name)?;
                    return Ok(pred.simplify(args, sr.clone(), op, sl)?);
                }
                _ => {}
            }
            match &op {
                BinOperator::Cmp(op) => match op {
                    Compare::Eq => {
                        if sl == sr {
                            return Ok(Expr::Bool(true));
                        }
                    }
                    Compare::Ne => {
                        if sl == sr {
                            return Ok(Expr::Bool(false));
                        }
                    }
                    Compare::Gt | Compare::Gte | Compare::Lt | Compare::Lte => {
                        if sl == sr {
                            return Ok(Expr::Bool(false));
                        }
                    }
                },
                BinOperator::Logic(opt) => {
                    return simplify_logic_expr(op.clone(), opt.clone(), sl, sr);
                }
                _ => {}
            }
            Expr::Binary {
                left: Box::new(sl),
                op: op,
                right: Box::new(sr),
            }
        }
        e => e,
    };
    Ok(e)
}

fn simplify_logic_expr(op: BinOperator, opt: Logic, sl: Expr, sr: Expr) -> anyhow::Result<Expr> {
    let b = match opt {
        Logic::And => {
            if sl == sr {
                return Ok(sl); // A & A => A
            }

            match (&sl, &sr) {
                (Expr::Bool(a), _) | (_, Expr::Bool(a)) => {
                    if *a {
                        // A & True => A
                        sl
                    } else {
                        // A & False => False
                        Expr::Bool(false)
                    }
                }

                (l, Expr::Not(inner)) => {
                    if **inner == *l {
                        return Ok(Expr::Bool(false)); // A & !A => False
                    }

                    Expr::Binary {
                        left: Box::new(sl),
                        op: op,
                        right: Box::new(sr),
                    }
                }
                (Expr::Not(inner), r) => {
                    if **inner == *r {
                        return Ok(Expr::Bool(false)); // !A & A => False
                    }
                    Expr::Binary {
                        left: Box::new(sl),
                        op: op,
                        right: Box::new(sr),
                    }
                }

                _ => Expr::Binary {
                    left: Box::new(sl),
                    op: op,
                    right: Box::new(sr),
                },
            }
        }
        Logic::Or => {
            if sl == sr {
                return Ok(sl); // A | A => A
            }

            //排序
            let (sl, sr) = if sl < sr { (sl, sr) } else { (sr, sl) };
            match (&sl, &sr) {
                (Expr::Bool(a), _) | (_, Expr::Bool(a)) => {
                    if *a {
                        // A | True => True
                        Expr::Bool(true)
                    } else {
                        // A | False => A
                        sl
                    }
                }

                (l, Expr::Not(inner)) => {
                    if **inner == *l {
                        return Ok(Expr::Bool(true)); // A | !A => True
                    }
                    Expr::Binary {
                        left: Box::new(sl),
                        op: op,
                        right: Box::new(sr),
                    }
                }
                (Expr::Not(inner), r) => {
                    if **inner == *r {
                        return Ok(Expr::Bool(true)); // !A | A => True
                    }
                    Expr::Binary {
                        left: Box::new(sl),
                        op: op,
                        right: Box::new(sr),
                    }
                }

                _ => Expr::Binary {
                    left: Box::new(sl),
                    op: op,
                    right: Box::new(sr),
                },
            }
        }
    };
    return Ok(b);
}
