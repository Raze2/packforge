//! `pf-expr` — a tiny, deterministic, sandboxed expression language.
//!
//! * Numbers are millimetres (`f64`). Literals may carry a unit: `10mm`, `2cm`, `0.1m`, `1in`.
//! * Operators: `+ - * / % ^`, `== != < <= > >=`, `and or not` (also `&& || !`), `c ? a : b`.
//! * Functions: `min max clamp abs round floor ceil sqrt sin cos tan atan2 rad deg`.
//!   Trigonometry works in **degrees**; `rad(x)`/`deg(x)` convert.
//! * Strings in single or double quotes, compared with `==` / `!=`.
//! * Names may contain dots (`board.caliper`) and are resolved through an [`Env`].
//!
//! There are no loops, assignments or I/O, and source size and nesting depth are capped,
//! so evaluation always terminates quickly.

use std::collections::BTreeSet;
use std::fmt;

mod parse;

pub use parse::parse;

const MAX_SOURCE_LEN: usize = 4096;
const MAX_DEPTH: usize = 64;
const NUM_EPS: f64 = 1e-9;

/// Byte range in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{message} (at {}..{})", span.start, span.end)]
pub struct ExprError {
    pub message: String,
    pub span: Span,
}

impl ExprError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Bool(bool),
    Str(String),
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Num(_) => "number",
            Value::Bool(_) => "boolean",
            Value::Str(_) => "string",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Num(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => write!(f, "'{s}'"),
        }
    }
}

/// Name lookup for evaluation.
pub trait Env {
    fn get(&self, name: &str) -> Option<Value>;
}

impl<F: Fn(&str) -> Option<Value>> Env for F {
    fn get(&self, name: &str) -> Option<Value> {
        self(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Num(f64),
    Bool(bool),
    Str(String),
    Var(String),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    /// Names referenced by this expression (function names excluded).
    pub fn variables(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        self.collect_vars(&mut out);
        out
    }

    fn collect_vars(&self, out: &mut BTreeSet<String>) {
        match &self.kind {
            ExprKind::Var(n) => {
                out.insert(n.clone());
            }
            ExprKind::Unary(_, e) => e.collect_vars(out),
            ExprKind::Binary(_, a, b) => {
                a.collect_vars(out);
                b.collect_vars(out);
            }
            ExprKind::Cond(c, a, b) => {
                c.collect_vars(out);
                a.collect_vars(out);
                b.collect_vars(out);
            }
            ExprKind::Call(_, args) => args.iter().for_each(|a| a.collect_vars(out)),
            ExprKind::Num(_) | ExprKind::Bool(_) | ExprKind::Str(_) => {}
        }
    }

    pub fn eval(&self, env: &dyn Env) -> Result<Value, ExprError> {
        let v = match &self.kind {
            ExprKind::Num(n) => Value::Num(*n),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Str(s) => Value::Str(s.clone()),
            ExprKind::Var(name) => env
                .get(name)
                .ok_or_else(|| ExprError::new(format!("unknown name '{name}'"), self.span))?,
            ExprKind::Unary(op, e) => {
                let v = e.eval(env)?;
                match op {
                    UnaryOp::Neg => Value::Num(-num(&v, e.span)?),
                    UnaryOp::Not => Value::Bool(!boolean(&v, e.span)?),
                }
            }
            ExprKind::Binary(op, a, b) => self.eval_binary(*op, a, b, env)?,
            ExprKind::Cond(c, a, b) => {
                if boolean(&c.eval(env)?, c.span)? {
                    a.eval(env)?
                } else {
                    b.eval(env)?
                }
            }
            ExprKind::Call(name, args) => self.eval_call(name, args, env)?,
        };
        if let Value::Num(n) = v {
            if !n.is_finite() {
                return Err(ExprError::new("result is not a finite number", self.span));
            }
        }
        Ok(v)
    }

    /// Evaluates and requires a number.
    pub fn eval_num(&self, env: &dyn Env) -> Result<f64, ExprError> {
        num(&self.eval(env)?, self.span)
    }

    /// Evaluates and requires a boolean.
    pub fn eval_bool(&self, env: &dyn Env) -> Result<bool, ExprError> {
        boolean(&self.eval(env)?, self.span)
    }

    fn eval_binary(
        &self,
        op: BinaryOp,
        a: &Expr,
        b: &Expr,
        env: &dyn Env,
    ) -> Result<Value, ExprError> {
        use BinaryOp::*;
        // Short-circuit logic first.
        if matches!(op, And | Or) {
            let l = boolean(&a.eval(env)?, a.span)?;
            if (op == And && !l) || (op == Or && l) {
                return Ok(Value::Bool(l));
            }
            return Ok(Value::Bool(boolean(&b.eval(env)?, b.span)?));
        }
        let (l, r) = (a.eval(env)?, b.eval(env)?);
        if matches!(op, Eq | Ne) {
            let equal = match (&l, &r) {
                (Value::Num(x), Value::Num(y)) => (x - y).abs() <= NUM_EPS,
                (Value::Bool(x), Value::Bool(y)) => x == y,
                (Value::Str(x), Value::Str(y)) => x == y,
                _ => {
                    return Err(ExprError::new(
                        format!("cannot compare {} with {}", l.type_name(), r.type_name()),
                        self.span,
                    ))
                }
            };
            return Ok(Value::Bool(if op == Eq { equal } else { !equal }));
        }
        let (x, y) = (num(&l, a.span)?, num(&r, b.span)?);
        Ok(match op {
            Add => Value::Num(x + y),
            Sub => Value::Num(x - y),
            Mul => Value::Num(x * y),
            Div | Rem => {
                if y == 0.0 {
                    return Err(ExprError::new("division by zero", b.span));
                }
                Value::Num(if op == Div { x / y } else { x % y })
            }
            Pow => Value::Num(x.powf(y)),
            Lt => Value::Bool(x < y),
            Le => Value::Bool(x <= y + NUM_EPS),
            Gt => Value::Bool(x > y),
            Ge => Value::Bool(x + NUM_EPS >= y),
            Eq | Ne | And | Or => unreachable!("handled above"),
        })
    }

    fn eval_call(&self, name: &str, args: &[Expr], env: &dyn Env) -> Result<Value, ExprError> {
        let nums = args
            .iter()
            .map(|a| a.eval_num(env))
            .collect::<Result<Vec<f64>, _>>()?;
        let arity = |n: usize| -> Result<(), ExprError> {
            if nums.len() == n {
                Ok(())
            } else {
                Err(ExprError::new(
                    format!("{name}() takes {n} argument(s), got {}", nums.len()),
                    self.span,
                ))
            }
        };
        let r = match name {
            "min" | "max" => {
                if nums.is_empty() {
                    return Err(ExprError::new(
                        format!("{name}() needs at least one argument"),
                        self.span,
                    ));
                }
                let f = if name == "min" { f64::min } else { f64::max };
                nums[1..].iter().fold(nums[0], |acc, v| f(acc, *v))
            }
            "clamp" => {
                arity(3)?;
                if nums[1] > nums[2] {
                    return Err(ExprError::new(
                        "clamp(): lower bound > upper bound",
                        self.span,
                    ));
                }
                nums[0].clamp(nums[1], nums[2])
            }
            "abs" => {
                arity(1)?;
                nums[0].abs()
            }
            "round" => {
                arity(1)?;
                nums[0].round()
            }
            "floor" => {
                arity(1)?;
                nums[0].floor()
            }
            "ceil" => {
                arity(1)?;
                nums[0].ceil()
            }
            "sqrt" => {
                arity(1)?;
                if nums[0] < 0.0 {
                    return Err(ExprError::new("sqrt() of a negative number", self.span));
                }
                nums[0].sqrt()
            }
            "sin" => {
                arity(1)?;
                nums[0].to_radians().sin()
            }
            "cos" => {
                arity(1)?;
                nums[0].to_radians().cos()
            }
            "tan" => {
                arity(1)?;
                nums[0].to_radians().tan()
            }
            "atan2" => {
                arity(2)?;
                nums[0].atan2(nums[1]).to_degrees()
            }
            "rad" => {
                arity(1)?;
                nums[0].to_radians()
            }
            "deg" => {
                arity(1)?;
                nums[0].to_degrees()
            }
            _ => {
                return Err(ExprError::new(
                    format!("unknown function '{name}'"),
                    self.span,
                ))
            }
        };
        Ok(Value::Num(r))
    }
}

fn num(v: &Value, span: Span) -> Result<f64, ExprError> {
    match v {
        Value::Num(n) => Ok(*n),
        other => Err(ExprError::new(
            format!("expected a number, found {}", other.type_name()),
            span,
        )),
    }
}

fn boolean(v: &Value, span: Span) -> Result<bool, ExprError> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => Err(ExprError::new(
            format!("expected a boolean, found {}", other.type_name()),
            span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Option<Value> {
        match name {
            "L" => Some(Value::Num(300.0)),
            "W" => Some(Value::Num(200.0)),
            "board.caliper" => Some(Value::Num(4.0)),
            "side" => Some(Value::Str("left".into())),
            _ => None,
        }
    }

    fn eval(src: &str) -> Value {
        parse(src)
            .unwrap_or_else(|e| panic!("parse {src}: {e}"))
            .eval(&env)
            .unwrap_or_else(|e| panic!("eval {src}: {e}"))
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(eval("1 + 2 * 3"), Value::Num(7.0));
        assert_eq!(eval("(1 + 2) * 3"), Value::Num(9.0));
        assert_eq!(eval("-2 ^ 2"), Value::Num(-4.0));
        assert_eq!(eval("2 ^ 3 ^ 2"), Value::Num(512.0));
        assert_eq!(eval("10 % 4"), Value::Num(2.0));
    }

    #[test]
    fn units_convert_to_mm() {
        assert_eq!(eval("1in"), Value::Num(25.4));
        assert_eq!(eval("2cm + 5mm"), Value::Num(25.0));
        assert_eq!(eval("0.5m"), Value::Num(500.0));
    }

    #[test]
    fn names_functions_and_ternary() {
        assert_eq!(eval("L + board.caliper"), Value::Num(304.0));
        assert_eq!(eval("max(1.5 * board.caliper, 4mm)"), Value::Num(6.0));
        assert_eq!(eval("clamp(W * 0.1, 25, 40)"), Value::Num(25.0));
        assert_eq!(eval("side == 'left' ? 35 : 0"), Value::Num(35.0));
        assert_eq!(eval("side != \"left\" or L > W"), Value::Bool(true));
        assert_eq!(eval("not (L < W) && true"), Value::Bool(true));
        let s = eval("sin(30)");
        assert!(matches!(s, Value::Num(v) if (v - 0.5).abs() < 1e-12));
    }

    #[test]
    fn variables_are_collected() {
        let e = parse("max(L, W) + board.caliper * k").expect("parse");
        let vars: Vec<_> = e.variables().into_iter().collect();
        assert_eq!(vars, ["L", "W", "board.caliper", "k"]);
    }

    #[test]
    fn errors_have_spans() {
        let e = parse("L + nope").expect("parse").eval(&env).unwrap_err();
        assert!(e.message.contains("nope"));
        assert_eq!((e.span.start, e.span.end), (4, 8));
        assert!(parse("1 +").is_err());
        assert!(parse("3furlongs").is_err());
        assert!(parse("1 / 0").expect("parse").eval(&env).is_err());
        assert!(parse("L ? 1 : 2").expect("parse").eval(&env).is_err());
        assert!(parse("sqrt(-1)").expect("parse").eval(&env).is_err());
        assert!(parse("foo(1)").expect("parse").eval(&env).is_err());
    }

    #[test]
    fn depth_is_limited() {
        let deep = format!("{}1{}", "(".repeat(200), ")".repeat(200));
        assert!(parse(&deep).is_err());
    }
}
