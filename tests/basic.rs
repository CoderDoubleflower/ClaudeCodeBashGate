mod common;

use common::{argv, parsed};

#[test]
fn parses_basic_commands() {
    let cases = [
        ("echo hello", vec!["echo", "hello"]),
        ("git status", vec!["git", "status"]),
        ("cat 'a b.txt'", vec!["cat", "a b.txt"]),
    ];

    for (source, expected) in cases {
        let program = parsed(source);
        assert_eq!(program.commands.len(), 1);
        assert_eq!(program.commands[0].argv, expected);
        assert_eq!(program.commands[0].text, source);
        assert_eq!(program.commands[0].span.start_byte, 0);
        assert_eq!(program.commands[0].span.end_byte, source.len());
    }
}

#[test]
fn preserves_original_source_and_trims_command_span_only_via_ast() {
    let source = "  echo hello  ";
    let program = parsed(source);

    assert_eq!(program.source, source);
    assert_eq!(program.commands[0].text, "echo hello");
    assert_eq!(program.commands[0].span.start_byte, 2);
    assert_eq!(program.commands[0].span.end_byte, 12);
}

#[test]
fn parses_static_literal_concatenation() {
    let program = parsed(r#"echo pre"mid"'post'"#);
    assert_eq!(argv(&program), vec![vec!["echo", "premidpost"]]);
}

#[test]
fn parses_empty_and_whitespace_only_input() {
    for source in ["", "   ", "\t\n"] {
        let program = parsed(source);
        assert!(program.commands.is_empty());
        assert!(program.operators.is_empty());
        assert_eq!(program.source, source);
    }
}
