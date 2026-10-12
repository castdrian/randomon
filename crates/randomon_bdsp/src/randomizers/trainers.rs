use std::collections::BTreeMap;

use randomon_unity::Value;

use crate::assets::{BundleId, path_ids};
use crate::context::{Context, similar_strength_pool};
use crate::data::{GameData, is_legendary};
use crate::error::{Result, data_error};
use crate::settings::TrainerMode;

const SECTION: &str = "Trainers";
const PARTY_SIZE: usize = 6;
const MAX_LEVEL: i64 = 100;
const PERFECT_IV: i128 = 31;
const IV_FIELDS: [&str; 6] = [
    "TalentHp",
    "TalentAtk",
    "TalentDef",
    "TalentSpAtk",
    "TalentSpDef",
    "TalentAgi",
];
const TUTORIAL_TRAINERS: std::ops::RangeInclusive<i64> = 487..=492;
const RIVAL_BATTLES: [i64; 21] = [
    351, 352, 353, 354, 355, 356, 357, 358, 359, 360, 361, 362, 481, 493, 494, 663, 664, 665, 666,
    667, 668,
];
const GYM_LEADERS: [i64; 16] = [
    175, 179, 240, 241, 242, 243, 244, 245, 676, 677, 678, 679, 680, 681, 682, 683,
];
const ELITE_FOUR: [i64; 12] = [185, 186, 187, 188, 696, 697, 698, 699, 701, 702, 703, 704];
const CHAMPION: [i64; 3] = [191, 700, 705];
const GALACTIC_ADMINS: [i64; 9] = [220, 308, 309, 310, 311, 312, 313, 314, 401];
const ORIGINAL_STARTER_LINES: [[u16; 3]; 3] = [[387, 388, 389], [390, 391, 392], [393, 394, 395]];

fn is_boss(trainer: i64) -> bool {
    RIVAL_BATTLES.contains(&trainer)
        || GYM_LEADERS.contains(&trainer)
        || ELITE_FOUR.contains(&trainer)
        || CHAMPION.contains(&trainer)
        || GALACTIC_ADMINS.contains(&trainer)
}

fn has_type_theme(trainer: i64) -> bool {
    GYM_LEADERS.contains(&trainer) || ELITE_FOUR.contains(&trainer)
}

fn slot_field(slot: usize, name: &str) -> String {
    format!("P{slot}{name}")
}

fn party_species(trainer: &Value) -> Result<Vec<u16>> {
    let mut species = Vec::new();
    for slot in 1..=PARTY_SIZE {
        if trainer.int_field(&slot_field(slot, "Level"))? > 0 {
            species.push(trainer.int_field(&slot_field(slot, "MonsNo"))? as u16);
        }
    }
    Ok(species)
}

fn dominant_type(data: &GameData, party: &[u16]) -> Option<u8> {
    let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
    for species in party.iter().filter_map(|&id| data.species(id)) {
        let mut types = species.types.to_vec();
        types.dedup();
        for kind in types {
            *counts.entry(kind).or_default() += 1;
        }
    }
    let best = counts.values().copied().max()?;
    if best * 2 < party.len().max(1) {
        return None;
    }
    party
        .iter()
        .filter_map(|&id| data.species(id))
        .flat_map(|species| species.types)
        .find(|kind| counts.get(kind) == Some(&best))
}

pub fn learned_moves(learnset: &[u16], level: i64) -> Vec<u16> {
    let mut moves: Vec<u16> = Vec::new();
    for pair in learnset.as_chunks::<2>().0 {
        if i64::from(pair[0]) <= level && pair[1] != 0 {
            let move_id = pair[1];
            moves.retain(|&known| known != move_id);
            moves.push(move_id);
        }
    }
    if moves.is_empty()
        && let Some(pair) = learnset.as_chunks::<2>().0.first()
    {
        moves.push(pair[1]);
    }
    let keep_from = moves.len().saturating_sub(4);
    moves.split_off(keep_from)
}

fn load_learnsets(context: &Context<'_>) -> Result<BTreeMap<u16, Vec<u16>>> {
    let table = context
        .bundles
        .read(BundleId::PersonalMasterdatas, path_ids::WAZA_OBOE_TABLE)?;
    let entries = table
        .field("WazaOboe")?
        .as_array()
        .ok_or_else(|| data_error("WazaOboe is not an array"))?;
    let mut learnsets = BTreeMap::new();
    for entry in entries {
        let id = entry.int_field("id")?;
        let Ok(id) = u16::try_from(id) else {
            continue;
        };
        let values = entry
            .field("ar")?
            .as_array()
            .ok_or_else(|| data_error("learnset is not an array"))?
            .iter()
            .map(|value| value.int().map(|number| number as u16))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        learnsets.insert(id, values);
    }
    Ok(learnsets)
}

struct TrainerPlan {
    pool: Vec<u16>,
    legendary_pool: Vec<u16>,
    learnsets: BTreeMap<u16, Vec<u16>>,
}

fn starter_replacement(context: &Context<'_>, original: u16) -> Option<u16> {
    let starters = context.starters?;
    if !context.settings.trainer_rival_carries_starter {
        return None;
    }
    ORIGINAL_STARTER_LINES
        .iter()
        .zip(starters)
        .find_map(|(line, new_root)| {
            let stage = line.iter().position(|&species| species == original)?;
            Some(context.data.evolve_to_stage(new_root, stage))
        })
}

fn choose_species(
    context: &mut Context<'_>,
    plan: &TrainerPlan,
    original: u16,
    theme: Option<u8>,
    used: &[u16],
) -> Result<u16> {
    if let Some(replacement) = starter_replacement(context, original) {
        return Ok(replacement);
    }
    if context.settings.trainers == TrainerMode::Unchanged {
        return Ok(original);
    }
    let base = if is_legendary(original) {
        &plan.legendary_pool
    } else {
        &plan.pool
    };
    let themed: Vec<u16> = match theme {
        Some(kind) => base
            .iter()
            .copied()
            .filter(|&id| context.data.species(id).is_some_and(|s| s.has_type(kind)))
            .collect(),
        None => base.clone(),
    };
    let themed = if themed.is_empty() {
        base.clone()
    } else {
        themed
    };
    let fresh: Vec<u16> = themed
        .iter()
        .copied()
        .filter(|id| !used.contains(id))
        .collect();
    let candidates = if fresh.is_empty() { themed } else { fresh };
    let pool = if context.settings.trainers == TrainerMode::SimilarStrength {
        similar_strength_pool(
            &context.data,
            &candidates,
            context.data.base_stat_total(original),
        )
    } else {
        candidates
    };
    context.pick(&pool, "trainer Pokémon")
}

fn choose_ability(context: &mut Context<'_>, species: u16) -> Result<Option<u16>> {
    let Some(info) = context.data.species(species) else {
        return Ok(None);
    };
    let mut options: Vec<u16> = info.abilities[..2]
        .iter()
        .copied()
        .filter(|&ability| ability != 0)
        .collect();
    options.dedup();
    if options.is_empty() {
        return Ok(None);
    }
    context.pick(&options, "ability").map(Some)
}

fn scale_level(level: i64, percent: i32) -> i64 {
    let scaled = (level as f64 * (100.0 + f64::from(percent)) / 100.0).round() as i64;
    scaled.clamp(1, MAX_LEVEL)
}

fn edit_trainer(
    context: &mut Context<'_>,
    plan: &TrainerPlan,
    trainer: &mut Value,
) -> Result<Option<String>> {
    let id = trainer.int_field("ID")?;
    if TUTORIAL_TRAINERS.contains(&id) {
        return Ok(None);
    }
    let original_party = party_species(trainer)?;
    let theme = if context.settings.trainer_type_themes && has_type_theme(id) {
        dominant_type(&context.data, &original_party)
    } else {
        None
    };
    let mut used = Vec::new();
    let mut summary = Vec::new();
    for slot in 1..=PARTY_SIZE {
        let level_field = slot_field(slot, "Level");
        let mut level = trainer.int_field(&level_field)?;
        if level <= 0 {
            continue;
        }
        if context.settings.trainer_level_percent != 0 {
            level = scale_level(level, context.settings.trainer_level_percent);
            trainer.set_int_field(&level_field, i128::from(level))?;
        }
        let species_field = slot_field(slot, "MonsNo");
        let original = trainer.int_field(&species_field)? as u16;
        if !context.data.valid_species(original) {
            continue;
        }
        let species = choose_species(context, plan, original, theme, &used)?;
        used.push(species);
        let changed = species != original;
        if changed {
            trainer.set_int_field(&species_field, i128::from(species))?;
            trainer.set_int_field(&slot_field(slot, "FormNo"), 0)?;
        }
        if (changed || context.abilities_changed)
            && let Some(ability) = choose_ability(context, species)?
        {
            trainer.set_int_field(&slot_field(slot, "Tokusei"), i128::from(ability))?;
        }
        if changed || context.learnsets_changed || context.settings.trainer_level_percent != 0 {
            let learnset = plan.learnsets.get(&species).cloned().unwrap_or_default();
            let moves = learned_moves(&learnset, level);
            for index in 0..4 {
                let move_id = moves.get(index).copied().unwrap_or(0);
                trainer.set_int_field(
                    &slot_field(slot, &format!("Waza{}", index + 1)),
                    i128::from(move_id),
                )?;
            }
        }
        if context.settings.trainer_max_ivs && is_boss(id) {
            for field in IV_FIELDS {
                trainer.set_int_field(&slot_field(slot, field), PERFECT_IV)?;
            }
        }
        summary.push(format!("{} Lv{level}", context.data.names.species(species)));
    }
    Ok(Some(format!("#{id}: {}", summary.join(", "))))
}

pub fn randomize_trainers(context: &mut Context<'_>) -> Result<()> {
    let touches_trainers = context.settings.trainers != TrainerMode::Unchanged
        || context.settings.trainer_level_percent != 0
        || context.learnsets_changed
        || context.abilities_changed
        || (context.starters.is_some() && context.settings.trainer_rival_carries_starter)
        || context.settings.trainer_max_ivs;
    if !touches_trainers {
        return Ok(());
    }
    let pool: Vec<u16> = context
        .data
        .species
        .keys()
        .copied()
        .filter(|&species| !is_legendary(species))
        .collect();
    let legendary_pool: Vec<u16> = context
        .data
        .species
        .keys()
        .copied()
        .filter(|&species| is_legendary(species))
        .collect();
    let plan = TrainerPlan {
        pool,
        legendary_pool,
        learnsets: load_learnsets(context)?,
    };
    let mut table = context
        .bundles
        .read(BundleId::Masterdatas, path_ids::TRAINER_TABLE)?;
    let trainers = table
        .field_mut("TrainerPoke")?
        .as_array_mut()
        .ok_or_else(|| data_error("TrainerPoke is not an array"))?;
    let mut lines = Vec::new();
    let trainer_count = trainers.len();
    for (done, trainer) in trainers.iter_mut().enumerate() {
        context.detail(done, trainer_count, || {
            format!("Trainer {} of {trainer_count}", done + 1)
        });
        if let Some(line) = edit_trainer(context, &plan, trainer)? {
            lines.push(line);
        }
    }
    context
        .bundles
        .write(BundleId::Masterdatas, path_ids::TRAINER_TABLE, &table)?;
    if context.settings.trainers != TrainerMode::Unchanged {
        for line in lines {
            context.log.line(SECTION, line);
        }
    }
    Ok(())
}
