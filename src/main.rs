#![forbid(unsafe_code)]

use std::env;
use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

use thiserror::Error;

use bashgate::BashGate;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("bashgate: {error}");
            ExitCode::from(64)
        }
    }
}

fn run() -> Result<(), CliError> {
    let mut arguments = env::args().skip(1);
    let Some(subcommand) = arguments.next() else {
        return Err(CliError::Usage(usage()));
    };

    match subcommand.as_str() {
        "-h" | "--help" | "help" => {
            println!("{}", usage());
            Ok(())
        }
        "-V" | "--version" | "version" => {
            println!("bashgate {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "parse" => {
            let remaining: Vec<String> = arguments.collect();
            let source = if remaining.is_empty() {
                read_command_from_stdin()?
            } else {
                remaining.join(" ")
            };
            let result = BashGate::default().parse(&source);
            let stdout = io::stdout();
            let mut output = stdout.lock();
            serde_json::to_writer_pretty(&mut output, &result)?;
            writeln!(output)?;
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "unknown subcommand: {other}\n\n{}",
            usage()
        ))),
    }
}

fn read_command_from_stdin() -> Result<String, CliError> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        return Err(CliError::Usage(format!(
            "parse requires a command argument or piped stdin\n\n{}",
            usage()
        )));
    }

    let mut input = String::new();
    stdin.lock().read_to_string(&mut input)?;
    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }
    Ok(input)
}

fn usage() -> String {
    "Usage:\n  bashgate parse 'git status && echo ok'\n  echo 'git status && echo ok' | bashgate parse"
        .to_owned()
}

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Usage(String),
    #[error("failed to read or write data: {0}")]
    Io(#[from] io::Error),
    #[error("failed to serialize JSON: {0}")]
    Json(#[from] serde_json::Error),
}
