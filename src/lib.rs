#![forbid(unsafe_code)]

mod argument;
mod parser;
mod redirect;
mod structure;
mod walker;

pub mod limits;
pub mod model;

pub use limits::ParseLimits;
pub use model::{
    EnvAssignment, GateResult, OperatorOccurrence, ParseError, ParsedProgram, Redirect,
    RedirectOp, ShellOperator, SimpleCommand, Span, TooComplex, TooComplexReason,
};
pub use parser::BashGate;

#[must_use]
pub fn parse(source: &str) -> GateResult {
    BashGate::default().parse(source)
}
