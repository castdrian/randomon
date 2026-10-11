#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bdsp;
mod bundle;
mod capabilities;
mod debug;
mod engine;
mod game_picker;
mod options;
mod rom;
mod section;
mod settings_files;
mod support;
mod ui;

fn main() -> iced::Result {
    env_logger::init();
    app::run()
}
