#[path = "../../randomon_ctr/tests/parity_harness/mod.rs"]
mod harness;

use std::path::PathBuf;

use harness::grid::{SHARED_CASES, base, fuzz_grid, kitchen_sink, misc_tweak_cases};
use harness::{Case, ParityRun, repo_root};
use randomon_core::HandlerRegistry;
use randomon_gen7::Gen7Factory;
use randomon_settings::{
    AllyPokemonMod, AuraMod, MiscTweak, Settings, TotemPokemonMod, WildPokemonMod,
};

type Tweak = fn(&mut Settings);

const TOTEM_CASES: &[(&str, Tweak)] = &[
    ("totems_random", |s| {
        s.totem_pokemon_mod = TotemPokemonMod::Random;
        s.ally_pokemon_mod = AllyPokemonMod::Random;
        s.aura_mod = AuraMod::Random;
        s.randomize_totem_held_items = true;
        s.allow_totem_alt_formes = true;
    }),
    ("totems_similar", |s| {
        s.totem_pokemon_mod = TotemPokemonMod::SimilarStrength;
        s.ally_pokemon_mod = AllyPokemonMod::SimilarStrength;
        s.aura_mod = AuraMod::SameStrength;
        s.totem_levels_modified = true;
        s.totem_level_modifier = 20;
    }),
    ("wild_time_based", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.use_time_based_encounters = true;
    }),
    ("wild_area_time_based", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.use_time_based_encounters = true;
        s.balance_shaking_grass = true;
    }),
];

const ROM_ENV: &str = "RANDOMON_PARITY_GEN7_ROM";
const UPDATE_ENV: &str = "RANDOMON_PARITY_GEN7_UPDATE";
const FUZZ_ENV: &str = "RANDOMON_PARITY_FUZZ";
const SEEDS: &[i64] = &[1, -987_654_321];
const UPDATE_SEEDS: &[i64] = &[42];
const FULL_ROM_CASES: &[&str] = &["defaults", "kitchen_sink", "tweak_all"];
const GEN7_TWEAKS: &[MiscTweak] = &[
    MiscTweak::FastestText,
    MiscTweak::BanLuckyEgg,
    MiscTweak::RetainAltFormes,
    MiscTweak::SosBattlesForAll,
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
    for (name, mask) in misc_tweak_cases(GEN7_TWEAKS) {
        let mut settings = base(rom_name);
        settings.current_misc_tweaks = mask;
        cases.push(Case { name, settings });
    }
    for (name, tweak) in TOTEM_CASES {
        let mut settings = base(rom_name);
        tweak(&mut settings);
        cases.push(Case {
            name: (*name).to_owned(),
            settings,
        });
    }
    let mut settings = base(rom_name);
    kitchen_sink(&mut settings);
    settings.totem_pokemon_mod = TotemPokemonMod::Random;
    settings.ally_pokemon_mod = AllyPokemonMod::Random;
    settings.aura_mod = AuraMod::Random;
    settings.current_misc_tweaks = harness::grid::tweak_mask(GEN7_TWEAKS);
    cases.push(Case {
        name: "kitchen_sink".to_owned(),
        settings,
    });
    let fuzz_count = std::env::var(FUZZ_ENV)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(30);
    for (name, settings) in fuzz_grid(rom_name, fuzz_count, GEN7_TWEAKS) {
        cases.push(Case { name, settings });
    }
    cases
}

#[test]
fn gen7_matches_upr_zx() {
    let Some(rom) = std::env::var_os(ROM_ENV).map(PathBuf::from) else {
        eprintln!("{ROM_ENV} is not set; skipping the Gen 7 parity grid");
        return;
    };
    let registry = HandlerRegistry::new().with(Gen7Factory);
    let info = registry
        .game_info_file(&rom, &rom.to_string_lossy())
        .expect("ROM is a supported Gen 7 game");
    let _ = repo_root();
    ParityRun {
        label: "gen7",
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
