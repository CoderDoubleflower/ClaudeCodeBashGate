mod common;

use common::{argv, operators, parsed};

#[test]
fn parses_multi_stage_pipeline() {
    let program = parsed("cat a | grep foo | head -10");

    assert_eq!(
        argv(&program),
        vec![vec!["cat", "a"], vec!["grep", "foo"], vec!["head", "-10"],]
    );
    assert_eq!(operators(&program), ["|", "|"]);
    assert_eq!(program.commands[0].text, "cat a");
    assert_eq!(program.commands[1].text, "grep foo");
    assert_eq!(program.commands[2].text, "head -10");
}

#[test]
fn parses_pipe_both() {
    let program = parsed("producer |& consumer");
    assert_eq!(argv(&program), vec![vec!["producer"], vec!["consumer"]]);
    assert_eq!(operators(&program), ["|&"]);
}

#[test]
fn trailing_redirect_belongs_to_last_pipeline_command() {
    let source = "cat a | grep x >> result.txt";
    let program = parsed(source);

    assert!(program.commands[0].redirects.is_empty());
    assert_eq!(program.commands[1].redirects[0].target, "result.txt");
    assert_eq!(program.commands[1].text, "grep x >> result.txt");
    assert_eq!(program.commands[1].span.end_byte, source.len());
}

#[test]
fn pipe_inside_quotes_does_not_create_pipeline() {
    let program = parsed(r#"echo "left | right""#);
    assert_eq!(program.commands.len(), 1);
    assert!(program.operators.is_empty());
}
