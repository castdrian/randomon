use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use randomon_core::{
    HandlerRegistry, RandomizationOptions, RomHandler, RomOutput, write_directory_output,
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

use crate::capabilities::Capabilities;

const LARGEST_INSPECTED_ROM: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub enum RomSource {
    Bytes(Arc<Vec<u8>>),
    File(PathBuf),
}

#[derive(Debug, Clone)]
pub struct EngineGame {
    pub source: RomSource,
    pub path: PathBuf,
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
    REGISTRY.get_or_init(|| {
        HandlerRegistry::new()
            .with(Gen1Factory)
            .with(Gen2Factory)
            .with(Gen4Factory)
            .with(Gen3Factory)
            .with(Gen5Factory)
            .with(Gen6Factory)
            .with(Gen7Factory)
    })
}

fn path_name(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn inspect_from_file(path: &Path) -> Option<EngineGame> {
    registry().detect_file(path)?;
    let random_source = RandomSource::with_seed(0).into_shared();
    let handler = registry()
        .load_file(path, &path_name(path), &random_source)
        .ok()?;
    Some(engine_game(
        handler.as_ref(),
        RomSource::File(path.to_path_buf()),
        path,
    ))
}

fn inspect_in_memory(path: &Path, size_bytes: u64) -> Option<EngineGame> {
    if size_bytes > LARGEST_INSPECTED_ROM {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    registry().find(&bytes)?;
    let random_source = RandomSource::with_seed(0).into_shared();
    let handler = registry()
        .load(bytes.clone(), &path_name(path), &random_source)
        .ok()?;
    Some(engine_game(
        handler.as_ref(),
        RomSource::Bytes(Arc::new(bytes)),
        path,
    ))
}

fn engine_game(handler: &dyn RomHandler, source: RomSource, path: &Path) -> EngineGame {
    EngineGame {
        rom_name: handler.rom_name(),
        rom_code: handler.rom_code(),
        generation: handler.generation_of_pokemon(),
        support_level: handler.support_level(),
        default_extension: handler.default_extension(),
        capabilities: capabilities_of(handler),
        source,
        path: path.to_path_buf(),
    }
}

pub fn inspect(path: &Path, size_bytes: u64) -> Option<EngineGame> {
    guarded(|| inspect_from_file(path).or_else(|| inspect_in_memory(path, size_bytes)))
        .ok()
        .flatten()
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .unwrap_or_else(|| "unknown error".to_owned())
}

pub(crate) fn guarded<T>(work: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).map_err(|payload| {
        format!(
            "randomon ran into an internal error ({}). Please report it with your settings string and seed.",
            panic_message(payload)
        )
    })
}

fn is_3ds(game: &EngineGame) -> bool {
    matches!(game.generation, 6 | 7)
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

pub async fn choose_output(
    game: EngineGame,
    source_file_name: String,
    game_update: Option<PathBuf>,
) -> Option<PathBuf> {
    if is_3ds(&game) && game_update.is_some() {
        return rfd::AsyncFileDialog::new()
            .set_title("Choose a folder for the LayeredFS output")
            .pick_folder()
            .await
            .map(|handle| handle.path().to_path_buf());
    }
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
    game_update: Option<PathBuf>,
) -> Result<RandomizedGame, String> {
    tokio::task::spawn_blocking(move || {
        guarded(|| randomize_blocking(&game, &settings, seed, output, game_update))?
    })
    .await
    .map_err(|error| error.to_string())?
}

fn randomize_blocking(
    game: &EngineGame,
    settings: &Settings,
    seed: i64,
    output: PathBuf,
    game_update: Option<PathBuf>,
) -> Result<RandomizedGame, String> {
    let game_update = game_update.filter(|_| is_3ds(game));
    let save_as_directory = game_update.is_some();
    if !save_as_directory && is_3ds(game) && output == game.path {
        return Err(
            "You cannot overwrite the original ROM when you save a DS randomization.".to_owned(),
        );
    }
    let options = RandomizationOptions {
        file_name: path_name(&game.path),
        save_as_directory,
        output_path: (!save_as_directory).then(|| output.clone()),
        game_update,
        ..RandomizationOptions::default()
    };
    let result = match &game.source {
        RomSource::Bytes(bytes) => registry().randomize(bytes, settings, seed, &options),
        RomSource::File(path) => registry().randomize_file(path, settings, seed, &options),
    }
    .map_err(|error| error.to_string())?;
    match &result.rom {
        RomOutput::File(bytes) => {
            std::fs::write(&output, bytes).map_err(|error| error.to_string())?
        }
        RomOutput::Directory(files) => {
            write_directory_output(&output, files).map_err(|error| error.to_string())?
        }
        RomOutput::Written(_) => {}
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
        let done = randomize_blocking(&game, &settings, 123_456_789, output.clone(), None)
            .expect("randomizes");
        let written = std::fs::read(&done.output).expect("output written");
        assert_eq!(written.len() as u64, size);
        let log = std::fs::read(done.log.expect("log written")).expect("log readable");
        assert!(log.starts_with(&[0xEF, 0xBB, 0xBF]));
        assert!(String::from_utf8_lossy(&log).contains("Random Seed: 123456789"));
    }

    fn env_path(name: &str) -> Option<PathBuf> {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| path.exists())
    }

    fn code_of(rom: &Path, update: Option<&Path>) -> Vec<u8> {
        let source = randomon_ctr::rom::open_file_source(rom).expect("base rom opens");
        let mut ctr = randomon_ctr::CtrRom::open(source, "rom").expect("base rom parses");
        if let Some(update) = update {
            let source = randomon_ctr::rom::open_file_source(update).expect("update opens");
            ctr.load_update(source).expect("update parses");
        }
        ctr.read_code().expect("code.bin reads")
    }

    fn files_under(directory: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut pending = vec![directory.to_path_buf()];
        while let Some(current) = pending.pop() {
            for entry in std::fs::read_dir(&current).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    found.push(path);
                }
            }
        }
        found
    }

    #[test]
    fn randomizing_with_a_game_update_writes_layeredfs_built_from_the_update() {
        let (Some(rom), Some(update)) = (
            env_path("RANDOMON_GEN6_ROM"),
            env_path("RANDOMON_GEN6_UPDATE"),
        ) else {
            return;
        };
        let size = std::fs::metadata(&rom).expect("rom metadata").len();
        let game = inspect(&rom, size).expect("engine recognises the 3DS rom");
        let settings = Settings {
            rom_name: Some(game.rom_name.clone()),
            selected_exp_curve: Some(randomon_settings::ExpCurve::MediumFast),
            trainers_mod: randomon_settings::TrainersMod::Random,
            wild_pokemon_mod: randomon_settings::WildPokemonMod::Random,
            starters_mod: randomon_settings::StartersMod::CompletelyRandom,
            current_misc_tweaks: randomon_settings::MiscTweak::FastestText.value(),
            ..Settings::default()
        };
        let output = std::env::temp_dir().join("randomon-update-flow");
        let _ = std::fs::remove_dir_all(&output);
        std::fs::create_dir_all(&output).expect("output folder");
        let done = randomize_blocking(&game, &settings, 42, output.clone(), Some(update.clone()))
            .expect("randomizes with the update");
        let log_path = done.log.expect("log written");
        let log =
            String::from_utf8_lossy(&std::fs::read(&log_path).expect("log readable")).into_owned();
        assert!(log.contains("Randomization of"));
        assert!(!log.contains("(Unpatched) completed"));
        let titles: Vec<PathBuf> = std::fs::read_dir(&output)
            .expect("output listing")
            .flatten()
            .map(|entry| entry.path())
            .collect();
        assert_eq!(titles.len(), 1, "one title folder");
        let title = &titles[0];
        let romfs_files = files_under(&title.join("romfs"));
        assert!(!romfs_files.is_empty(), "changed romfs files are written");
        let written_code = std::fs::read(title.join("code.bin")).expect("code.bin written");
        let update_code = code_of(&rom, Some(&update));
        let base_code = code_of(&rom, None);
        assert_eq!(written_code.len(), update_code.len());
        if base_code.len() != update_code.len() {
            assert_ne!(written_code.len(), base_code.len());
        }
        let _ = std::fs::remove_dir_all(&output);
        let _ = std::fs::remove_file(&log_path);
    }
}
