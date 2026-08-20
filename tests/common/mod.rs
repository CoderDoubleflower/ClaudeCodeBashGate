#![allow(dead_code)]

use bashgate::{GateResult, ParsedProgram, TooComplex};

pub fn parsed(source: &str) -> ParsedProgram {
    match bashgate::parse(source) {
        GateResult::Parsed(program) => program,
        other => panic!("expected Parsed for {source:?}, got {other:?}"),
    }
}

pub fn too_complex(source: &str) -> TooComplex {
    match bashgate::parse(source) {
        GateResult::TooComplex(error) => error,
        other => panic!("expected TooComplex for {source:?}, got {other:?}"),
    }
}

pub fn argv(program: &ParsedProgram) -> Vec<Vec<&str>> {
    program
        .commands
        .iter()
        .map(|command| command.argv.iter().map(String::as_str).collect())
        .collect()
}

pub fn operators(program: &ParsedProgram) -> Vec<&str> {
    program
        .operators
        .iter()
        .map(|operator| operator.op.as_str())
        .collect()
}
