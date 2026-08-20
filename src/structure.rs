use tree_sitter::Node;

use crate::model::{OperatorOccurrence, ShellOperator, Span, TooComplex};

#[must_use]
pub(crate) fn node_span(node: Node<'_>) -> Span {
    Span {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    }
}

pub(crate) fn source_slice<'a>(
    source: &'a str,
    start_byte: usize,
    end_byte: usize,
) -> Result<&'a str, TooComplex> {
    source.get(start_byte..end_byte).ok_or_else(|| {
        TooComplex::invalid_structure(format!(
            "tree-sitter span {start_byte}..{end_byte} is not a UTF-8 boundary"
        ))
    })
}

#[must_use]
pub(crate) fn operator_occurrence(node: Node<'_>) -> Option<OperatorOccurrence> {
    let op = match node.kind() {
        "&&" => ShellOperator::And,
        "||" => ShellOperator::Or,
        "|" => ShellOperator::Pipe,
        "|&" => ShellOperator::PipeBoth,
        ";" => ShellOperator::Sequence,
        "&" => ShellOperator::Background,
        "\n" => ShellOperator::Newline,
        _ => return None,
    };
    Some(OperatorOccurrence {
        op,
        span: node_span(node),
    })
}
