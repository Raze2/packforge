//! Lexer and Pratt parser.

use crate::{BinaryOp, Expr, ExprError, ExprKind, Span, UnaryOp, MAX_DEPTH, MAX_SOURCE_LEN};

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Str(String),
    Op(&'static str),
    LParen,
    RParen,
    Comma,
    Question,
    Colon,
    Eof,
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    span: Span,
}

fn lex(src: &str) -> Result<Vec<Token>, ExprError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let start = i;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let tok = if c.is_ascii_digit() || (c == '.' && next_is_digit(bytes, i)) {
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let text = &src[start..i];
            let value: f64 = text
                .parse()
                .map_err(|_| err(format!("invalid number '{text}'"), start, i))?;
            let unit_start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            let factor = match &src[unit_start..i] {
                "" | "mm" => 1.0,
                "cm" => 10.0,
                "m" => 1000.0,
                "in" => 25.4,
                other => return Err(err(format!("unknown unit '{other}'"), unit_start, i)),
            };
            Tok::Num(value * factor)
        } else if c.is_ascii_alphabetic() || c == '_' {
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
            {
                i += 1;
            }
            match &src[start..i] {
                "and" => Tok::Op("&&"),
                "or" => Tok::Op("||"),
                "not" => Tok::Op("!"),
                word => Tok::Ident(word.to_string()),
            }
        } else if c == '\'' || c == '"' {
            i += 1;
            let body_start = i;
            while i < bytes.len() && bytes[i] as char != c {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(err("unterminated string", start, i));
            }
            let s = src[body_start..i].to_string();
            i += 1;
            Tok::Str(s)
        } else {
            let two = src.get(i..i + 2).unwrap_or("");
            let op2 = ["==", "!=", "<=", ">=", "&&", "||"]
                .into_iter()
                .find(|o| *o == two);
            if let Some(op) = op2 {
                i += 2;
                Tok::Op(op)
            } else {
                i += 1;
                match c {
                    '+' => Tok::Op("+"),
                    '-' => Tok::Op("-"),
                    '*' => Tok::Op("*"),
                    '/' => Tok::Op("/"),
                    '%' => Tok::Op("%"),
                    '^' => Tok::Op("^"),
                    '<' => Tok::Op("<"),
                    '>' => Tok::Op(">"),
                    '!' => Tok::Op("!"),
                    '(' => Tok::LParen,
                    ')' => Tok::RParen,
                    ',' => Tok::Comma,
                    '?' => Tok::Question,
                    ':' => Tok::Colon,
                    other => return Err(err(format!("unexpected character '{other}'"), start, i)),
                }
            }
        };
        out.push(Token {
            tok,
            span: Span { start, end: i },
        });
    }
    out.push(Token {
        tok: Tok::Eof,
        span: Span {
            start: src.len(),
            end: src.len(),
        },
    });
    Ok(out)
}

fn next_is_digit(bytes: &[u8], i: usize) -> bool {
    bytes.get(i + 1).is_some_and(|b| b.is_ascii_digit())
}

fn err(message: impl Into<String>, start: usize, end: usize) -> ExprError {
    ExprError::new(message, Span { start, end })
}

/// Parses an expression. Fails on syntax errors, oversized input or excessive nesting.
pub fn parse(src: &str) -> Result<Expr, ExprError> {
    if src.len() > MAX_SOURCE_LEN {
        return Err(err("expression is too long", 0, src.len()));
    }
    let tokens = lex(src)?;
    let mut p = Parser { tokens, pos: 0 };
    let e = p.expr(0, 0)?;
    let t = p.peek();
    if t.tok != Tok::Eof {
        return Err(ExprError::new("unexpected token", t.span));
    }
    Ok(e)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

fn binary_op(op: &str) -> Option<(BinaryOp, u8, u8)> {
    // (operator, left binding power, right binding power); `^` is right-associative.
    Some(match op {
        "||" => (BinaryOp::Or, 2, 3),
        "&&" => (BinaryOp::And, 4, 5),
        "==" => (BinaryOp::Eq, 6, 7),
        "!=" => (BinaryOp::Ne, 6, 7),
        "<" => (BinaryOp::Lt, 8, 9),
        "<=" => (BinaryOp::Le, 8, 9),
        ">" => (BinaryOp::Gt, 8, 9),
        ">=" => (BinaryOp::Ge, 8, 9),
        "+" => (BinaryOp::Add, 10, 11),
        "-" => (BinaryOp::Sub, 10, 11),
        "*" => (BinaryOp::Mul, 12, 13),
        "/" => (BinaryOp::Div, 12, 13),
        "%" => (BinaryOp::Rem, 12, 13),
        "^" => (BinaryOp::Pow, 16, 15),
        _ => return None,
    })
}

const PREFIX_BP: u8 = 14;
const TERNARY_BP: u8 = 1;

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn next(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, want: Tok, what: &str) -> Result<Token, ExprError> {
        let t = self.next();
        if t.tok == want {
            Ok(t)
        } else {
            Err(ExprError::new(format!("expected {what}"), t.span))
        }
    }

    fn expr(&mut self, min_bp: u8, depth: usize) -> Result<Expr, ExprError> {
        if depth > MAX_DEPTH {
            return Err(ExprError::new(
                "expression is nested too deeply",
                self.peek().span,
            ));
        }
        let mut lhs = self.prefix(depth)?;
        loop {
            let t = self.peek().clone();
            match t.tok {
                Tok::Question if TERNARY_BP >= min_bp => {
                    self.next();
                    let then = self.expr(0, depth + 1)?;
                    self.expect(Tok::Colon, "':' in conditional")?;
                    let otherwise = self.expr(TERNARY_BP, depth + 1)?;
                    let span = Span {
                        start: lhs.span.start,
                        end: otherwise.span.end,
                    };
                    lhs = Expr {
                        kind: ExprKind::Cond(Box::new(lhs), Box::new(then), Box::new(otherwise)),
                        span,
                    };
                }
                Tok::Op(op) => {
                    let Some((bop, lbp, rbp)) = binary_op(op) else {
                        return Err(ExprError::new(format!("unexpected '{op}'"), t.span));
                    };
                    if lbp < min_bp {
                        break;
                    }
                    self.next();
                    let rhs = self.expr(rbp, depth + 1)?;
                    let span = Span {
                        start: lhs.span.start,
                        end: rhs.span.end,
                    };
                    lhs = Expr {
                        kind: ExprKind::Binary(bop, Box::new(lhs), Box::new(rhs)),
                        span,
                    };
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn prefix(&mut self, depth: usize) -> Result<Expr, ExprError> {
        let t = self.next();
        let kind = match t.tok {
            Tok::Num(n) => ExprKind::Num(n),
            Tok::Str(s) => ExprKind::Str(s),
            Tok::Ident(name) => match name.as_str() {
                "true" => ExprKind::Bool(true),
                "false" => ExprKind::Bool(false),
                _ if self.peek().tok == Tok::LParen => {
                    self.next();
                    let mut args = Vec::new();
                    if self.peek().tok != Tok::RParen {
                        loop {
                            args.push(self.expr(0, depth + 1)?);
                            if self.peek().tok == Tok::Comma {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    let close = self.expect(Tok::RParen, "')'")?;
                    return Ok(Expr {
                        kind: ExprKind::Call(name, args),
                        span: Span {
                            start: t.span.start,
                            end: close.span.end,
                        },
                    });
                }
                _ => ExprKind::Var(name),
            },
            Tok::LParen => {
                let inner = self.expr(0, depth + 1)?;
                let close = self.expect(Tok::RParen, "')'")?;
                return Ok(Expr {
                    kind: inner.kind,
                    span: Span {
                        start: t.span.start,
                        end: close.span.end,
                    },
                });
            }
            Tok::Op(op @ ("-" | "!")) => {
                let operand = self.expr(PREFIX_BP, depth + 1)?;
                let span = Span {
                    start: t.span.start,
                    end: operand.span.end,
                };
                let uop = if op == "-" {
                    UnaryOp::Neg
                } else {
                    UnaryOp::Not
                };
                return Ok(Expr {
                    kind: ExprKind::Unary(uop, Box::new(operand)),
                    span,
                });
            }
            Tok::Eof => return Err(ExprError::new("unexpected end of expression", t.span)),
            _ => return Err(ExprError::new("unexpected token", t.span)),
        };
        Ok(Expr { kind, span: t.span })
    }
}
