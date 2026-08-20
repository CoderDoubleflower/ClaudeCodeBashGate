use std::ops::Range;

use tree_sitter::Node;

use crate::argument::{parse_argument, parse_env_assignment};
use crate::limits::ParseLimits;
use crate::model::{
    OperatorOccurrence, ParsedProgram, Redirect, ShellOperator, SimpleCommand, Span, TooComplex,
};
use crate::redirect::{parse_numeric_fd, parse_redirect};
use crate::structure::{node_span, operator_occurrence, source_slice};

pub(crate) fn walk_program(
    root: Node<'_>,
    source: &str,
    limits: ParseLimits,
) -> Result<ParsedProgram, TooComplex> {
    let mut walker = Walker {
        source,
        limits,
        commands: Vec::new(),
        operators: Vec::new(),
    };
    walker.walk_node(root)?;
    walker
        .commands
        .sort_by_key(|command| (command.span.start_byte, command.span.end_byte));
    add_implicit_newline_operators(source, &walker.commands, &mut walker.operators)?;
    walker
        .operators
        .sort_by_key(|operator| (operator.span.start_byte, operator.span.end_byte));

    Ok(ParsedProgram {
        source: source.to_owned(),
        commands: walker.commands,
        operators: walker.operators,
    })
}

fn add_implicit_newline_operators(
    source: &str,
    commands: &[SimpleCommand],
    operators: &mut Vec<OperatorOccurrence>,
) -> Result<(), TooComplex> {
    for pair in commands.windows(2) {
        let previous = &pair[0];
        let next = &pair[1];
        let gap_start = previous.span.end_byte;
        let gap_end = next.span.start_byte;

        if gap_start > gap_end {
            return Err(TooComplex::invalid_structure(format!(
                "overlapping command spans: {gap_start} exceeds {gap_end}"
            )));
        }
        if operators.iter().any(|operator| {
            operator.span.start_byte >= gap_start && operator.span.end_byte <= gap_end
        }) {
            continue;
        }

        let gap = source_slice(source, gap_start, gap_end)?;
        if let Some(relative_offset) = gap.as_bytes().iter().position(|byte| *byte == b'\n') {
            let start_byte = gap_start + relative_offset;
            operators.push(OperatorOccurrence {
                op: ShellOperator::Newline,
                span: Span {
                    start_byte,
                    end_byte: start_byte + 1,
                },
            });
        }
    }
    Ok(())
}

fn is_dynamic_expansion_node(kind: &str) -> bool {
    matches!(
        kind,
        "command_substitution"
            | "process_substitution"
            | "simple_expansion"
            | "expansion"
            | "arithmetic_expansion"
            | "brace_expression"
            | "extglob_pattern"
    )
}

fn dynamic_expansion_error(kind: &str) -> TooComplex {
    TooComplex::dynamic(
        kind,
        "runtime expansion cannot be represented as a trustworthy static argv entry",
    )
}

fn trailing_fd_candidate(
    body: Node<'_>,
    source: &str,
) -> Result<Option<(u32, String, usize)>, TooComplex> {
    if body.kind() != "command" || body.named_child_count() < 2 {
        return Ok(None);
    }

    let Some(last_child) = body.named_child(body.named_child_count() - 1) else {
        return Ok(None);
    };
    if !matches!(last_child.kind(), "word" | "number") {
        return Ok(None);
    }

    let raw = source_slice(source, last_child.start_byte(), last_child.end_byte())?;
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(None);
    }

    Ok(Some((
        parse_numeric_fd(raw)?,
        raw.to_owned(),
        last_child.end_byte(),
    )))
}

struct Walker<'a> {
    source: &'a str,
    limits: ParseLimits,
    commands: Vec<SimpleCommand>,
    operators: Vec<OperatorOccurrence>,
}

impl Walker<'_> {
    fn walk_node(&mut self, node: Node<'_>) -> Result<Range<usize>, TooComplex> {
        let start = self.commands.len();
        match node.kind() {
            "program" | "list" | "pipeline" => self.walk_structural(node)?,
            "redirected_statement" => self.walk_redirected_statement(node)?,
            "command" => self.walk_command(node)?,
            "declaration_command" => self.walk_declaration_command(node)?,
            "comment" => {}
            other if is_dynamic_expansion_node(other) => {
                return Err(dynamic_expansion_error(other));
            }
            other => return Err(TooComplex::unsupported(other)),
        }
        Ok(start..self.commands.len())
    }

    fn walk_structural(&mut self, node: Node<'_>) -> Result<(), TooComplex> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(operator) = operator_occurrence(child) {
                self.operators.push(operator);
                continue;
            }
            if child.kind() == "comment" {
                continue;
            }
            if !child.is_named() {
                return Err(TooComplex::unsupported(child.kind()));
            }
            self.walk_node(child)?;
        }
        Ok(())
    }

    fn walk_redirected_statement(&mut self, node: Node<'_>) -> Result<(), TooComplex> {
        let body = node.child_by_field_name("body").ok_or_else(|| {
            TooComplex::invalid_structure(
                "redirected_statement without a body is not supported in the MVP",
            )
        })?;

        let mut parsed_redirects: Vec<(Redirect, Span)> = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "file_redirect" | "heredoc_redirect" | "herestring_redirect" => {
                    parsed_redirects.push((parse_redirect(child, self.source)?, node_span(child)));
                }
                "comment" => {}
                _ => {}
            }
        }
        if parsed_redirects.is_empty() {
            return Err(TooComplex::invalid_structure(
                "redirected_statement has no redirect nodes",
            ));
        }

        let mut split_fd = trailing_fd_candidate(body, self.source)?;
        let range = self.walk_node(body)?;
        if range.is_empty() {
            return Err(TooComplex::invalid_structure(
                "redirected_statement body produced no simple command",
            ));
        }
        let last_index = range
            .clone()
            .max_by_key(|index| {
                let span = self.commands[*index].span;
                (span.start_byte, span.end_byte)
            })
            .ok_or_else(|| {
                TooComplex::invalid_structure("redirected_statement command range disappeared")
            })?;

        let command = &mut self.commands[last_index];
        let mut new_end = command.span.end_byte;
        for (mut redirect, span) in parsed_redirects {
            if redirect.fd.is_none()
                && split_fd
                    .as_ref()
                    .is_some_and(|(_, _, end_byte)| *end_byte == span.start_byte)
            {
                let (fd, raw, _) = split_fd.take().ok_or_else(|| {
                    TooComplex::invalid_structure("split redirect fd candidate disappeared")
                })?;
                if command.argv.last().map(String::as_str) != Some(raw.as_str()) {
                    return Err(TooComplex::invalid_structure(
                        "split redirect fd does not match the trailing parsed argument",
                    ));
                }
                command.argv.pop();
                redirect.fd = Some(fd);
            }
            new_end = new_end.max(span.end_byte);
            command.redirects.push(redirect);
        }
        command.span.end_byte = new_end;
        command.text = source_slice(self.source, command.span.start_byte, new_end)?.to_owned();
        Ok(())
    }

    fn walk_command(&mut self, node: Node<'_>) -> Result<(), TooComplex> {
        self.ensure_command_capacity()?;

        let mut argv = Vec::new();
        let mut env = Vec::new();
        let mut redirects = Vec::new();
        let mut seen_name = false;
        let mut trailing_number_end = None;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "variable_assignment" if !seen_name => {
                    env.push(parse_env_assignment(child, self.source)?);
                    trailing_number_end = None;
                }
                "variable_assignment" => {
                    return Err(TooComplex::invalid_structure(
                        "variable_assignment appeared after the command name",
                    ));
                }
                "command_name" if !seen_name => {
                    argv.push(parse_argument(child, self.source)?);
                    seen_name = true;
                    trailing_number_end = None;
                }
                "command_name" => {
                    return Err(TooComplex::invalid_structure(
                        "command contains more than one command_name node",
                    ));
                }
                "file_redirect" | "heredoc_redirect" | "herestring_redirect" => {
                    let mut redirect = parse_redirect(child, self.source)?;
                    if redirect.fd.is_none() && trailing_number_end == Some(child.start_byte()) {
                        let descriptor = argv.pop().ok_or_else(|| {
                            TooComplex::invalid_structure(
                                "adjacent redirect descriptor has no preceding argv entry",
                            )
                        })?;
                        redirect.fd = Some(parse_numeric_fd(&descriptor)?);
                    }
                    trailing_number_end = None;
                    redirects.push(redirect);
                }
                "word" | "string" | "raw_string" | "number" | "concatenation" if seen_name => {
                    let raw = source_slice(self.source, child.start_byte(), child.end_byte())?;
                    let is_unquoted_descriptor = matches!(child.kind(), "word" | "number")
                        && !raw.is_empty()
                        && raw.bytes().all(|byte| byte.is_ascii_digit());
                    argv.push(parse_argument(child, self.source)?);
                    trailing_number_end = is_unquoted_descriptor.then_some(child.end_byte());
                }
                "comment" => {
                    trailing_number_end = None;
                }
                other if is_dynamic_expansion_node(other) => {
                    return Err(dynamic_expansion_error(other));
                }
                other => return Err(TooComplex::unsupported(other)),
            }
        }

        if argv.is_empty() {
            return Err(TooComplex::invalid_structure(
                "command did not contain a statically resolvable executable",
            ));
        }

        let span = node_span(node);
        let text = source_slice(self.source, span.start_byte, span.end_byte)?.to_owned();
        self.commands.push(SimpleCommand {
            text,
            argv,
            env,
            redirects,
            span,
        });
        Ok(())
    }

    fn walk_declaration_command(&mut self, node: Node<'_>) -> Result<(), TooComplex> {
        self.ensure_command_capacity()?;

        let mut argv = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "declare" | "typeset" | "export" | "readonly" | "local" if argv.is_empty() => {
                    argv.push(child.kind().to_owned());
                }
                "variable_assignment" => {
                    let assignment = parse_env_assignment(child, self.source)?;
                    argv.push(format!("{}={}", assignment.name, assignment.value));
                }
                "variable_name" => {
                    argv.push(
                        source_slice(self.source, child.start_byte(), child.end_byte())?.to_owned(),
                    );
                }
                "word" | "string" | "raw_string" | "number" | "concatenation" => {
                    argv.push(parse_argument(child, self.source)?);
                }
                "comment" => {}
                other if is_dynamic_expansion_node(other) => {
                    return Err(dynamic_expansion_error(other));
                }
                other => return Err(TooComplex::unsupported(other)),
            }
        }

        if argv.is_empty() {
            return Err(TooComplex::invalid_structure(
                "declaration_command has no recognized builtin",
            ));
        }

        let span = node_span(node);
        self.commands.push(SimpleCommand {
            text: source_slice(self.source, span.start_byte, span.end_byte)?.to_owned(),
            argv,
            env: Vec::new(),
            redirects: Vec::new(),
            span,
        });
        Ok(())
    }

    fn ensure_command_capacity(&self) -> Result<(), TooComplex> {
        if self.commands.len() >= self.limits.max_commands {
            return Err(TooComplex::limit(
                "max_commands",
                format!(
                    "command count exceeds configured maximum of {}",
                    self.limits.max_commands
                ),
            ));
        }
        Ok(())
    }
}
