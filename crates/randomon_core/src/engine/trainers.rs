use std::collections::{BTreeMap, BTreeSet};

use randomon_data::constants::abilities;
use randomon_data::{MultiBattleStatus, PokemonId, Trainer, TrainerPokemon, Type, moves_at_level};
use randomon_java::JavaRng;
use randomon_settings::{AbilitiesMod, MiscTweak, Settings, TrainersMod};

use crate::engine::choose::{StrengthWindow, first_non_empty};
use crate::engine::evolutions::{
    final_evos, fully_evolve, num_evolutions, set_evo_chain_as_illegal,
};
use crate::engine::pool::{check_pokemon_restrictions, non_cosmetic, pokemon_of_type_choice};
use crate::engine::util::{modified_level, pick, random_index, remove_all};
use crate::error::{RandomizationError, Result};
use crate::handler::RomHandler;

pub fn trainer_should_not_get_buffs(trainer: &Trainer) -> bool {
    trainer.tag.as_deref().is_some_and(|tag| {
        tag.starts_with("RIVAL1-") || tag.starts_with("FRIEND1-") || tag.ends_with("NOTSTRONG")
    })
}

fn apply_level_modifier_to_trainer_pokemon(trainer: &mut Trainer, level_modifier: i32) {
    if level_modifier != 0 {
        for pokemon in &mut trainer.pokemon {
            pokemon.level = modified_level(pokemon.level, level_modifier);
        }
    }
}

pub fn set_forme_for_trainer_pokemon<H: RomHandler + ?Sized>(
    handler: &H,
    tp: &mut TrainerPokemon,
    picked: PokemonId,
) -> Result<()> {
    let data = &handler.core().data;
    let mut random = handler.core().engine.random.clone();
    let mut check_cosmetics = true;
    tp.forme_suffix = String::new();
    tp.forme = 0;
    let picked_pokemon = data.pk(picked);
    if picked_pokemon.forme_number > 0 {
        tp.forme = picked_pokemon.forme_number;
        tp.forme_suffix = picked_pokemon.forme_suffix.clone();
        tp.pokemon = picked_pokemon
            .base_forme
            .ok_or(RandomizationError::NullPointer("baseForme"))?;
        check_cosmetics = false;
    }
    let shown = data.pk(tp.pokemon);
    if check_cosmetics && shown.cosmetic_forms > 0 {
        tp.forme = shown
            .cosmetic_form_number(random_index(&mut random, shown.cosmetic_forms as usize)? as i32);
    } else if !check_cosmetics && picked_pokemon.cosmetic_forms > 0 {
        tp.forme += picked_pokemon
            .cosmetic_form_number(
                random_index(&mut random, picked_pokemon.cosmetic_forms as usize)? as i32,
            );
    }
    Ok(())
}

pub fn random_ability_slot<H: RomHandler + ?Sized>(handler: &H, pokemon: PokemonId) -> i32 {
    random_ability_slot_excluding(handler, pokemon, &[])
}

fn random_ability_slot_excluding<H: RomHandler + ?Sized>(
    handler: &H,
    pokemon: PokemonId,
    excluded: &[i32],
) -> i32 {
    let per_pokemon = handler.abilities_per_pokemon().clamp(0, 3);
    if per_pokemon == 0 {
        return 0;
    }
    let pk = handler.core().data.pk(pokemon);
    let abilities = [pk.ability1, pk.ability2, pk.ability3];
    let filled: Vec<i32> = (1..=per_pokemon)
        .filter(|&slot| abilities[(slot - 1) as usize] != 0)
        .collect();
    let allowed: Vec<i32> = filled
        .iter()
        .copied()
        .filter(|slot| !excluded.contains(slot))
        .collect();
    let mut random = handler.core().engine.random.clone();
    pick(&mut random, &first_non_empty(&[&allowed, &filled, &[1]])).unwrap_or(1)
}

pub fn valid_ability_slot_from_original<H: RomHandler + ?Sized>(
    handler: &H,
    pokemon: PokemonId,
    original_slot: i32,
) -> i32 {
    if original_slot == 2 && handler.core().data.pk(pokemon).ability2 == 0 {
        1
    } else {
        original_slot
    }
}

fn placement_history<H: RomHandler + ?Sized>(handler: &H, pokemon: PokemonId) -> i32 {
    handler
        .core()
        .engine
        .placement_history
        .get(&pokemon)
        .copied()
        .unwrap_or(0)
}

fn placement_average<H: RomHandler + ?Sized>(handler: &H) -> f64 {
    let history = &handler.core().engine.placement_history;
    if history.is_empty() {
        return 0.0;
    }
    let sum: i64 = history.values().map(|&count| i64::from(count)).sum();
    sum as f64 / history.len() as f64
}

struct TypeChoice {
    weight_by_frequency: bool,
    no_legendaries: bool,
    allow_alt_formes: bool,
}

fn ensure_type_weightings<H: RomHandler + ?Sized>(handler: &mut H, choice: &TypeChoice) {
    if handler.core().engine.total_type_weighting != 0 {
        return;
    }
    for &kind in Type::ALL {
        if handler.type_in_game(kind) {
            let count = pokemon_of_type_choice(
                handler,
                kind,
                choice.no_legendaries,
                choice.allow_alt_formes,
            )
            .len() as i32;
            let engine = &mut handler.core_mut().engine;
            engine.type_weightings.insert(kind, count);
            engine.total_type_weighting += count;
        }
    }
}

fn pick_type<H: RomHandler + ?Sized>(
    handler: &mut H,
    choice: &TypeChoice,
    excluded: &BTreeSet<Type>,
) -> Result<Option<Type>> {
    ensure_type_weightings(handler, choice);
    let populated: Vec<(Type, i32)> = handler
        .core()
        .engine
        .type_weightings
        .iter()
        .filter(|&(_, &weight)| weight > 0)
        .map(|(&kind, &weight)| (kind, weight))
        .collect();
    let fresh: Vec<(Type, i32)> = populated
        .iter()
        .copied()
        .filter(|(kind, _)| !excluded.contains(kind))
        .collect();
    let candidates = if fresh.is_empty() { populated } else { fresh };
    if candidates.is_empty() {
        return Ok(None);
    }
    let mut random = handler.core().engine.random.clone();
    if !choice.weight_by_frequency {
        let kinds: Vec<Type> = candidates.iter().map(|&(kind, _)| kind).collect();
        return pick(&mut random, &kinds).map(Some);
    }
    let total: i32 = candidates.iter().map(|&(_, weight)| weight).sum();
    let type_pick = random_index(&mut random, total.max(1) as usize)? as i32;
    let mut position = 0;
    for &(kind, weight) in &candidates {
        if position + weight > type_pick {
            return Ok(Some(kind));
        }
        position += weight;
    }
    Ok(candidates.last().map(|&(kind, _)| kind))
}

pub struct ReplacementOptions {
    pub use_power_levels: bool,
    pub kind: Option<Type>,
    pub no_legendaries: bool,
    pub wonder_guard_allowed: bool,
    pub use_placement_history: bool,
    pub swap_mega_evos: bool,
    pub abilities_are_randomized: bool,
    pub allow_alt_formes: bool,
    pub ban_irregular_alt_formes: bool,
}

fn has_wonder_guard<H: RomHandler + ?Sized>(handler: &H, pokemon: PokemonId) -> bool {
    let pk = handler.core().data.pk(pokemon);
    pk.ability1 == abilities::WONDER_GUARD
        || pk.ability2 == abilities::WONDER_GUARD
        || pk.ability3 == abilities::WONDER_GUARD
}

fn mega_evolution_sources<H: RomHandler + ?Sized>(handler: &H) -> Vec<PokemonId> {
    let data = &handler.core().data;
    let mut sources: Vec<PokemonId> = Vec::new();
    for &mega in &handler.core().engine.pool.mega_evolutions {
        let mega = data.mega(mega);
        if mega.method == 1 && !sources.contains(&mega.from) {
            sources.push(mega.from);
        }
    }
    sources
}

fn pick_trainer_poke_replacement<H: RomHandler + ?Sized>(
    handler: &mut H,
    current: PokemonId,
    options: &ReplacementOptions,
) -> Result<PokemonId> {
    let mut pick_from = if options.swap_mega_evos {
        mega_evolution_sources(handler)
    } else {
        handler.core().engine.cached_all_list.clone()
    };
    if options.use_placement_history {
        let average = placement_average(handler);
        pick_from.retain(|&pokemon| f64::from(placement_history(handler, pokemon)) < average * 2.0);
        if pick_from.is_empty() {
            pick_from = handler.core().engine.cached_all_list.clone();
        }
    } else if let Some(kind) = options.kind
        && handler.core().engine.cached_replacement_lists.is_some()
    {
        let cached = handler
            .core()
            .engine
            .cached_replacement_lists
            .as_ref()
            .and_then(|lists| lists.get(&kind).cloned());
        let typed = match cached {
            Some(typed) => typed,
            None => {
                let mut typed = pokemon_of_type_choice(
                    handler,
                    kind,
                    options.no_legendaries,
                    options.allow_alt_formes,
                );
                remove_all(&mut typed, &handler.get_banned_formes_for_player_pokemon());
                if !options.abilities_are_randomized {
                    remove_all(&mut typed, &handler.get_ability_dependent_formes());
                }
                if options.ban_irregular_alt_formes {
                    remove_all(&mut typed, &handler.get_irregular_formes());
                }
                if let Some(lists) = handler.core_mut().engine.cached_replacement_lists.as_mut() {
                    lists.insert(kind, typed.clone());
                }
                typed
            }
        };
        if options.swap_mega_evos {
            let filtered: Vec<PokemonId> = typed
                .iter()
                .copied()
                .filter(|pokemon| pick_from.contains(pokemon))
                .collect();
            pick_from = if filtered.is_empty() { typed } else { filtered };
        } else {
            pick_from = typed;
        }
    }
    let banned = &handler.core().engine.banned_list;
    let without_banned: Vec<PokemonId> = pick_from
        .iter()
        .copied()
        .filter(|pokemon| !banned.contains(pokemon))
        .collect();
    if !without_banned.is_empty() {
        pick_from = without_banned;
    }
    if pick_from.is_empty() {
        let engine = &handler.core().engine;
        pick_from = first_non_empty(&[&engine.cached_all_list, &engine.pool.main, &[current]]);
    }
    let mut random = handler.core().engine.random.clone();
    let data = &handler.core().data;
    let allowed =
        |pokemon: PokemonId| options.wonder_guard_allowed || !has_wonder_guard(handler, pokemon);
    let mut candidates: Vec<PokemonId> = pick_from
        .iter()
        .copied()
        .filter(|&pokemon| allowed(pokemon))
        .collect();
    if candidates.is_empty() {
        candidates = pick_from;
    }
    if !options.use_power_levels {
        return pick(&mut random, &candidates);
    }
    let window = StrengthWindow::around(data.pk(current).bst_for_power_levels(), 3, 2);
    let mut can_pick = window.candidates(data, &candidates, |_| true);
    if can_pick.is_empty() {
        can_pick = candidates;
    }
    if options.use_placement_history {
        let average = placement_average(handler);
        let filtered: Vec<PokemonId> = can_pick
            .iter()
            .copied()
            .filter(|&pokemon| f64::from(placement_history(handler, pokemon)) < average)
            .collect();
        if !filtered.is_empty() {
            can_pick = filtered;
        }
    }
    pick(&mut random, &can_pick)
}

fn trainer_group(tag: Option<&str>) -> String {
    let group = tag.unwrap_or("");
    match group.find('-') {
        Some(position) => group[..position].to_owned(),
        None => group.to_owned(),
    }
}

pub fn randomize_trainer_pokes<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let use_power_levels = settings.trainers_use_pokemon_of_similar_strength;
    let weight_by_frequency = settings.trainers_match_typing_distribution;
    let no_legendaries = settings.trainers_block_legendaries;
    let no_early_wonder_guard = settings.trainers_block_early_wonder_guard;
    let level_modifier = if settings.trainers_level_modified {
        settings.trainers_level_modifier
    } else {
        0
    };
    let is_type_themed = settings.trainers_mod == TrainersMod::TypeThemed;
    let is_type_themed_elite4_gym_only = settings.trainers_mod == TrainersMod::TypeThemedElite4Gyms;
    let distribution = settings.trainers_mod == TrainersMod::Distributed;
    let main_playthrough = settings.trainers_mod == TrainersMod::MainPlaythrough;
    let include_formes = settings.allow_trainer_alternate_formes;
    let ban_irregular_alt_formes = settings.ban_irregular_alt_formes;
    let swap_mega_evos = settings.swap_trainer_mega_evos;
    let shiny_chance = settings.shiny_chance;
    let abilities_are_randomized = settings.abilities_mod == AbilitiesMod::Randomize;
    let elite_four_unique_number = settings.elite_four_unique_pokemon_number;
    let force_fully_evolved = settings.trainers_force_fully_evolved;
    let force_fully_evolved_level = settings.trainers_force_fully_evolved_level;
    let force_challenge_mode =
        settings.current_misc_tweaks & MiscTweak::ForceChallengeMode.value() > 0;
    let rival_carries_starter = settings.rival_carries_starter_throughout;
    let type_choice = TypeChoice {
        weight_by_frequency,
        no_legendaries,
        allow_alt_formes: include_formes,
    };
    check_pokemon_restrictions(handler);

    let mut cached_all = {
        let pool = &handler.core().engine.pool;
        let mut list = if no_legendaries {
            pool.no_legendary.clone()
        } else {
            pool.main.clone()
        };
        if include_formes {
            if no_legendaries {
                list.extend(pool.no_legendary_alts.iter().copied());
            } else {
                list.extend(pool.alt_formes.iter().copied());
            }
        }
        list
    };
    cached_all = non_cosmetic(handler, &cached_all);
    let mut banned = handler.get_banned_formes_for_trainer_pokemon();
    if !abilities_are_randomized {
        banned.extend(handler.get_ability_dependent_formes());
    }
    if ban_irregular_alt_formes {
        banned.extend(handler.get_irregular_formes());
    }
    remove_all(&mut cached_all, &banned);
    {
        let engine = &mut handler.core_mut().engine;
        engine.cached_replacement_lists = Some(BTreeMap::new());
        engine.cached_all_list = cached_all;
    }
    let mut current_trainers = handler.get_trainers()?;
    let mut random = handler.core().engine.random.clone();

    let mut trainer_types: BTreeMap<usize, Type> = BTreeMap::new();
    let mut used_uber_types: BTreeSet<Type> = BTreeSet::new();
    if is_type_themed || is_type_themed_elite4_gym_only {
        {
            let engine = &mut handler.core_mut().engine;
            engine.type_weightings = BTreeMap::new();
            engine.total_type_weighting = 0;
        }
        let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (position, trainer) in current_trainers.iter().enumerate() {
            if trainer.tag.as_deref() == Some("IRIVAL") {
                continue;
            }
            let group = trainer_group(trainer.tag.as_deref());
            if group.starts_with("GYM")
                || group.starts_with("ELITE")
                || ((group.starts_with("CHAMPION") || group.starts_with("THEMED"))
                    && !is_type_themed_elite4_gym_only)
            {
                groups.entry(group).or_default().push(position);
            } else if group.starts_with("GIO") {
                groups.entry("GYM8".to_owned()).or_default().push(position);
            }
        }
        let mut used_gym_types: BTreeSet<Type> = BTreeSet::new();
        let mut used_elite_types: BTreeSet<Type> = BTreeSet::new();
        let no_exclusions = BTreeSet::new();
        for (group, members) in groups.iter_mut() {
            randomon_java::shuffle(members, &mut random);
            let excluded = if group.starts_with("GYM") {
                &used_gym_types
            } else if group.starts_with("ELITE") {
                &used_elite_types
            } else {
                &no_exclusions
            };
            let type_for_group = pick_type(handler, &type_choice, excluded)?;
            if group.starts_with("GYM") {
                used_gym_types.extend(type_for_group);
            }
            if group.starts_with("ELITE") {
                used_elite_types.extend(type_for_group);
            }
            if group == "CHAMPION" {
                used_uber_types.extend(type_for_group);
            }
            for &member in members.iter() {
                let index = current_trainers[member].index as usize;
                match type_for_group {
                    Some(kind) => {
                        trainer_types.insert(index, kind);
                    }
                    None => {
                        trainer_types.remove(&index);
                    }
                }
            }
        }
    }

    let mut scrambled: Vec<usize> = (0..current_trainers.len()).collect();
    randomon_java::shuffle(&mut scrambled, &mut random);
    let elite_four_unique = elite_four_unique_number > 0;
    let mut illegal_if_evolved: Vec<PokemonId> = Vec::new();
    let mut banned_from_unique: Vec<PokemonId> = Vec::new();
    let mut illegal_evo_chains = false;
    let elite_four_indices = handler.get_elite_four_trainers(force_challenge_mode);
    if elite_four_unique {
        let is_elite = |position: usize| elite_four_indices.contains(&(position as i32 + 1));
        scrambled.sort_by_key(|&position| std::cmp::Reverse(is_elite(position)));
        illegal_evo_chains = force_fully_evolved;
        if rival_carries_starter {
            let starters: Vec<PokemonId> = handler.get_starters().into_iter().take(3).collect();
            let has_rival_final_battle = handler.has_rival_final_battle();
            for starter in starters {
                banned_from_unique.push(starter);
                set_evo_chain_as_illegal(
                    &handler.core().data,
                    starter,
                    &mut banned_from_unique,
                    true,
                );
                if has_rival_final_battle {
                    let finals = final_evos(&handler.core().data, starter);
                    remove_all(&mut handler.core_mut().engine.cached_all_list, &finals);
                    if illegal_evo_chains {
                        illegal_if_evolved.push(starter);
                        set_evo_chain_as_illegal(
                            &handler.core().data,
                            starter,
                            &mut illegal_if_evolved,
                            true,
                        );
                    }
                }
            }
        }
    }
    let main_playthrough_trainers = handler.get_main_playthrough_trainers();

    for &position in &scrambled {
        apply_level_modifier_to_trainer_pokemon(&mut current_trainers[position], level_modifier);
        let trainer_index = current_trainers[position].index;
        let tag = current_trainers[position].tag.clone();
        if tag.as_deref() == Some("IRIVAL") {
            continue;
        }
        let mut type_for_trainer = trainer_types.get(&(trainer_index as usize)).copied();
        if type_for_trainer.is_none() && is_type_themed {
            if tag.as_deref() == Some("UBER") {
                type_for_trainer = pick_type(handler, &type_choice, &used_uber_types)?;
                used_uber_types.extend(type_for_trainer);
            } else {
                type_for_trainer = pick_type(handler, &type_choice, &BTreeSet::new())?;
            }
        }
        let mut evolves_into_wrong_type: Vec<PokemonId> = Vec::new();
        if let Some(kind) = type_for_trainer {
            let typed = pokemon_of_type_choice(handler, kind, no_legendaries, include_formes);
            for &pokemon in &typed {
                let evolved = fully_evolve(handler, pokemon, trainer_index)?;
                if !typed.contains(&evolved) {
                    evolves_into_wrong_type.push(pokemon);
                }
            }
        }
        let mut order: Vec<usize> = (0..current_trainers[position].pokemon.len()).collect();
        let mut elite_four_track = false;
        let mut elite_four_rival = false;
        if elite_four_unique && elite_four_indices.contains(&trainer_index) {
            elite_four_track = true;
            order.reverse();
            let pokemon = &current_trainers[position].pokemon;
            order.sort_by(|&first, &second| pokemon[second].level.cmp(&pokemon[first].level));
            let tag_text = tag
                .as_deref()
                .ok_or(RandomizationError::NullPointer("trainer tag"))?;
            if rival_carries_starter && (tag_text.contains("RIVAL") || tag_text.contains("FRIEND"))
            {
                elite_four_rival = true;
            }
        }
        let uses_placement = distribution
            || (main_playthrough && main_playthrough_trainers.contains(&trainer_index));
        for (rank, &slot) in order.iter().enumerate() {
            let mut tp = current_trainers[position].pokemon[slot].clone();
            let swap_this_mega = swap_mega_evos && can_mega_evolve(handler, &tp);
            let wonder_guard_allowed = !no_early_wonder_guard || tp.level >= 20;
            let set_unique = elite_four_track && elite_four_unique_number > rank as i32;
            let will_force_evolve = force_fully_evolved && tp.level >= force_fully_evolved_level;
            let old_pokemon = if tp.forme > 0 {
                handler.get_alt_forme_of_pokemon(tp.pokemon, tp.forme)
            } else {
                tp.pokemon
            };
            let mut banned_list = handler.core().engine.used_as_unique_list.clone();
            if illegal_evo_chains && will_force_evolve {
                banned_list.extend(illegal_if_evolved.iter().copied());
            }
            if set_unique {
                banned_list.extend(banned_from_unique.iter().copied());
            }
            if will_force_evolve {
                banned_list.extend(evolves_into_wrong_type.iter().copied());
            }
            handler.core_mut().engine.banned_list = banned_list;
            let new_pokemon = pick_trainer_poke_replacement(
                handler,
                old_pokemon,
                &ReplacementOptions {
                    use_power_levels,
                    kind: type_for_trainer,
                    no_legendaries,
                    wonder_guard_allowed,
                    use_placement_history: uses_placement,
                    swap_mega_evos: swap_this_mega,
                    abilities_are_randomized,
                    allow_alt_formes: include_formes,
                    ban_irregular_alt_formes,
                },
            )?;
            if uses_placement {
                *handler
                    .core_mut()
                    .engine
                    .placement_history
                    .entry(new_pokemon)
                    .or_insert(0) += 1;
            }
            tp.pokemon = new_pokemon;
            set_forme_for_trainer_pokemon(handler, &mut tp, new_pokemon)?;
            tp.ability_slot = random_ability_slot(handler, new_pokemon);
            tp.reset_moves = true;
            if !elite_four_rival {
                if set_unique {
                    let actual = if will_force_evolve {
                        final_evos(&handler.core().data, new_pokemon)
                    } else {
                        vec![new_pokemon]
                    };
                    for actual_pokemon in actual {
                        handler
                            .core_mut()
                            .engine
                            .used_as_unique_list
                            .push(actual_pokemon);
                        if illegal_evo_chains {
                            set_evo_chain_as_illegal(
                                &handler.core().data,
                                actual_pokemon,
                                &mut illegal_if_evolved,
                                will_force_evolve,
                            );
                        }
                    }
                }
                if elite_four_track {
                    banned_from_unique.push(new_pokemon);
                    if illegal_evo_chains {
                        set_evo_chain_as_illegal(
                            &handler.core().data,
                            new_pokemon,
                            &mut banned_from_unique,
                            will_force_evolve,
                        );
                    }
                }
            } else {
                elite_four_rival = false;
            }
            if swap_this_mega {
                let megas = handler
                    .core()
                    .data
                    .pk(new_pokemon)
                    .mega_evolutions_from
                    .clone();
                if !megas.is_empty() {
                    let chosen = pick(&mut random, &megas)?;
                    tp.held_item = handler.core().data.mega(chosen).argument;
                }
            }
            if shiny_chance && random.next_int_bounded(256) == 0 {
                tp.ivs |= 1 << 30;
            }
            current_trainers[position].pokemon[slot] = tp;
        }
    }
    handler.set_trainers(&current_trainers, false)
}

fn can_mega_evolve<H: RomHandler + ?Sized>(handler: &H, tp: &TrainerPokemon) -> bool {
    if tp.held_item == 0 {
        return false;
    }
    let data = &handler.core().data;
    data.pk(tp.pokemon)
        .mega_evolutions_from
        .iter()
        .any(|&mega| data.mega(mega).argument == tp.held_item)
}

fn moveset_for_items<H: RomHandler + ?Sized>(
    handler: &H,
    tp: &TrainerPokemon,
    movesets: &crate::handler::Movesets,
) -> Result<[i32; 4]> {
    if !tp.reset_moves {
        return Ok(tp.moves);
    }
    let number = handler
        .core()
        .data
        .pk(handler.get_alt_forme_of_pokemon(tp.pokemon, tp.forme))
        .number;
    let movepool = movesets
        .get(&number)
        .ok_or(RandomizationError::NullPointer("movesets.get"))?;
    Ok(moves_at_level(movepool, tp.level, 0))
}

fn randomize_held_item<H: RomHandler + ?Sized>(
    handler: &H,
    tp: &mut TrainerPokemon,
    settings: &Settings,
    moveset: &[i32; 4],
) -> Result<()> {
    if tp.has_z_crystal {
        return Ok(());
    }
    if tp.has_mega_stone && settings.swap_trainer_mega_evos {
        return Ok(());
    }
    let choices = if settings.sensible_items_only_for_trainer_pokemon {
        handler.get_sensible_held_items_for(
            tp,
            settings.consumable_items_only_for_trainer_pokemon,
            moveset,
        )
    } else if settings.consumable_items_only_for_trainer_pokemon {
        handler.get_all_consumable_held_items()
    } else {
        handler.get_all_held_items()
    };
    let mut random = handler.core().engine.random.clone();
    tp.held_item = pick(&mut random, &choices)?;
    Ok(())
}

pub fn randomize_trainer_held_items<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let give_to_boss = settings.randomize_held_items_for_boss_trainer_pokemon;
    let give_to_important = settings.randomize_held_items_for_important_trainer_pokemon;
    let give_to_regular = settings.randomize_held_items_for_regular_trainer_pokemon;
    let highest_level_only = settings.highest_level_only_gets_items_for_trainer_pokemon;
    let movesets = handler.get_moves_learnt();
    let mut trainers = handler.get_trainers()?;
    for trainer in &mut trainers {
        if trainer_should_not_get_buffs(trainer) {
            continue;
        }
        if !give_to_regular && !trainer.is_important() && !trainer.is_boss() {
            continue;
        }
        if !give_to_important && trainer.is_important() {
            continue;
        }
        if !give_to_boss && trainer.is_boss() {
            continue;
        }
        trainer.set_pokemon_have_items(true);
        if highest_level_only {
            let mut max_level = -1;
            let mut highest: Option<usize> = None;
            for (slot, tp) in trainer.pokemon.iter().enumerate() {
                if tp.level > max_level {
                    highest = Some(slot);
                    max_level = tp.level;
                }
            }
            let Some(slot) = highest else {
                continue;
            };
            let moveset = moveset_for_items(handler, &trainer.pokemon[slot], &movesets)?;
            randomize_held_item(handler, &mut trainer.pokemon[slot], settings, &moveset)?;
        } else {
            for slot in 0..trainer.pokemon.len() {
                let moveset = moveset_for_items(handler, &trainer.pokemon[slot], &movesets)?;
                randomize_held_item(handler, &mut trainer.pokemon[slot], settings, &moveset)?;
                if trainer.requires_unique_held_items {
                    let mut attempts = 0;
                    while !trainer.pokemon_have_unique_held_items() && attempts < 100 {
                        randomize_held_item(
                            handler,
                            &mut trainer.pokemon[slot],
                            settings,
                            &moveset,
                        )?;
                        attempts += 1;
                    }
                    if !trainer.pokemon_have_unique_held_items() {
                        trainer.pokemon[slot].held_item = 0;
                    }
                }
            }
        }
    }
    handler.set_trainers(&trainers, false)
}

fn pick_random_evolution_of<H: RomHandler + ?Sized>(
    handler: &H,
    base: PokemonId,
    must_evolve_itself: bool,
) -> Result<PokemonId> {
    let data = &handler.core().data;
    let candidates: Vec<PokemonId> = data
        .evolution_targets(base)
        .into_iter()
        .filter(|&target| !must_evolve_itself || !data.evolutions_from(target).is_empty())
        .collect();
    let targets = data.evolution_targets(base);
    let choices = first_non_empty(&[&candidates, &targets, &[base]]);
    let mut random = handler.core().engine.random.clone();
    pick(&mut random, &choices)
}

fn level_of_starter(trainers: &[Trainer], tag: &str) -> i32 {
    for trainer in trainers {
        if trainer.tag.as_deref() == Some(tag) {
            let mut highest = trainer.pokemon[0].level;
            let count = trainer.pokemon.len();
            for index in 1..count {
                let bonus = if index == count - 1 { 2 } else { 0 };
                if trainer.pokemon[index].level + bonus > highest {
                    highest = trainer.pokemon[index].level;
                }
            }
            return highest;
        }
    }
    0
}

fn change_starter_with_tag<H: RomHandler + ?Sized>(
    handler: &H,
    trainers: &mut [Trainer],
    tag: &str,
    starter: PokemonId,
    ability_slot: i32,
) -> Result<()> {
    for trainer in trainers.iter_mut() {
        if trainer.tag.as_deref() != Some(tag) {
            continue;
        }
        let best = if trainer.force_starter_position >= 0 {
            trainer.force_starter_position as usize
        } else {
            let mut best = 0;
            let count = trainer.pokemon.len();
            for index in 1..count {
                let bonus = if index == count - 1 { 2 } else { 0 };
                if trainer.pokemon[index].level + bonus > trainer.pokemon[best].level {
                    best = index;
                }
            }
            best
        };
        let mut tp = trainer.pokemon[best].clone();
        tp.pokemon = starter;
        set_forme_for_trainer_pokemon(handler, &mut tp, starter)?;
        tp.reset_moves = true;
        tp.ability_slot = ability_slot;
        trainer.pokemon[best] = tp;
    }
    Ok(())
}

fn highest_rival_number(trainers: &[Trainer], prefix: &str) -> Result<i32> {
    let mut highest = 0;
    for trainer in trainers {
        if let Some(tag) = trainer.tag.as_deref()
            && tag.starts_with(prefix)
        {
            let dash = tag.find('-').ok_or(RandomizationError::IndexOutOfBounds(
                "rival tag without number",
            ))?;
            let number: i32 = tag[prefix.len()..dash].parse().map_err(|_| {
                RandomizationError::randomization(format!(
                    "For input string: \"{}\"",
                    &tag[prefix.len()..dash]
                ))
            })?;
            highest = highest.max(number);
        }
    }
    Ok(highest)
}

fn rival_carries_starter_update<H: RomHandler + ?Sized>(
    handler: &mut H,
    trainers: &mut [Trainer],
    prefix: &str,
    pokemon_offset: i32,
) -> Result<()> {
    let highest = highest_rival_number(trainers, prefix)?;
    if highest == 0 {
        return Ok(());
    }
    let starters = handler.get_starters();
    if handler.is_yellow() {
        let Some(&rival_starter) = starters.get(1).or_else(|| starters.first()) else {
            return Ok(());
        };
        let mut rival_starter = rival_starter;
        let times_evolves = num_evolutions(&handler.core().data, rival_starter, 2);
        let ability_slot = 0;
        if times_evolves == 0 {
            for j in 1..=3 {
                change_starter_with_tag(
                    handler,
                    trainers,
                    &format!("{prefix}{j}-0"),
                    rival_starter,
                    ability_slot,
                )?;
            }
            for j in 4..=7 {
                for i in 0..3 {
                    change_starter_with_tag(
                        handler,
                        trainers,
                        &format!("{prefix}{j}-{i}"),
                        rival_starter,
                        ability_slot,
                    )?;
                }
            }
        } else if times_evolves == 1 {
            for j in 1..=3 {
                change_starter_with_tag(
                    handler,
                    trainers,
                    &format!("{prefix}{j}-0"),
                    rival_starter,
                    ability_slot,
                )?;
            }
            rival_starter = pick_random_evolution_of(handler, rival_starter, false)?;
            for j in 4..=7 {
                for i in 0..3 {
                    change_starter_with_tag(
                        handler,
                        trainers,
                        &format!("{prefix}{j}-{i}"),
                        rival_starter,
                        ability_slot,
                    )?;
                }
            }
        } else if times_evolves == 2 {
            for j in 1..=2 {
                change_starter_with_tag(
                    handler,
                    trainers,
                    &format!("{prefix}{j}-0"),
                    rival_starter,
                    ability_slot,
                )?;
            }
            rival_starter = pick_random_evolution_of(handler, rival_starter, true)?;
            change_starter_with_tag(
                handler,
                trainers,
                &format!("{prefix}3-0"),
                rival_starter,
                ability_slot,
            )?;
            for i in 0..3 {
                change_starter_with_tag(
                    handler,
                    trainers,
                    &format!("{prefix}4-{i}"),
                    rival_starter,
                    ability_slot,
                )?;
            }
            rival_starter = pick_random_evolution_of(handler, rival_starter, false)?;
            for j in 5..=7 {
                for i in 0..3 {
                    change_starter_with_tag(
                        handler,
                        trainers,
                        &format!("{prefix}{j}-{i}"),
                        rival_starter,
                        ability_slot,
                    )?;
                }
            }
        }
        return Ok(());
    }
    for i in 0..3 {
        let Some(&this_starter) =
            starters.get(((i + pokemon_offset) % 3) as usize % starters.len().max(1))
        else {
            return Ok(());
        };
        let mut this_starter = this_starter;
        let times_evolves = num_evolutions(&handler.core().data, this_starter, 2);
        let ability_slot = random_ability_slot_excluding(handler, this_starter, &[3]);
        let tag = |j: i32| format!("{prefix}{j}-{i}");
        if times_evolves == 0 {
            for j in 1..=highest {
                change_starter_with_tag(handler, trainers, &tag(j), this_starter, ability_slot)?;
            }
        } else if times_evolves == 1 {
            let mut j = 1;
            while j <= highest / 2 {
                if level_of_starter(trainers, &tag(j)) >= 30 {
                    break;
                }
                change_starter_with_tag(handler, trainers, &tag(j), this_starter, ability_slot)?;
                j += 1;
            }
            this_starter = pick_random_evolution_of(handler, this_starter, false)?;
            let evolved_slot =
                valid_ability_slot_from_original(handler, this_starter, ability_slot);
            while j <= highest {
                change_starter_with_tag(handler, trainers, &tag(j), this_starter, evolved_slot)?;
                j += 1;
            }
        } else if times_evolves == 2 {
            let mut j = 1;
            while j <= highest {
                if level_of_starter(trainers, &tag(j)) >= 16 {
                    break;
                }
                change_starter_with_tag(handler, trainers, &tag(j), this_starter, ability_slot)?;
                j += 1;
            }
            this_starter = pick_random_evolution_of(handler, this_starter, true)?;
            let mut evolved_slot =
                valid_ability_slot_from_original(handler, this_starter, ability_slot);
            while j <= highest {
                if level_of_starter(trainers, &tag(j)) >= 36 {
                    break;
                }
                change_starter_with_tag(handler, trainers, &tag(j), this_starter, evolved_slot)?;
                j += 1;
            }
            this_starter = pick_random_evolution_of(handler, this_starter, false)?;
            evolved_slot = valid_ability_slot_from_original(handler, this_starter, ability_slot);
            while j <= highest {
                change_starter_with_tag(handler, trainers, &tag(j), this_starter, evolved_slot)?;
                j += 1;
            }
        }
    }
    Ok(())
}

pub fn rival_carries_starter<H: RomHandler + ?Sized>(handler: &mut H) -> Result<()> {
    check_pokemon_restrictions(handler);
    let mut trainers = handler.get_trainers()?;
    let rival_offset = if handler.core().engine.is_oras { 0 } else { 1 };
    rival_carries_starter_update(handler, &mut trainers, "RIVAL", rival_offset)?;
    rival_carries_starter_update(handler, &mut trainers, "FRIEND", 2)?;
    handler.set_trainers(&trainers, false)
}

pub fn force_fully_evolved_trainer_pokes<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let min_level = settings.trainers_force_fully_evolved_level;
    check_pokemon_restrictions(handler);
    let mut trainers = handler.get_trainers()?;
    for trainer in &mut trainers {
        let index = trainer.index;
        for tp in &mut trainer.pokemon {
            if tp.level >= min_level {
                let evolved = fully_evolve(handler, tp.pokemon, index)?;
                if evolved != tp.pokemon {
                    tp.pokemon = evolved;
                    set_forme_for_trainer_pokemon(handler, tp, evolved)?;
                    tp.ability_slot =
                        valid_ability_slot_from_original(handler, evolved, tp.ability_slot);
                    tp.reset_moves = true;
                }
            }
        }
    }
    handler.set_trainers(&trainers, false)
}

pub fn only_change_trainer_levels<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let level_modifier = settings.trainers_level_modifier;
    let mut trainers = handler.get_trainers()?;
    for trainer in &mut trainers {
        apply_level_modifier_to_trainer_pokemon(trainer, level_modifier);
    }
    handler.set_trainers(&trainers, false)
}

pub fn add_trainer_pokemon<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let additional_normal = settings.additional_regular_trainer_pokemon;
    let additional_important = settings.additional_important_trainer_pokemon;
    let additional_boss = settings.additional_boss_trainer_pokemon;
    let mut trainers = handler.get_trainers()?;
    for trainer in &mut trainers {
        let additional = if trainer.is_boss() {
            additional_boss
        } else if trainer.is_important() {
            if trainer.skip_important() {
                continue;
            }
            additional_important
        } else {
            additional_normal
        };
        if additional == 0 {
            continue;
        }
        let lowest = trainer
            .pokemon
            .iter()
            .map(|tp| tp.level)
            .fold(100, i32::min);
        let potential: Vec<TrainerPokemon> = trainer
            .pokemon
            .iter()
            .filter(|tp| tp.level == lowest)
            .cloned()
            .collect();
        let max_pokemon = if trainer.multi_battle_status != MultiBattleStatus::Never {
            3
        } else {
            6
        };
        for i in 0..additional {
            if trainer.pokemon.len() >= max_pokemon {
                break;
            }
            if potential.is_empty() {
                return Err(RandomizationError::randomization("/ by zero"));
            }
            let second_to_last = trainer.pokemon.len().checked_sub(1).ok_or(
                RandomizationError::IndexOutOfBounds("trainer without Pokemon"),
            )?;
            let mut added = potential[i as usize % potential.len()].copy();
            added.held_item = 0;
            trainer.pokemon.insert(second_to_last, added);
        }
    }
    handler.set_trainers(&trainers, false)
}

pub fn double_battle_mode<H: RomHandler + ?Sized>(handler: &mut H) -> Result<()> {
    let mut trainers = handler.get_trainers()?;
    for trainer in &mut trainers {
        if trainer.pokemon.len() != 1
            || trainer.multi_battle_status == MultiBattleStatus::Always
            || trainer_should_not_get_buffs(trainer)
        {
            continue;
        }
        let copy = trainer.pokemon[0].copy();
        trainer.pokemon.push(copy);
    }
    handler.set_trainers(&trainers, true)
}
