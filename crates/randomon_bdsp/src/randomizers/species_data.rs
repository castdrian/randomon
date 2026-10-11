use std::collections::BTreeMap;

use randomon_unity::Value;

use crate::assets::{BundleId, path_ids};
use crate::context::{Context, similar_strength_pool};
use crate::data::{BROKEN_ABILITIES, MAX_ABILITY, MAX_SPECIES, TM_COUNT, is_legendary};
use crate::error::{Result, data_error};
use crate::settings::{EvolutionMode, LearnsetMode, TmCompatibilityMode};

const EARLY_MOVE_POWER_LIMIT: u8 = 60;
const SAME_TYPE_SHARE: f64 = 0.5;
const TM_CHANCE_SAME_TYPE: f64 = 0.9;
const TM_CHANCE_NORMAL: f64 = 0.5;
const TM_CHANCE_OTHER: f64 = 0.25;
const TM_CHANCE_RANDOM: f64 = 0.5;
const NORMAL_TYPE: u8 = 0;
const EVOLUTION_TARGET_INDEX: usize = 2;
const EVOLUTION_FORM_INDEX: usize = 3;
const EVOLUTION_RECORD_LENGTH: usize = 5;

fn array_mut<'a>(value: &'a mut Value, name: &str) -> Result<&'a mut Vec<Value>> {
    value
        .field_mut(name)?
        .as_array_mut()
        .ok_or_else(|| data_error(format!("{name} is not an array")))
}

pub fn randomize_evolutions(context: &mut Context<'_>) -> Result<()> {
    let mode = context.settings.evolutions;
    if mode == EvolutionMode::Unchanged {
        return Ok(());
    }
    let candidates: Vec<u16> = context
        .data
        .species
        .keys()
        .copied()
        .filter(|&species| !is_legendary(species))
        .collect();
    let mut table = context
        .bundles
        .read(BundleId::PersonalMasterdatas, path_ids::EVOLVE_TABLE)?;
    let mut changes = Vec::new();
    for entry in array_mut(&mut table, "Evolve")? {
        let id = entry.int_field("id")?;
        if !(1..=i64::from(MAX_SPECIES)).contains(&id) {
            continue;
        }
        let source = id as u16;
        let records = array_mut(entry, "ar")?;
        for record in records.as_chunks_mut::<EVOLUTION_RECORD_LENGTH>().0 {
            let original = record[EVOLUTION_TARGET_INDEX].int()? as u16;
            if !context.data.valid_species(original) {
                continue;
            }
            let others: Vec<u16> = candidates
                .iter()
                .copied()
                .filter(|&species| species != source)
                .collect();
            let pool = if mode == EvolutionMode::SimilarStrength {
                similar_strength_pool(
                    &context.data,
                    &others,
                    context.data.base_stat_total(original),
                )
            } else {
                others
            };
            let target = context.pick(&pool, "evolution")?;
            record[EVOLUTION_TARGET_INDEX].set_int(i128::from(target))?;
            record[EVOLUTION_FORM_INDEX].set_int(0)?;
            changes.push((source, original, target));
        }
    }
    context.bundles.write(
        BundleId::PersonalMasterdatas,
        path_ids::EVOLVE_TABLE,
        &table,
    )?;
    for (source, original, target) in changes {
        let names = &context.data.names;
        let line = format!(
            "{} -> {} (was {})",
            names.species(source),
            names.species(target),
            names.species(original)
        );
        context.log.line("Evolutions", line);
    }
    context.reload_data()
}

fn ability_pool() -> Vec<u16> {
    (1..=MAX_ABILITY)
        .filter(|ability| !BROKEN_ABILITIES.contains(ability))
        .collect()
}

pub fn randomize_abilities(context: &mut Context<'_>) -> Result<()> {
    if !context.settings.abilities {
        return Ok(());
    }
    let pool = ability_pool();
    let follow = context.settings.abilities_follow_evolutions;
    let mut by_family: BTreeMap<u16, [u16; 3]> = BTreeMap::new();
    let mut table = context
        .bundles
        .read(BundleId::PersonalMasterdatas, path_ids::PERSONAL_TABLE)?;
    for entry in array_mut(&mut table, "Personal")?.iter_mut().skip(1) {
        let monsno = entry.int_field("monsno")? as u16;
        let key = if follow {
            context.data.family_root(monsno)
        } else {
            monsno
        };
        let abilities = match by_family.get(&key) {
            Some(&abilities) if follow => abilities,
            _ => {
                let picked = context.rng.sample(&pool, 3)?;
                let abilities = [picked[0], picked[1], picked[2]];
                by_family.insert(key, abilities);
                abilities
            }
        };
        for (field, ability) in ["tokusei1", "tokusei2", "tokusei3"].iter().zip(abilities) {
            entry.set_int_field(field, i128::from(ability))?;
        }
    }
    context.bundles.write(
        BundleId::PersonalMasterdatas,
        path_ids::PERSONAL_TABLE,
        &table,
    )?;
    context.abilities_changed = true;
    context.reload_data()?;
    for (&id, species) in &context.data.species {
        let names = &context.data.names;
        let [first, second, hidden] = species.abilities;
        let line = format!(
            "{}: {} / {} / {} (hidden)",
            names.species(id),
            names.ability(first),
            names.ability(second),
            names.ability(hidden)
        );
        context.log.line("Abilities", line);
    }
    Ok(())
}

fn species_types(entry_id: i64, context: &Context<'_>) -> Vec<u8> {
    u16::try_from(entry_id)
        .ok()
        .and_then(|id| context.data.species(id))
        .map(|species| species.types.to_vec())
        .unwrap_or_default()
}

fn pick_move(
    context: &mut Context<'_>,
    types: &[u8],
    used: &[u16],
    damaging_only: bool,
) -> Result<u16> {
    let prefer_type = context.settings.learnsets == LearnsetMode::RandomPreferSameType
        && !types.is_empty()
        && context.chance(SAME_TYPE_SHARE);
    let candidates: Vec<u16> = context
        .data
        .moves
        .values()
        .filter(|info| !used.contains(&info.id))
        .filter(|info| {
            !damaging_only || (info.is_damaging() && info.power <= EARLY_MOVE_POWER_LIMIT)
        })
        .filter(|info| !prefer_type || types.contains(&info.kind))
        .map(|info| info.id)
        .collect();
    if candidates.is_empty() {
        let fallback: Vec<u16> = context
            .data
            .moves
            .keys()
            .copied()
            .filter(|id| !used.contains(id))
            .collect();
        return context.pick(&fallback, "learnset move");
    }
    context.pick(&candidates, "learnset move")
}

pub fn randomize_learnsets(context: &mut Context<'_>) -> Result<()> {
    if context.settings.learnsets == LearnsetMode::Unchanged {
        return Ok(());
    }
    let mut table = context
        .bundles
        .read(BundleId::PersonalMasterdatas, path_ids::WAZA_OBOE_TABLE)?;
    for entry in array_mut(&mut table, "WazaOboe")? {
        let id = entry.int_field("id")?;
        let types = species_types(id, context);
        let learnset = array_mut(entry, "ar")?;
        if learnset.len() < 2 {
            continue;
        }
        let mut used: Vec<u16> = Vec::new();
        for (index, pair) in learnset.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let chosen = pick_move(context, &types, &used, index == 0)?;
            pair[1].set_int(i128::from(chosen))?;
            used.push(chosen);
        }
    }
    context.bundles.write(
        BundleId::PersonalMasterdatas,
        path_ids::WAZA_OBOE_TABLE,
        &table,
    )?;
    context.learnsets_changed = true;
    context.log.line(
        "Learnsets",
        "Level-up learnsets randomized; every Pokémon starts with a damaging move.",
    );
    Ok(())
}

pub fn randomize_tms(context: &mut Context<'_>) -> Result<()> {
    if !context.settings.tms {
        return Ok(());
    }
    let pool: Vec<u16> = context.data.moves.keys().copied().collect();
    let mut table = context
        .bundles
        .read(BundleId::PersonalMasterdatas, path_ids::ITEM_TABLE)?;
    let machines = array_mut(&mut table, "WazaMachine")?;
    let slots: Vec<usize> = machines
        .iter()
        .enumerate()
        .filter(|(_, machine)| {
            machine
                .int_field("machineNo")
                .is_ok_and(|number| (1..=TM_COUNT).contains(&number))
        })
        .map(|(index, _)| index)
        .collect();
    let chosen = context.rng.sample(&pool, slots.len())?;
    let mut lines = Vec::new();
    for (&slot, &move_id) in slots.iter().zip(&chosen) {
        let machine = &mut machines[slot];
        machine.set_int_field("wazaNo", i128::from(move_id))?;
        lines.push(format!(
            "TM{:02}: {}",
            machine.int_field("machineNo")?,
            context.data.names.move_name(move_id)
        ));
    }
    context
        .bundles
        .write(BundleId::PersonalMasterdatas, path_ids::ITEM_TABLE, &table)?;
    for line in lines {
        context.log.line("TMs", line);
    }
    Ok(())
}

fn tm_move_types(context: &Context<'_>) -> Result<BTreeMap<i64, u8>> {
    let table = context
        .bundles
        .read(BundleId::PersonalMasterdatas, path_ids::ITEM_TABLE)?;
    let machines = table
        .field("WazaMachine")?
        .as_array()
        .ok_or_else(|| data_error("WazaMachine is not an array"))?;
    let mut types = BTreeMap::new();
    for machine in machines {
        let number = machine.int_field("machineNo")?;
        if !(1..=TM_COUNT).contains(&number) {
            continue;
        }
        let move_id = machine.int_field("wazaNo")? as u16;
        let kind = context
            .data
            .moves
            .get(&move_id)
            .map_or(NORMAL_TYPE, |info| info.kind);
        types.insert(number, kind);
    }
    Ok(types)
}

fn compatibility_chance(mode: TmCompatibilityMode, move_type: u8, types: [u8; 2]) -> f64 {
    match mode {
        TmCompatibilityMode::Full => 1.0,
        TmCompatibilityMode::Random | TmCompatibilityMode::Unchanged => TM_CHANCE_RANDOM,
        TmCompatibilityMode::TypeAware if types.contains(&move_type) => TM_CHANCE_SAME_TYPE,
        TmCompatibilityMode::TypeAware if move_type == NORMAL_TYPE => TM_CHANCE_NORMAL,
        TmCompatibilityMode::TypeAware => TM_CHANCE_OTHER,
    }
}

pub fn randomize_tm_compatibility(context: &mut Context<'_>) -> Result<()> {
    let mode = context.settings.tm_compatibility;
    if mode == TmCompatibilityMode::Unchanged {
        return Ok(());
    }
    let move_types = tm_move_types(context)?;
    let mut table = context
        .bundles
        .read(BundleId::PersonalMasterdatas, path_ids::PERSONAL_TABLE)?;
    for entry in array_mut(&mut table, "Personal")?.iter_mut().skip(1) {
        let types = [
            entry.int_field("type1")? as u8,
            entry.int_field("type2")? as u8,
        ];
        let mut words = [0u32; 4];
        for number in 1..=TM_COUNT {
            let move_type = move_types.get(&number).copied().unwrap_or(NORMAL_TYPE);
            let chance = compatibility_chance(mode, move_type, types);
            if chance >= 1.0 || context.chance(chance) {
                let bit = (number - 1) as usize;
                words[bit / 32] |= 1 << (bit % 32);
            }
        }
        for (field, word) in ["machine1", "machine2", "machine3", "machine4"]
            .iter()
            .zip(words)
        {
            entry.set_int_field(field, i128::from(word))?;
        }
    }
    context.bundles.write(
        BundleId::PersonalMasterdatas,
        path_ids::PERSONAL_TABLE,
        &table,
    )?;
    let description = match mode {
        TmCompatibilityMode::Full => "Every Pokémon can learn every TM.",
        TmCompatibilityMode::TypeAware => {
            "TM compatibility randomized: 90% same type, 50% Normal, 25% other types."
        }
        _ => "TM compatibility randomized at 50%.",
    };
    context.log.line("TM compatibility", description);
    Ok(())
}
