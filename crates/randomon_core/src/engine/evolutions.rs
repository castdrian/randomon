use std::collections::{HashMap, HashSet, VecDeque};

use randomon_data::constants::{global, species};
use randomon_data::{Evolution, EvolutionId, EvolutionType, EvolutionUpdate, GameData, PokemonId};
use randomon_settings::{AbilitiesMod, Settings};

use crate::engine::choose::{StrengthWindow, first_non_empty};
use crate::engine::pool::{
    check_pokemon_restrictions, find_pokemon_in_pool_with_species_id, non_cosmetic,
};
use crate::engine::util::{contains_all, pick, random_index, remove_all, retain_all};
use crate::error::{RandomizationError, Result};
use crate::handler::RomHandler;
use crate::state::EvolutionUpdates;

pub fn evolutionary_relatives(data: &GameData, pokemon: PokemonId) -> Vec<PokemonId> {
    let mut relatives: Vec<PokemonId> = Vec::new();
    let mut queue: VecDeque<PokemonId> = VecDeque::from([pokemon]);
    while let Some(current) = queue.pop_front() {
        let neighbours = data
            .evolution_targets(current)
            .into_iter()
            .chain(data.evolution_sources(current));
        for neighbour in neighbours {
            if neighbour != pokemon && !relatives.contains(&neighbour) {
                relatives.push(neighbour);
                queue.push_back(neighbour);
            }
        }
    }
    relatives
}

pub fn pre_evolution_chain(data: &GameData, pokemon: PokemonId) -> Vec<PokemonId> {
    let mut chain = Vec::new();
    let mut current = pokemon;
    while let Some(earlier) = data.first_pre_evolution(current) {
        if earlier == pokemon || chain.contains(&earlier) {
            break;
        }
        chain.push(earlier);
        current = earlier;
    }
    chain
}

pub fn earliest_pre_evolution(data: &GameData, pokemon: PokemonId) -> PokemonId {
    pre_evolution_chain(data, pokemon)
        .last()
        .copied()
        .unwrap_or(pokemon)
}

fn is_cyclic(
    data: &GameData,
    pokemon: PokemonId,
    visited: &mut HashSet<PokemonId>,
    stack: &mut HashSet<PokemonId>,
) -> bool {
    if !visited.contains(&pokemon) {
        visited.insert(pokemon);
        stack.insert(pokemon);
        for target in data.evolution_targets(pokemon) {
            if (!visited.contains(&target) && is_cyclic(data, target, visited, stack))
                || stack.contains(&target)
            {
                return true;
            }
        }
    }
    stack.remove(&pokemon);
    false
}

fn evo_cycle_check(data: &mut GameData, from: PokemonId, to: PokemonId) -> bool {
    let temporary =
        data.temporary_evolution(Evolution::new(from, to, false, EvolutionType::None, 0));
    data.evolutions_from_mut(from).push(temporary);
    let cyclic = is_cyclic(data, from, &mut HashSet::new(), &mut HashSet::new());
    data.remove_first_equal_from(from, temporary);
    cyclic
}

pub fn num_pre_evolutions(data: &GameData, pokemon: PokemonId, max_interested: i32) -> i32 {
    count_pre_evolutions(data, pokemon, 0, max_interested)
}

fn count_pre_evolutions(
    data: &GameData,
    pokemon: PokemonId,
    depth: i32,
    max_interested: i32,
) -> i32 {
    if data.evolutions_to(pokemon).is_empty() {
        return 0;
    }
    if depth == max_interested - 1 {
        return 1;
    }
    data.evolution_sources(pokemon)
        .into_iter()
        .map(|source| count_pre_evolutions(data, source, depth + 1, max_interested) + 1)
        .fold(0, i32::max)
}

pub fn num_evolutions(data: &GameData, pokemon: PokemonId, max_interested: i32) -> i32 {
    count_evolutions(data, pokemon, 0, max_interested)
}

fn count_evolutions(data: &GameData, pokemon: PokemonId, depth: i32, max_interested: i32) -> i32 {
    if data.evolutions_from(pokemon).is_empty() {
        return 0;
    }
    if depth == max_interested - 1 {
        return 1;
    }
    data.evolution_targets(pokemon)
        .into_iter()
        .map(|target| count_evolutions(data, target, depth + 1, max_interested) + 1)
        .fold(0, i32::max)
}

pub fn related_pokemon(data: &GameData, original: PokemonId) -> Vec<PokemonId> {
    let mut results = vec![original];
    let mut queue = VecDeque::from([original]);
    while let Some(check) = queue.pop_front() {
        for target in data.evolution_targets(check) {
            if !results.contains(&target) {
                results.push(target);
                queue.push_back(target);
            }
        }
        for source in data.evolution_sources(check) {
            if !results.contains(&source) {
                results.push(source);
                queue.push_back(source);
            }
        }
    }
    results
}

pub fn fully_evolve<H: RomHandler + ?Sized>(
    handler: &mut H,
    pokemon: PokemonId,
    trainer_index: i32,
) -> Result<PokemonId> {
    if handler.core().engine.fully_evolved_random_seed == -1 {
        let mut random = handler.core().engine.random.clone();
        let seed = random_index(&mut random, global::LARGEST_NUMBER_OF_SPLIT_EVOS as usize)? as i32;
        handler.core_mut().engine.fully_evolved_random_seed = seed;
    }
    let seed = handler.core().engine.fully_evolved_random_seed;
    let data = &handler.core().data;
    let mut current = pokemon;
    let mut seen = vec![current];
    loop {
        let targets = data.evolution_targets(current);
        if targets.is_empty() || targets.iter().any(|target| seen.contains(target)) {
            break;
        }
        let index = seed
            .wrapping_add(trainer_index)
            .wrapping_rem(targets.len() as i32);
        current = targets[usize::try_from(index)
            .map_err(|_| RandomizationError::IndexOutOfBounds("split evolution"))?];
        seen.push(current);
    }
    Ok(current)
}

pub fn final_evos(data: &GameData, pokemon: PokemonId) -> Vec<PokemonId> {
    let mut finals = Vec::new();
    let mut seen = HashSet::from([pokemon]);
    let mut stack = vec![pokemon];
    while let Some(current) = stack.pop() {
        let targets = data.evolution_targets(current);
        if targets.is_empty() {
            finals.push(current);
        }
        for target in targets.into_iter().rev() {
            if seen.insert(target) {
                stack.push(target);
            }
        }
    }
    finals
}

pub fn set_evo_chain_as_illegal(
    data: &GameData,
    pokemon: PokemonId,
    illegal: &mut Vec<PokemonId>,
    will_force_evolve: bool,
) {
    illegal.extend(reachable(data, pokemon, |data, id| {
        data.evolution_sources(id)
    }));
    if will_force_evolve {
        illegal.extend(reachable(data, pokemon, |data, id| {
            data.evolution_targets(id)
        }));
    }
}

fn reachable(
    data: &GameData,
    start: PokemonId,
    next: impl Fn(&GameData, PokemonId) -> Vec<PokemonId>,
) -> Vec<PokemonId> {
    let mut found = Vec::new();
    let mut seen = HashSet::from([start]);
    let mut stack = vec![start];
    while let Some(current) = stack.pop() {
        for neighbour in next(data, current) {
            if seen.insert(neighbour) {
                found.push(neighbour);
                stack.push(neighbour);
            }
        }
    }
    found
}

fn evolution_update(
    data: &GameData,
    evo: EvolutionId,
    kind: EvolutionType,
    extra: String,
    condensed: bool,
    additional: bool,
) -> EvolutionUpdate {
    let evolution = data.evo(evo);
    EvolutionUpdate {
        from: evolution.from,
        to: evolution.to,
        from_name: data.full_name(evolution.from),
        to_name: data.full_name(evolution.to),
        kind,
        extra_info: extra,
        condensed,
        additional,
    }
}

fn insert_update(updates: &mut EvolutionUpdates, update: EvolutionUpdate) {
    updates.entry(update.sort_key()).or_insert(update);
}

pub fn add_evo_update_level(data: &GameData, updates: &mut EvolutionUpdates, evo: EvolutionId) {
    let level = data.evo(evo).extra_info.to_string();
    insert_update(
        updates,
        evolution_update(data, evo, EvolutionType::Level, level, false, false),
    );
}

pub fn add_evo_update_stone(
    data: &GameData,
    updates: &mut EvolutionUpdates,
    evo: EvolutionId,
    item: &str,
) {
    insert_update(
        updates,
        evolution_update(
            data,
            evo,
            EvolutionType::Stone,
            item.to_owned(),
            false,
            false,
        ),
    );
}

pub fn add_evo_update_happiness(data: &GameData, updates: &mut EvolutionUpdates, evo: EvolutionId) {
    insert_update(
        updates,
        evolution_update(
            data,
            evo,
            EvolutionType::Happiness,
            String::new(),
            false,
            false,
        ),
    );
}

pub fn add_evo_update_held_item(
    data: &GameData,
    updates: &mut EvolutionUpdates,
    evo: EvolutionId,
    item: &str,
) {
    insert_update(
        updates,
        evolution_update(
            data,
            evo,
            EvolutionType::LevelItemDay,
            item.to_owned(),
            false,
            false,
        ),
    );
}

pub fn add_evo_update_party(
    data: &GameData,
    updates: &mut EvolutionUpdates,
    evo: EvolutionId,
    other: &str,
) {
    insert_update(
        updates,
        evolution_update(
            data,
            evo,
            EvolutionType::LevelWithOther,
            other.to_owned(),
            false,
            false,
        ),
    );
}

pub fn add_evo_update_condensed(
    data: &GameData,
    updates: &mut EvolutionUpdates,
    evo: EvolutionId,
    additional: bool,
) {
    let level = data.evo(evo).extra_info.to_string();
    insert_update(
        updates,
        evolution_update(data, evo, EvolutionType::Level, level, true, additional),
    );
}

pub fn condense_level_evolutions<H: RomHandler + ?Sized>(
    handler: &mut H,
    max_level: i32,
    max_intermediate_level: i32,
) {
    let all = handler.get_pokemon();
    let core = handler.core_mut();
    for pokemon in all.into_iter().flatten() {
        for evo in core.data.evolutions_from(pokemon).to_vec() {
            let (kind, extra, to) = {
                let evolution = core.data.evo(evo);
                (evolution.kind, evolution.extra_info, evolution.to)
            };
            if kind.uses_level() {
                let intermediate = !core.data.evolutions_from(to).is_empty();
                if extra > max_intermediate_level && intermediate {
                    core.data.evo_mut(evo).extra_info = max_intermediate_level;
                    add_evo_update_condensed(
                        &core.data,
                        &mut core.engine.easier_evolution_updates,
                        evo,
                        false,
                    );
                } else if extra > max_level {
                    core.data.evo_mut(evo).extra_info = max_level;
                    add_evo_update_condensed(
                        &core.data,
                        &mut core.engine.easier_evolution_updates,
                        evo,
                        false,
                    );
                }
            }
            if core.data.evo(evo).kind == EvolutionType::LevelUpsideDown {
                core.data.evo_mut(evo).kind = EvolutionType::Level;
                add_evo_update_condensed(
                    &core.data,
                    &mut core.engine.easier_evolution_updates,
                    evo,
                    false,
                );
            }
        }
    }
}

fn evolution_pool_and_banned<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
    ban_irregular: bool,
) -> (Vec<PokemonId>, Vec<PokemonId>, Vec<PokemonId>) {
    let abilities_randomized = settings.abilities_mod == AbilitiesMod::Randomize;
    check_pokemon_restrictions(handler);
    let mut pool = if handler.alt_formes_can_have_different_evolutions() {
        handler.core().engine.pool.main_incl_formes.clone()
    } else {
        handler.core().engine.pool.main.clone()
    };
    let mut banned = handler.get_banned_formes_for_player_pokemon();
    if !abilities_randomized {
        banned.extend(handler.get_ability_dependent_formes());
    }
    if ban_irregular {
        banned.extend(handler.get_irregular_formes());
    }
    let data = &handler.core().data;
    let cosmetic: Vec<PokemonId> = pool
        .iter()
        .copied()
        .filter(|&pk| data.pk(pk).actually_cosmetic)
        .collect();
    pool.retain(|&pk| !data.pk(pk).actually_cosmetic);
    (pool, banned, cosmetic)
}

fn old_evo_pairs<H: RomHandler + ?Sized>(
    handler: &H,
    pool: &[PokemonId],
) -> HashSet<(PokemonId, PokemonId)> {
    let data = &handler.core().data;
    let mut pairs = HashSet::new();
    for &pokemon in pool {
        for &evo in data.evolutions_from(pokemon) {
            let evolution = data.evo(evo);
            pairs.insert((evolution.from, evolution.to));
            if handler.generation_of_pokemon() >= 7
                && data.pk(evolution.from).number == species::COSMOEM
            {
                let opposite = if data.pk(evolution.to).number == species::SOLGALEO {
                    species::LUNALA
                } else {
                    species::SOLGALEO
                };
                if let Some(target) = find_pokemon_in_pool_with_species_id(handler, pool, opposite)
                {
                    pairs.insert((evolution.from, target));
                }
            }
        }
    }
    pairs
}

fn clear_evolutions(data: &mut GameData, pool: &[PokemonId]) {
    for &pokemon in pool {
        data.evolutions_from_mut(pokemon).clear();
        data.evolutions_to_mut(pokemon).clear();
    }
}

fn evolution_candidates<H: RomHandler + ?Sized>(
    handler: &H,
    allow_alt_formes: bool,
) -> Vec<PokemonId> {
    if allow_alt_formes {
        non_cosmetic(handler, &handler.core().engine.pool.main_incl_formes)
    } else {
        handler.core().engine.pool.main.clone()
    }
}

fn shares_type_with(data: &GameData, candidate: PokemonId, from: PokemonId) -> bool {
    let pk = data.pk(candidate);
    let source = data.pk(from);
    pk.primary_type == source.primary_type
        || (source.secondary_type.is_some() && pk.primary_type == source.secondary_type)
        || (pk.secondary_type.is_some() && pk.secondary_type == source.primary_type)
        || (source.secondary_type.is_some()
            && pk.secondary_type.is_some()
            && pk.secondary_type == source.secondary_type)
}

fn apply_forme_to_new_evolution<H: RomHandler + ?Sized>(
    handler: &mut H,
    evo: &mut Evolution,
    picked: PokemonId,
) -> Result<()> {
    let mut random = handler.core().engine.random.clone();
    let data = &handler.core().data;
    let picked_pokemon = data.pk(picked);
    let mut check_cosmetics = true;
    if picked_pokemon.forme_number > 0 {
        evo.forme = picked_pokemon.forme_number;
        evo.forme_suffix = picked_pokemon.forme_suffix.clone();
        check_cosmetics = false;
    }
    let target = data.pk(evo.to);
    if check_cosmetics && target.cosmetic_forms > 0 {
        evo.forme =
            target.cosmetic_form_number(
                random_index(&mut random, target.cosmetic_forms as usize)? as i32
            );
    } else if !check_cosmetics && picked_pokemon.cosmetic_forms > 0 {
        evo.forme += picked_pokemon
            .cosmetic_form_number(
                random_index(&mut random, picked_pokemon.cosmetic_forms as usize)? as i32,
            );
    }
    Ok(())
}

fn exceeds_stage_limit(
    data: &mut GameData,
    from: PokemonId,
    candidate: PokemonId,
    stage_limit: i32,
    original: &HashMap<PokemonId, Vec<EvolutionId>>,
) -> Result<bool> {
    let temporary = data.temporary_evolution(Evolution::new(
        from,
        candidate,
        false,
        EvolutionType::None,
        0,
    ));
    data.evolutions_from_mut(from).push(temporary);
    data.evolutions_to_mut(candidate).push(temporary);
    let mut exceeded = false;
    for related in related_pokemon(data, from) {
        let pre_evolutions = num_pre_evolutions(data, related, stage_limit);
        if pre_evolutions >= stage_limit {
            exceeded = true;
            break;
        } else if pre_evolutions == stage_limit - 1 && data.evolutions_from(related).is_empty() {
            let had_evolutions = original
                .get(&related)
                .map_or(!data.evolutions_from(related).is_empty(), |evos| {
                    !evos.is_empty()
                });
            if had_evolutions {
                exceeded = true;
                break;
            }
        }
    }
    data.remove_first_equal_from(from, temporary);
    data.remove_first_equal_to(candidate, temporary);
    Ok(exceeded)
}

struct RandomEvolutionConstraints<'a> {
    from: PokemonId,
    banned: &'a [PokemonId],
    new_pairs: &'a HashSet<(PokemonId, PokemonId)>,
    old_pairs: &'a HashSet<(PokemonId, PokemonId)>,
    stage_limit: i32,
    original: &'a HashMap<PokemonId, Vec<EvolutionId>>,
}

#[derive(Clone, Copy)]
struct Relaxation {
    force_change: bool,
    stage_limit: bool,
    growth_curve: bool,
}

const RELAXATIONS: [Relaxation; 4] = [
    Relaxation {
        force_change: true,
        stage_limit: true,
        growth_curve: true,
    },
    Relaxation {
        force_change: false,
        stage_limit: true,
        growth_curve: true,
    },
    Relaxation {
        force_change: false,
        stage_limit: false,
        growth_curve: true,
    },
    Relaxation {
        force_change: false,
        stage_limit: false,
        growth_curve: false,
    },
];

fn random_evolution_replacements<H: RomHandler + ?Sized>(
    handler: &mut H,
    allow_alt_formes: bool,
    force_change: bool,
    constraints: &RandomEvolutionConstraints<'_>,
) -> Result<Vec<PokemonId>> {
    let from = constraints.from;
    let chosen_list = evolution_candidates(handler, allow_alt_formes);
    for relaxation in RELAXATIONS {
        let mut replacements = Vec::new();
        for &candidate in &chosen_list {
            let data = &handler.core().data;
            if candidate == from
                || constraints.banned.contains(&candidate)
                || constraints.new_pairs.contains(&(from, candidate))
            {
                continue;
            }
            if relaxation.growth_curve
                && data.pk(candidate).growth_curve != data.pk(from).growth_curve
            {
                continue;
            }
            if relaxation.force_change
                && force_change
                && constraints.old_pairs.contains(&(from, candidate))
            {
                continue;
            }
            let data = &mut handler.core_mut().data;
            if evo_cycle_check(data, from, candidate) {
                continue;
            }
            if relaxation.stage_limit
                && exceeds_stage_limit(
                    data,
                    from,
                    candidate,
                    constraints.stage_limit,
                    constraints.original,
                )?
            {
                continue;
            }
            replacements.push(candidate);
        }
        if !replacements.is_empty() {
            return Ok(replacements);
        }
    }
    Ok(Vec::new())
}

pub fn randomize_evolutions<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let similar_strength = settings.evos_similar_strength;
    let same_type = settings.evos_same_typing;
    let force_change = settings.evos_force_change;
    let allow_alt_formes = settings.evos_allow_alt_formes;
    let stage_limit = if settings.evos_max_three_stages {
        3
    } else {
        10
    };
    let (mut pool, banned, cosmetic_pool) =
        evolution_pool_and_banned(handler, settings, settings.ban_irregular_alt_formes);
    let original: HashMap<PokemonId, Vec<EvolutionId>> = pool
        .iter()
        .map(|&pk| (pk, handler.core().data.evolutions_from(pk).to_vec()))
        .collect();
    let old_pairs = if force_change {
        old_evo_pairs(handler, &pool)
    } else {
        HashSet::new()
    };
    let mut new_pairs: HashSet<(PokemonId, PokemonId)> = HashSet::new();
    let mut random = handler.core().engine.random.clone();
    clear_evolutions(&mut handler.core_mut().data, &pool);
    randomon_java::shuffle(&mut pool, &mut random);
    for &from in &pool {
        for &old_evo in &original[&from] {
            let old = handler.core().data.evo(old_evo).clone();
            let constraints = RandomEvolutionConstraints {
                from,
                banned: &banned,
                new_pairs: &new_pairs,
                old_pairs: &old_pairs,
                stage_limit,
                original: &original,
            };
            let mut replacements = random_evolution_replacements(
                handler,
                allow_alt_formes,
                force_change,
                &constraints,
            )?;
            if replacements.is_empty() {
                continue;
            }
            if replacements.len() > 1 && same_type {
                let data = &handler.core().data;
                let include: Vec<PokemonId> = replacements
                    .iter()
                    .copied()
                    .filter(|&candidate| {
                        if data.pk(from).number == species::EEVEE {
                            let target_primary = data.pk(old.to).primary_type;
                            let pk = data.pk(candidate);
                            pk.primary_type == target_primary
                                || (pk.secondary_type.is_some()
                                    && pk.secondary_type == target_primary)
                        } else {
                            shares_type_with(data, candidate, from)
                        }
                    })
                    .collect();
                if !include.is_empty() {
                    retain_all(&mut replacements, &include);
                }
            }
            let already_picked = handler.core().engine.already_picked.clone();
            if !contains_all(&already_picked, &replacements) && !similar_strength {
                remove_all(&mut replacements, &already_picked);
            }
            let picked = if replacements.len() == 1 {
                replacements[0]
            } else if similar_strength {
                pick_evo_power_lvl_replacement(handler, &replacements, old.to)?
            } else {
                pick(&mut random, &replacements)?
            };
            handler.core_mut().engine.already_picked.push(picked);
            let mut new_evo =
                Evolution::new(from, picked, old.carry_stats, old.kind, old.extra_info);
            apply_forme_to_new_evolution(handler, &mut new_evo, picked)?;
            if new_evo.kind == EvolutionType::LevelFemaleEspurr {
                new_evo.kind = EvolutionType::LevelFemaleOnly;
            }
            let data = &mut handler.core_mut().data;
            let id = data.new_evolution(new_evo);
            data.evolutions_from_mut(from).push(id);
            data.evolutions_to_mut(picked).push(id);
            new_pairs.insert((from, picked));
        }
    }
    share_cosmetic_evolutions(&mut handler.core_mut().data, &cosmetic_pool);
    Ok(())
}

fn share_cosmetic_evolutions(data: &mut GameData, cosmetic_pool: &[PokemonId]) {
    for &pokemon in cosmetic_pool {
        if let Some(base) = data.pk(pokemon).base_forme {
            data.share_evolutions_from(pokemon, base);
        }
    }
}

fn pick_evo_power_lvl_replacement<H: RomHandler + ?Sized>(
    handler: &mut H,
    pool: &[PokemonId],
    current: PokemonId,
) -> Result<PokemonId> {
    let data = &handler.core().data;
    let already_picked = &handler.core().engine.already_picked;
    let window = StrengthWindow::around(data.pk(current).bst_for_power_levels(), 3, 3);
    let fresh = window.candidates(data, pool, |pokemon| !already_picked.contains(&pokemon));
    let any = window.candidates(data, pool, |_| true);
    let mut random = handler.core().engine.random.clone();
    pick(&mut random, &first_non_empty(&[&fresh, &any, pool]))
}

pub fn randomize_evolutions_every_level<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let same_type = settings.evos_same_typing;
    let force_change = settings.evos_force_change;
    let allow_alt_formes = settings.evos_allow_alt_formes;
    let (mut pool, banned, cosmetic_pool) = evolution_pool_and_banned(handler, settings, false);
    let old_pairs = if force_change {
        old_evo_pairs(handler, &pool)
    } else {
        HashSet::new()
    };
    let mut random = handler.core().engine.random.clone();
    clear_evolutions(&mut handler.core_mut().data, &pool);
    randomon_java::shuffle(&mut pool, &mut random);
    for &from in &pool {
        let chosen_list = evolution_candidates(handler, allow_alt_formes);
        let data = &handler.core().data;
        let mut replacements = Vec::new();
        for relaxation in RELAXATIONS {
            replacements = chosen_list
                .iter()
                .copied()
                .filter(|&candidate| {
                    candidate != from
                        && !banned.contains(&candidate)
                        && (!relaxation.growth_curve
                            || data.pk(candidate).growth_curve == data.pk(from).growth_curve)
                        && !(relaxation.force_change
                            && force_change
                            && old_pairs.contains(&(from, candidate)))
                })
                .collect();
            if !replacements.is_empty() {
                break;
            }
        }
        if replacements.is_empty() {
            continue;
        }
        if replacements.len() > 1 && same_type {
            let include: Vec<PokemonId> = replacements
                .iter()
                .copied()
                .filter(|&candidate| {
                    let pk = data.pk(candidate);
                    let source = data.pk(from);
                    pk.primary_type == source.primary_type
                        || (source.secondary_type.is_some()
                            && pk.primary_type == source.secondary_type)
                        || (pk.secondary_type.is_some() && pk.secondary_type == source.primary_type)
                        || (pk.secondary_type.is_some()
                            && pk.secondary_type == source.secondary_type)
                })
                .collect();
            if !include.is_empty() {
                retain_all(&mut replacements, &include);
            }
        }
        let picked = if replacements.len() == 1 {
            replacements[0]
        } else {
            pick(&mut random, &replacements)?
        };
        let mut new_evo = Evolution::new(from, picked, false, EvolutionType::Level, 1);
        new_evo.level = 1;
        apply_forme_to_new_evolution(handler, &mut new_evo, picked)?;
        let data = &mut handler.core_mut().data;
        let id = data.new_evolution(new_evo);
        data.evolutions_from_mut(from).push(id);
        data.evolutions_to_mut(picked).push(id);
    }
    share_cosmetic_evolutions(&mut handler.core_mut().data, &cosmetic_pool);
    Ok(())
}
