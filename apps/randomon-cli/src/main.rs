use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use randomon_core::{HandlerRegistry, RandomizationOptions, RomOutput};
use randomon_gen1::Gen1Factory;
use randomon_gen2::Gen2Factory;
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
        help = "3DS game update (.cia) to apply first"
    )]
    game_update: Option<PathBuf>,

    #[arg(short = 'l', help = "Also write the randomization log")]
    save_log: bool,
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

fn run(arguments: Arguments) -> Result<(), String> {
    let settings = load_settings(&arguments)?;
    if arguments.game_update.is_some() {
        return Err("3DS game updates arrive with the Gen 6 and 7 handlers.".to_owned());
    }
    let rom = std::fs::read(&arguments.input)
        .map_err(|error| format!("Couldn't read {}: {error}", arguments.input.display()))?;
    let registry = HandlerRegistry::new().with(Gen1Factory).with(Gen2Factory);
    let file_name = file_name_of(&arguments.input);
    let game = registry
        .game_info(&rom, &file_name)
        .map_err(|_| format!("Could not load {file_name} - it is not a supported ROM yet."))?;
    let mut save_as_directory = arguments.save_as_directory;
    if save_as_directory && !matches!(game.generation, 6 | 7) {
        save_as_directory = false;
        eprintln!(
            "WARNING: Saving as directory does not make sense for non-3DS games, ignoring \"-d\" flag..."
        );
    }
    let output = if save_as_directory {
        arguments.output.clone()
    } else {
        fix_filename(&arguments.output, &game.default_extension)
    };
    let seed = arguments.seed.unwrap_or_else(RandomSource::pick_seed);
    let options = RandomizationOptions {
        file_name: file_name_of(&output),
        save_as_directory,
        handler_logging: false,
        ..RandomizationOptions::default()
    };
    let result = registry
        .randomize(&rom, &settings, seed, &options)
        .map_err(|error| error.to_string())?;
    if result.feedback.changed_starter {
        eprintln!(
            "WARNING: The custom starters you chose aren't available in this game, so they were changed."
        );
    }
    write_output(&output, &result.rom)?;
    if arguments.save_log {
        write_log(&output, &result.log);
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
        RomOutput::Directory(files) => {
            for file in files {
                let destination = output.join(&file.path);
                if let Some(parent) = destination.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                std::fs::write(&destination, &file.bytes).map_err(|error| error.to_string())?;
            }
            Ok(())
        }
    }
}

fn write_log(output: &Path, log: &str) {
    let mut path = output.as_os_str().to_owned();
    path.push(".log");
    let mut contents = UTF8_BYTE_ORDER_MARK.to_vec();
    contents.extend_from_slice(log.as_bytes());
    if std::fs::write(PathBuf::from(path), contents).is_err() {
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
