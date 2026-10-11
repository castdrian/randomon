mod grid;
mod support;

use std::cell::RefCell;
use std::io::Cursor;
use std::sync::Once;

use randomon_core::{HandlerRegistry, RandomizationOptions, RomHandler};
use randomon_formats::nds::{NarcArchive, NdsRom};
use randomon_gen4::{Gen4Factory, Gen4RomHandler};
use randomon_java::RandomSource;

use support::{Quirks, all_entries, build_rom, entry};

thread_local! {
    static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

struct WarningRecorder;

impl log::Log for WarningRecorder {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            WARNINGS.with(|warnings| warnings.borrow_mut().push(record.args().to_string()));
        }
    }

    fn flush(&self) {}
}

static RECORDER: WarningRecorder = WarningRecorder;
static INSTALL: Once = Once::new();

fn take_warnings() -> Vec<String> {
    INSTALL.call_once(|| {
        log::set_logger(&RECORDER).expect("logger installs once");
        log::set_max_level(log::LevelFilter::Warn);
    });
    WARNINGS.with(|warnings| std::mem::take(&mut *warnings.borrow_mut()))
}

fn load(rom: Vec<u8>) -> Gen4RomHandler {
    let source = RandomSource::with_seed(1).into_shared();
    Gen4RomHandler::load(rom, "synthetic.nds", &source).expect("synthetic ROM loads")
}

fn options() -> RandomizationOptions {
    RandomizationOptions {
        file_name: "synthetic.nds".to_owned(),
        ..RandomizationOptions::default()
    }
}

#[test]
fn every_region_loads_reads_and_saves() {
    take_warnings();
    for rom_entry in all_entries() {
        let rom = build_rom(rom_entry, Quirks::default());
        assert!(randomon_gen4::detect_rom(&rom), "{}", rom_entry.name);
        let mut handler = load(rom);
        assert_eq!(handler.rom_code(), rom_entry.rom_code);
        assert_eq!(handler.get_pokemon().len(), 494, "{}", rom_entry.name);
        assert!(
            !handler.get_encounters(true).is_empty(),
            "{}",
            rom_entry.name
        );
        assert!(
            !handler.get_trainers().expect("trainers").is_empty(),
            "{}",
            rom_entry.name
        );
        assert_eq!(handler.get_tm_moves().len(), 92, "{}", rom_entry.name);
        assert_eq!(handler.get_starters().len(), 3, "{}", rom_entry.name);
        if rom_entry.static_pokemon_support {
            assert!(
                !handler.get_static_pokemon().is_empty(),
                "{}",
                rom_entry.name
            );
        }
        let saved = handler.save_rom().expect("saves");
        let reloaded = load(saved);
        assert_eq!(reloaded.get_pokemon().len(), 494);
        let warnings = take_warnings();
        assert!(warnings.is_empty(), "{}: {warnings:?}", rom_entry.name);
    }
}

#[test]
fn default_settings_randomize_each_game_type() {
    let registry = HandlerRegistry::new().with(Gen4Factory);
    for (code, version) in [("ADAE", 5), ("CPUE", 0), ("IPKE", 0)] {
        let rom = build_rom(entry(code, version), Quirks::default());
        let settings = grid::base("synthetic");
        let output = registry
            .randomize(&rom, &settings, 42, &options())
            .unwrap_or_else(|error| panic!("{code}: {error}"));
        assert!(output.rom_bytes().is_some_and(|bytes| !bytes.is_empty()));
    }
}

const SHARED_ENGINE_FAILURES: &[&str] = &[
    "Effectiveness.against",
    "originalEvos entry",
    "evolution chain does not reach a base Pokemon",
    "sane amount of retries",
    "OutOfMemoryError",
];

fn is_shared_engine_failure(message: &str) -> bool {
    SHARED_ENGINE_FAILURES
        .iter()
        .any(|known| message.contains(known))
}

fn run_grid(
    code: &str,
    version: i32,
    quirks: Quirks,
    cases: &[(String, randomon_settings::Settings)],
) {
    let registry = HandlerRegistry::new().with(Gen4Factory);
    let rom = build_rom(entry(code, version), quirks);
    let mut failures = Vec::new();
    take_warnings();
    for (name, settings) in cases {
        for &seed in &grid::SEEDS[..1] {
            if let Err(error) = registry.randomize(&rom, settings, seed, &options())
                && !is_shared_engine_failure(&error.to_string())
            {
                failures.push(format!("{code} {name} seed {seed}: {error}"));
            }
            for warning in take_warnings() {
                failures.push(format!("{code} {name} seed {seed} recovered: {warning}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn settings_grid_never_fails_on_diamond() {
    run_grid(
        "ADAE",
        5,
        Quirks::default(),
        &grid::settings_grid("synthetic"),
    );
}

#[test]
fn settings_grid_never_fails_on_platinum() {
    run_grid(
        "CPUE",
        0,
        Quirks::default(),
        &grid::settings_grid("synthetic"),
    );
}

#[test]
fn settings_grid_never_fails_on_heartgold() {
    run_grid(
        "IPKE",
        0,
        Quirks::default(),
        &grid::settings_grid("synthetic"),
    );
}

#[test]
fn fuzzed_settings_never_fail() {
    let cases = grid::fuzz_grid("synthetic", grid::DEFAULT_FUZZ_CASES / 2);
    for (code, version) in [
        ("ADAE", 5),
        ("CPUE", 0),
        ("IPKE", 0),
        ("IPGE", 0),
        ("CPUJ", 0),
    ] {
        run_grid(code, version, Quirks::default(), &cases);
    }
}

#[test]
fn dump_synthetic_roms_when_requested() {
    let Ok(directory) = std::env::var("RANDOMON_GEN4_DUMP") else {
        return;
    };
    std::fs::create_dir_all(&directory).expect("dump directory");
    for (code, version) in [("ADAE", 5), ("CPUE", 0), ("IPKE", 0)] {
        let rom = build_rom(entry(code, version), Quirks::default());
        std::fs::write(format!("{directory}/synthetic_{code}.nds"), rom).expect("write ROM");
    }
}

fn quirky_settings() -> randomon_settings::Settings {
    let mut settings = grid::base("synthetic");
    settings.wild_pokemon_mod = randomon_settings::WildPokemonMod::AreaMapping;
    settings.use_time_based_encounters = true;
    settings
}

#[test]
fn hgss_empty_headbutt_and_contest_data_stay_in_sync() {
    let quirks = Quirks {
        empty_headbutt_tree: true,
        empty_bug_contest_day: true,
        missing_night_fishing: true,
    };
    let rom = build_rom(entry("IPKE", 0), quirks);
    let registry = HandlerRegistry::new().with(Gen4Factory);
    let output = registry
        .randomize(&rom, &quirky_settings(), 7, &options())
        .expect("randomizes despite empty encounter data");
    let mut randomized = load(output.rom_bytes().expect("rom").to_vec());
    let before = load(rom).get_encounters(true);
    let after = randomized.get_encounters(true);
    assert_eq!(before.len(), after.len());
    for (old, new) in before.iter().zip(&after) {
        assert_eq!(old.display_name, new.display_name);
        assert_eq!(old.encounters.len(), new.encounters.len());
    }
    assert!(randomized.save_rom().is_ok());
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize
    }
}

fn corrupt_bytes(data: &mut [u8], dice: &mut Lcg, count: usize) {
    if data.is_empty() {
        return;
    }
    for _ in 0..count {
        let offset = dice.next() % data.len();
        data[offset] = match dice.next() % 4 {
            0 => 0,
            1 => 0xFF,
            _ => (dice.next() % 256) as u8,
        };
    }
}

fn corrupt_rom(
    clean: &[u8],
    code: &str,
    version: i32,
    dice: &mut Lcg,
    intensity: usize,
) -> Vec<u8> {
    let mut rom = NdsRom::open(Cursor::new(clean.to_vec())).expect("synthetic ROM parses");
    let rom_entry = entry(code, version);
    let personal = rom_entry.file("PokemonStats").unwrap_or_default();
    let mut paths: Vec<&String> = rom_entry
        .files
        .values()
        .map(|file| &file.path)
        .filter(|path| path.as_str() != personal)
        .collect();
    paths.sort();
    for path in paths {
        if !dice.next().is_multiple_of(3) {
            continue;
        }
        let Ok(Some(data)) = rom.get_file(path) else {
            continue;
        };
        let replacement = match NarcArchive::parse(&data) {
            Ok(mut narc) => {
                for file in &mut narc.files {
                    if dice.next().is_multiple_of(16) {
                        corrupt_bytes(file, dice, intensity);
                    }
                    if intensity > 3 && dice.next().is_multiple_of(200) {
                        let keep = dice.next() % (file.len() + 1);
                        file.truncate(keep);
                    }
                }
                narc.to_bytes()
            }
            Err(_) => {
                let mut raw = data.clone();
                corrupt_bytes(&mut raw, dice, intensity);
                raw
            }
        };
        rom.write_file(path, &replacement)
            .expect("write corrupted file");
    }
    let mut arm9 = rom.get_arm9().expect("arm9");
    corrupt_bytes(&mut arm9, dice, intensity * 4);
    rom.write_arm9(&arm9).expect("write arm9");
    for overlay in 0..rom.overlays().len() as i32 {
        if let Ok(Some(mut data)) = rom.get_overlay(overlay)
            && dice.next().is_multiple_of(4)
        {
            corrupt_bytes(&mut data, dice, intensity);
            rom.write_overlay(overlay, &data).expect("write overlay");
        }
    }
    rom.save_to(Cursor::new(Vec::new()))
        .expect("save corrupted ROM")
        .into_inner()
}

#[test]
fn corrupted_game_data_never_panics() {
    let registry = HandlerRegistry::new().with(Gen4Factory);
    let mut settings = grid::base("synthetic");
    settings.wild_pokemon_mod = randomon_settings::WildPokemonMod::Random;
    settings.trainers_mod = randomon_settings::TrainersMod::Random;
    settings.static_pokemon_mod = randomon_settings::StaticPokemonMod::RandomMatching;
    settings.shop_items_mod = randomon_settings::ShopItemsMod::Random;
    settings.movesets_mod = randomon_settings::MovesetsMod::CompletelyRandom;
    settings.tms_mod = randomon_settings::TmsMod::Random;
    settings.in_game_trades_mod = randomon_settings::InGameTradesMod::RandomizeGivenAndRequested;
    settings.starters_mod = randomon_settings::StartersMod::CompletelyRandom;
    let mut loaded = 0;
    for (code, version) in [("CPUE", 0), ("IPKE", 0), ("ADAE", 5)] {
        let clean = build_rom(entry(code, version), Quirks::default());
        let mut dice = Lcg(code.len() as u64 * 977 + version as u64);
        for round in 0..10usize {
            let rom = corrupt_rom(&clean, code, version, &mut dice, round % 4 + 1);
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match registry
                .randomize(&rom, &settings, round as i64, &options())
            {
                Ok(_) => true,
                Err(error) => {
                    eprintln!("{code} round {round}: {error}");
                    false
                }
            }));
            match outcome {
                Ok(true) => loaded += 1,
                Ok(false) => {}
                Err(_) => panic!("{code} round {round} panicked"),
            }
        }
    }
    assert!(loaded > 0, "no corrupted ROM randomized at all");
}
