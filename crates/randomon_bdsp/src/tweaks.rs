use std::collections::BTreeMap;

use randomon_unity::Value;

use crate::assets::{BODY_SIZES, BundleId, FAST_RUNNERS, path_ids};
use crate::error::{Result, data_error};
use crate::handler::BdspRomHandler;
use crate::tables::{Table, int};

const FOLLOW_WALK_SPEED: f64 = 1.7;
const FOLLOW_RUN_SPEED: f64 = 3.1;
const FOLLOW_FAST_RUN_SPEED: f64 = 5.0;
const FOLLOW_WALK_START: f64 = 0.1;
const FOLLOW_RUN_START: f64 = 0.6;
const RAYQUAZA: i32 = 384;
const FLYING_MOVE_TYPE: i128 = 2;
const TRUE_SIZE_SCALES: [&str; 6] = [
    "BattleScale",
    "ContestScale",
    "FieldScale",
    "FieldChikaScale",
    "FieldWalkingScale",
    "FieldFureaiScale",
];
const REMOVED_WEATHER: [i32; 6] = [8, 14, 15, 16, 18, 19];
const HEAVY_FOG: i32 = 7;
const LIGHT_FOG: i128 = 6;
const QUALITY_VSYNC_OFFSET: usize = 0x58;
const TIME_MANAGER_LENGTH: usize = 16;
const TIME_MANAGER_CLASS: i32 = 5;
const QUALITY_SETTINGS_CLASS: i32 = 47;
pub const FASTER_GAME_SPEED: f32 = 1.5;

fn body_sizes() -> BTreeMap<i32, f64> {
    BODY_SIZES
        .lines()
        .filter_map(|line| {
            let (species, size) = line.trim().split_once(',')?;
            Some((species.trim().parse().ok()?, size.trim().parse().ok()?))
        })
        .collect()
}

fn fast_runners() -> Vec<i32> {
    FAST_RUNNERS
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
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

impl BdspRomHandler {
    fn edit_catalog(&mut self, edit: impl Fn(&mut Value) -> Result<()>) -> Result<()> {
        self.tables
            .rows_mut(Table::PokemonInfo, "Catalog")?
            .iter_mut()
            .try_for_each(edit)
    }

    pub(crate) fn apply_following_speeds(&mut self) -> Result<()> {
        let sizes = body_sizes();
        let runners = fast_runners();
        self.edit_catalog(|entry| {
            let species = int(entry, "MonsNo")?;
            let run_speed = if runners.contains(&species) {
                FOLLOW_FAST_RUN_SPEED
            } else {
                FOLLOW_RUN_SPEED
            };
            entry.set_float_field("WalkSpeed", FOLLOW_WALK_SPEED)?;
            entry.set_float_field("RunSpeed", run_speed)?;
            entry.set_float_field("WalkStart", FOLLOW_WALK_START)?;
            entry.set_float_field("RunStart", FOLLOW_RUN_START)?;
            if int(entry, "Waitmoving")? == 1 {
                entry.set_int_field("Waitmoving", 0)?;
            }
            if let Some(&size) = sizes.get(&species) {
                entry.set_float_field("BodySize", size)?;
            }
            if species == RAYQUAZA {
                entry.set_int_field("MoveType", FLYING_MOVE_TYPE)?;
            }
            Ok(())
        })
    }

    pub(crate) fn apply_true_size(&mut self) -> Result<()> {
        self.edit_catalog(|entry| {
            for field in TRUE_SIZE_SCALES {
                entry.set_float_field(field, 1.0)?;
            }
            entry.set_int_field("ContestSize", 1)?;
            Ok(())
        })
    }

    fn edit_zones(&mut self, edit: impl Fn(&mut Value) -> Result<()>) -> Result<()> {
        self.tables
            .rows_mut(Table::MapInfo, "ZoneData")?
            .iter_mut()
            .try_for_each(edit)
    }

    pub(crate) fn remove_weather(&mut self) -> Result<()> {
        self.edit_zones(|zone| {
            let weather = int(zone, "Weather")?;
            if REMOVED_WEATHER.contains(&weather) {
                zone.set_int_field("Weather", 0)?;
            } else if weather == HEAVY_FOG {
                zone.set_int_field("Weather", LIGHT_FOG)?;
            }
            Ok(())
        })
    }

    pub(crate) fn allow_bike_everywhere(&mut self) -> Result<()> {
        self.edit_zones(|zone| {
            zone.set_int_field("Bicycle", 1)?;
            Ok(())
        })
    }

    fn check_engine_layout(&self) -> Result<()> {
        if !self.bundles.has(BundleId::GlobalGameManagers) {
            return Err(data_error(
                "globalgamemanagers is required for engine tweaks",
            ));
        }
        let class_of = |path_id| self.bundles.class_id(BundleId::GlobalGameManagers, path_id);
        if class_of(path_ids::TIME_MANAGER) != Some(TIME_MANAGER_CLASS)
            || class_of(path_ids::QUALITY_SETTINGS) != Some(QUALITY_SETTINGS_CLASS)
        {
            return Err(data_error("globalgamemanagers has an unexpected layout"));
        }
        Ok(())
    }

    pub(crate) fn speed_up_game(&mut self, multiplier: f32) -> Result<()> {
        self.check_engine_layout()?;
        let raw = self
            .bundles
            .raw(BundleId::GlobalGameManagers, path_ids::TIME_MANAGER)?;
        let scaled = scaled_time_manager(&raw, multiplier)?;
        self.bundles
            .set_raw(BundleId::GlobalGameManagers, path_ids::TIME_MANAGER, scaled)
    }

    pub(crate) fn enable_sixty_fps(&mut self) -> Result<()> {
        self.check_engine_layout()?;
        let mut raw = self
            .bundles
            .raw(BundleId::GlobalGameManagers, path_ids::QUALITY_SETTINGS)?;
        let byte = raw
            .get_mut(QUALITY_VSYNC_OFFSET)
            .ok_or_else(|| data_error("QualitySettings is too short"))?;
        *byte = 1;
        self.bundles.set_raw(
            BundleId::GlobalGameManagers,
            path_ids::QUALITY_SETTINGS,
            raw,
        )
    }

    pub(crate) fn has_engine_settings(&self) -> bool {
        self.check_engine_layout().is_ok()
    }
}
