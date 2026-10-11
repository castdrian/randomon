mod grid;

use randomon_core::{HandlerRegistry, RandomizationOptions, RomHandler};
use randomon_gen3::Gen3Factory;
use randomon_java::RandomSource;
use randomon_settings::LineSeparator;

const ROM_ENV: &str = "RANDOMON_GEN3_ROM";
const FUZZ_ENV: &str = "RANDOMON_GEN3_FUZZ";
const DEFAULT_FUZZ_CASES: usize = 40;

fn rom_from_env() -> Option<(String, Vec<u8>)> {
    let Ok(path) = std::env::var(ROM_ENV) else {
        eprintln!("{ROM_ENV} is not set; skipping the real Gen 3 ROM checks");
        return None;
    };
    let bytes = std::fs::read(&path).expect("ROM is readable");
    Some((path, bytes))
}

fn fuzz_cases() -> usize {
    std::env::var(FUZZ_ENV)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(DEFAULT_FUZZ_CASES)
}

fn load(bytes: Vec<u8>, path: &str) -> Box<dyn RomHandler> {
    let registry = HandlerRegistry::new().with(Gen3Factory);
    let source = RandomSource::with_seed(0).into_shared();
    registry
        .load(bytes, path, &source)
        .expect("ROM is a supported Gen 3 game")
}

#[test]
fn real_rom_loads_and_reads_every_table() {
    let Some((path, bytes)) = rom_from_env() else {
        return;
    };
    let handler = load(bytes, &path);
    eprintln!(
        "{} ({}), valid checksum: {}",
        handler.rom_name(),
        handler.rom_code(),
        handler.is_rom_valid()
    );
    assert!(handler.get_pokemon().len() > 386);
    assert_eq!(handler.get_starters().len(), 3);
    assert!(!handler.get_trainers().expect("trainers read").is_empty());
    assert!(!handler.get_encounters(false).is_empty());
    assert!(!handler.get_moves_learnt().is_empty());
    assert_eq!(handler.get_tm_moves().len(), 50);
    assert!(!handler.get_ingame_trades().is_empty());
    assert!(!handler.get_current_field_tms().is_empty());
    if handler.can_change_static_pokemon() {
        assert!(!handler.get_static_pokemon().is_empty());
    }
}

#[test]
fn real_rom_round_trips_unchanged_data() {
    let Some((path, bytes)) = rom_from_env() else {
        return;
    };
    let mut handler = load(bytes, &path);
    let trainers = handler.get_trainers().expect("trainers read");
    let movesets = handler.get_moves_learnt();
    let saved = handler.save_rom().expect("saves");
    let reloaded = load(saved, &path);
    assert_eq!(reloaded.get_moves_learnt(), movesets);
    let reread = reloaded.get_trainers().expect("trainers read");
    assert_eq!(reread.len(), trainers.len());
}

#[test]
fn real_rom_randomizes_across_the_settings_grid() {
    let Some((path, bytes)) = rom_from_env() else {
        return;
    };
    let registry = HandlerRegistry::new().with(Gen3Factory);
    let name = registry
        .game_info(&bytes, &path)
        .expect("ROM is supported")
        .rom_name;
    let options = RandomizationOptions {
        file_name: path.clone(),
        line_separator: LineSeparator::Lf,
        ..RandomizationOptions::default()
    };
    let mut failures = Vec::new();
    let cases = grid::settings_grid(&name)
        .into_iter()
        .chain(grid::fuzz_grid(&name, fuzz_cases()));
    for (index, (case, settings)) in cases.enumerate() {
        let seed = grid::SEEDS[index % grid::SEEDS.len()];
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            registry.randomize(&bytes, &settings, seed, &options)
        }));
        match result {
            Err(_) => failures.push(format!("{case} seed {seed}: panicked")),
            Ok(Err(error)) => failures.push(format!("{case} seed {seed}: {error}")),
            Ok(Ok(output)) => {
                let rom = output.rom_bytes().expect("file output");
                if registry.find(rom).is_none() {
                    failures.push(format!("{case} seed {seed}: output is not detected"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
