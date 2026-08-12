#![forbid(unsafe_code)]

//! Binary entry point for the `mdsearch` CLI.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(home_directory) = std::env::var_os("HOME").map(PathBuf::from) else {
        let mut stderr = io::stderr().lock();
        if writeln!(stderr, "home directory is unavailable").is_err() {
            return ExitCode::FAILURE;
        }
        return ExitCode::FAILURE;
    };

    match kv_app::run(std::env::args_os(), &home_directory) {
        Ok(output) => {
            let mut stdout = io::stdout().lock();
            if writeln!(stdout, "{output}").is_err() {
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
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
