#[path = "../../randomon_ctr/tests/parity_harness/mod.rs"]
mod harness;

use std::path::PathBuf;

use harness::grid::{SHARED_CASES, base, fuzz_grid, kitchen_sink, misc_tweak_cases};
use harness::{Case, ParityRun, repo_root};
use randomon_core::HandlerRegistry;
use randomon_gen6::Gen6Factory;
use randomon_settings::MiscTweak;

const ROM_ENV: &str = "RANDOMON_PARITY_GEN6_ROM";
const UPDATE_ENV: &str = "RANDOMON_PARITY_GEN6_UPDATE";
const FUZZ_ENV: &str = "RANDOMON_PARITY_FUZZ";
const SEEDS: &[i64] = &[1, -987_654_321];
const UPDATE_SEEDS: &[i64] = &[42];
const FULL_ROM_CASES: &[&str] = &["defaults", "kitchen_sink", "tweak_all"];
const GEN6_TWEAKS: &[MiscTweak] = &[
    MiscTweak::FastestText,
    MiscTweak::BanLuckyEgg,
    MiscTweak::RetainAltFormes,
    MiscTweak::NationalDexAtStart,
];

fn cases(rom_name: &str) -> Vec<Case> {
    let mut cases: Vec<Case> = SHARED_CASES
        .iter()
        .map(|(name, tweak)| {
            let mut settings = base(rom_name);
            tweak(&mut settings);
            Case {
                name: (*name).to_owned(),
                settings,
            }
        })
        .collect();
    for (name, mask) in misc_tweak_cases(GEN6_TWEAKS) {
        let mut settings = base(rom_name);
        settings.current_misc_tweaks = mask;
        cases.push(Case { name, settings });
    }
    let mut settings = base(rom_name);
    kitchen_sink(&mut settings);
    settings.current_misc_tweaks = harness::grid::tweak_mask(GEN6_TWEAKS);
    cases.push(Case {
        name: "kitchen_sink".to_owned(),
        settings,
    });
    let fuzz_count = std::env::var(FUZZ_ENV)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(30);
    for (name, settings) in fuzz_grid(rom_name, fuzz_count, GEN6_TWEAKS) {
        cases.push(Case { name, settings });
    }
    cases
}

#[test]
fn gen6_matches_upr_zx() {
    let Some(rom) = std::env::var_os(ROM_ENV).map(PathBuf::from) else {
        eprintln!("{ROM_ENV} is not set; skipping the Gen 6 parity grid");
        return;
    };
    let registry = HandlerRegistry::new().with(Gen6Factory);
    let info = registry
        .game_info_file(&rom, &rom.to_string_lossy())
        .expect("ROM is a supported Gen 6 game");
    let _ = repo_root();
    ParityRun {
        label: "gen6",
        registry,
        update: std::env::var_os(UPDATE_ENV).map(PathBuf::from),
        cases: cases(&info.rom_name),
        rom,
        seeds: SEEDS.to_vec(),
        update_seeds: UPDATE_SEEDS.to_vec(),
        full_rom_cases: FULL_ROM_CASES
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
    }
    .run();
}
