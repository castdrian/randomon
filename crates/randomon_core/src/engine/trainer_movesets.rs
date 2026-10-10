use randomon_data::constants::{abilities, global};
use randomon_data::move_synergy as synergy;
use randomon_data::{
    GameData, MoveCategory, MoveId, MoveStatChange, StatChangeMoveType, StatChangeType, StatusType,
    TrainerPokemon,
};
use randomon_java::JavaRng;
use randomon_settings::{EvolutionsMod, Settings};

use crate::engine::util::{distinct, pick, remove_all, remove_first};
use crate::error::{RandomizationError, Result};
use crate::handler::RomHandler;
use crate::state::TrainerMovesetCaches;

const MAX_ADDITIONAL_MOVES: i64 = 50_000_000;

fn ensure_caches<H: RomHandler + ?Sized>(handler: &mut H) {
    let mut caches = std::mem::take(&mut handler.core_mut().engine.moveset_caches);
    if caches.level_up_moves.is_none() {
        caches.level_up_moves = Some(handler.get_moves_learnt());
    }
    if caches.egg_moves.is_none() {
        caches.egg_moves = Some(handler.get_egg_moves());
    }
    if caches.tm_compat.is_none() {
        caches.tm_compat = Some(handler.get_tmhm_compatibility());
    }
    if caches.tm_moves.is_none() {
        caches.tm_moves = Some(handler.get_tm_moves());
    }
    if caches.tutor_compat.is_none() && handler.has_move_tutors() {
        caches.tutor_compat = Some(handler.get_move_tutor_compatibility());
    }
    if caches.tutor_moves.is_none() {
        caches.tutor_moves = Some(handler.get_move_tutor_moves());
    }
    handler.core_mut().engine.moveset_caches = caches;
}

fn move_id(data: &GameData, number: i32) -> Result<MoveId> {
    data.move_by_number(number)
        .map(|mv| MoveId::from_index(mv.number as usize))
        .ok_or(RandomizationError::NullPointer("moves.get"))
}

fn learns_compatible_move(
    mv: &randomon_data::Move,
    level: i32,
    probability: f64,
    random: &mut impl JavaRng,
) -> bool {
    if mv.power > 1
        && f64::from(level * 3) > mv.effective_power()
        && random.next_double() < probability
    {
        return true;
    }
    (mv.power <= 1 && random.next_int_bounded(100) < level) || random.next_int_bounded(200) < level
}

fn add_compatible_moves(
    data: &GameData,
    random: &mut impl JavaRng,
    pool: &mut Vec<MoveId>,
    level: i32,
    compat: &[bool],
    move_numbers: &[i32],
    probability: f64,
) -> Result<()> {
    for &number in move_numbers {
        let first_index = move_numbers
            .iter()
            .position(|&other| other == number)
            .unwrap_or(0);
        let compatible = *compat
            .get(first_index + 1)
            .ok_or(RandomizationError::IndexOutOfBounds("compatibility flags"))?;
        if !compatible {
            continue;
        }
        let id = move_id(data, number)?;
        if learns_compatible_move(data.mv(id), level, probability, random) {
            pool.push(id);
        }
    }
    Ok(())
}

pub fn move_selection_pool_at_level<H: RomHandler + ?Sized>(
    handler: &mut H,
    tp: &TrainerPokemon,
    cyclic_evolutions: bool,
) -> Result<Vec<MoveId>> {
    ensure_caches(handler);
    let forme = handler.get_alt_forme_of_pokemon(tp.pokemon, tp.forme);
    let alt_formes_differ = handler.alt_formes_can_have_different_evolutions();
    let has_tutors = handler.has_move_tutors();
    let mut random = handler.core().engine.random.clone();
    let core = handler.core();
    let data = &core.data;
    let caches: &TrainerMovesetCaches = &core.engine.moveset_caches;
    let level_up = caches
        .level_up_moves
        .as_ref()
        .ok_or(RandomizationError::NullPointer("allLevelUpMoves"))?;
    let forme_number = data.pk(forme).number;
    let own_moves = level_up
        .get(&forme_number)
        .ok_or(RandomizationError::NullPointer("allLevelUpMoves.get"))?;
    let mut pool: Vec<MoveId> = Vec::new();
    for learnt in own_moves {
        if (learnt.level <= tp.level && learnt.level != 0) || (learnt.level == 0 && tp.level >= 30)
        {
            pool.push(move_id(data, learnt.move_number)?);
        }
    }
    pool = distinct(&pool);
    if !cyclic_evolutions {
        let mut pre_evolution = if alt_formes_differ { forme } else { tp.pokemon };
        while let Some(earlier) = data.first_pre_evolution(pre_evolution) {
            pre_evolution = earlier;
            let number = data.pk(pre_evolution).number;
            let moves = level_up
                .get(&number)
                .ok_or(RandomizationError::NullPointer("allLevelUpMoves.get"))?;
            let mut added = Vec::new();
            for learnt in moves {
                if learnt.level <= tp.level && random.next_double() < 0.5 {
                    added.push(move_id(data, learnt.move_number)?);
                }
            }
            pool.extend(distinct(&added));
        }
    }
    let tm_compat_map = caches
        .tm_compat
        .as_ref()
        .ok_or(RandomizationError::NullPointer("allTMCompat"))?;
    let tm_compat = tm_compat_map
        .get(&forme)
        .ok_or(RandomizationError::NullPointer("allTMCompat.get"))?;
    let tm_moves = caches.tm_moves.as_deref().unwrap_or_default();
    add_compatible_moves(
        data,
        &mut random,
        &mut pool,
        tp.level,
        tm_compat,
        tm_moves,
        0.6,
    )?;
    if has_tutors {
        let tutor_map = caches
            .tutor_compat
            .as_ref()
            .ok_or(RandomizationError::NullPointer("allTutorCompat"))?;
        let tutor_compat = tutor_map
            .get(&forme)
            .ok_or(RandomizationError::NullPointer("allTutorCompat.get"))?;
        let tutor_moves = caches.tutor_moves.as_deref().unwrap_or_default();
        add_compatible_moves(
            data,
            &mut random,
            &mut pool,
            tp.level,
            tutor_compat,
            tutor_moves,
            0.6,
        )?;
    }
    if !cyclic_evolutions {
        let mut first_evolution = if alt_formes_differ { forme } else { tp.pokemon };
        while let Some(earlier) = data.first_pre_evolution(first_evolution) {
            first_evolution = earlier;
        }
        let egg_moves = caches
            .egg_moves
            .as_ref()
            .ok_or(RandomizationError::NullPointer("allEggMoves"))?;
        if let Some(eggs) = egg_moves.get(&data.pk(first_evolution).number) {
            for &egg in eggs {
                if random.next_double() < 0.1 {
                    pool.push(move_id(data, egg)?);
                }
            }
        }
    }
    Ok(distinct(&pool))
}

fn fill_moves(tp: &mut TrainerPokemon, data: &GameData, moves: &[MoveId]) {
    for slot in 0..4 {
        tp.moves[slot] = moves.get(slot).map_or(0, |&id| data.mv(id).number);
    }
}

fn add_biased(target: &mut Vec<MoveId>, source: &[MoveId], bias: f64) {
    let mut i = 0usize;
    while (i as f64) < bias * source.len() as f64 {
        target.push(source[i % source.len()]);
        i += 1;
    }
}

fn shuffled(mut list: Vec<MoveId>, random: &mut impl JavaRng) -> Vec<MoveId> {
    randomon_java::shuffle(&mut list, random);
    list
}

fn without_moves(list: &[MoveId], removals: &[MoveId]) -> Vec<MoveId> {
    let mut result = list.to_vec();
    for removal in removals {
        remove_first(&mut result, removal);
    }
    result
}

fn soft_synergy_error(_: synergy::MissingEffectivenessTable) -> RandomizationError {
    RandomizationError::NullPointer("Effectiveness.against")
}

pub fn pick_trainer_movesets<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let cyclic = settings.evolutions_mod == EvolutionsMod::RandomEveryLevel;
    let double_battle_mode = settings.double_battle_mode;
    let mut trainers = handler.get_trainers()?;
    let generation = handler.generation_of_pokemon();
    let effectiveness_updated = handler.is_effectiveness_updated();
    let perfect_accuracy = handler.core().engine.perfect_accuracy;
    let mut random = handler.core().engine.random.clone();
    for trainer in &mut trainers {
        trainer.set_pokemon_have_custom_moves(true);
        let important = trainer.is_important();
        let boss = trainer.is_boss();
        for slot in 0..trainer.pokemon.len() {
            let mut tp = trainer.pokemon[slot].clone();
            tp.reset_moves = false;
            let pool = move_selection_pool_at_level(handler, &tp, cyclic)?;
            let ability = handler.get_ability_for_trainer_pokemon(&tp);
            let pk_id = handler.get_alt_forme_of_pokemon(tp.pokemon, tp.forme);
            let data = &handler.core().data;
            let mut moves_at_level =
                trim_move_list(data, &mut tp, pool, double_battle_mode, ability);
            if moves_at_level.is_empty() {
                trainer.pokemon[slot] = tp;
                continue;
            }
            let trainer_type_modifier = if important {
                1.5
            } else if boss {
                2.0
            } else {
                1.0
            };
            let move_pool_size_modifier = moves_at_level.len() as f64 / 10.0;
            let bonus = trainer_type_modifier * move_pool_size_modifier;
            let atk_spatk_ratio_modifier = 0.75;
            let stab_bias = 0.25 * bonus;
            let hard_ability_bias = bonus;
            let soft_ability_bias = 0.5 * bonus;
            let stat_bias = 0.5 * bonus;
            let soft_move_bias = 0.25 * bonus;
            let hard_move_bias = bonus;
            let soft_move_anti_bias = 0.5;
            let pk = data.pk(pk_id);
            let stab: Vec<MoveId> = moves_at_level
                .iter()
                .copied()
                .filter(|&id| {
                    data.mv(id).kind == pk.primary_type
                        && data.mv(id).category != MoveCategory::Status
                })
                .collect();
            let stab = shuffled(stab, &mut random);
            add_biased(&mut moves_at_level, &stab, stab_bias);
            if pk.secondary_type.is_some() {
                let stab: Vec<MoveId> = moves_at_level
                    .iter()
                    .copied()
                    .filter(|&id| {
                        data.mv(id).kind == pk.secondary_type
                            && data.mv(id).category != MoveCategory::Status
                    })
                    .collect();
                let stab = shuffled(stab, &mut random);
                add_biased(&mut moves_at_level, &stab, stab_bias);
            }
            let hard_ability = shuffled(
                synergy::hard_ability_move_synergy(
                    data,
                    ability,
                    pk.primary_type,
                    pk.secondary_type,
                    &moves_at_level,
                    generation,
                    perfect_accuracy,
                ),
                &mut random,
            );
            add_biased(&mut moves_at_level, &hard_ability, hard_ability_bias);
            let soft_ability = shuffled(
                synergy::soft_ability_move_synergy(
                    data,
                    ability,
                    &moves_at_level,
                    pk.primary_type,
                    pk.secondary_type,
                ),
                &mut random,
            );
            add_biased(&mut moves_at_level, &soft_ability, soft_ability_bias);
            let soft_anti = synergy::soft_ability_move_anti_synergy(data, ability, &moves_at_level);
            let without_soft_anti = without_moves(&moves_at_level, &soft_anti);
            if !without_soft_anti.is_empty() {
                moves_at_level = without_soft_anti;
            }
            let distinct_moves = distinct(&moves_at_level);
            if distinct_moves.len() <= 4 {
                fill_moves(&mut tp, data, &distinct_moves);
                trainer.pokemon[slot] = tp;
                continue;
            }
            let stat_synergy = shuffled(
                synergy::stat_move_synergy(data, pk, &moves_at_level),
                &mut random,
            );
            add_biased(&mut moves_at_level, &stat_synergy, stat_bias);
            let stat_anti = synergy::stat_move_anti_synergy(data, pk, &moves_at_level);
            let without_stat_anti = without_moves(&moves_at_level, &stat_anti);
            if !without_stat_anti.is_empty() {
                moves_at_level = without_stat_anti;
            }
            let distinct_moves = distinct(&moves_at_level);
            if distinct_moves.len() <= 4 {
                fill_moves(&mut tp, data, &distinct_moves);
                trainer.pokemon[slot] = tp;
                continue;
            }
            let mut atk_spatk_ratio = f64::from(pk.attack) / f64::from(pk.spatk);
            match ability {
                abilities::HUGE_POWER | abilities::PURE_POWER => atk_spatk_ratio *= 2.0,
                abilities::HUSTLE | abilities::GORILLA_TACTICS => atk_spatk_ratio *= 1.5,
                abilities::MOXIE => atk_spatk_ratio *= 1.1,
                abilities::SOUL_HEART => atk_spatk_ratio *= 0.9,
                _ => {}
            }
            let physical: Vec<MoveId> = moves_at_level
                .iter()
                .copied()
                .filter(|&id| data.mv(id).category == MoveCategory::Physical)
                .collect();
            let special: Vec<MoveId> = moves_at_level
                .iter()
                .copied()
                .filter(|&id| data.mv(id).category == MoveCategory::Special)
                .collect();
            if atk_spatk_ratio < 1.0 && !special.is_empty() {
                atk_spatk_ratio = 1.0 / atk_spatk_ratio;
                let accepted = atk_spatk_ratio_modifier * atk_spatk_ratio;
                let additional =
                    i64::from((physical.len() as f64 * accepted) as i32) - special.len() as i64;
                add_random_copies(&mut moves_at_level, &special, additional, &mut random)?;
            } else if !physical.is_empty() {
                let accepted = atk_spatk_ratio_modifier * atk_spatk_ratio;
                let additional =
                    i64::from((special.len() as f64 * accepted) as i32) - physical.len() as i64;
                add_random_copies(&mut moves_at_level, &physical, additional, &mut random)?;
            }
            let picked = pick_moves(
                data,
                &mut random,
                moves_at_level,
                MovePickBiases {
                    hard_move_bias,
                    soft_move_bias,
                    soft_move_anti_bias,
                },
                generation,
                effectiveness_updated,
                perfect_accuracy,
            )?;
            fill_moves(&mut tp, data, &picked);
            trainer.pokemon[slot] = tp;
        }
    }
    handler.set_trainers(&trainers, false)
}

fn add_random_copies(
    target: &mut Vec<MoveId>,
    source: &[MoveId],
    additional: i64,
    random: &mut impl JavaRng,
) -> Result<()> {
    if additional > MAX_ADDITIONAL_MOVES {
        return Err(RandomizationError::randomization(
            "java.lang.OutOfMemoryError: Java heap space",
        ));
    }
    for _ in 0..additional.max(0) {
        target.push(pick(random, source)?);
    }
    Ok(())
}

struct MovePickBiases {
    hard_move_bias: f64,
    soft_move_bias: f64,
    soft_move_anti_bias: f64,
}

fn pick_moves(
    data: &GameData,
    random: &mut impl JavaRng,
    mut moves_at_level: Vec<MoveId>,
    biases: MovePickBiases,
    generation: i32,
    effectiveness_updated: bool,
    perfect_accuracy: i32,
) -> Result<Vec<MoveId>> {
    let mut picked: Vec<MoveId> = Vec::new();
    for i in 1..=4usize {
        if i == 4 {
            let dependents: Vec<MoveId> = distinct(
                &moves_at_level
                    .iter()
                    .copied()
                    .filter(|&id| global::REQUIRES_OTHER_MOVE.contains(&data.mv(id).number))
                    .collect::<Vec<_>>(),
            );
            for dependent in dependents {
                let required =
                    synergy::requires_other_move(data, data.mv(dependent), &moves_at_level);
                let has_required = required.iter().any(|move_id| picked.contains(move_id));
                if !has_required {
                    moves_at_level.retain(|&id| id != dependent);
                }
            }
        }
        let pick_from = if i == 1 {
            let good: Vec<MoveId> = moves_at_level
                .iter()
                .copied()
                .filter(|&id| data.mv(id).is_good_damaging(perfect_accuracy))
                .collect();
            if good.is_empty() {
                moves_at_level.clone()
            } else {
                good
            }
        } else {
            moves_at_level.clone()
        };
        let chosen = pick(random, &pick_from)?;
        picked.push(chosen);
        if i == 4 {
            break;
        }
        moves_at_level.retain(|&id| id != chosen);
        let hard_anti = synergy::hard_move_anti_synergy(data, data.mv(chosen), &moves_at_level);
        remove_all(&mut moves_at_level, &hard_anti);
        let remaining = distinct(&moves_at_level);
        if remaining.len() <= 4 - i {
            picked.extend(remaining);
            break;
        }
        let hard_synergy = synergy::move_synergy(data, data.mv(chosen), &moves_at_level);
        let hard_synergy = shuffled(hard_synergy, random);
        add_biased(&mut moves_at_level, &hard_synergy, biases.hard_move_bias);
        let soft_synergy = synergy::soft_move_synergy(
            data,
            data.mv(chosen),
            &moves_at_level,
            generation,
            effectiveness_updated,
        )
        .map_err(soft_synergy_error)?;
        let soft_synergy = shuffled(soft_synergy, random);
        add_biased(&mut moves_at_level, &soft_synergy, biases.soft_move_bias);
        let soft_anti = shuffled(
            synergy::soft_move_anti_synergy(data, data.mv(chosen), &moves_at_level),
            random,
        );
        let mut j = 0usize;
        while (j as f64) < biases.soft_move_anti_bias * soft_anti.len() as f64 {
            if distinct(&moves_at_level).len() <= 4 - i {
                break;
            }
            remove_first(&mut moves_at_level, &soft_anti[j % soft_anti.len()]);
            j += 1;
        }
        let remaining = distinct(&moves_at_level);
        if remaining.len() <= 4 - i {
            picked.extend(remaining);
            break;
        }
    }
    Ok(picked)
}

fn trim_move_list(
    data: &GameData,
    tp: &mut TrainerPokemon,
    moves: Vec<MoveId>,
    double_battle_mode: bool,
    ability: i32,
) -> Vec<MoveId> {
    if moves.len() <= 4 {
        fill_moves(tp, data, &moves);
        return Vec::new();
    }
    let mut moves: Vec<MoveId> = moves
        .into_iter()
        .filter(|&id| {
            let number = data.mv(id).number;
            !global::USELESS_MOVES.contains(&number)
                && (double_battle_mode || !global::DOUBLE_BATTLE_MOVES.contains(&number))
        })
        .collect();
    if moves.len() <= 4 {
        fill_moves(tp, data, &moves);
        return Vec::new();
    }
    let obsolete = obsolete_moves(data, &moves);
    remove_all(&mut moves, &obsolete);
    if moves.len() <= 4 {
        fill_moves(tp, data, &moves);
        return Vec::new();
    }
    let dependents: Vec<MoveId> = moves
        .iter()
        .copied()
        .filter(|&id| global::REQUIRES_OTHER_MOVE.contains(&data.mv(id).number))
        .collect();
    for dependent in dependents {
        if synergy::requires_other_move(data, data.mv(dependent), &moves).is_empty() {
            remove_first(&mut moves, &dependent);
        }
    }
    if moves.len() <= 4 {
        fill_moves(tp, data, &moves);
        return Vec::new();
    }
    let mut without_hard_anti = moves.clone();
    remove_all(
        &mut without_hard_anti,
        &synergy::hard_ability_move_anti_synergy(data, ability, &moves),
    );
    if !without_hard_anti.is_empty() {
        moves = without_hard_anti;
    }
    if moves.len() <= 4 {
        fill_moves(tp, data, &moves);
        return Vec::new();
    }
    moves
}

fn active_stat_changes(changes: &[MoveStatChange; 3]) -> Vec<MoveStatChange> {
    changes
        .iter()
        .copied()
        .filter(|change| change.kind != StatChangeType::None)
        .collect()
}

fn obsolete_moves(data: &GameData, moves: &[MoveId]) -> Vec<MoveId> {
    let mut obsoleted: Vec<MoveId> = Vec::new();
    for &id in moves {
        let mv = data.mv(id);
        if global::CANNOT_OBSOLETE_MOVES.contains(&mv.number) {
            continue;
        }
        if mv.power > 0 {
            for &other_id in moves {
                let other = data.mv(other_id);
                let obsoletes = !global::CANNOT_BE_OBSOLETED_MOVES.contains(&other.number)
                    && mv.kind == other.kind
                    && ((((mv.stat_change_move_type == other.stat_change_move_type
                        && mv.stat_changes[0] == other.stat_changes[0])
                        || (other.stat_change_move_type == StatChangeMoveType::NoneOrUnknown
                            && mv.has_beneficial_stat_change()))
                        && mv.absorb_percent >= other.absorb_percent
                        && !mv.is_charge_move
                        && !mv.is_recharge_move)
                        || other.effective_power() <= 30.0)
                    && mv.hitratio >= other.hitratio
                    && mv.category == other.category
                    && mv.priority >= other.priority
                    && other.power > 0
                    && mv.effective_power() > other.effective_power();
                if obsoletes {
                    obsoleted.push(other_id);
                }
            }
        } else if mv.stat_change_move_type == StatChangeMoveType::NoDamageUser
            || mv.stat_change_move_type == StatChangeMoveType::NoDamageTarget
        {
            let first_changes = active_stat_changes(&mv.stat_changes);
            for &other_id in moves {
                let other = data.mv(other_id);
                let candidate = other_id != id
                    && other.power <= 0
                    && other.stat_change_move_type == mv.stat_change_move_type
                    && (other.status_type == mv.status_type
                        || other.status_type == StatusType::None);
                if !candidate {
                    continue;
                }
                let mut second_changes = active_stat_changes(&other.stat_changes);
                if second_changes.len() > first_changes.len() {
                    continue;
                }
                let first_filtered: Vec<MoveStatChange> = first_changes
                    .iter()
                    .copied()
                    .filter(|change| !second_changes.contains(change))
                    .collect();
                second_changes.retain(|change| !first_changes.contains(change));
                if !first_filtered.is_empty() && second_changes.is_empty() {
                    if !global::CANNOT_BE_OBSOLETED_MOVES.contains(&other.number) {
                        obsoleted.push(other_id);
                    }
                    continue;
                }
                if first_filtered.is_empty() && second_changes.is_empty() {
                    continue;
                }
                let mut maybe_better = false;
                for first in &first_filtered {
                    let mut can_still_be_better = false;
                    for second in &second_changes {
                        if first.kind == second.kind {
                            can_still_be_better = true;
                            if (mv.stat_change_move_type == StatChangeMoveType::NoDamageUser
                                && first.stages > second.stages)
                                || (mv.stat_change_move_type == StatChangeMoveType::NoDamageTarget
                                    && first.stages < second.stages)
                            {
                                maybe_better = true;
                            } else {
                                can_still_be_better = false;
                            }
                        }
                    }
                    if !can_still_be_better {
                        maybe_better = false;
                        break;
                    }
                }
                if maybe_better && !global::CANNOT_BE_OBSOLETED_MOVES.contains(&other.number) {
                    obsoleted.push(other_id);
                }
            }
        }
    }
    distinct(&obsoleted)
}
