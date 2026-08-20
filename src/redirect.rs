use tree_sitter::Node;

use crate::argument::parse_argument;
use crate::model::{Redirect, RedirectOp, TooComplex};
use crate::structure::source_slice;

pub(crate) fn parse_redirect(node: Node<'_>, source: &str) -> Result<Redirect, TooComplex> {
    match node.kind() {
        "heredoc_redirect" => {
            return Err(TooComplex::unsupported_with_detail(
                "heredoc_redirect",
                "heredoc bodies are recognized but not interpreted in the MVP",
            ));
        }
        "herestring_redirect" => {
            return Err(TooComplex::unsupported_with_detail(
                "herestring_redirect",
                "herestring expansion is not interpreted in the MVP",
            ));
        }
        "file_redirect" => {}
        other => return Err(TooComplex::unsupported(other)),
    }

    let mut fd = None;
    let mut op = None;
    let mut operator_start = None;
    let mut targets = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "file_descriptor" => {
                let text = source_slice(source, child.start_byte(), child.end_byte())?;
                fd = Some(parse_numeric_fd(text)?);
            }
            ">" => {
                op = Some(RedirectOp::Write);
                operator_start = Some(child.start_byte());
            }
            ">>" => {
                op = Some(RedirectOp::Append);
                operator_start = Some(child.start_byte());
            }
            "<" => {
                op = Some(RedirectOp::Read);
                operator_start = Some(child.start_byte());
            }
            ">&" => {
                op = Some(RedirectOp::DuplicateOutput);
                operator_start = Some(child.start_byte());
            }
            ">|" => {
                op = Some(RedirectOp::Clobber);
                operator_start = Some(child.start_byte());
            }
            "<&" => {
                op = Some(RedirectOp::DuplicateInput);
                operator_start = Some(child.start_byte());
            }
            "&>" => {
                op = Some(RedirectOp::WriteBoth);
                operator_start = Some(child.start_byte());
            }
            "&>>" => {
                op = Some(RedirectOp::AppendBoth);
                operator_start = Some(child.start_byte());
            }
            "<&-" | ">&-" => {
                return Err(TooComplex::unsupported_with_detail(
                    child.kind(),
                    "file-descriptor closing redirects are not supported in the MVP",
                ));
            }
            "comment" => {}
            _ if child.is_named() => targets.push(parse_argument(child, source)?),
            other => return Err(TooComplex::unsupported(other)),
        }
    }

    let op = op.ok_or_else(|| {
        TooComplex::invalid_structure("file_redirect has no recognized redirect operator")
    })?;
    let operator_start = operator_start.ok_or_else(|| {
        TooComplex::invalid_structure("file_redirect operator has no source position")
    })?;

    if fd.is_none() {
        let prefix = source_slice(source, node.start_byte(), operator_start)?;
        if !prefix.is_empty() {
            if !prefix.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(TooComplex::invalid_structure(format!(
                    "unexpected redirect prefix before operator: {prefix:?}"
                )));
            }
            fd = Some(parse_numeric_fd(prefix)?);
        }
    }

    if targets.len() != 1 {
        return Err(TooComplex::invalid_structure(format!(
            "redirect must have exactly one static target, found {}",
            targets.len()
        )));
    }

    Ok(Redirect {
        op,
        target: targets.remove(0),
        fd,
    })
}

fn parse_numeric_fd(text: &str) -> Result<u32, TooComplex> {
    text.parse::<u32>().map_err(|_| {
        TooComplex::dynamic(
            "file_descriptor",
            "only numeric file descriptors fitting in u32 are supported",
        )
    })
}
