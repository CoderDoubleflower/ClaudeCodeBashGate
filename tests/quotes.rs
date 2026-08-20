mod common;

use bashgate::TooComplexReason;
use common::{argv, operators, parsed, too_complex};

#[test]
fn quoted_operators_are_literal_arguments() {
    let cases = [
        (r#"echo "a | b""#, vec!["echo", "a | b"]),
        ("echo 'a && b'", vec!["echo", "a && b"]),
        (r#"printf '%s\n' "a;b""#, vec!["printf", "%s\\n", "a;b"]),
        (r#"echo ">""#, vec!["echo", ">"]),
    ];

    for (source, expected) in cases {
        let program = parsed(source);
        assert_eq!(program.commands[0].argv, expected);
        assert!(operators(&program).is_empty());
        assert!(program.commands[0].redirects.is_empty());
    }
}

#[test]
fn resolves_supported_double_quote_escapes() {
    let program = parsed(r#"echo "a\"b\\c\q""#);
    assert_eq!(argv(&program), vec![vec!["echo", "a\"b\\c\\q"]]);
}

#[test]
fn rejects_runtime_expansion_in_or_outside_quotes() {
    for source in [
        "echo $UNTRUSTED",
        "echo ${FOO}",
        "echo $((1+2))",
        r#"echo "$(git status)""#,
        r#"echo "$HOME""#,
    ] {
        let error = too_complex(source);
        assert!(matches!(
            error.reason,
            TooComplexReason::DynamicExpansion | TooComplexReason::UnsupportedNode
        ));
    }
}

#[test]
fn rejects_unquoted_shell_expansions() {
    for source in ["echo *.txt", "echo file?.txt", "echo {a,b}", "echo ~/src"] {
        assert_eq!(too_complex(source).reason, TooComplexReason::DynamicExpansion);
    }
}

#[test]
fn rejects_unquoted_backslash_interpretation() {
    let error = too_complex(r"echo foo\|bar");
    assert_eq!(error.reason, TooComplexReason::DynamicExpansion);
}
