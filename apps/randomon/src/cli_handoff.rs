use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

const CLI_NAME: &str = "randomon-cli";

pub fn command_line_arguments() -> Vec<OsString> {
    std::env::args_os()
        .skip(1)
        .filter(|argument| !argument.to_string_lossy().starts_with("-psn_"))
        .collect()
}

pub fn sibling_cli() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let candidate = executable
        .parent()?
        .join(format!("{CLI_NAME}{}", std::env::consts::EXE_SUFFIX));
    candidate.is_file().then_some(candidate)
}

pub fn run_cli(cli: PathBuf, arguments: Vec<OsString>) -> ExitCode {
    match Command::new(&cli).args(arguments).status() {
        Ok(status) => status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .map_or(ExitCode::FAILURE, ExitCode::from),
        Err(error) => {
            eprintln!("Couldn't start {}: {error}", cli.display());
            ExitCode::FAILURE
        }
    }
}
