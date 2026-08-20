mod common;

use bashgate::{Redirect, RedirectOp, TooComplexReason};
use common::{parsed, too_complex};

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
        let program = parsed(source);
        assert_eq!(program.commands.len(), 1, "source: {source}");
        assert_eq!(program.commands[0].redirects, [expected], "source: {source}");
        assert_eq!(program.commands[0].text, source);
        assert_eq!(program.commands[0].span.end_byte, source.len());
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
