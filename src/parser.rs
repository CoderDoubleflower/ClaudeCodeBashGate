use std::panic::{catch_unwind, AssertUnwindSafe};

use thiserror::Error;
use tree_sitter::{Language, LanguageError, Node, Parser};

use crate::limits::ParseLimits;
use crate::model::{GateResult, ParseError, ParsedProgram, TooComplex};
use crate::walker::walk_program;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BashGate {
    limits: ParseLimits,
}

impl Default for BashGate {
    fn default() -> Self {
        Self {
            limits: ParseLimits::default(),
        }
    }
}

impl BashGate {
    #[must_use]
    pub const fn new(limits: ParseLimits) -> Self {
        Self { limits }
    }

    #[must_use]
    pub const fn limits(&self) -> ParseLimits {
        self.limits
    }

    #[must_use]
    pub fn parse(&self, source: &str) -> GateResult {
        if source.len() > self.limits.max_command_bytes {
            return GateResult::TooComplex(TooComplex::limit(
                "max_command_bytes",
                format!(
                    "command is {} bytes; configured maximum is {} bytes",
                    source.len(),
                    self.limits.max_command_bytes
                ),
            ));
        }
        if let Some(error) = differential_precheck(source) {
            return GateResult::TooComplex(error);
        }
        if source
            .chars()
            .all(|character| matches!(character, ' ' | '\t' | '\n'))
        {
            return GateResult::Parsed(ParsedProgram {
                source: source.to_owned(),
                commands: Vec::new(),
                operators: Vec::new(),
            });
        }

        let mut parser = match configured_parser(self.limits) {
            Ok(parser) => parser,
            Err(error) => {
                return GateResult::ParseError(ParseError {
                    message: error.to_string(),
                });
            }
        };

        let parsed = catch_unwind(AssertUnwindSafe(|| parser.parse(source.as_bytes(), None)));
        let tree = match parsed {
            Ok(Some(tree)) => tree,
            Ok(None) => {
                return GateResult::TooComplex(TooComplex::parser_aborted(
                    "tree-sitter parsing timed out or was cancelled",
                ));
            }
            Err(_) => {
                return GateResult::TooComplex(TooComplex::parser_aborted(
                    "tree-sitter panicked while parsing adversarial input",
                ));
            }
        };

        let root = tree.root_node();
        if let Err(error) = validate_tree(root, self.limits) {
            return GateResult::TooComplex(error);
        }

        match walk_program(root, source, self.limits) {
            Ok(program) => GateResult::Parsed(program),
            Err(error) => GateResult::TooComplex(error),
        }
    }
}

#[derive(Debug, Error)]
enum ParserSetupError {
    #[error("failed to load the tree-sitter-bash grammar: {0}")]
    Language(#[from] LanguageError),
}

fn configured_parser(limits: ParseLimits) -> Result<Parser, ParserSetupError> {
    let mut parser = Parser::new();
    let language: Language = tree_sitter_bash::LANGUAGE.into();
    parser.set_language(&language)?;
    #[allow(deprecated)]
    parser.set_timeout_micros(limits.effective_timeout_micros());
    Ok(parser)
}

fn validate_tree(root: Node<'_>, limits: ParseLimits) -> Result<(), TooComplex> {
    let max_nodes = limits.effective_max_nodes();
    let max_depth = limits.effective_max_depth();
    let mut stack = vec![(root, 0_usize)];
    let mut node_count = 0_usize;

    while let Some((node, depth)) = stack.pop() {
        node_count = node_count.saturating_add(1);
        if node_count > max_nodes {
            return Err(TooComplex::limit(
                "max_nodes",
                format!("AST node count exceeds configured maximum of {max_nodes}"),
            ));
        }
        if depth > max_depth {
            return Err(TooComplex::limit(
                "max_depth",
                format!("AST depth exceeds configured maximum of {max_depth}"),
            ));
        }
        if node.is_error() || node.is_missing() || node.kind() == "ERROR" {
            return Err(TooComplex::error_node(node.kind()));
        }

        for index in (0..node.child_count()).rev() {
            if let Some(child) = node.child(index) {
                stack.push((child, depth.saturating_add(1)));
            }
        }
    }
    Ok(())
}

fn differential_precheck(source: &str) -> Option<TooComplex> {
    if source.bytes().any(|byte| {
        matches!(
            byte,
            0x00..=0x08 | 0x0B..=0x1F | 0x7F
        )
    }) {
        return Some(TooComplex::suspicious(
            "contains NUL, carriage return, or another control character",
        ));
    }

    if source.chars().any(is_suspicious_unicode_whitespace) {
        return Some(TooComplex::suspicious(
            "contains invisible or non-ASCII Unicode whitespace",
        ));
    }

    let bytes = source.as_bytes();
    for index in 0..bytes.len().saturating_sub(1) {
        if bytes[index] != b'\\' {
            continue;
        }
        match bytes[index + 1] {
            b' ' | b'\t' => {
                return Some(TooComplex::suspicious(
                    "contains backslash-escaped horizontal whitespace",
                ));
            }
            b'\n' if index > 0 && !matches!(bytes[index - 1], b' ' | b'\t' | b'\n' | b'\\') => {
                return Some(TooComplex::suspicious(
                    "contains backslash-newline word joining",
                ));
            }
            _ => {}
        }
    }

    None
}

fn is_suspicious_unicode_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200B}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}
