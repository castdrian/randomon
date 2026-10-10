use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

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
    let arguments = Arguments::parse();
    if arguments.settings_file.is_none() && arguments.settings_string.is_none() {
        eprintln!("Provide settings with -s <file.rnqs> or --settings-string <string>.");
        return ExitCode::from(2);
    }
    eprintln!(
        "The randomization engine is still being ported; {} was not modified.",
        arguments.input.display()
    );
    ExitCode::from(3)
}
