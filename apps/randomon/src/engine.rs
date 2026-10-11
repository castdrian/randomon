use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use randomon_core::{HandlerRegistry, RandomizationOptions, RomHandler, RomOutput};
use randomon_gen1::Gen1Factory;
use randomon_gen5::Gen5Factory;
use randomon_java::RandomSource;
use randomon_settings::Settings;

use crate::capabilities::Capabilities;

const LARGEST_INSPECTED_ROM: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct EngineGame {
    pub bytes: Arc<Vec<u8>>,
    pub rom_name: String,
    pub rom_code: String,
    pub generation: i32,
    pub support_level: String,
    pub default_extension: String,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone)]
pub struct RandomizedGame {
    pub output: PathBuf,
    pub log: Option<PathBuf>,
    pub check_value: i32,
}

pub fn registry() -> &'static HandlerRegistry {
    static REGISTRY: OnceLock<HandlerRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| HandlerRegistry::new().with(Gen1Factory).with(Gen5Factory))
}

pub fn inspect(path: &Path, size_bytes: u64) -> Option<EngineGame> {
    if size_bytes > LARGEST_INSPECTED_ROM {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    registry().find(&bytes)?;
    let file_name = file_name_of(path);
    let source = RandomSource::with_seed(0).into_shared();
    let handler = registry().load(bytes.clone(), &file_name, &source).ok()?;
    Some(EngineGame {
        rom_name: handler.rom_name(),
        rom_code: handler.rom_code(),
        generation: handler.generation_of_pokemon(),
        support_level: handler.support_level(),
        default_extension: handler.default_extension(),
        capabilities: capabilities_of(handler.as_ref()),
        bytes: Arc::new(bytes),
    })
}

fn capabilities_of(handler: &dyn RomHandler) -> Capabilities {
    let generation = handler.generation_of_pokemon();
    Capabilities {
        generation: Some(generation),
        is_yellow: handler.is_yellow(),
        has_mega_evolutions: handler.has_mega_evolutions(),
        has_starter_alt_formes: handler.has_starter_alt_formes(),
        supports_starter_held_items: handler.supports_starter_held_items(),
        can_change_static_pokemon: handler.can_change_static_pokemon(),
        has_main_game_legendaries: handler.has_main_game_legendaries(),
        has_static_alt_formes: handler.has_static_alt_formes(),
        force_swap_static_mega_evos: handler.force_swap_static_mega_evos(),
        has_static_music_fix: handler.has_static_music_fix(),
        has_physical_special_split: handler.has_physical_special_split(),
        supports_four_starting_moves: handler.supports_four_starting_moves(),
        has_functional_formes: handler.has_functional_formes(),
        has_time_based_encounters: handler.has_time_based_encounters(),
        has_wild_alt_formes: handler.has_wild_alt_formes(),
        has_move_tutors: handler.has_move_tutors(),
        has_shop_randomization: handler.has_shop_randomization(),
        abilities_per_pokemon: handler.abilities_per_pokemon(),
        misc_tweaks_available: handler.misc_tweaks_available(),
        species_count: handler.get_pokemon().len().saturating_sub(1),
    }
}

pub async fn choose_output(game: EngineGame, source_file_name: String) -> Option<PathBuf> {
    let stem = Path::new(&source_file_name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "randomized".to_owned());
    rfd::AsyncFileDialog::new()
        .set_title("Save the randomized ROM")
        .add_filter("ROM", &[game.default_extension.as_str()])
        .set_file_name(format!("{stem} (Randomized).{}", game.default_extension))
        .save_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

pub async fn randomize(
    game: EngineGame,
    settings: Settings,
    seed: i64,
    output: PathBuf,
) -> Result<RandomizedGame, String> {
    tokio::task::spawn_blocking(move || randomize_blocking(&game, &settings, seed, output))
        .await
        .map_err(|error| error.to_string())?
}

fn randomize_blocking(
    game: &EngineGame,
    settings: &Settings,
    seed: i64,
    output: PathBuf,
) -> Result<RandomizedGame, String> {
    let options = RandomizationOptions {
        file_name: file_name_of(&output),
        ..RandomizationOptions::default()
    };
    let result = registry()
        .randomize(&game.bytes, settings, seed, &options)
        .map_err(|error| error.to_string())?;
    match &result.rom {
        RomOutput::File(bytes) => {
            std::fs::write(&output, bytes).map_err(|error| error.to_string())?
        }
        RomOutput::Directory(_) => {
            return Err("Folder output isn't available for this game.".to_owned());
        }
    }
    let log_path = log_path_for(&output);
    let log = std::fs::write(&log_path, with_byte_order_mark(&result.log))
        .ok()
        .map(|()| log_path);
    Ok(RandomizedGame {
        output,
        log,
        check_value: result.check_value,
    })
}

fn with_byte_order_mark(log: &str) -> Vec<u8> {
    let mut contents = vec![0xEF, 0xBB, 0xBF];
    contents.extend_from_slice(log.as_bytes());
    contents
}

fn log_path_for(output: &Path) -> PathBuf {
    let mut name = output.as_os_str().to_owned();
    name.push(".log");
    PathBuf::from(name)
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom_under_test() -> Option<PathBuf> {
        std::env::var_os("RANDOMON_GBC_ROM").map(PathBuf::from)
    }

    #[test]
    fn randomizes_a_gen1_rom_end_to_end() {
        let Some(rom) = rom_under_test() else {
            return;
        };
        let size = std::fs::metadata(&rom).expect("rom metadata").len();
        let game = inspect(&rom, size).expect("engine recognises the rom");
        let settings = Settings {
            rom_name: Some(game.rom_name.clone()),
            selected_exp_curve: Some(randomon_settings::ExpCurve::MediumFast),
            ..Settings::default()
        };
        let output = std::env::temp_dir().join("randomon-engine-test.gbc");
        let done =
            randomize_blocking(&game, &settings, 123_456_789, output.clone()).expect("randomizes");
        let written = std::fs::read(&done.output).expect("output written");
        assert_eq!(written.len(), game.bytes.len());
        let log = std::fs::read(done.log.expect("log written")).expect("log readable");
        assert!(log.starts_with(&[0xEF, 0xBB, 0xBF]));
        assert!(String::from_utf8_lossy(&log).contains("Random Seed: 123456789"));
    }
}
