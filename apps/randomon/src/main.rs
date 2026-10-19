#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bundle;
mod capabilities;
mod cli_handoff;
mod config;
mod debug;
mod engine;
mod game_picker;
mod keys;
mod options;
mod rom;
mod run;
mod section;
mod settings_files;
mod support;
mod ui;
mod updater;

fn main() -> std::process::ExitCode {
    let arguments = cli_handoff::command_line_arguments();
    if !arguments.is_empty()
        && let Some(cli) = cli_handoff::sibling_cli()
    {
        return cli_handoff::run_cli(cli, arguments);
    }
    env_logger::init();
    match app::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
