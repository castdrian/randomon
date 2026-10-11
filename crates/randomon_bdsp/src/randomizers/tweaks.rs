use std::collections::BTreeMap;

use crate::assets::{BODY_SIZES, BundleId, FAST_RUNNERS, path_ids};
use crate::context::Context;
use crate::error::{Result, data_error};

const FOLLOW_WALK_SPEED: f64 = 1.7;
const FOLLOW_RUN_SPEED: f64 = 3.1;
const FOLLOW_FAST_RUN_SPEED: f64 = 5.0;
const FOLLOW_WALK_START: f64 = 0.1;
const FOLLOW_RUN_START: f64 = 0.6;
const RAYQUAZA: i64 = 384;
const FLYING_MOVE_TYPE: i128 = 2;
const TRUE_SIZE_SCALES: [&str; 6] = [
    "BattleScale",
    "ContestScale",
    "FieldScale",
    "FieldChikaScale",
    "FieldWalkingScale",
    "FieldFureaiScale",
];
const REMOVED_WEATHER: [i64; 6] = [8, 14, 15, 16, 18, 19];
const HEAVY_FOG: i64 = 7;
const LIGHT_FOG: i64 = 6;
const QUALITY_VSYNC_OFFSET: usize = 0x58;
const TIME_MANAGER_LENGTH: usize = 16;
const TIME_MANAGER_CLASS: i32 = 5;
const QUALITY_SETTINGS_CLASS: i32 = 47;

fn body_sizes() -> BTreeMap<i64, f64> {
    BODY_SIZES
        .lines()
        .filter_map(|line| {
            let (species, size) = line.trim().split_once(',')?;
            Some((species.trim().parse().ok()?, size.trim().parse().ok()?))
        })
        .collect()
}

fn fast_runners() -> Vec<i64> {
    FAST_RUNNERS
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}

fn edit_catalog(
    context: &mut Context<'_>,
    edit: impl Fn(&mut randomon_unity::Value) -> Result<()>,
) -> Result<()> {
    context
        .bundles
        .update(BundleId::Masterdatas, path_ids::POKEMON_INFO, |table| {
            let entries = table
                .field_mut("Catalog")?
                .as_array_mut()
                .ok_or_else(|| data_error("Catalog is not an array"))?;
            entries.iter_mut().try_for_each(&edit)
        })
}

pub fn apply_following_speeds(context: &mut Context<'_>) -> Result<()> {
    if !context.settings.hgss_following {
        return Ok(());
    }
    let sizes = body_sizes();
    let runners = fast_runners();
    edit_catalog(context, |entry| {
        let species = entry.int_field("MonsNo")?;
        let run_speed = if runners.contains(&species) {
            FOLLOW_FAST_RUN_SPEED
        } else {
            FOLLOW_RUN_SPEED
        };
        entry.set_float_field("WalkSpeed", FOLLOW_WALK_SPEED)?;
        entry.set_float_field("RunSpeed", run_speed)?;
        entry.set_float_field("WalkStart", FOLLOW_WALK_START)?;
        entry.set_float_field("RunStart", FOLLOW_RUN_START)?;
        if entry.int_field("Waitmoving")? == 1 {
            entry.set_int_field("Waitmoving", 0)?;
        }
        if let Some(&size) = sizes.get(&species) {
            entry.set_float_field("BodySize", size)?;
        }
        if species == RAYQUAZA {
            entry.set_int_field("MoveType", FLYING_MOVE_TYPE)?;
        }
        Ok(())
    })?;
    context
        .log
        .line("Tweaks", "HG/SS-style following Pokémon speeds.");
    Ok(())
}

pub fn apply_true_size(context: &mut Context<'_>) -> Result<()> {
    if !context.settings.true_size {
        return Ok(());
    }
    edit_catalog(context, |entry| {
        for field in TRUE_SIZE_SCALES {
            entry.set_float_field(field, 1.0)?;
        }
        entry.set_int_field("ContestSize", 1)?;
        Ok(())
    })?;
    context
        .log
        .line("Tweaks", "Pokémon shown at their true size.");
    Ok(())
}

pub fn apply_map_tweaks(context: &mut Context<'_>) -> Result<()> {
    let remove_weather = context.settings.remove_weather;
    let bike = context.settings.bike_everywhere;
    if !remove_weather && !bike {
        return Ok(());
    }
    context
        .bundles
        .update(BundleId::Gamesettings, path_ids::MAP_INFO, |table| {
            let zones = table
                .field_mut("ZoneData")?
                .as_array_mut()
                .ok_or_else(|| data_error("ZoneData is not an array"))?;
            for zone in zones.iter_mut() {
                if remove_weather {
                    let weather = zone.int_field("Weather")?;
                    if REMOVED_WEATHER.contains(&weather) {
                        zone.set_int_field("Weather", 0)?;
                    } else if weather == HEAVY_FOG {
                        zone.set_int_field("Weather", i128::from(LIGHT_FOG))?;
                    }
                }
                if bike {
                    zone.set_int_field("Bicycle", 1)?;
                }
            }
            Ok(())
        })?;
    if remove_weather {
        context
            .log
            .line("Tweaks", "Fog and disruptive weather removed.");
    }
    if bike {
        context.log.line("Tweaks", "Bicycle allowed everywhere.");
    }
    Ok(())
}

fn scaled_time_manager(raw: &[u8], multiplier: f32) -> Result<Vec<u8>> {
    if raw.len() != TIME_MANAGER_LENGTH {
        return Err(data_error(format!("TimeManager is {} bytes", raw.len())));
    }
    let value = |index: usize| {
        f32::from_le_bytes([
            raw[index * 4],
            raw[index * 4 + 1],
            raw[index * 4 + 2],
            raw[index * 4 + 3],
        ])
    };
    let values = [
        value(0) / multiplier,
        value(1),
        value(2) * multiplier,
        value(3) / multiplier,
    ];
    Ok(values
        .iter()
        .flat_map(|number| number.to_le_bytes())
        .collect())
}

pub fn apply_engine_tweaks(context: &mut Context<'_>) -> Result<()> {
    let multiplier = context.settings.speed_multiplier_percent;
    let sixty = context.settings.sixty_fps;
    if multiplier == 100 && !sixty {
        return Ok(());
    }
    if !context.bundles.has(BundleId::GlobalGameManagers) {
        return Err(data_error(
            "globalgamemanagers is required for engine tweaks",
        ));
    }
    let bundles = &context.bundles;
    let class_of = |path_id| bundles.class_id(BundleId::GlobalGameManagers, path_id);
    if class_of(path_ids::TIME_MANAGER) != Some(TIME_MANAGER_CLASS)
        || class_of(path_ids::QUALITY_SETTINGS) != Some(QUALITY_SETTINGS_CLASS)
    {
        return Err(data_error("globalgamemanagers has an unexpected layout"));
    }
    if multiplier != 100 {
        let raw = context
            .bundles
            .raw(BundleId::GlobalGameManagers, path_ids::TIME_MANAGER)?;
        let scaled = scaled_time_manager(&raw, multiplier as f32 / 100.0)?;
        context
            .bundles
            .set_raw(BundleId::GlobalGameManagers, path_ids::TIME_MANAGER, scaled)?;
        context
            .log
            .line("Tweaks", format!("Game speed set to {multiplier}%."));
    }
    if sixty {
        let mut raw = context
            .bundles
            .raw(BundleId::GlobalGameManagers, path_ids::QUALITY_SETTINGS)?;
        let byte = raw
            .get_mut(QUALITY_VSYNC_OFFSET)
            .ok_or_else(|| data_error("QualitySettings is too short"))?;
        *byte = 1;
        context.bundles.set_raw(
            BundleId::GlobalGameManagers,
            path_ids::QUALITY_SETTINGS,
            raw,
        )?;
        context.log.line("Tweaks", "60 FPS mode enabled.");
    }
    Ok(())
}
