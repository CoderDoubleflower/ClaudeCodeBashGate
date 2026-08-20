mod common;

use bashgate::TooComplexReason;
use common::{argv, operators, parsed, too_complex};

#[test]
fn parses_compound_operators() {
    let cases = [
        ("a && b", vec!["&&"]),
        ("a || b", vec!["||"]),
        ("a ; b", vec![";"]),
        ("a &", vec!["&"]),
        ("a\nb", vec!["\n"]),
    ];

    for (source, expected_operators) in cases {
        let program = parsed(source);
        assert_eq!(program.commands.len(), source.matches('b').count() + 1);
        assert_eq!(operators(&program), expected_operators, "source: {source}");
    }
}

#[test]
fn mixed_structures_are_sorted_by_source_position() {
    let cases = [
        (
            "a | b && c | d",
            vec![vec!["a"], vec!["b"], vec!["c"], vec!["d"]],
            vec!["|", "&&", "|"],
        ),
        (
            "a && b || c",
            vec![vec!["a"], vec!["b"], vec!["c"]],
            vec!["&&", "||"],
        ),
        (
            "a ; b | c",
            vec![vec!["a"], vec!["b"], vec!["c"]],
            vec![";", "|"],
        ),
    ];

    for (source, expected_argv, expected_operators) in cases {
        let program = parsed(source);
        assert_eq!(argv(&program), expected_argv, "source: {source}");
        assert_eq!(operators(&program), expected_operators, "source: {source}");
    }
}

#[test]
fn comments_do_not_become_commands() {
    let program = parsed("a # note\nb");
    assert_eq!(argv(&program), vec![vec!["a"], vec!["b"]]);
    assert_eq!(operators(&program), ["\n"]);
}

#[test]
fn rejects_control_flow_and_other_non_mvp_statements() {
    let cases = [
        ("for x in a; do echo x; done", "for_statement"),
        ("while true; do echo x; done", "while_statement"),
        ("if true; then echo x; fi", "if_statement"),
        ("case x in x) echo y;; esac", "case_statement"),
        ("f() { echo x; }", "function_definition"),
        ("(echo x)", "subshell"),
        ("[ -f file ]", "test_command"),
    ];

    for (source, node_type) in cases {
        let error = too_complex(source);
        assert_eq!(error.reason, TooComplexReason::UnsupportedNode);
        assert_eq!(error.node_type.as_deref(), Some(node_type));
    }
}
