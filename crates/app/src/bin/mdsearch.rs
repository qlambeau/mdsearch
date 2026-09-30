#![forbid(unsafe_code)]

//! Binary entry point for the `mdsearch` CLI.

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    match kv_app::run_from_environment(std::env::args_os()) {
        Ok(output) => {
            if output.is_empty() {
                return ExitCode::SUCCESS;
            }
            let mut stdout = io::stdout().lock();
            if writeln!(stdout, "{output}").is_err() {
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(kv_app::AppError::Arguments(error)) => {
            let exit_code = match error.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => 0,
                _ => 2,
            };
            if error.print().is_err() {
                return ExitCode::FAILURE;
            }
            ExitCode::from(exit_code)
        }
        Err(error) => {
            let mut stderr = io::stderr().lock();
            if writeln!(stderr, "{error}").is_err() {
                return ExitCode::FAILURE;
            }
            ExitCode::FAILURE
        }
    }
}
