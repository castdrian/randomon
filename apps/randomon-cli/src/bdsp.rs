use std::path::Path;

use randomon_bdsp::{
    BdspSettings, OutputLayout, detect_games, looks_like_bdsp_romfs, randomize_romfs,
};
use randomon_java::RandomSource;

pub struct BdspArguments<'a> {
    pub romfs: &'a Path,
    pub output: &'a Path,
    pub settings_string: Option<&'a str>,
    pub seed: Option<i64>,
    pub layout: Option<&'a str>,
}

pub fn is_bdsp_input(input: &Path) -> bool {
    input.is_dir() && looks_like_bdsp_romfs(input)
}

fn layout_from(text: Option<&str>) -> Result<OutputLayout, String> {
    match text {
        None => Ok(OutputLayout::default()),
        Some(text) => OutputLayout::parse(text).ok_or_else(|| {
            format!("Unknown layout {text:?}; use atmosphere, yuzu, ryujinx or emulator.")
        }),
    }
}

pub fn run(arguments: &BdspArguments<'_>) -> Result<(), String> {
    let settings = match arguments.settings_string {
        Some(text) => {
            BdspSettings::from_settings_string(text).map_err(|error| error.to_string())?
        }
        None => BdspSettings::default(),
    };
    let layout = layout_from(arguments.layout)?;
    let seed = arguments.seed.unwrap_or_else(RandomSource::pick_seed) as u64;
    let games = detect_games(arguments.romfs);
    let randomized =
        randomize_romfs(arguments.romfs, &settings, seed).map_err(|error| error.to_string())?;
    let written = randomized
        .write_to(arguments.output, layout, &games)
        .map_err(|error| error.to_string())?;
    println!("Seed: {seed}");
    println!("Settings: {}", settings.to_settings_string());
    println!("Layout: {}", layout.label());
    println!(
        "Wrote {} files to {}",
        written.len(),
        arguments.output.display()
    );
    Ok(())
}
