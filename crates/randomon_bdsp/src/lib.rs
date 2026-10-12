mod assets;
mod bundles;
mod context;
mod data;
mod error;
mod exefs;
mod log;
mod output;
mod progress;
mod randomizers;
mod rng;
mod settings;

use std::path::Path;

use progress::Tracker;

pub use assets::{BundleId, path_ids};
pub use bundles::{Bundles, data_directory, looks_like_bdsp_romfs};
pub use data::{BROKEN_ABILITIES, GameData, LEGENDARIES, MAX_ABILITY, MAX_SPECIES, is_legendary};
pub use error::{BdspError, Result};
pub use exefs::{
    Game, Ips32Patch, STARTER_OFFSETS, StarterOffsets, movz_w, starter_patch, starter_patches,
};
pub use output::{
    LOG_FILE_NAME, MOD_NAME, ModTarget, OutputFile, OutputLayout, RandomizedMod, WrittenMod,
    detect_games,
};
pub use progress::{FINISHED_LABEL, LOADING_LABEL, PACKING_LABEL, Progress, WRITING_LABEL};
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

struct Step {
    run: fn(&mut Context<'_>) -> Result<()>,
    label: &'static str,
    runs_with: fn(&BdspSettings) -> bool,
}

const STEPS: [Step; 15] = [
    Step {
        run: species_data::randomize_evolutions,
        label: "Randomizing evolutions",
        runs_with: |s| s.evolutions != EvolutionMode::Unchanged,
    },
    Step {
        run: starters::randomize,
        label: "Picking starters",
        runs_with: |s| s.starters != StarterMode::Unchanged,
    },
    Step {
        run: species_data::randomize_abilities,
        label: "Randomizing abilities",
        runs_with: |s| s.abilities,
    },
    Step {
        run: species_data::randomize_learnsets,
        label: "Randomizing movesets",
        runs_with: |s| s.learnsets != LearnsetMode::Unchanged,
    },
    Step {
        run: species_data::randomize_tms,
        label: "Randomizing TMs",
        runs_with: |s| s.tms,
    },
    Step {
        run: species_data::randomize_tm_compatibility,
        label: "Randomizing TM compatibility",
        runs_with: |s| s.tm_compatibility != TmCompatibilityMode::Unchanged,
    },
    Step {
        run: wild::randomize_encounters,
        label: "Randomizing wild Pokémon",
        runs_with: |s| s.wild != WildMode::Unchanged || s.wild_level_percent != 0,
    },
    Step {
        run: wild::randomize_underground,
        label: "Randomizing the Underground",
        runs_with: |s| s.underground,
    },
    Step {
        run: trainers::randomize_trainers,
        label: "Randomizing trainers",
        runs_with: changes_trainers,
    },
    Step {
        run: items::randomize_field_items,
        label: "Randomizing field items",
        runs_with: |s| s.field_items,
    },
    Step {
        run: items::randomize_shops,
        label: "Randomizing shops",
        runs_with: |s| s.shops,
    },
    Step {
        run: tweaks::apply_following_speeds,
        label: "Tuning following Pokémon",
        runs_with: |s| s.hgss_following,
    },
    Step {
        run: tweaks::apply_true_size,
        label: "Applying true sizes",
        runs_with: |s| s.true_size,
    },
    Step {
        run: tweaks::apply_map_tweaks,
        label: "Applying map tweaks",
        runs_with: |s| s.remove_weather || s.bike_everywhere,
    },
    Step {
        run: tweaks::apply_engine_tweaks,
        label: "Applying speed tweaks",
        runs_with: |s| s.speed_multiplier_percent != 100 || s.sixty_fps,
    },
];

fn changes_trainers(settings: &BdspSettings) -> bool {
    settings.trainers != TrainerMode::Unchanged
        || settings.trainer_level_percent != 0
        || settings.learnsets != LearnsetMode::Unchanged
        || settings.abilities
        || (settings.starters != StarterMode::Unchanged && settings.trainer_rival_carries_starter)
        || settings.trainer_max_ivs
}

fn active_step_count(settings: &BdspSettings) -> usize {
    STEPS
        .iter()
        .filter(|step| (step.runs_with)(settings))
        .count()
}

const LOAD_AND_PACK_STEPS: usize = 2;

pub fn randomize_romfs(romfs: &Path, settings: &BdspSettings, seed: u64) -> Result<RandomizedMod> {
    randomize_romfs_with_progress(romfs, settings, seed, &|_| {})
}

pub fn randomize_romfs_with_progress(
    romfs: &Path,
    settings: &BdspSettings,
    seed: u64,
    on_progress: &dyn Fn(Progress),
) -> Result<RandomizedMod> {
    let tracker = Tracker::new(
        active_step_count(settings) + LOAD_AND_PACK_STEPS,
        on_progress,
    );
    let randomized = load_and_randomize(romfs, settings, seed, &tracker)?;
    tracker.finish();
    Ok(randomized)
}

pub fn randomize_romfs_to(
    romfs: &Path,
    settings: &BdspSettings,
    seed: u64,
    target: &ModTarget,
    on_progress: &dyn Fn(Progress),
) -> Result<WrittenMod> {
    let tracker = Tracker::new(
        active_step_count(settings) + LOAD_AND_PACK_STEPS + 1,
        on_progress,
    );
    let randomized = load_and_randomize(romfs, settings, seed, &tracker)?;
    tracker.begin(WRITING_LABEL);
    let files = randomized.write_to_with_progress(
        &target.directory,
        target.layout,
        &target.games,
        &|done, total, path| tracker.detail(done, total, || format!("Writing {}", path.display())),
    )?;
    tracker.complete();
    tracker.finish();
    Ok(target.written(files))
}

fn load_and_randomize(
    romfs: &Path,
    settings: &BdspSettings,
    seed: u64,
    tracker: &Tracker<'_>,
) -> Result<RandomizedMod> {
    settings.validate()?;
    tracker.begin(LOADING_LABEL);
    let bundles = Bundles::load_with_progress(romfs, &|done, total, path| {
        tracker.detail(done, total, || format!("Reading {}", path.display()))
    })?;
    tracker.complete();
    randomize_tracked(bundles, settings, seed, tracker)
}

pub fn randomize_bundles(
    bundles: Bundles,
    settings: &BdspSettings,
    seed: u64,
) -> Result<RandomizedMod> {
    let tracker = Tracker::new(active_step_count(settings) + 1, &|_| {});
    randomize_tracked(bundles, settings, seed, &tracker)
}

fn randomize_tracked(
    bundles: Bundles,
    settings: &BdspSettings,
    seed: u64,
    tracker: &Tracker<'_>,
) -> Result<RandomizedMod> {
    settings.validate()?;
    let data = GameData::load(&bundles)?;
    let mut context = Context {
        rng: rng::SeededRng::from_seed(seed),
        data,
        bundles,
        settings,
        log: SpoilerLog::default(),
        starters: None,
        exefs_patches: Vec::new(),
        learnsets_changed: false,
        abilities_changed: false,
        tracker,
    };
    for step in &STEPS {
        let runs = (step.runs_with)(settings);
        if runs {
            tracker.begin(step.label);
        }
        (step.run)(&mut context)?;
        if runs {
            tracker.complete();
        }
    }
    tracker.begin(PACKING_LABEL);
    let mut romfs_files = Vec::new();
    let modified: Vec<BundleId> = context.bundles.modified().collect();
    let bundle_count = modified.len();
    for (index, bundle) in modified.into_iter().enumerate() {
        tracker.detail(index, bundle_count, || {
            format!("Packing {}", bundle.relative_path())
        });
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
    tracker.complete();
    Ok(RandomizedMod {
        romfs_files,
        exefs_patches: context.exefs_patches,
        log: context.log.render(&header),
    })
}
