mod bdsp;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Mutex;

use clap::Parser;
use randomon_core::{
    HandlerRegistry, ProgressReporter, RandomizationError, RandomizationOptions, RomOutput,
    write_directory_output,
};
use randomon_gen1::Gen1Factory;
use randomon_gen2::Gen2Factory;
use randomon_gen3::Gen3Factory;
use randomon_gen4::Gen4Factory;
use randomon_gen5::Gen5Factory;
use randomon_gen6::Gen6Factory;
use randomon_gen7::Gen7Factory;
use randomon_java::RandomSource;
use randomon_settings::Settings;

const UTF8_BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];
const ROM_EXTENSIONS: [&str; 5] = ["sgb", "gbc", "gba", "nds", "cxi"];

#[derive(Parser, Debug)]
#[command(
    name = "randomon-cli",
    version,
    about = "Randomize Pokémon games with settings compatible with Universal Pokemon Randomizer ZX"
)]
struct Arguments {
    #[arg(short = 'i', value_name = "SOURCE_ROM", help = "ROM to randomize")]
    input: PathBuf,

    #[arg(
        short = 'o',
        value_name = "OUTPUT",
        help = "Where to write the randomized ROM or LayeredFS folder"
    )]
    output: PathBuf,

    #[arg(
        short = 's',
        value_name = "SETTINGS_FILE",
        conflicts_with = "settings_string",
        help = "UPR ZX quick settings file (.rnqs)"
    )]
    settings_file: Option<PathBuf>,

    #[arg(
        long,
        value_name = "SETTINGS_STRING",
        help = "UPR ZX settings string, with its version prefix"
    )]
    settings_string: Option<String>,

    #[arg(
        long,
        value_name = "SEED",
        help = "Seed to use instead of a random one"
    )]
    seed: Option<i64>,

    #[arg(short = 'd', help = "Save 3DS output as a LayeredFS folder")]
    save_as_directory: bool,

    #[arg(
        short = 'u',
        value_name = "UPDATE_FILE",
        help = "3DS game update (.cia) to apply first; forces LayeredFS output"
    )]
    game_update: Option<PathBuf>,

    #[arg(
        long = "aes-keys",
        value_name = "AES_KEYS_FILE",
        help = "aes_keys.txt (or boot9.bin) used to decrypt encrypted 3DS ROMs and updates"
    )]
    aes_keys: Option<PathBuf>,

    #[arg(short = 'l', help = "Also write the randomization log")]
    save_log: bool,

    #[arg(
        long,
        value_name = "BDSP_SETTINGS_STRING",
        help = "Brilliant Diamond/Shining Pearl settings string (input is a romfs folder)"
    )]
    bdsp_settings_string: Option<String>,

    #[arg(
        long,
        value_name = "LAYOUT",
        help = "BDSP output layout: atmosphere, yuzu, ryujinx or emulator"
    )]
    layout: Option<String>,
}

fn main() -> ExitCode {
    match run(Arguments::parse()) {
        Ok(()) => {
            println!("Randomized successfully!");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("ERROR: {message}");
            ExitCode::FAILURE
        }
    }
}

fn registry() -> HandlerRegistry {
    HandlerRegistry::new()
        .with(Gen1Factory)
        .with(Gen2Factory)
        .with(Gen4Factory)
        .with(Gen3Factory)
        .with(Gen5Factory)
        .with(Gen6Factory)
        .with(Gen7Factory)
}

fn is_3ds(generation: i32) -> bool {
    matches!(generation, 6 | 7)
}

fn output_mode(arguments: &Arguments, generation: i32) -> bool {
    let mut save_as_directory = arguments.save_as_directory;
    if arguments.game_update.is_some() && is_3ds(generation) {
        if !save_as_directory {
            eprintln!("WARNING: Forcing save as directory since a game update was supplied.");
        }
        save_as_directory = true;
    }
    if save_as_directory && !is_3ds(generation) {
        save_as_directory = false;
        eprintln!(
            "WARNING: Saving as directory does not make sense for non-3DS games, ignoring \"-d\" flag..."
        );
    }
    save_as_directory
}

fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

fn run(arguments: Arguments) -> Result<(), String> {
    if bdsp::is_bdsp_input(&arguments.input) {
        return bdsp::run(&bdsp::BdspArguments {
            romfs: &arguments.input,
            output: &arguments.output,
            settings_string: arguments.bdsp_settings_string.as_deref(),
            seed: arguments.seed,
            layout: arguments.layout.as_deref(),
        });
    }
    if let Some(keys) = &arguments.aes_keys {
        randomon_formats::ctr::crypto::set_aes_keys_path(Some(absolute(keys)));
    }
    let settings = load_settings(&arguments)?;
    let registry = registry();
    let input = absolute(&arguments.input);
    let input_name = input.to_string_lossy().into_owned();
    let game = registry
        .game_info_file(&input, &input_name)
        .map_err(|error| match error {
            RandomizationError::Randomization(message) | RandomizationError::Io(message)
                if registry.detect_file(&input).is_some() =>
            {
                message
            }
            _ => format!(
                "Could not load {} - it is not a supported ROM yet.",
                file_name_of(&input)
            ),
        })?;
    let save_as_directory = output_mode(&arguments, game.generation);
    let output = if save_as_directory {
        arguments.output.clone()
    } else {
        fix_filename(&arguments.output, &game.default_extension)
    };
    if !save_as_directory && game.generation >= 4 && absolute(&output) == input {
        return Err(
            "You cannot overwrite the original ROM when you save a DS randomization.\nPlease choose a different filename."
                .to_owned(),
        );
    }
    let seed = arguments.seed.unwrap_or_else(RandomSource::pick_seed);
    let game_update = arguments
        .game_update
        .clone()
        .filter(|_| is_3ds(game.generation));
    let options = RandomizationOptions {
        file_name: input_name,
        save_as_directory,
        handler_logging: false,
        output_path: (!save_as_directory).then(|| output.clone()),
        game_update,
        progress: stage_lines_on_terminal(),
        ..RandomizationOptions::default()
    };
    let result = registry
        .randomize_file(&input, &settings, seed, &options)
        .map_err(|error| error.to_string())?;
    if result.feedback.changed_starter {
        eprintln!(
            "WARNING: The custom starters you chose aren't available in this game, so they were changed."
        );
    }
    write_output(&output, &result.rom)?;
    if arguments.save_log {
        write_log(&log_path_for(&output, save_as_directory), &result.log);
    }
    Ok(())
}

fn load_settings(arguments: &Arguments) -> Result<Settings, String> {
    if let Some(path) = &arguments.settings_file {
        let bytes = std::fs::read(path)
            .map_err(|error| format!("Couldn't read {}: {error}", path.display()))?;
        let settings = Settings::read_rnqs(&bytes).map_err(|error| error.to_string())?;
        if settings.updated_from_old_version {
            eprintln!(
                "WARNING: This settings file was made by an older version and has been upgraded."
            );
        }
        return Ok(settings);
    }
    if let Some(text) = &arguments.settings_string {
        let text = text.trim();
        return Settings::from_versioned_settings_string(text)
            .map(|versioned| versioned.settings)
            .or_else(|_| Settings::from_settings_string(text))
            .map_err(|error| error.to_string());
    }
    Err("Provide settings with -s <file.rnqs> or --settings-string <string>.".to_owned())
}

fn fix_filename(original: &Path, default_extension: &str) -> PathBuf {
    let mut path = original.to_string_lossy().into_owned();
    let banned = ROM_EXTENSIONS
        .iter()
        .filter(|extension| **extension != default_extension);
    for extension in banned {
        if let Some(stem) = path.strip_suffix(&format!(".{extension}")) {
            path = format!("{stem}.{default_extension}");
            break;
        }
    }
    if !path.ends_with(&format!(".{default_extension}")) {
        path.push('.');
        path.push_str(default_extension);
    }
    PathBuf::from(path)
}

fn write_output(output: &Path, rom: &RomOutput) -> Result<(), String> {
    match rom {
        RomOutput::File(bytes) => std::fs::write(output, bytes)
            .map_err(|error| format!("Couldn't write {}: {error}", output.display())),
        RomOutput::Directory(files) => write_directory_output(output, files)
            .map_err(|error| format!("Couldn't write {}: {error}", output.display())),
        RomOutput::Written(_) => Ok(()),
    }
}

fn stage_lines_on_terminal() -> Option<ProgressReporter> {
    if !std::io::stderr().is_terminal() {
        return None;
    }
    let last_label = Mutex::new("");
    Some(ProgressReporter::new(move |progress| {
        let Ok(mut last) = last_label.lock() else {
            return;
        };
        if *last != progress.label {
            *last = progress.label;
            eprintln!("[{:>3}%] {}", progress.percent(), progress.label);
        }
    }))
}

fn log_path_for(output: &Path, save_as_directory: bool) -> PathBuf {
    let mut name = output
        .file_name()
        .map(|name| name.to_owned())
        .unwrap_or_else(|| "randomized".into());
    name.push(".log");
    if save_as_directory {
        return output.join(name);
    }
    output.with_file_name(name)
}

fn write_log(path: &Path, log: &str) {
    let mut contents = UTF8_BYTE_ORDER_MARK.to_vec();
    contents.extend_from_slice(log.as_bytes());
    if std::fs::write(path, contents).is_err() {
        eprintln!("WARNING: Could not write log.");
    }
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_sit_next_to_rom_files_and_inside_output_folders() {
        assert_eq!(
            log_path_for(Path::new("/out/yellow.gbc"), false),
            PathBuf::from("/out/yellow.gbc.log")
        );
        assert_eq!(
            log_path_for(Path::new("/out/Output"), true),
            PathBuf::from("/out/Output/Output.log")
        );
    }

    #[test]
    fn appends_the_default_extension() {
        assert_eq!(
            fix_filename(Path::new("/out/yellow"), "gbc"),
            PathBuf::from("/out/yellow.gbc")
        );
    }

    #[test]
    fn replaces_another_rom_extension() {
        assert_eq!(
            fix_filename(Path::new("/out/yellow.gba"), "gbc"),
            PathBuf::from("/out/yellow.gbc")
        );
    }

    #[test]
    fn keeps_the_default_extension() {
        assert_eq!(
            fix_filename(Path::new("/out/yellow.gbc"), "gbc"),
            PathBuf::from("/out/yellow.gbc")
        );
    }

    #[test]
    fn appends_after_unknown_extensions() {
        assert_eq!(
            fix_filename(Path::new("/out/yellow.bin"), "gbc"),
            PathBuf::from("/out/yellow.bin.gbc")
        );
    }
}
