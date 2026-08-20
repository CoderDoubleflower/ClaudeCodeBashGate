mod common;

use bashgate::TooComplexReason;
use common::{parsed, too_complex};

#[test]
fn extracts_leading_environment_assignments() {
    let program = parsed(r#"FOO=bar BAR="hello world" node app.js"#);
    let command = &program.commands[0];

    assert_eq!(command.argv, ["node", "app.js"]);
    assert_eq!(command.env.len(), 2);
    assert_eq!(command.env[0].name, "FOO");
    assert_eq!(command.env[0].value, "bar");
    assert_eq!(command.env[1].name, "BAR");
    assert_eq!(command.env[1].value, "hello world");
}

#[test]
fn supports_empty_assignment_values() {
    let program = parsed("EMPTY= env");
    assert_eq!(program.commands[0].env[0].name, "EMPTY");
    assert_eq!(program.commands[0].env[0].value, "");
    assert_eq!(program.commands[0].argv, ["env"]);
}

#[test]
fn redirects_before_the_executable_do_not_become_argv_or_env() {
    let program = parsed("FOO=bar 2>error.log command arg");
    let command = &program.commands[0];
    assert_eq!(command.env[0].name, "FOO");
    assert_eq!(command.argv, ["command", "arg"]);
    assert_eq!(command.redirects.len(), 1);
}

#[test]
fn rejects_dynamic_append_and_array_assignments() {
    let cases = [
        "FOO=$BAR command",
        "FOO=$(date) command",
        "FOO+=bar command",
        "ARR=(a b) command",
    ];

    for source in cases {
        let error = too_complex(source);
        assert!(matches!(
            error.reason,
            TooComplexReason::DynamicExpansion | TooComplexReason::UnsupportedNode
        ));
    }
}

#[test]
fn parses_declaration_commands_as_commands_not_environment_prefixes() {
    let program = parsed(r#"export FOO="hello world""#);
    assert_eq!(program.commands[0].argv, ["export", "FOO=hello world"]);
    assert!(program.commands[0].env.is_empty());
}
