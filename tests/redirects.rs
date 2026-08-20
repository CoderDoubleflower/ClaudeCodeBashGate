mod common;

use std::fmt::Write as _;

use bashgate::{Redirect, RedirectOp, TooComplexReason};
use common::{parsed, too_complex};
use tree_sitter::{Language, Node, Parser};

#[test]
fn extracts_file_redirects() {
    let cases = [
        (
            "cat a > b",
            Redirect {
                op: RedirectOp::Write,
                target: "b".to_owned(),
                fd: None,
            },
        ),
        (
            "echo x 2>> error.log",
            Redirect {
                op: RedirectOp::Append,
                target: "error.log".to_owned(),
                fd: Some(2),
            },
        ),
        (
            "read x < input.txt",
            Redirect {
                op: RedirectOp::Read,
                target: "input.txt".to_owned(),
                fd: None,
            },
        ),
        (
            "echo x &> all.log",
            Redirect {
                op: RedirectOp::WriteBoth,
                target: "all.log".to_owned(),
                fd: None,
            },
        ),
        (
            "echo x &>> all.log",
            Redirect {
                op: RedirectOp::AppendBoth,
                target: "all.log".to_owned(),
                fd: None,
            },
        ),
        (
            "echo hi 2>&1",
            Redirect {
                op: RedirectOp::DuplicateOutput,
                target: "1".to_owned(),
                fd: Some(2),
            },
        ),
        (
            "cat 0<&3",
            Redirect {
                op: RedirectOp::DuplicateInput,
                target: "3".to_owned(),
                fd: Some(0),
            },
        ),
        (
            "echo hi >| output",
            Redirect {
                op: RedirectOp::Clobber,
                target: "output".to_owned(),
                fd: None,
            },
        ),
    ];

    for (source, expected) in cases {
        if source == "cat 0<&3" {
            eprintln!("AST for {source:?}:\n{}", dump_ast(source));
        }
        let program = parsed(source);
        assert_eq!(program.commands.len(), 1, "source: {source}");
        assert_eq!(
            program.commands[0].redirects,
            [expected],
            "source: {source}"
        );
        assert_eq!(program.commands[0].text, source);
        assert_eq!(program.commands[0].span.end_byte, source.len());
    }
}

fn dump_ast(source: &str) -> String {
    let mut parser = Parser::new();
    let language: Language = tree_sitter_bash::LANGUAGE.into();
    parser
        .set_language(&language)
        .expect("load tree-sitter-bash grammar");
    let tree = parser.parse(source, None).expect("parse debug command");
    let mut output = String::new();
    dump_node(tree.root_node(), source, 0, &mut output);
    output
}

fn dump_node(node: Node<'_>, source: &str, depth: usize, output: &mut String) {
    let text = source
        .get(node.start_byte()..node.end_byte())
        .unwrap_or("<invalid UTF-8 span>");
    writeln!(
        output,
        "{}{} named={} {}..{} {text:?}",
        "  ".repeat(depth),
        node.kind(),
        node.is_named(),
        node.start_byte(),
        node.end_byte()
    )
    .expect("write AST dump");

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        dump_node(child, source, depth + 1, output);
    }
}

#[test]
fn resolves_quoted_redirect_target_without_splitting() {
    let program = parsed(r#"cat input > "result file.txt""#);
    assert_eq!(program.commands[0].redirects[0].target, "result file.txt");
}

#[test]
fn quoted_redirect_symbols_are_not_redirects() {
    let program = parsed(r#"echo ">>""#);
    assert_eq!(program.commands[0].argv, ["echo", ">>"]);
    assert!(program.commands[0].redirects.is_empty());
}

#[test]
fn rejects_dynamic_redirect_targets_and_complex_redirect_forms() {
    for source in [
        r#"cat > "$OUT""#,
        "cat <<EOF\nbody\nEOF",
        "cat <<< input",
        "echo hi 2>&-",
    ] {
        let error = too_complex(source);
        assert!(matches!(
            error.reason,
            TooComplexReason::DynamicExpansion | TooComplexReason::UnsupportedNode
        ));
    }
}
