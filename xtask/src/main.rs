#![forbid(unsafe_code)]

//! Repository automation entry point.

use std::env;
use std::fmt;
use std::process::{Command, ExitCode};

pub mod eval;

#[derive(Debug)]
struct CommandFailed {
    command: String,
}

impl fmt::Display for CommandFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "command failed: {}", self.command)
    }
}

impl std::error::Error for CommandFailed {}

fn main() -> ExitCode {
    main_with(run())
}

fn main_with(result: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            use std::io::Write;

            let mut stderr = std::io::stderr().lock();
            if writeln!(stderr, "{error}").is_err() {
                return ExitCode::FAILURE;
            }
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = |program: &str,
                      arguments: Vec<&'static str>,
                      environment: Option<(&'static str, &'static str)>|
     -> Result<(), Box<dyn std::error::Error>> {
        run_command(program, &arguments, environment)
    };

    run_with_args(env::args().skip(1), &mut runner)
}

fn run_with_args<I, F>(args: I, runner: &mut F) -> Result<(), Box<dyn std::error::Error>>
where
    I: IntoIterator<Item = String>,
    F: FnMut(
        &str,
        Vec<&'static str>,
        Option<(&'static str, &'static str)>,
    ) -> Result<(), Box<dyn std::error::Error>>,
{
    let mut arguments = args.into_iter();
    let command = arguments.next().unwrap_or_default();

    match command.as_str() {
        "ci" if arguments.next().is_none() => run_ci(runner),
        "eval" => {
            let options = eval::parse_eval_args(arguments)?;
            let mut stdout = std::io::stdout().lock();
            eval::run_eval(&options, &mut stdout)?;
            Ok(())
        }
        _ => Err("usage: cargo xtask <ci|eval>".into()),
    }
}

fn run_ci<F>(runner: &mut F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(
        &str,
        Vec<&'static str>,
        Option<(&'static str, &'static str)>,
    ) -> Result<(), Box<dyn std::error::Error>>,
{
    runner("cargo", vec!["fmt", "--all", "--", "--check"], None)?;
    runner(
        "cargo",
        vec![
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
        None,
    )?;
    runner("cargo", vec!["test", "--workspace", "--all-features"], None)?;
    runner(
        "cargo",
        vec!["doc", "--workspace", "--no-deps"],
        Some(("RUSTDOCFLAGS", "-D warnings")),
    )?;
    runner("cargo", vec!["deny", "check"], None)?;
    runner(
        "cargo",
        vec![
            "llvm-cov",
            "--workspace",
            "--fail-under-lines",
            "85",
            "--ignore-filename-regex",
            "vendor/sqlite-vector-rs",
        ],
        None,
    )?;

    Ok(())
}

fn run_command(
    program: &str,
    arguments: &[&str],
    environment: Option<(&str, &str)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new(program);
    command.args(arguments);

    if let Some((key, value)) = environment {
        command.env(key, value);
    }

    let status = command.status()?;

    if !status.success() {
        return Err(CommandFailed {
            command: format_command(program, arguments),
        }
        .into());
    }

    Ok(())
}

fn format_command(program: &str, arguments: &[&str]) -> String {
    std::iter::once(program)
        .chain(arguments.iter().copied())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::{format_command, main_with, run_with_args};

    #[test]
    fn rejects_unknown_commands() {
        let mut runner = |_program: &str,
                          _arguments: Vec<&'static str>,
                          _environment: Option<(&'static str, &'static str)>|
         -> Result<(), Box<dyn std::error::Error>> { Ok(()) };

        let result = run_with_args([String::from("unknown")], &mut runner);

        assert!(result.is_err());
    }

    #[test]
    fn runs_eval_subcommand() -> Result<(), Box<dyn std::error::Error>> {
        let mut runner = |_program: &str,
                          _arguments: Vec<&'static str>,
                          _environment: Option<(&'static str, &'static str)>|
         -> Result<(), Box<dyn std::error::Error>> { Ok(()) };

        run_with_args(
            [String::from("eval"), String::from("--verify-only")],
            &mut runner,
        )?;

        Ok(())
    }

    #[test]
    fn formats_a_command_for_failure_output() {
        assert_eq!(
            format_command("cargo", &["test", "--workspace"]),
            "cargo test --workspace"
        );
    }

    #[test]
    fn runs_every_ci_gate_in_order() -> Result<(), Box<dyn std::error::Error>> {
        let mut commands = Vec::new();
        let mut runner = |program: &str,
                          arguments: Vec<&'static str>,
                          environment: Option<(&'static str, &'static str)>|
         -> Result<(), Box<dyn std::error::Error>> {
            commands.push((
                format_command(program, &arguments),
                environment.map(|(key, value)| (key.to_owned(), value.to_owned())),
            ));
            Ok(())
        };

        run_with_args([String::from("ci")], &mut runner)?;

        assert_eq!(commands.len(), 6);
        assert_eq!(
            commands.first().map(|command| &command.0),
            Some(&String::from("cargo fmt --all -- --check"))
        );
        assert_eq!(
            commands.get(3).map(|command| &command.1),
            Some(&Some((
                String::from("RUSTDOCFLAGS"),
                String::from("-D warnings")
            )))
        );
        assert!(commands.get(5).is_some_and(|command| {
            command
                .0
                .contains("--ignore-filename-regex vendor/sqlite-vector-rs")
        }));

        Ok(())
    }

    #[test]
    fn accepts_a_successful_command() -> Result<(), Box<dyn std::error::Error>> {
        super::run_command("rustc", &["--version"], None)?;

        Ok(())
    }

    #[test]
    fn reports_a_failed_command() {
        let result = super::run_command("rustc", &["--this-flag-does-not-exist"], None);

        assert!(result.is_err());
    }

    #[test]
    fn maps_a_successful_run_to_success_exit_code() {
        assert_eq!(main_with(Ok(())), std::process::ExitCode::SUCCESS);
    }

    #[test]
    fn maps_a_failed_run_to_failure_exit_code() {
        let error = io::Error::other("test failure");

        assert_eq!(
            main_with(Err(error.into())),
            std::process::ExitCode::FAILURE
        );
    }
}
