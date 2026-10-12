use std::collections::BTreeMap;

use randomon_unity::Value;

use crate::assets::{BundleId, path_ids};
use crate::context::{Context, similar_strength_pool, without};
use crate::data::{GameData, generation_of, is_legendary};
use crate::error::{Result, data_error};
use crate::settings::{BdspSettings, WildMode};

const MAX_LEVEL: i64 = 100;
const SPECIES_FIELD: &str = "monsNo";

pub struct WildPools {
    pub regular: Vec<u16>,
    pub legendary: Vec<u16>,
}

pub fn wild_pools(data: &GameData, settings: &BdspSettings) -> WildPools {
    let generations = settings.allowed_generations();
    let allowed: Vec<u16> = data
        .species
        .keys()
        .copied()
        .filter(|&species| generations.contains(&generation_of(species)))
        .collect();
    let legendary: Vec<u16> = allowed
        .iter()
        .copied()
        .filter(|&s| is_legendary(s))
        .collect();
    let regular: Vec<u16> = allowed
        .iter()
        .copied()
        .filter(|&species| settings.wild_allow_legendaries || !is_legendary(species))
        .collect();
    WildPools {
        legendary: if legendary.is_empty() {
            regular.clone()
        } else {
            legendary
        },
        regular,
    }
}

pub struct Replacer<'p> {
    pools: &'p WildPools,
    global: BTreeMap<u16, u16>,
}

impl<'p> Replacer<'p> {
    pub fn new(pools: &'p WildPools) -> Self {
        Self {
            pools,
            global: BTreeMap::new(),
        }
    }

    fn base_pool(&self, context: &Context<'_>, original: u16) -> Vec<u16> {
        let keep_legendary = context.settings.wild_keep_legendaries && is_legendary(original);
        let pool = if keep_legendary {
            &self.pools.legendary
        } else {
            &self.pools.regular
        };
        if context.settings.wild_similar_strength {
            similar_strength_pool(&context.data, pool, context.data.base_stat_total(original))
        } else {
            pool.clone()
        }
    }

    pub fn fresh(&self, context: &mut Context<'_>, original: u16, avoid: &[u16]) -> Result<u16> {
        let pool = without(&self.base_pool(context, original), avoid);
        context.pick(&pool, "wild Pokémon")
    }

    pub fn replace(
        &mut self,
        context: &mut Context<'_>,
        original: u16,
        area: &mut BTreeMap<u16, u16>,
    ) -> Result<u16> {
        match context.settings.wild {
            WildMode::Unchanged => Ok(original),
            WildMode::PerSlot => self.fresh(context, original, &[]),
            WildMode::AreaMapping => {
                if let Some(&mapped) = area.get(&original) {
                    return Ok(mapped);
                }
                let used: Vec<u16> = area.values().copied().collect();
                let mapped = self.fresh(context, original, &used)?;
                area.insert(original, mapped);
                Ok(mapped)
            }
            WildMode::GlobalMapping => {
                if let Some(&mapped) = self.global.get(&original) {
                    return Ok(mapped);
                }
                let used: Vec<u16> = self.global.values().copied().collect();
                let mapped = self.fresh(context, original, &used)?;
                self.global.insert(original, mapped);
                Ok(mapped)
            }
        }
    }
}

fn slot_lists(area: &mut Value) -> Vec<&mut Vec<Value>> {
    let Some(fields) = area.as_struct_mut() else {
        return Vec::new();
    };
    fields
        .iter_mut()
        .filter_map(|(_, value)| value.as_array_mut())
        .filter(|items| {
            items
                .first()
                .and_then(Value::as_struct)
                .is_some_and(|fields| fields.get(SPECIES_FIELD).is_some())
        })
        .collect()
}

fn scale_level(level: i64, percent: i32) -> i64 {
    if level <= 0 {
        return level;
    }
    let scaled = (level as f64 * (100.0 + f64::from(percent)) / 100.0).round() as i64;
    scaled.clamp(1, MAX_LEVEL)
}

fn edit_slot(
    context: &mut Context<'_>,
    replacer: &mut Replacer<'_>,
    slot: &mut Value,
    area_map: &mut BTreeMap<u16, u16>,
) -> Result<()> {
    let original = slot.int_field(SPECIES_FIELD)?;
    if original <= 0 {
        return Ok(());
    }
    let original = original as u16;
    if context.data.valid_species(original) {
        let replacement = replacer.replace(context, original, area_map)?;
        slot.set_int_field(SPECIES_FIELD, i128::from(replacement))?;
    }
    let percent = context.settings.wild_level_percent;
    if percent != 0 {
        let min = scale_level(slot.int_field("minlv")?, percent);
        let max = scale_level(slot.int_field("maxlv")?, percent).max(min);
        slot.set_int_field("minlv", i128::from(min))?;
        slot.set_int_field("maxlv", i128::from(max))?;
    }
    Ok(())
}

fn randomize_table(
    context: &mut Context<'_>,
    replacer: &mut Replacer<'_>,
    path_id: i64,
) -> Result<Vec<String>> {
    let mut table = context.bundles.read(BundleId::Gamesettings, path_id)?;
    let mut lines = Vec::new();
    let areas = table
        .field_mut("table")?
        .as_array_mut()
        .ok_or_else(|| data_error("encounter table is not an array"))?;
    let area_count = areas.len();
    for (done, area) in areas.iter_mut().enumerate() {
        let zone = area.int_field("zoneID")?;
        context.detail(done, area_count, || {
            format!("Zone {zone} · area {} of {area_count}", done + 1)
        });
        let mut area_map = BTreeMap::new();
        for slots in slot_lists(area) {
            for slot in slots.iter_mut() {
                edit_slot(context, replacer, slot, &mut area_map)?;
            }
        }
        if !area_map.is_empty() {
            let names = &context.data.names;
            let pairs: Vec<String> = area_map
                .iter()
                .map(|(&from, &to)| format!("{} -> {}", names.species(from), names.species(to)))
                .collect();
            lines.push(format!("Zone {zone}: {}", pairs.join(", ")));
        }
    }
    if context.settings.safari && context.settings.wild != WildMode::Unchanged {
        randomize_safari(context, replacer, &mut table)?;
    }
    context
        .bundles
        .write(BundleId::Gamesettings, path_id, &table)?;
    Ok(lines)
}

fn randomize_safari(
    context: &mut Context<'_>,
    replacer: &mut Replacer<'_>,
    table: &mut Value,
) -> Result<()> {
    let entries = table
        .field_mut("safari")?
        .as_array_mut()
        .ok_or_else(|| data_error("safari table is not an array"))?;
    let mut used = Vec::new();
    for entry in entries.iter_mut() {
        let original = entry.int_field("MonsNo")?;
        if original <= 0 || !context.data.valid_species(original as u16) {
            continue;
        }
        let replacement = replacer.fresh(context, original as u16, &used)?;
        used.push(replacement);
        entry.set_int_field("MonsNo", i128::from(replacement))?;
    }
    Ok(())
}

pub fn randomize_encounters(context: &mut Context<'_>) -> Result<()> {
    if context.settings.wild == WildMode::Unchanged && context.settings.wild_level_percent == 0 {
        return Ok(());
    }
    let pools = wild_pools(&context.data, context.settings);
    let mut replacer = Replacer::new(&pools);
    let tables = [
        ("Brilliant Diamond", path_ids::FIELD_ENCOUNT_TABLE_D),
        ("Shining Pearl", path_ids::FIELD_ENCOUNT_TABLE_P),
    ];
    for (game, path_id) in tables {
        let lines = randomize_table(context, &mut replacer, path_id)?;
        if context.settings.wild != WildMode::Unchanged {
            let section = format!("Wild Pokémon ({game})");
            for line in lines {
                context.log.line(&section, line);
            }
        }
    }
    if context.settings.wild_level_percent != 0 {
        let line = format!(
            "Wild levels changed by {:+}%.",
            context.settings.wild_level_percent
        );
        context.log.line("Wild levels", line);
    }
    Ok(())
}

pub fn randomize_underground(context: &mut Context<'_>) -> Result<()> {
    if !context.settings.underground {
        return Ok(());
    }
    let mut settings = context.settings.clone();
    settings.wild_allow_legendaries = false;
    let pools = wild_pools(&context.data, &settings);
    let replacer = Replacer::new(&pools);
    for path_id in path_ids::UG_ENCOUNT_TABLES {
        let mut table = context.bundles.read(BundleId::UgData, path_id)?;
        let name = table
            .field("m_Name")?
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let entries = table
            .field_mut("table")?
            .as_array_mut()
            .ok_or_else(|| data_error("underground table is not an array"))?;
        let mut used: Vec<u16> = Vec::new();
        for entry in entries.iter_mut() {
            let original = entry.int_field("monsno")?;
            if original <= 0 || !context.data.valid_species(original as u16) {
                continue;
            }
            let replacement = replacer.fresh(context, original as u16, &used)?;
            used.push(replacement);
            entry.set_int_field("monsno", i128::from(replacement))?;
        }
        let names: Vec<String> = used
            .iter()
            .map(|&id| context.data.names.species(id))
            .collect();
        context.bundles.write(BundleId::UgData, path_id, &table)?;
        context
            .log
            .line("Underground", format!("{name}: {}", names.join(", ")));
    }
    Ok(())
}
