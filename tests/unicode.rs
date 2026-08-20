mod common;

use bashgate::TooComplexReason;
use common::{parsed, too_complex};

#[test]
fn uses_utf8_byte_offsets() {
    let source = "echo café";
    let program = parsed(source);
    let command = &program.commands[0];

    assert_eq!(command.argv, ["echo", "café"]);
    assert_eq!(command.span.start_byte, 0);
    assert_eq!(command.span.end_byte, source.len());
    assert_eq!(source.len(), 10);
}

#[test]
fn accepts_visible_non_ascii_literal_text() {
    let source = "echo 你好";
    let program = parsed(source);
    assert_eq!(program.commands[0].argv, ["echo", "你好"]);
    assert_eq!(program.commands[0].span.end_byte, source.len());
}

#[test]
fn rejects_invisible_unicode_whitespace() {
    for character in ['\u{00A0}', '\u{200B}', '\u{2028}', '\u{FEFF}'] {
        let source = format!("echo{character}hidden");
        assert_eq!(
            too_complex(&source).reason,
            TooComplexReason::SuspiciousInput
        );
    }
}

#[test]
fn rejects_control_characters_and_parser_differentials() {
    for source in [
        "echo\0hidden",
        "echo\rhidden",
        "echo foo\\ bar",
        "tr\\\naceroute",
        "\\\necho ok",
        "echo \\\nok",
    ] {
        assert_eq!(
            too_complex(source).reason,
            TooComplexReason::SuspiciousInput,
            "source: {source:?}"
        );
    }
}
