mod grid;

use std::collections::BTreeMap;

use randomon_core::{HandlerRegistry, RandomizationOptions};
use randomon_gen3::{Gen3Factory, synthetic};
use randomon_settings::{LineSeparator, Settings};

const FUZZ_ENV: &str = "RANDOMON_GEN3_FUZZ";
const DEFAULT_FUZZ_CASES: usize = 24;
const GRID_ENTRIES: &[&str] = &["Emerald (U)", "Fire Red (U) 1.1", "Ruby (U)"];
const FUZZ_ENTRIES: &[&str] = &[
    "Emerald (U)",
    "Fire Red (U) 1.0",
    "Leaf Green (U) 1.1",
    "Sapphire (U)",
    "Ruby (J)",
    "Emerald (J)",
    "Fire Red (J)",
    "Emerald (S)",
];

fn fuzz_cases() -> usize {
    std::env::var(FUZZ_ENV)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(DEFAULT_FUZZ_CASES)
}

fn options() -> RandomizationOptions {
    RandomizationOptions {
        file_name: "synthetic.gba".to_owned(),
        line_separator: LineSeparator::Lf,
        ..RandomizationOptions::default()
    }
}

fn run_case(
    registry: &HandlerRegistry,
    rom: &[u8],
    entry: &str,
    case: &str,
    settings: &Settings,
    seed: i64,
) -> Result<(), String> {
    let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        registry.randomize(rom, settings, seed, &options())
    }))
    .map_err(|_| format!("{entry} {case} seed {seed}: panicked"))?
    .map_err(|error| format!("{entry} {case} seed {seed}: {error}"))?;
    let bytes = output
        .rom_bytes()
        .ok_or_else(|| format!("{entry} {case}: no ROM output"))?;
    if !output.log.contains("Randomizer Version") {
        return Err(format!("{entry} {case}: log is incomplete"));
    }
    if registry.find(bytes).is_none() {
        return Err(format!("{entry} {case}: output is no longer detected"));
    }
    registry
        .game_info(bytes, "output.gba")
        .map_err(|error| format!("{entry} {case}: output does not reload: {error}"))?;
    Ok(())
}

fn run_cases(entries: &[&str], cases: impl Fn(&str) -> Vec<(String, Settings)>) {
    let registry = HandlerRegistry::new().with(Gen3Factory);
    let mut failures = Vec::new();
    let mut runs = 0;
    let roms: BTreeMap<&str, Vec<u8>> = entries
        .iter()
        .map(|&entry| (entry, synthetic::build(entry)))
        .collect();
    for (index, (case, mut settings)) in cases(entries[0]).into_iter().enumerate() {
        let entry = entries[index % entries.len()];
        settings.rom_name = Some(entry.to_owned());
        let seed = grid::SEEDS[index % grid::SEEDS.len()];
        runs += 1;
        if let Err(failure) = run_case(&registry, &roms[entry], entry, &case, &settings, seed) {
            failures.push(failure);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {runs} runs failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn curated_settings_grid_randomizes_synthetic_roms() {
    run_cases(GRID_ENTRIES, grid::settings_grid);
}

#[test]
fn fuzzed_settings_never_fail_on_synthetic_roms() {
    run_cases(FUZZ_ENTRIES, |name| grid::fuzz_grid(name, fuzz_cases()));
}
