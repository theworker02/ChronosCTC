use ctc_dag::Epoch;
use serde::{Deserialize, Serialize};

/// Chronal register declaration: `chronal reg name @ τk;`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegisterDecl {
    pub name: String,
    pub epoch: Epoch,
}

/// Expression language for evolve equations (affine + product closed form).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Const(f64),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
}

/// `evolve name = expr;`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvolveEq {
    pub target: String,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Reg(RegisterDecl),
    /// Explicit retrocausal wire: output of `from` feeds input of `to`.
    Retrocausal { from: String, to: String },
    Evolve(EvolveEq),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChronalModule {
    pub name: String,
    pub stmts: Vec<Stmt>,
}

impl ChronalModule {
    pub fn registers(&self) -> Vec<&RegisterDecl> {
        self.stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::Reg(r) => Some(r),
                _ => None,
            })
            .collect()
    }

    pub fn evolve_eqs(&self) -> Vec<&EvolveEq> {
        self.stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::Evolve(e) => Some(e),
                _ => None,
            })
            .collect()
    }

    pub fn retrocausal_edges(&self) -> Vec<(&str, &str)> {
        self.stmts
            .iter()
            .filter_map(|s| match s {
                Stmt::Retrocausal { from, to } => Some((from.as_str(), to.as_str())),
                _ => None,
            })
            .collect()
    }
}
