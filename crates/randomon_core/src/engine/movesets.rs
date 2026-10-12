use crate::engine::detail;
use std::collections::{BTreeMap, HashSet};

use randomon_data::constants::{global, moves as m};
use randomon_data::{MoveCategory, MoveId, MoveLearnt, PokemonId, Type};
use randomon_java::JavaRng;
use randomon_settings::{MovesetsMod, Settings};

use crate::engine::pool::find_pokemon_in_pool_with_species_id;
use crate::engine::util::{contains_all, pick, remove_all};
use crate::error::{RandomizationError, Result};
use crate::handler::RomHandler;

struct MoveSets {
    valid: Vec<MoveId>,
    valid_damaging: Vec<MoveId>,
    valid_by_type: BTreeMap<Type, Vec<MoveId>>,
    valid_damaging_by_type: BTreeMap<Type, Vec<MoveId>>,
}

fn create_sets_of_moves<H: RomHandler + ?Sized>(handler: &H, no_broken: bool) -> Result<MoveSets> {
    let mut banned: HashSet<i32> = HashSet::new();
    if no_broken {
        banned.extend(handler.get_game_breaking_moves());
    }
    banned.extend(handler.get_hm_moves());
    banned.extend(handler.get_moves_banned_from_levelup());
    banned.extend(global::z_moves());
    banned.extend(handler.get_illegal_moves());
    let perfect_accuracy = handler.core().engine.perfect_accuracy;
    let data = &handler.core().data;
    let mut sets = MoveSets {
        valid: Vec::new(),
        valid_damaging: Vec::new(),
        valid_by_type: BTreeMap::new(),
        valid_damaging_by_type: BTreeMap::new(),
    };
    for id in data.move_ids().into_iter().flatten() {
        let mv = data.mv(id);
        if global::banned_random_move(mv.number) || banned.contains(&mv.number) {
            continue;
        }
        sets.valid.push(id);
        if let Some(kind) = mv.kind {
            sets.valid_by_type.entry(kind).or_default().push(id);
        }
        if !global::banned_for_damaging_move(mv.number) && mv.is_good_damaging(perfect_accuracy) {
            sets.valid_damaging.push(id);
            if let Some(kind) = mv.kind {
                sets.valid_damaging_by_type
                    .entry(kind)
                    .or_default()
                    .push(id);
            }
        }
    }
    let mut average_powers: BTreeMap<Type, f64> = BTreeMap::new();
    let mut total_average = 0.0;
    for (&kind, type_moves) in &sets.valid_by_type {
        let attacking_sum = type_moves.iter().fold(0i32, |sum, &id| {
            let mv = data.mv(id);
            if mv.power > 0 {
                (f64::from(sum) + mv.effective_power()) as i32
            } else {
                sum
            }
        });
        let average = f64::from(attacking_sum) / type_moves.len() as f64;
        average_powers.insert(kind, average);
        total_average += average;
    }
    total_average /= sets.valid_by_type.len() as f64;
    let min_average = total_average * 0.75;
    let max_average = total_average * 1.25;
    let mut random = handler.core().engine.random.clone();
    for (kind, average) in average_powers {
        let mut average_for_type = average;
        let type_moves = sets
            .valid_by_type
            .get_mut(&kind)
            .ok_or(RandomizationError::NullPointer("validTypeMoves.get"))?;
        let mut already_picked: Vec<MoveId> = Vec::new();
        let mut iterations = 0;
        while average_for_type < min_average && iterations < 10_000 {
            let threshold = average_for_type;
            let mut stronger: Vec<MoveId> = type_moves
                .iter()
                .copied()
                .filter(|&id| data.mv(id).effective_power() > threshold)
                .collect();
            if stronger.is_empty() {
                break;
            }
            if contains_all(&already_picked, &stronger) {
                already_picked = Vec::new();
            } else {
                remove_all(&mut stronger, &already_picked);
            }
            let extra = pick(&mut random, &stronger)?;
            average_for_type = (average_for_type * type_moves.len() as f64
                + data.mv(extra).effective_power())
                / (type_moves.len() + 1) as f64;
            type_moves.push(extra);
            already_picked.push(extra);
            iterations += 1;
        }
        iterations = 0;
        while average_for_type > max_average && iterations < 10_000 {
            let threshold = average_for_type;
            let mut weaker: Vec<MoveId> = type_moves
                .iter()
                .copied()
                .filter(|&id| data.mv(id).effective_power() < threshold)
                .collect();
            if weaker.is_empty() {
                break;
            }
            if contains_all(&already_picked, &weaker) {
                already_picked = Vec::new();
            } else {
                remove_all(&mut weaker, &already_picked);
            }
            let extra = pick(&mut random, &weaker)?;
            average_for_type = (average_for_type * type_moves.len() as f64
                + data.mv(extra).effective_power())
                / (type_moves.len() + 1) as f64;
            type_moves.push(extra);
            already_picked.push(extra);
            iterations += 1;
        }
    }
    Ok(sets)
}

fn check_for_unused_move<H: RomHandler + ?Sized>(
    handler: &H,
    list: &[MoveId],
    already_used: &[i32],
) -> bool {
    let data = &handler.core().data;
    list.iter()
        .any(|&id| !already_used.contains(&data.mv(id).number))
}

fn preferred_type<H: RomHandler + ?Sized>(
    handler: &H,
    pokemon: PokemonId,
    random: &mut impl JavaRng,
) -> Option<Type> {
    let pk = handler.core().data.pk(pokemon);
    let picked = random.next_double();
    if (pk.primary_type == Some(Type::Normal) && pk.secondary_type.is_some())
        || pk.secondary_type == Some(Type::Normal)
    {
        let other = if pk.primary_type == Some(Type::Normal) {
            pk.secondary_type
        } else {
            pk.primary_type
        };
        if picked < 0.1 {
            Some(Type::Normal)
        } else if picked < 0.4 {
            other
        } else {
            None
        }
    } else if pk.secondary_type.is_some() {
        if picked < 0.2 {
            pk.primary_type
        } else if picked < 0.4 {
            pk.secondary_type
        } else {
            None
        }
    } else if picked < 0.4 {
        pk.primary_type
    } else {
        None
    }
}

fn choose_pick_list<'a, H: RomHandler + ?Sized>(
    handler: &H,
    sets: &'a MoveSets,
    attempt_damaging: bool,
    type_of_move: Option<Type>,
    learnt: &[i32],
    attack_ratio: f64,
    random: &mut impl JavaRng,
) -> Vec<MoveId> {
    let mut pick_list: &'a [MoveId] = &sets.valid;
    if attempt_damaging {
        if let Some(kind) = type_of_move {
            match sets.valid_damaging_by_type.get(&kind) {
                Some(typed) if check_for_unused_move(handler, typed, learnt) => pick_list = typed,
                _ => {
                    if check_for_unused_move(handler, &sets.valid_damaging, learnt) {
                        pick_list = &sets.valid_damaging;
                    }
                }
            }
        } else if check_for_unused_move(handler, &sets.valid_damaging, learnt) {
            pick_list = &sets.valid_damaging;
        }
        let forced = if random.next_double() < attack_ratio {
            MoveCategory::Physical
        } else {
            MoveCategory::Special
        };
        let data = &handler.core().data;
        let filtered: Vec<MoveId> = pick_list
            .iter()
            .copied()
            .filter(|&id| data.mv(id).category == forced)
            .collect();
        if !filtered.is_empty() && check_for_unused_move(handler, &filtered, learnt) {
            return filtered;
        }
    } else if let Some(kind) = type_of_move
        && let Some(typed) = sets.valid_by_type.get(&kind)
        && check_for_unused_move(handler, typed, learnt)
    {
        pick_list = typed;
    }
    pick_list.to_vec()
}

fn pick_unlearnt_move<H: RomHandler + ?Sized>(
    handler: &H,
    pick_list: &[MoveId],
    learnt: &[i32],
    random: &mut impl JavaRng,
) -> Result<i32> {
    let data = &handler.core().data;
    let unlearnt: Vec<MoveId> = pick_list
        .iter()
        .copied()
        .filter(|&id| !learnt.contains(&data.mv(id).number))
        .collect();
    let candidates = if unlearnt.is_empty() {
        pick_list
    } else {
        &unlearnt
    };
    Ok(data.mv(pick(random, candidates)?).number)
}

pub fn randomize_moves_learnt<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let type_themed = settings.movesets_mod == MovesetsMod::RandomPreferSameType;
    let force_starting_moves =
        handler.supports_four_starting_moves() && settings.start_with_guaranteed_moves;
    let force_starting_count = settings.guaranteed_move_count;
    let good_damaging_percentage = if settings.movesets_force_good_damaging {
        f64::from(settings.movesets_good_damaging_percent) / 100.0
    } else {
        0.0
    };
    let evolution_moves_for_all = settings.evolution_moves_for_all;
    let mut movesets = handler.get_moves_learnt();
    let sets = create_sets_of_moves(handler, settings.block_broken_moveset_moves)?;
    let mut random = handler.core().engine.random.clone();
    let numbers: Vec<i32> = movesets.keys().copied().collect();
    let moveset_count = numbers.len();
    for (done, number) in numbers.into_iter().enumerate() {
        detail::counted(handler, done, moveset_count, "Moveset");
        let Some(pokemon) = find_pokemon_in_pool_with_species_id(
            handler,
            &handler.core().engine.pool.main_incl_formes,
            number,
        ) else {
            continue;
        };
        let attack_ratio = handler
            .core()
            .data
            .pk(pokemon)
            .attack_special_attack_ratio();
        let mut moves = movesets.remove(&number).unwrap_or_default();
        if moves.is_empty() {
            movesets.insert(number, moves);
            continue;
        }
        if force_starting_moves {
            let level_one_count = moves.iter().filter(|learnt| learnt.level == 1).count() as i32;
            for _ in level_one_count..force_starting_count {
                moves.insert(
                    0,
                    MoveLearnt {
                        move_number: 0,
                        level: 1,
                    },
                );
            }
        }
        if evolution_moves_for_all {
            let first = moves
                .first()
                .ok_or(RandomizationError::IndexOutOfBounds("moves.get(0)"))?;
            if first.level != 0 {
                moves.insert(
                    0,
                    MoveLearnt {
                        move_number: 0,
                        level: 0,
                    },
                );
            }
        }
        let pk = handler.core().data.pk(pokemon);
        if pk.actually_cosmetic {
            let base = pk
                .base_forme
                .ok_or(RandomizationError::NullPointer("baseForme"))?;
            let base_number = handler.core().data.pk(base).number;
            let base_moves = movesets
                .get(&base_number)
                .ok_or(RandomizationError::NullPointer("movesets.get"))?;
            for (index, learnt) in moves.iter_mut().enumerate() {
                learnt.move_number = base_moves
                    .get(index)
                    .ok_or(RandomizationError::IndexOutOfBounds("base forme moveset"))?
                    .move_number;
            }
            movesets.insert(number, moves);
            continue;
        }
        let first = moves
            .first()
            .ok_or(RandomizationError::IndexOutOfBounds("moves.get(0)"))?;
        let mut level_one_index = if first.level == 1 { 0 } else { 1 };
        while level_one_index < moves.len() && moves[level_one_index].level == 1 {
            level_one_index += 1;
        }
        level_one_index = level_one_index.saturating_sub(1);
        let mut good_damaging_left =
            randomon_java::math::round_double(good_damaging_percentage * moves.len() as f64) as i32;
        let mut learnt: Vec<i32> = Vec::new();
        let mut level_one_attacking_move = 0;
        for index in 0..moves.len() {
            let attempt_damaging = index == level_one_index || good_damaging_left > 0;
            let type_of_move = if type_themed {
                preferred_type(handler, pokemon, &mut random)
            } else {
                None
            };
            let pick_list = choose_pick_list(
                handler,
                &sets,
                attempt_damaging,
                type_of_move,
                &learnt,
                attack_ratio,
                &mut random,
            );
            let chosen = pick_unlearnt_move(handler, &pick_list, &learnt, &mut random)?;
            if index == level_one_index {
                level_one_attacking_move = chosen;
            } else {
                good_damaging_left -= 1;
            }
            learnt.push(chosen);
        }
        randomon_java::shuffle(&mut learnt, &mut random);
        let current = *learnt
            .get(level_one_index)
            .ok_or(RandomizationError::IndexOutOfBounds("learnt.get(lv1index)"))?;
        if current != level_one_attacking_move
            && let Some(position) = learnt
                .iter()
                .position(|&chosen| chosen == level_one_attacking_move)
        {
            learnt[position] = learnt[level_one_index];
            learnt[level_one_index] = level_one_attacking_move;
        }
        for (index, &chosen) in learnt.iter().enumerate() {
            moves[index].move_number = chosen;
            if index == level_one_index {
                moves[index].level = 1;
            }
        }
        movesets.insert(number, moves);
    }
    handler.set_moves_learnt(&movesets)
}

pub fn randomize_egg_moves<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let type_themed = settings.movesets_mod == MovesetsMod::RandomPreferSameType;
    let good_damaging_percentage = if settings.movesets_force_good_damaging {
        f64::from(settings.movesets_good_damaging_percent) / 100.0
    } else {
        0.0
    };
    let mut movesets = handler.get_egg_moves();
    let sets = create_sets_of_moves(handler, settings.block_broken_moveset_moves)?;
    let mut random = handler.core().engine.random.clone();
    let numbers: Vec<i32> = movesets.keys().copied().collect();
    let moveset_count = numbers.len();
    for (done, number) in numbers.into_iter().enumerate() {
        detail::counted(handler, done, moveset_count, "Moveset");
        let Some(pokemon) = find_pokemon_in_pool_with_species_id(
            handler,
            &handler.core().engine.pool.main_incl_formes,
            number,
        ) else {
            continue;
        };
        let attack_ratio = handler
            .core()
            .data
            .pk(pokemon)
            .attack_special_attack_ratio();
        let mut moves = movesets.remove(&number).unwrap_or_default();
        let pk = handler.core().data.pk(pokemon);
        if pk.actually_cosmetic {
            let base = pk
                .base_forme
                .ok_or(RandomizationError::NullPointer("baseForme"))?;
            let base_number = handler.core().data.pk(base).number;
            let base_moves = movesets
                .get(&base_number)
                .ok_or(RandomizationError::NullPointer("movesets.get"))?;
            for (index, slot) in moves.iter_mut().enumerate() {
                *slot = *base_moves
                    .get(index)
                    .ok_or(RandomizationError::IndexOutOfBounds("base forme egg moves"))?;
            }
            movesets.insert(number, moves);
            continue;
        }
        let mut good_damaging_left =
            randomon_java::math::round_double(good_damaging_percentage * moves.len() as f64) as i32;
        let mut learnt: Vec<i32> = Vec::new();
        for _ in 0..moves.len() {
            let attempt_damaging = good_damaging_left > 0;
            let type_of_move = if type_themed {
                preferred_type(handler, pokemon, &mut random)
            } else {
                None
            };
            let pick_list = choose_pick_list(
                handler,
                &sets,
                attempt_damaging,
                type_of_move,
                &learnt,
                attack_ratio,
                &mut random,
            );
            let chosen = pick_unlearnt_move(handler, &pick_list, &learnt, &mut random)?;
            good_damaging_left -= 1;
            learnt.push(chosen);
        }
        randomon_java::shuffle(&mut learnt, &mut random);
        moves.copy_from_slice(&learnt);
        movesets.insert(number, moves);
    }
    handler.set_egg_moves(&movesets)
}

pub fn order_damaging_moves_by_damage<H: RomHandler + ?Sized>(handler: &mut H) -> Result<()> {
    let mut movesets = handler.get_moves_learnt();
    let mut random = handler.core().engine.random.clone();
    let data = &handler.core().data;
    for moves in movesets.values_mut() {
        let mut indices: Vec<usize> = Vec::new();
        let mut damaging: Vec<i32> = Vec::new();
        for (index, learnt) in moves.iter().enumerate() {
            if learnt.level == 0 {
                continue;
            }
            let mv = data
                .move_by_number(learnt.move_number)
                .ok_or(RandomizationError::NullPointer("allMoves.get"))?;
            if mv.power > 1 {
                indices.push(index);
                damaging.push(mv.number);
            }
        }
        randomon_java::shuffle(&mut damaging, &mut random);
        damaging.sort_by(|&first, &second| {
            let power = |number: i32| {
                data.move_by_number(number)
                    .map_or(0.0, |mv| mv.effective_power())
            };
            power(first).total_cmp(&power(second))
        });
        for (slot, &number) in damaging.iter().enumerate() {
            moves[indices[slot]].move_number = number;
        }
    }
    handler.set_moves_learnt(&movesets)
}

pub fn metronome_only_mode<H: RomHandler + ?Sized>(handler: &mut H) -> Result<()> {
    let mut movesets = handler.get_moves_learnt();
    for moves in movesets.values_mut() {
        if !moves.is_empty() {
            moves.clear();
            moves.push(MoveLearnt {
                move_number: m::METRONOME,
                level: 1,
            });
        }
    }
    handler.set_moves_learnt(&movesets)?;
    let mut trainers = handler.get_trainers()?;
    for trainer in &mut trainers {
        for tp in &mut trainer.pokemon {
            tp.reset_moves = true;
        }
    }
    handler.set_trainers(&trainers, false)?;
    let tm_moves = vec![m::METRONOME; handler.get_tm_moves().len()];
    handler.set_tm_moves(&tm_moves)?;
    if handler.has_move_tutors() {
        let tutor_moves = vec![m::METRONOME; handler.get_move_tutor_moves().len()];
        handler.set_move_tutor_moves(&tutor_moves)?;
    }
    let hms = handler.get_hm_moves();
    let data = &mut handler.core_mut().data;
    if let Some(metronome) = data
        .move_by_number(m::METRONOME)
        .map(|mv| MoveId::from_index(mv.number as usize))
    {
        data.mv_mut(metronome).pp = 40;
    }
    for hm in hms {
        if let Some(id) = data
            .move_by_number(hm)
            .map(|mv| MoveId::from_index(mv.number as usize))
        {
            data.mv_mut(id).pp = 0;
        }
    }
    Ok(())
}
