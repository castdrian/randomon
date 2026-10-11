mod assets;
mod bundles;
mod context;
mod data;
mod error;
mod exefs;
mod log;
mod output;
mod randomizers;
mod settings;

use std::path::Path;

use randomon_python_random::PythonRandom;

pub use assets::{BundleId, path_ids};
pub use bundles::{Bundles, data_directory, looks_like_bdsp_romfs};
pub use data::{BROKEN_ABILITIES, GameData, LEGENDARIES, MAX_ABILITY, MAX_SPECIES, is_legendary};
pub use error::{BdspError, Result};
pub use exefs::{
    Game, Ips32Patch, STARTER_OFFSETS, StarterOffsets, movz_w, starter_patch, starter_patches,
};
pub use output::{LOG_FILE_NAME, MOD_NAME, OutputFile, OutputLayout, RandomizedMod, detect_games};
pub use randomizers::items::{
    decode_item_argument, encode_item_argument, is_randomizable_item, item_pool,
};
pub use randomizers::trainers::learned_moves;
pub use settings::{
    BdspSettings, EVOLUTION_MODES, EvolutionMode, LEARNSET_MODES, LEVEL_PERCENT_RANGE,
    LearnsetMode, ModeLabel, SETTINGS_PREFIX, SETTINGS_VERSION, SPEED_PERCENT_RANGE, STARTER_MODES,
    SettingsError, StarterMode, TM_COMPATIBILITY_MODES, TRAINER_MODES, TmCompatibilityMode,
    TrainerMode, WILD_MODES, WildMode,
};

use context::Context;
use log::SpoilerLog;
use randomizers::{items, species_data, starters, trainers, tweaks, wild};

type Step = fn(&mut Context<'_>) -> Result<()>;

const STEPS: [Step; 15] = [
    species_data::randomize_evolutions,
    starters::randomize,
    species_data::randomize_abilities,
    species_data::randomize_learnsets,
    species_data::randomize_tms,
    species_data::randomize_tm_compatibility,
    wild::randomize_encounters,
    wild::randomize_underground,
    trainers::randomize_trainers,
    items::randomize_field_items,
    items::randomize_shops,
    tweaks::apply_following_speeds,
    tweaks::apply_true_size,
    tweaks::apply_map_tweaks,
    tweaks::apply_engine_tweaks,
];

pub fn randomize_romfs(romfs: &Path, settings: &BdspSettings, seed: u64) -> Result<RandomizedMod> {
    let bundles = Bundles::load(romfs)?;
    randomize_bundles(bundles, settings, seed)
}

pub fn randomize_bundles(
    bundles: Bundles,
    settings: &BdspSettings,
    seed: u64,
) -> Result<RandomizedMod> {
    settings.validate()?;
    let data = GameData::load(&bundles)?;
    let mut context = Context {
        rng: PythonRandom::from_u128(u128::from(seed)),
        data,
        bundles,
        settings,
        log: SpoilerLog::default(),
        starters: None,
        exefs_patches: Vec::new(),
        learnsets_changed: false,
        abilities_changed: false,
    };
    for step in STEPS {
        step(&mut context)?;
    }
    let mut romfs_files = Vec::new();
    let modified: Vec<BundleId> = context.bundles.modified().collect();
    for bundle in modified {
        romfs_files.push((
            bundle.relative_path().to_owned(),
            context.bundles.serialize(bundle)?,
        ));
    }
    let header = vec![
        format!("randomon BDSP randomizer {}", env!("CARGO_PKG_VERSION")),
        format!("Seed: {seed}"),
        format!("Settings: {}", settings.to_settings_string()),
    ];
    Ok(RandomizedMod {
        romfs_files,
        exefs_patches: context.exefs_patches,
        log: context.log.render(&header),
    })
}
