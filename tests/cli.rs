use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn parses_command_argument_as_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_bashgate"))
        .args(["parse", "git status && echo ok"])
        .output()
        .expect("run bashgate");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse stdout JSON");
    assert_eq!(value["kind"], "parsed");
    assert_eq!(value["operators"][0], "&&");
}

#[test]
fn parses_piped_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bashgate"))
        .arg("parse")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn bashgate");

    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(b"echo hello\n")
        .expect("write command");
    let output = child.wait_with_output().expect("wait for bashgate");

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse stdout JSON");
    assert_eq!(value["commands"][0]["argv"][0], "echo");
    assert_eq!(value["commands"][0]["argv"][1], "hello");
}

#[test]
fn invalid_subcommand_returns_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_bashgate"))
        .arg("unknown")
        .output()
        .expect("run bashgate");

    assert_eq!(output.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown subcommand"));
}
