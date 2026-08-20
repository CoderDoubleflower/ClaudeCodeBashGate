use tree_sitter::Node;

use crate::model::{EnvAssignment, TooComplex};
use crate::structure::source_slice;

pub(crate) fn parse_argument(node: Node<'_>, source: &str) -> Result<String, TooComplex> {
    match node.kind() {
        "command_name" => parse_command_name(node, source),
        "word" => parse_unquoted_word(node, source),
        "string" => parse_double_quoted(node, source),
        "raw_string" => parse_single_quoted(node, source),
        "number" => parse_number(node, source),
        "concatenation" => parse_concatenation(node, source),
        "_empty_value" => Ok(String::new()),
        "command_substitution"
        | "process_substitution"
        | "simple_expansion"
        | "expansion"
        | "arithmetic_expansion"
        | "brace_expression"
        | "extglob_pattern" => Err(TooComplex::dynamic(
            node.kind(),
            "runtime expansion cannot be represented as a trustworthy static argv entry",
        )),
        other => Err(TooComplex::unsupported(other)),
    }
}

pub(crate) fn parse_env_assignment(
    node: Node<'_>,
    source: &str,
) -> Result<EnvAssignment, TooComplex> {
    if node.kind() != "variable_assignment" {
        return Err(TooComplex::unsupported(node.kind()));
    }

    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| TooComplex::invalid_structure("variable assignment has no name field"))?;
    if name_node.kind() != "variable_name" {
        return Err(TooComplex::unsupported_with_detail(
            name_node.kind(),
            "array/subscript assignments are not supported in the MVP",
        ));
    }
    let name = source_slice(source, name_node.start_byte(), name_node.end_byte())?;
    if !is_valid_identifier(name) {
        return Err(TooComplex::invalid_structure(format!(
            "invalid environment variable name: {name}"
        )));
    }

    let mut assignment_operator = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "=" | "+=" => assignment_operator = Some(child.kind()),
            _ => {}
        }
    }
    match assignment_operator {
        Some("=") => {}
        Some("+=") => {
            return Err(TooComplex::unsupported_with_detail(
                "variable_assignment",
                "append assignments (+=) are not supported in the MVP",
            ));
        }
        _ => {
            return Err(TooComplex::invalid_structure(
                "variable assignment has no recognized assignment operator",
            ));
        }
    }

    let value = if let Some(value_node) = node.child_by_field_name("value") {
        if value_node.start_byte() == value_node.end_byte() {
            String::new()
        } else {
            parse_argument(value_node, source)?
        }
    } else {
        let text = source_slice(source, node.start_byte(), node.end_byte())?;
        if text.ends_with('=') {
            String::new()
        } else {
            return Err(TooComplex::invalid_structure(
                "variable assignment has no value field",
            ));
        }
    };

    Ok(EnvAssignment {
        name: name.to_owned(),
        value,
    })
}

fn parse_command_name(node: Node<'_>, source: &str) -> Result<String, TooComplex> {
    let count = node.named_child_count();
    if count != 1 {
        return Err(TooComplex::invalid_structure(format!(
            "command_name must contain exactly one literal child, found {count}"
        )));
    }
    let child = node
        .named_child(0)
        .ok_or_else(|| TooComplex::invalid_structure("command_name child disappeared"))?;
    parse_argument(child, source)
}

fn parse_unquoted_word(node: Node<'_>, source: &str) -> Result<String, TooComplex> {
    if node.named_child_count() != 0 {
        return Err(TooComplex::dynamic(
            node.kind(),
            "word contains a nested expansion",
        ));
    }
    let text = source_slice(source, node.start_byte(), node.end_byte())?;

    if text.contains('\\') {
        return Err(TooComplex::dynamic(
            "word",
            "unquoted backslash escaping is intentionally rejected rather than guessed",
        ));
    }
    if text.starts_with('~') {
        return Err(TooComplex::dynamic(
            "word",
            "tilde expansion changes the runtime argv",
        ));
    }
    if text
        .chars()
        .any(|character| matches!(character, '*' | '?' | '[' | ']' | '{' | '}'))
    {
        return Err(TooComplex::dynamic(
            "word",
            "glob or brace expansion may change the number or value of runtime arguments",
        ));
    }

    Ok(text.to_owned())
}

fn parse_concatenation(node: Node<'_>, source: &str) -> Result<String, TooComplex> {
    let mut value = String::new();
    let mut saw_literal = false;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.start_byte() == child.end_byte() {
            continue;
        }
        if !child.is_named() {
            return Err(TooComplex::dynamic(
                child.kind(),
                "concatenation contains a non-literal shell token",
            ));
        }

        match child.kind() {
            "word" | "string" | "raw_string" | "number" | "concatenation" => {
                value.push_str(&parse_argument(child, source)?);
                saw_literal = true;
            }
            other => {
                return Err(TooComplex::dynamic(
                    other,
                    "concatenation contains a runtime expansion",
                ));
            }
        }
    }

    if !saw_literal {
        return Err(TooComplex::invalid_structure(
            "concatenation contains no statically resolvable literal components",
        ));
    }
    Ok(value)
}

fn parse_number(node: Node<'_>, source: &str) -> Result<String, TooComplex> {
    if node.named_child_count() != 0 {
        return Err(TooComplex::dynamic(
            "number",
            "numeric token contains a runtime expansion",
        ));
    }
    Ok(source_slice(source, node.start_byte(), node.end_byte())?.to_owned())
}

fn parse_single_quoted(node: Node<'_>, source: &str) -> Result<String, TooComplex> {
    let raw = source_slice(source, node.start_byte(), node.end_byte())?;
    if raw.len() < 2 || !raw.starts_with('\'') || !raw.ends_with('\'') {
        return Err(TooComplex::invalid_structure(
            "raw_string is missing single-quote delimiters",
        ));
    }
    Ok(raw[1..raw.len() - 1].to_owned())
}

fn parse_double_quoted(node: Node<'_>, source: &str) -> Result<String, TooComplex> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "string_content" {
            return Err(TooComplex::dynamic(
                child.kind(),
                "double-quoted string contains an expansion",
            ));
        }
    }

    let raw = source_slice(source, node.start_byte(), node.end_byte())?;
    if raw.len() < 2 || !raw.starts_with('"') || !raw.ends_with('"') {
        return Err(TooComplex::invalid_structure(
            "string is missing double-quote delimiters",
        ));
    }
    decode_double_quoted(&raw[1..raw.len() - 1])
}

fn decode_double_quoted(inner: &str) -> Result<String, TooComplex> {
    let mut output = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }

        let Some(next) = characters.next() else {
            return Err(TooComplex::invalid_structure(
                "double-quoted string ends with an incomplete escape",
            ));
        };
        match next {
            '$' | '`' | '"' | '\\' => output.push(next),
            '\n' | '\r' => {
                return Err(TooComplex::dynamic(
                    "string",
                    "backslash-newline joining is intentionally rejected",
                ));
            }
            other => {
                output.push('\\');
                output.push(other);
            }
        }
    }
    Ok(output)
}

fn is_valid_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(first == b'_' || first.is_ascii_alphabetic()) {
        return false;
    }
    bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
}
