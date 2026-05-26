use std::fmt::Display;

use rust_decimal::Decimal;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Compare {
    Gt,
    Lt,
    Lte,
    Gte,
    Eq,
    Ne,
}
impl Display for Compare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Compare::Gt => write!(f, ">"),
            Compare::Lt => write!(f, "<"),
            Compare::Lte => write!(f, "<="),
            Compare::Gte => write!(f, ">="),
            Compare::Eq => write!(f, "=="),
            Compare::Ne => write!(f, "!="),
        }
    }
}
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Arithmetic {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}
impl Display for Arithmetic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arithmetic::Add => write!(f, "+"),
            Arithmetic::Sub => write!(f, "-"),
            Arithmetic::Mul => write!(f, "*"),
            Arithmetic::Div => write!(f, "/"),
            Arithmetic::Mod => write!(f, "%"),
            Arithmetic::Pow => write!(f, "^"),
        }
    }
}
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Logic {
    And,
    Or,
}
impl Display for Logic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Logic::And => write!(f, "&"),
            Logic::Or => write!(f, "|"),
        }
    }
}
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum BinOperator {
    Cmp(Compare),
    Arith(Arithmetic),
    Logic(Logic),
}
impl Display for BinOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOperator::Cmp(op) => write!(f, "{}", op),
            BinOperator::Arith(op) => write!(f, "{}", op),
            BinOperator::Logic(op) => write!(f, "{}", op),
        }
    }
}
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ArgValue {
    Str(String),
    Bool(bool),
    Number(Decimal),
}

impl ArgValue {
    pub fn must_number(self) -> anyhow::Result<Decimal> {
        match self {
            ArgValue::Number(n) => Ok(n),
            _ => anyhow::bail!("Not a number"),
        }
    }
    pub fn must_bool(self) -> anyhow::Result<bool> {
        match self {
            ArgValue::Bool(v) => Ok(v),
            _ => anyhow::bail!("Not a bool"),
        }
    }
    pub fn must_string(self) -> anyhow::Result<String> {
        match self {
            ArgValue::Str(s) => Ok(s),
            _ => anyhow::bail!("Not a string"),
        }
    }
}

/// 逻辑表达式树
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Expr {
    //函数 名称 参数
    Func(String, Vec<ArgValue>),
    Not(Box<Expr>),
    Binary {
        left: Box<Expr>,
        op: BinOperator,
        right: Box<Expr>,
    },
    Number(Decimal),
    Bool(bool),
}
impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Func(name, args) => {
                let s = to_func_key(name, args);
                write!(f, "{}", s)
            }
            Expr::Not(e) => write!(f, "!{}", e),
            Expr::Number(n) => write!(f, "{}", n),
            Expr::Bool(b) => write!(f, "{}", b),
            Expr::Binary { left, op, right } => {
                write!(f, "({} {} {})", left, op, right)
            }
        }
    }
}
pub(crate) fn to_func_key(name: &str, args: &[ArgValue]) -> String {
    let args = args
        .iter()
        .map(|v| match v {
            ArgValue::Str(s) => format!("\"{}\"", s),
            ArgValue::Bool(b) => format!("{}", b),
            ArgValue::Number(n) => format!("{}", n,),
        })
        .collect::<Vec<String>>();
    format!("{}({})", name, args.join(", "))
}
