use crate::ast::{ChronalModule, EvolveEq, Expr, RegisterDecl, Stmt};
use crate::error::{CompileError, CompileResult};
use ctc_dag::Epoch;

/// Parse a chronal module from DSL source.
///
/// Grammar (whitespace-insensitive, `;`-terminated statements):
///
/// ```text
/// module    ::= stmt*
/// stmt      ::= reg | retro | evolve
/// reg       ::= 'chronal' 'reg' IDENT '@' 'τ' INT ';'
///             | 'chronal' 'reg' IDENT '@' 'tau' INT ';'
/// retro     ::= 'retrocausal' '{' IDENT '->' IDENT '}' ';'
/// evolve    ::= 'evolve' IDENT '=' expr ';'
/// expr      ::= term (('+'|'-') term)*
/// term      ::= factor ('*' factor)*
/// factor    ::= '-' factor | IDENT | NUMBER | '(' expr ')'
/// ```
pub fn parse_module(name: &str, source: &str) -> CompileResult<ChronalModule> {
    let mut stmts = Vec::new();
    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw_line).trim().to_string();
        if line.is_empty() {
            continue;
        }
        if !line.ends_with(';') {
            return Err(CompileError::Parse {
                line: line_no,
                message: "statements must end with ';'".into(),
            });
        }
        let line = line.trim_end_matches(';').trim();
        let stmt = parse_stmt(line, line_no)?;
        stmts.push(stmt);
    }
    Ok(ChronalModule {
        name: name.into(),
        stmts,
    })
}

fn strip_comment(line: &str) -> &str {
    line.split("//").next().unwrap_or(line)
}

fn parse_stmt(line: &str, line_no: usize) -> CompileResult<Stmt> {
    if line.starts_with("chronal reg ") {
        return parse_reg(&line["chronal reg ".len()..], line_no).map(Stmt::Reg);
    }
    if line.starts_with("retrocausal") {
        return parse_retro(line, line_no);
    }
    if line.starts_with("evolve ") {
        return parse_evolve(&line["evolve ".len()..], line_no).map(Stmt::Evolve);
    }
    Err(CompileError::Parse {
        line: line_no,
        message: format!("unrecognized statement: {line}"),
    })
}

fn parse_reg(rest: &str, line_no: usize) -> CompileResult<RegisterDecl> {
    // name @ τN  or  name @ tauN
    let parts: Vec<&str> = rest.split('@').map(str::trim).collect();
    if parts.len() != 2 {
        return Err(CompileError::Parse {
            line: line_no,
            message: "expected `chronal reg NAME @ τN`".into(),
        });
    }
    let name = parts[0].to_string();
    if name.is_empty() || !is_ident(&name) {
        return Err(CompileError::Parse {
            line: line_no,
            message: format!("invalid register name `{name}`"),
        });
    }
    let tau_tok = parts[1].trim();
    let tau_str = tau_tok
        .strip_prefix('τ')
        .or_else(|| tau_tok.strip_prefix("tau"))
        .ok_or_else(|| CompileError::Parse {
            line: line_no,
            message: "epoch must look like τN or tauN".into(),
        })?;
    let tau: i64 = tau_str.parse().map_err(|_| CompileError::Parse {
        line: line_no,
        message: format!("invalid epoch `{tau_tok}`"),
    })?;
    Ok(RegisterDecl {
        name,
        epoch: Epoch(tau),
    })
}

fn parse_retro(line: &str, line_no: usize) -> CompileResult<Stmt> {
    // retrocausal { a -> b }
    let inner = line
        .strip_prefix("retrocausal")
        .unwrap()
        .trim()
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .map(str::trim)
        .ok_or_else(|| CompileError::Parse {
            line: line_no,
            message: "expected `retrocausal { a -> b }`".into(),
        })?;
    let parts: Vec<&str> = inner.split("->").map(str::trim).collect();
    if parts.len() != 2 || !is_ident(parts[0]) || !is_ident(parts[1]) {
        return Err(CompileError::Parse {
            line: line_no,
            message: "retrocausal edge must be `IDENT -> IDENT`".into(),
        });
    }
    Ok(Stmt::Retrocausal {
        from: parts[0].to_string(),
        to: parts[1].to_string(),
    })
}

fn parse_evolve(rest: &str, line_no: usize) -> CompileResult<EvolveEq> {
    let parts: Vec<&str> = rest.splitn(2, '=').map(str::trim).collect();
    if parts.len() != 2 || !is_ident(parts[0]) {
        return Err(CompileError::Parse {
            line: line_no,
            message: "expected `evolve NAME = EXPR`".into(),
        });
    }
    let body = parse_expr(parts[1], line_no)?;
    Ok(EvolveEq {
        target: parts[0].to_string(),
        body,
    })
}

fn parse_expr(input: &str, line_no: usize) -> CompileResult<Expr> {
    let tokens = tokenize(input, line_no)?;
    let mut pos = 0;
    let expr = parse_add(&tokens, &mut pos, line_no)?;
    if pos != tokens.len() {
        return Err(CompileError::Parse {
            line: line_no,
            message: format!("trailing tokens in expression near `{:?}`", tokens[pos]),
        });
    }
    Ok(expr)
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Num(f64),
    Plus,
    Minus,
    Star,
    LParen,
    RParen,
}

fn tokenize(input: &str, line_no: usize) -> CompileResult<Vec<Tok>> {
    let mut chars = input.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' => {
                chars.next();
            }
            '+' => {
                chars.next();
                tokens.push(Tok::Plus);
            }
            '-' => {
                chars.next();
                tokens.push(Tok::Minus);
            }
            '*' => {
                chars.next();
                tokens.push(Tok::Star);
            }
            '(' => {
                chars.next();
                tokens.push(Tok::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Tok::RParen);
            }
            '0'..='9' | '.' => {
                let mut s = String::new();
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() || d == '.' {
                        s.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let n: f64 = s.parse().map_err(|_| CompileError::Parse {
                    line: line_no,
                    message: format!("invalid number `{s}`"),
                })?;
                tokens.push(Tok::Num(n));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut s = String::new();
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_alphanumeric() || d == '_' {
                        s.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Tok::Ident(s));
            }
            other => {
                return Err(CompileError::Parse {
                    line: line_no,
                    message: format!("unexpected character `{other}`"),
                });
            }
        }
    }
    Ok(tokens)
}

fn parse_add(tokens: &[Tok], pos: &mut usize, line_no: usize) -> CompileResult<Expr> {
    let mut lhs = parse_term(tokens, pos, line_no)?;
    while let Some(tok) = tokens.get(*pos) {
        match tok {
            Tok::Plus => {
                *pos += 1;
                let rhs = parse_term(tokens, pos, line_no)?;
                lhs = Expr::Add(Box::new(lhs), Box::new(rhs));
            }
            Tok::Minus => {
                *pos += 1;
                let rhs = parse_term(tokens, pos, line_no)?;
                lhs = Expr::Sub(Box::new(lhs), Box::new(rhs));
            }
            _ => break,
        }
    }
    Ok(lhs)
}

fn parse_term(tokens: &[Tok], pos: &mut usize, line_no: usize) -> CompileResult<Expr> {
    let mut lhs = parse_factor(tokens, pos, line_no)?;
    while let Some(Tok::Star) = tokens.get(*pos) {
        *pos += 1;
        let rhs = parse_factor(tokens, pos, line_no)?;
        lhs = Expr::Mul(Box::new(lhs), Box::new(rhs));
    }
    Ok(lhs)
}

fn parse_factor(tokens: &[Tok], pos: &mut usize, line_no: usize) -> CompileResult<Expr> {
    match tokens.get(*pos) {
        Some(Tok::Minus) => {
            *pos += 1;
            let inner = parse_factor(tokens, pos, line_no)?;
            Ok(Expr::Neg(Box::new(inner)))
        }
        Some(Tok::Num(n)) => {
            *pos += 1;
            Ok(Expr::Const(*n))
        }
        Some(Tok::Ident(name)) => {
            *pos += 1;
            Ok(Expr::Var(name.clone()))
        }
        Some(Tok::LParen) => {
            *pos += 1;
            let inner = parse_add(tokens, pos, line_no)?;
            match tokens.get(*pos) {
                Some(Tok::RParen) => {
                    *pos += 1;
                    Ok(inner)
                }
                _ => Err(CompileError::Parse {
                    line: line_no,
                    message: "unbalanced '(' in expression".into(),
                }),
            }
        }
        _ => Err(CompileError::Parse {
            line: line_no,
            message: "expected expression factor".into(),
        }),
    }
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_retrocausal_module() {
        let src = r#"
            chronal reg x @ τ0;
            chronal reg y @ tau1;
            retrocausal { y -> x };
            evolve x = 0.5 * x + 0.25 * y;
            evolve y = 0.5 * y + 0.25 * x + 0.5;
        "#;
        let m = parse_module("demo", src).unwrap();
        assert_eq!(m.registers().len(), 2);
        assert_eq!(m.retrocausal_edges(), vec![("y", "x")]);
        assert_eq!(m.evolve_eqs().len(), 2);
    }
}
