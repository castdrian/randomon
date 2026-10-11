use std::collections::{BTreeMap, BTreeSet, HashSet};

use randomon_data::{EncounterSet, PokemonId, Type};
use randomon_settings::{AbilitiesMod, Settings, WildPokemonRestrictionMod};

use crate::engine::choose::{StrengthWindow, first_non_empty};
use crate::engine::pool::{
    all_pokemon_incl_formes_without_null, check_pokemon_restrictions, non_cosmetic,
    pokemon_of_type_choice,
};
use crate::engine::types::random_type;
use crate::engine::util::{modified_level, pick, random_index, remove_all, remove_first};
use crate::error::{RandomizationError, Result};
use crate::handler::RomHandler;

struct WildOptions {
    catch_em_all: bool,
    type_themed: bool,
    use_power_levels: bool,
    no_legendaries: bool,
    balance_shaking_grass: bool,
    level_modifier: i32,
    allow_alt_formes: bool,
    ban_irregular_alt_formes: bool,
    abilities_are_randomized: bool,
}

impl WildOptions {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            catch_em_all: settings.wild_pokemon_restriction_mod
                == WildPokemonRestrictionMod::CatchEmAll,
            type_themed: settings.wild_pokemon_restriction_mod
                == WildPokemonRestrictionMod::TypeThemeAreas,
            use_power_levels: settings.wild_pokemon_restriction_mod
                == WildPokemonRestrictionMod::SimilarStrength,
            no_legendaries: settings.block_wild_legendaries,
            balance_shaking_grass: settings.balance_shaking_grass,
            level_modifier: if settings.wild_levels_modified {
                settings.wild_level_modifier
            } else {
                0
            },
            allow_alt_formes: settings.allow_wild_alt_formes,
            ban_irregular_alt_formes: settings.ban_irregular_alt_formes,
            abilities_are_randomized: settings.abilities_mod == AbilitiesMod::Randomize,
        }
    }
}

fn wild_banned<H: RomHandler + ?Sized>(
    handler: &H,
    options: &WildOptions,
    include_player_formes: bool,
) -> Vec<PokemonId> {
    let mut banned = handler.banned_for_wild_encounters();
    if include_player_formes {
        banned.extend(handler.get_banned_formes_for_player_pokemon());
    }
    if !options.abilities_are_randomized {
        banned.extend(handler.get_ability_dependent_formes());
    }
    if options.ban_irregular_alt_formes {
        banned.extend(handler.get_irregular_formes());
    }
    banned
}

fn base_pool<H: RomHandler + ?Sized>(
    handler: &H,
    options: &WildOptions,
    strip_cosmetic: bool,
) -> Vec<PokemonId> {
    let pool = &handler.core().engine.pool;
    if options.allow_alt_formes {
        let list = if options.no_legendaries {
            &pool.no_legendary_incl_formes
        } else {
            &pool.main_incl_formes
        };
        if strip_cosmetic {
            non_cosmetic(handler, list)
        } else {
            list.clone()
        }
    } else if options.no_legendaries {
        pool.no_legendary.clone()
    } else {
        pool.main.clone()
    }
}

fn set_forme_for_encounter<H: RomHandler + ?Sized>(
    handler: &H,
    set: &mut EncounterSet,
    index: usize,
    picked: PokemonId,
) -> Result<()> {
    let data = &handler.core().data;
    let mut random = handler.core().engine.random.clone();
    let encounter = &mut set.encounters[index];
    let mut check_cosmetics = true;
    encounter.forme_number = 0;
    let current = data.pk(encounter.pokemon);
    if current.forme_number > 0 {
        encounter.forme_number = current.forme_number;
        encounter.pokemon = current
            .base_forme
            .ok_or(RandomizationError::NullPointer("baseForme"))?;
        check_cosmetics = false;
    }
    let shown = data.pk(encounter.pokemon);
    if check_cosmetics && shown.cosmetic_forms > 0 {
        encounter.forme_number = shown
            .cosmetic_form_number(random_index(&mut random, shown.cosmetic_forms as usize)? as i32);
    } else if !check_cosmetics && data.pk(picked).cosmetic_forms > 0 {
        let picked_pokemon = data.pk(picked);
        encounter.forme_number += picked_pokemon
            .cosmetic_form_number(
                random_index(&mut random, picked_pokemon.cosmetic_forms as usize)? as i32,
            );
    }
    Ok(())
}

fn assign<H: RomHandler + ?Sized>(
    handler: &H,
    set: &mut EncounterSet,
    index: usize,
    pokemon: PokemonId,
) -> Result<()> {
    set.encounters[index].pokemon = pokemon;
    set_forme_for_encounter(handler, set, index, pokemon)
}

fn apply_level_modifier(encounters: &mut [EncounterSet], level_modifier: i32) {
    if level_modifier == 0 {
        return;
    }
    for area in encounters {
        for encounter in &mut area.encounters {
            encounter.level = modified_level(encounter.level, level_modifier);
            encounter.max_level = modified_level(encounter.max_level, level_modifier);
        }
    }
}

fn pokemon_in_area(set: &EncounterSet) -> BTreeSet<PokemonId> {
    set.encounters
        .iter()
        .map(|encounter| encounter.pokemon)
        .collect()
}

fn scrambled_order<H: RomHandler + ?Sized>(handler: &H, count: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..count).collect();
    let mut random = handler.core().engine.random.clone();
    randomon_java::shuffle(&mut order, &mut random);
    order
}

fn is_cosmetic<H: RomHandler + ?Sized>(handler: &H, pokemon: PokemonId) -> bool {
    handler.core().data.pk(pokemon).actually_cosmetic
}

fn real_choices<H: RomHandler + ?Sized>(handler: &H, list: &[PokemonId]) -> Vec<PokemonId> {
    let real: Vec<PokemonId> = list
        .iter()
        .copied()
        .filter(|&pokemon| !is_cosmetic(handler, pokemon))
        .collect();
    first_non_empty(&[&real, list])
}

fn pick_real<H: RomHandler + ?Sized>(
    handler: &H,
    random: &mut impl randomon_java::JavaRng,
    list: &[PokemonId],
) -> Result<PokemonId> {
    pick(random, &real_choices(handler, list))
}

fn take_real<H: RomHandler + ?Sized>(
    handler: &H,
    random: &mut impl randomon_java::JavaRng,
    possible: &mut Vec<PokemonId>,
    refill: &[PokemonId],
) -> Result<PokemonId> {
    if !possible
        .iter()
        .any(|&pokemon| !is_cosmetic(handler, pokemon))
    {
        possible.extend_from_slice(refill);
    }
    let picked = pick_real(handler, random, possible)?;
    remove_first(possible, &picked);
    Ok(picked)
}

fn fallback_pool<H: RomHandler + ?Sized>(handler: &H, options: &WildOptions) -> Vec<PokemonId> {
    let pool = &handler.core().engine.pool;
    first_non_empty(&[&base_pool(handler, options, true), &pool.main])
}

fn temp_pickable<H: RomHandler + ?Sized>(
    handler: &H,
    options: &WildOptions,
    banned: &[PokemonId],
    area: &EncounterSet,
) -> Result<Vec<PokemonId>> {
    let mut pickable = base_pool(handler, options, true);
    remove_all(&mut pickable, banned);
    let without_area_bans: Vec<PokemonId> = pickable
        .iter()
        .copied()
        .filter(|pokemon| !area.banned_pokemon.contains(pokemon))
        .collect();
    let fallback = fallback_pool(handler, options);
    Ok(first_non_empty(&[&without_area_bans, &pickable, &fallback]))
}

fn refill(
    handler: &(impl RomHandler + ?Sized),
    options: &WildOptions,
    banned: &[PokemonId],
    list: &mut Vec<PokemonId>,
) {
    list.extend(base_pool(handler, options, true));
    remove_all(list, banned);
}

pub fn pick_entirely_random_pokemon<H: RomHandler + ?Sized>(
    handler: &mut H,
    include_formes: bool,
    no_legendaries: bool,
    area: &EncounterSet,
    banned: &[PokemonId],
) -> Result<PokemonId> {
    pick_entirely_random_pokemon_avoiding(
        handler,
        include_formes,
        no_legendaries,
        area,
        banned,
        &[],
    )
}

fn pick_entirely_random_pokemon_avoiding<H: RomHandler + ?Sized>(
    handler: &mut H,
    include_formes: bool,
    no_legendaries: bool,
    area: &EncounterSet,
    banned: &[PokemonId],
    avoid: &[PokemonId],
) -> Result<PokemonId> {
    check_pokemon_restrictions(handler);
    let pool = &handler.core().engine.pool;
    let source = match (no_legendaries, include_formes) {
        (true, true) => first_non_empty(&[&pool.no_legendary_incl_formes, &pool.main_incl_formes]),
        (true, false) => first_non_empty(&[&pool.no_legendary, &pool.main]),
        (false, true) => first_non_empty(&[&pool.main_incl_formes, &pool.main]),
        (false, false) => pool.main.clone(),
    };
    let real = real_choices(handler, &source);
    let allowed: Vec<PokemonId> = real
        .iter()
        .copied()
        .filter(|&pokemon| !banned.contains(&pokemon) && !area.is_banned(pokemon))
        .collect();
    let fresh: Vec<PokemonId> = allowed
        .iter()
        .copied()
        .filter(|pokemon| !avoid.contains(pokemon))
        .collect();
    let mut random = handler.core().engine.random.clone();
    pick(&mut random, &first_non_empty(&[&fresh, &allowed, &real]))
}

pub fn pick_wild_power_lvl_replacement<H: RomHandler + ?Sized>(
    handler: &H,
    pool: &[PokemonId],
    current: PokemonId,
    ban_same_pokemon: bool,
    used_up: Option<&[PokemonId]>,
    bst_balance_level: i32,
) -> Result<PokemonId> {
    let data = &handler.core().data;
    let balanced_bst = bst_balance_level * 10 + 250;
    let current_bst = data.pk(current).bst_for_power_levels().min(balanced_bst);
    let window = StrengthWindow::around(current_bst, 3, 3);
    let real = real_choices(handler, pool);
    let preferred = |pokemon: PokemonId| {
        (!ban_same_pokemon || pokemon != current)
            && used_up.is_none_or(|used| !used.contains(&pokemon))
    };
    let mut can_pick = window.candidates(data, &real, preferred);
    if can_pick.is_empty() {
        can_pick = window.candidates(data, &real, |_| true);
    }
    if can_pick.is_empty() {
        return Ok(current);
    }
    let mut random = handler.core().engine.random.clone();
    pick(&mut random, &can_pick)
}

fn power_level_pick_until_real<H: RomHandler + ?Sized>(
    handler: &H,
    pool: &[PokemonId],
    current: PokemonId,
    used_up: Option<&[PokemonId]>,
    bst_balance_level: i32,
) -> Result<PokemonId> {
    pick_wild_power_lvl_replacement(handler, pool, current, false, used_up, bst_balance_level)
}

fn find_themed_pool<H: RomHandler + ?Sized>(
    handler: &H,
    options: &WildOptions,
    banned: &[PokemonId],
    cache: &mut BTreeMap<Type, Vec<PokemonId>>,
    area: &EncounterSet,
    minimum_size: usize,
) -> Result<Vec<PokemonId>> {
    for _ in 0..1_000 {
        let theme = random_type(handler);
        let typed = cache.entry(theme).or_insert_with(|| {
            let mut typed = pokemon_of_type_choice(
                handler,
                theme,
                options.no_legendaries,
                options.allow_alt_formes,
            );
            remove_all(&mut typed, banned);
            typed
        });
        let mut possible = typed.clone();
        if !area.banned_pokemon.is_empty() {
            remove_all(&mut possible, &area.banned_pokemon);
        }
        if possible.len() >= minimum_size {
            return Ok(possible);
        }
    }
    let mut fallback = fallback_pool(handler, options);
    remove_all(&mut fallback, banned);
    Ok(first_non_empty(&[
        &fallback,
        &fallback_pool(handler, options),
    ]))
}

pub fn random_encounters<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let options = WildOptions::from_settings(settings);
    let use_time_of_day = settings.use_time_based_encounters;
    let mut current = handler.get_encounters(use_time_of_day);
    if handler.core().engine.is_oras {
        let mut collapsed = collapse_areas_oras(handler, &current);
        area_1to1_encounters_impl(handler, &mut collapsed.sets, settings)?;
        enhance_random_encounters_oras(handler, &mut collapsed.sets, settings)?;
        collapsed.write_back(&mut current);
        return handler.set_encounters(use_time_of_day, &current);
    }
    check_pokemon_restrictions(handler);
    let order = scrambled_order(handler, current.len());
    let banned = wild_banned(handler, &options, true);
    let mut random = handler.core().engine.random.clone();
    if options.catch_em_all {
        let mut all_pokes = base_pool(handler, &options, true);
        remove_all(&mut all_pokes, &banned);
        for &area_index in &order {
            let area = &mut current[area_index];
            let separate_pickable = !area.banned_pokemon.is_empty();
            let mut pickable_copy = if separate_pickable {
                let mut copy = all_pokes.clone();
                remove_all(&mut copy, &area.banned_pokemon);
                copy
            } else {
                Vec::new()
            };
            for index in 0..area.encounters.len() {
                if banned.contains(&area.encounters[index].pokemon) {
                    continue;
                }
                let pickable_len = if separate_pickable {
                    pickable_copy.len()
                } else {
                    all_pokes.len()
                };
                if pickable_len == 0 {
                    let temp = temp_pickable(handler, &options, &banned, area)?;
                    let picked = pick(&mut random, &temp)?;
                    assign(handler, area, index, picked)?;
                } else {
                    let pickable = if separate_pickable {
                        &mut pickable_copy
                    } else {
                        &mut all_pokes
                    };
                    let picked_index = random_index(&mut random, pickable.len())?;
                    let picked = pickable.remove(picked_index);
                    if separate_pickable {
                        remove_first(&mut all_pokes, &picked);
                    }
                    assign(handler, area, index, picked)?;
                    if all_pokes.is_empty() {
                        refill(handler, &options, &banned, &mut all_pokes);
                        if separate_pickable {
                            pickable_copy.extend(all_pokes.iter().copied());
                            remove_all(&mut pickable_copy, &area.banned_pokemon);
                        }
                    }
                }
            }
        }
    } else if options.type_themed {
        let mut cache: BTreeMap<Type, Vec<PokemonId>> = BTreeMap::new();
        for &area_index in &order {
            let area = &mut current[area_index];
            let possible = find_themed_pool(handler, &options, &banned, &mut cache, area, 1)?;
            for index in 0..area.encounters.len() {
                let picked = pick_real(handler, &mut random, &possible)?;
                assign(handler, area, index, picked)?;
            }
        }
    } else if options.use_power_levels {
        let mut allowed = base_pool(handler, &options, false);
        remove_all(&mut allowed, &banned);
        for &area_index in &order {
            let area = &mut current[area_index];
            let mut local_allowed = allowed.clone();
            if !area.banned_pokemon.is_empty() {
                remove_all(&mut local_allowed, &area.banned_pokemon);
            }
            for index in 0..area.encounters.len() {
                let encounter = &area.encounters[index];
                let balance_level =
                    if options.balance_shaking_grass && area.display_name_contains("Shaking") {
                        (encounter.level + encounter.max_level) / 2
                    } else {
                        100
                    };
                let picked = pick_wild_power_lvl_replacement(
                    handler,
                    &local_allowed,
                    encounter.pokemon,
                    false,
                    None,
                    balance_level,
                )?;
                assign(handler, area, index, picked)?;
            }
        }
    } else {
        for &area_index in &order {
            let area_snapshot = current[area_index].clone();
            for index in 0..area_snapshot.encounters.len() {
                let picked = pick_entirely_random_pokemon(
                    handler,
                    options.allow_alt_formes,
                    options.no_legendaries,
                    &area_snapshot,
                    &banned,
                )?;
                assign(handler, &mut current[area_index], index, picked)?;
            }
        }
    }
    apply_level_modifier(&mut current, options.level_modifier);
    handler.set_encounters(use_time_of_day, &current)
}

pub fn area_1to1_encounters<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let use_time_of_day = settings.use_time_based_encounters;
    let mut current = handler.get_encounters(use_time_of_day);
    if handler.core().engine.is_oras {
        let mut collapsed = collapse_areas_oras(handler, &current);
        area_1to1_encounters_impl(handler, &mut collapsed.sets, settings)?;
        collapsed.write_back(&mut current);
    } else {
        area_1to1_encounters_impl(handler, &mut current, settings)?;
    }
    handler.set_encounters(use_time_of_day, &current)
}

fn apply_area_map<H: RomHandler + ?Sized>(
    handler: &H,
    area: &mut EncounterSet,
    map: &BTreeMap<PokemonId, PokemonId>,
    skip: &[PokemonId],
) -> Result<()> {
    for index in 0..area.encounters.len() {
        if skip.contains(&area.encounters[index].pokemon) {
            continue;
        }
        let mapped = *map
            .get(&area.encounters[index].pokemon)
            .ok_or(RandomizationError::NullPointer("areaMap.get"))?;
        assign(handler, area, index, mapped)?;
    }
    Ok(())
}

fn area_1to1_encounters_impl<H: RomHandler + ?Sized>(
    handler: &mut H,
    current: &mut [EncounterSet],
    settings: &Settings,
) -> Result<()> {
    let options = WildOptions::from_settings(settings);
    check_pokemon_restrictions(handler);
    let banned = wild_banned(handler, &options, true);
    let order = scrambled_order(handler, current.len());
    let mut random = handler.core().engine.random.clone();
    if options.catch_em_all {
        let mut all_pokes = base_pool(handler, &options, true);
        remove_all(&mut all_pokes, &banned);
        for &area_index in &order {
            let area = &mut current[area_index];
            let in_area = pokemon_in_area(area);
            let mut area_map: BTreeMap<PokemonId, PokemonId> = BTreeMap::new();
            let separate_pickable = !area.banned_pokemon.is_empty();
            let mut pickable_copy = if separate_pickable {
                let mut copy = all_pokes.clone();
                remove_all(&mut copy, &area.banned_pokemon);
                copy
            } else {
                Vec::new()
            };
            for &area_pokemon in &in_area {
                let pickable_len = if separate_pickable {
                    pickable_copy.len()
                } else {
                    all_pokes.len()
                };
                if pickable_len == 0 {
                    let temp = temp_pickable(handler, &options, &banned, area)?;
                    area_map.insert(area_pokemon, pick(&mut random, &temp)?);
                } else {
                    let picked = pick(&mut random, &all_pokes)?;
                    area_map.insert(area_pokemon, picked);
                    if separate_pickable {
                        remove_first(&mut pickable_copy, &picked);
                    }
                    remove_first(&mut all_pokes, &picked);
                    if all_pokes.is_empty() {
                        refill(handler, &options, &banned, &mut all_pokes);
                        if separate_pickable {
                            pickable_copy.extend(all_pokes.iter().copied());
                            remove_all(&mut pickable_copy, &area.banned_pokemon);
                        }
                    }
                }
            }
            apply_area_map(handler, area, &area_map, &banned)?;
        }
    } else if options.type_themed {
        let mut cache: BTreeMap<Type, Vec<PokemonId>> = BTreeMap::new();
        for &area_index in &order {
            let area = &mut current[area_index];
            let in_area = pokemon_in_area(area);
            let mut possible =
                find_themed_pool(handler, &options, &banned, &mut cache, area, in_area.len())?;
            let themed_pool = possible.clone();
            let mut area_map: BTreeMap<PokemonId, PokemonId> = BTreeMap::new();
            for &area_pokemon in &in_area {
                let picked = take_real(handler, &mut random, &mut possible, &themed_pool)?;
                area_map.insert(area_pokemon, picked);
            }
            apply_area_map(handler, area, &area_map, &[])?;
        }
    } else if options.use_power_levels {
        let mut allowed = base_pool(handler, &options, false);
        remove_all(&mut allowed, &banned);
        for &area_index in &order {
            let area = &mut current[area_index];
            let in_area = pokemon_in_area(area);
            let mut area_map: BTreeMap<PokemonId, PokemonId> = BTreeMap::new();
            let mut used: Vec<PokemonId> = Vec::new();
            let mut local_allowed = allowed.clone();
            if !area.banned_pokemon.is_empty() {
                remove_all(&mut local_allowed, &area.banned_pokemon);
            }
            for &area_pokemon in &in_area {
                let picked = power_level_pick_until_real(
                    handler,
                    &local_allowed,
                    area_pokemon,
                    Some(&used),
                    100,
                )?;
                area_map.insert(area_pokemon, picked);
                used.push(picked);
            }
            apply_area_map(handler, area, &area_map, &[])?;
        }
    } else {
        for &area_index in &order {
            let area_snapshot = current[area_index].clone();
            let in_area = pokemon_in_area(&area_snapshot);
            let mut area_map: BTreeMap<PokemonId, PokemonId> = BTreeMap::new();
            for &area_pokemon in &in_area {
                let used: Vec<PokemonId> = area_map.values().copied().collect();
                let picked = pick_entirely_random_pokemon_avoiding(
                    handler,
                    options.allow_alt_formes,
                    options.no_legendaries,
                    &area_snapshot,
                    &banned,
                    &used,
                )?;
                area_map.insert(area_pokemon, picked);
            }
            apply_area_map(handler, &mut current[area_index], &area_map, &[])?;
        }
    }
    apply_level_modifier(current, options.level_modifier);
    Ok(())
}

pub fn game_1to1_encounters<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let options = WildOptions::from_settings(settings);
    let use_time_of_day = settings.use_time_based_encounters;
    check_pokemon_restrictions(handler);
    let mut translate: BTreeMap<PokemonId, PokemonId> = BTreeMap::new();
    let mut remaining_left = non_cosmetic(handler, &all_pokemon_incl_formes_without_null(handler));
    let mut remaining_right = base_pool(handler, &options, true);
    let banned = wild_banned(handler, &options, true);
    for &banned_pokemon in &banned {
        translate.insert(banned_pokemon, banned_pokemon);
        remove_first(&mut remaining_left, &banned_pokemon);
        remove_first(&mut remaining_right, &banned_pokemon);
    }
    let mut random = handler.core().engine.random.clone();
    while !remaining_left.is_empty() {
        if options.use_power_levels {
            let left_index = random_index(&mut random, remaining_left.len())?;
            let left = remaining_left.remove(left_index);
            let right = if remaining_right.len() == 1 {
                remaining_right[0]
            } else {
                pick_wild_power_lvl_replacement(handler, &remaining_right, left, true, None, 100)?
            };
            remove_first(&mut remaining_right, &right);
            translate.insert(left, right);
        } else {
            let left_index = random_index(&mut random, remaining_left.len())?;
            let left = remaining_left.remove(left_index);
            let data = &handler.core().data;
            let different: Vec<usize> = (0..remaining_right.len())
                .filter(|&index| data.pk(left).number != data.pk(remaining_right[index]).number)
                .collect();
            let all_indices: Vec<usize> = (0..remaining_right.len()).collect();
            let right_index = pick(&mut random, &first_non_empty(&[&different, &all_indices]))?;
            let right = remaining_right.remove(right_index);
            translate.insert(left, right);
        }
        if remaining_right.is_empty() {
            refill(handler, &options, &banned, &mut remaining_right);
        }
    }
    for pokemon in all_pokemon_incl_formes_without_null(handler) {
        translate.entry(pokemon).or_insert(pokemon);
    }
    let mut current = handler.get_encounters(use_time_of_day);
    for area in &mut current {
        for index in 0..area.encounters.len() {
            let mapped = *translate
                .get(&area.encounters[index].pokemon)
                .ok_or(RandomizationError::NullPointer("translateMap.get"))?;
            area.encounters[index].pokemon = mapped;
            if area.is_banned(mapped) {
                let temp = temp_pickable(handler, &options, &banned, area)?;
                area.encounters[index].pokemon = if options.use_power_levels {
                    pick_wild_power_lvl_replacement(handler, &temp, mapped, false, None, 100)?
                } else {
                    pick(&mut random, &temp)?
                };
            }
            let picked = area.encounters[index].pokemon;
            set_forme_for_encounter(handler, area, index, picked)?;
        }
    }
    apply_level_modifier(&mut current, options.level_modifier);
    handler.set_encounters(use_time_of_day, &current)
}

pub fn only_change_wild_levels<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let level_modifier = settings.wild_level_modifier;
    let mut current = handler.get_encounters(true);
    if level_modifier != 0 {
        apply_level_modifier(&mut current, level_modifier);
        handler.set_encounters(true, &current)?;
    }
    Ok(())
}

struct CollapsedAreas {
    sets: Vec<EncounterSet>,
    origins: Vec<Vec<(usize, usize)>>,
}

impl CollapsedAreas {
    fn write_back(&self, current: &mut [EncounterSet]) {
        for (set, origins) in self.sets.iter().zip(&self.origins) {
            for (encounter, &(area, index)) in set.encounters.iter().zip(origins) {
                current[area].encounters[index] = encounter.clone();
            }
        }
    }
}

fn zones_to_areas(current: &[EncounterSet], indices: &[usize]) -> BTreeMap<i32, Vec<usize>> {
    let mut zones: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for &index in indices {
        zones.entry(current[index].offset).or_default().push(index);
    }
    zones
}

fn dex_nav_crash_threshold<H: RomHandler + ?Sized>(handler: &H, areas: &[&EncounterSet]) -> usize {
    let data = &handler.core().data;
    areas
        .iter()
        .filter(|area| !area.display_name_contains("Rock Smash"))
        .map(|area| {
            area.encounters
                .iter()
                .map(|encounter| {
                    data.pk(encounter.pokemon)
                        .base_forme
                        .unwrap_or(encounter.pokemon)
                })
                .collect::<HashSet<PokemonId>>()
                .len()
        })
        .sum()
}

fn group_into(
    target: &mut EncounterSet,
    origins: &mut Vec<(usize, usize)>,
    area: &EncounterSet,
    area_origins: &[(usize, usize)],
) {
    target.encounters.extend(area.encounters.iter().cloned());
    origins.extend_from_slice(area_origins);
}

fn collapse_areas_oras<H: RomHandler + ?Sized>(
    handler: &H,
    current: &[EncounterSet],
) -> CollapsedAreas {
    let mut output = CollapsedAreas {
        sets: Vec::new(),
        origins: Vec::new(),
    };
    let all_indices: Vec<usize> = (0..current.len()).collect();
    for (zone, areas) in zones_to_areas(current, &all_indices) {
        let origins_of = |area: usize| -> Vec<(usize, usize)> {
            (0..current[area].encounters.len())
                .map(|index| (area, index))
                .collect()
        };
        let area_refs: Vec<&EncounterSet> = areas.iter().map(|&area| &current[area]).collect();
        if dex_nav_crash_threshold(handler, &area_refs) <= 18 {
            for &area in &areas {
                output.sets.push(current[area].clone());
                output.origins.push(origins_of(area));
            }
            continue;
        }
        let mut collapsed: Vec<(EncounterSet, Vec<(usize, usize)>)> = areas
            .iter()
            .map(|&area| (current[area].clone(), origins_of(area)))
            .collect();
        let mut rod_group = EncounterSet {
            offset: zone,
            display_name: Some("Rod Group".to_owned()),
            ..EncounterSet::default()
        };
        let mut rod_origins = Vec::new();
        for &area in &areas {
            let set = &current[area];
            if set.display_name_contains("Old Rod")
                || set.display_name_contains("Good Rod")
                || set.display_name_contains("Super Rod")
            {
                if let Some(position) = collapsed.iter().position(|(candidate, origins)| {
                    origins.first().map(|origin| origin.0) == Some(area)
                        && candidate.encounters.len() == set.encounters.len()
                }) {
                    collapsed.remove(position);
                }
                group_into(&mut rod_group, &mut rod_origins, set, &origins_of(area));
            }
        }
        if !rod_group.encounters.is_empty() {
            collapsed.push((rod_group, rod_origins));
        }
        let collapsed_refs: Vec<&EncounterSet> = collapsed.iter().map(|(set, _)| set).collect();
        if dex_nav_crash_threshold(handler, &collapsed_refs) <= 18 {
            for (set, origins) in collapsed {
                output.sets.push(set);
                output.origins.push(origins);
            }
            continue;
        }
        let mut grass_group = EncounterSet {
            offset: zone,
            display_name: Some("Grass Group".to_owned()),
            ..EncounterSet::default()
        };
        let mut grass_origins = Vec::new();
        for &area in &areas {
            let set = &current[area];
            if set.display_name_contains("Grass/Cave")
                || set.display_name_contains("Long Grass")
                || set.display_name_contains("Horde")
            {
                if let Some(position) = collapsed
                    .iter()
                    .position(|(_, origins)| origins.first().map(|origin| origin.0) == Some(area))
                {
                    collapsed.remove(position);
                }
                group_into(&mut grass_group, &mut grass_origins, set, &origins_of(area));
            }
        }
        if !grass_group.encounters.is_empty() {
            collapsed.push((grass_group, grass_origins));
        }
        for (set, origins) in collapsed {
            output.sets.push(set);
            output.origins.push(origins);
        }
    }
    output
}

fn type_for_area<H: RomHandler + ?Sized>(handler: &H, area: &EncounterSet) -> Option<Type> {
    let data = &handler.core().data;
    let resolve = |index: usize| {
        let encounter = &area.encounters[index];
        if encounter.forme_number != 0 {
            handler.get_alt_forme_of_pokemon(encounter.pokemon, encounter.forme_number)
        } else {
            encounter.pokemon
        }
    };
    let first = data.pk(resolve(0));
    let primary = first.primary_type;
    let mut primary_count = 1;
    let secondary = first.secondary_type;
    let mut secondary_count = i32::from(secondary.is_some());
    for index in 1..area.encounters.len() {
        let pokemon = data.pk(resolve(index));
        if pokemon.primary_type == primary || pokemon.secondary_type == primary {
            primary_count += 1;
        }
        if pokemon.primary_type == secondary || pokemon.secondary_type == secondary {
            secondary_count += 1;
        }
    }
    if primary_count > secondary_count {
        primary
    } else {
        secondary
    }
}

fn another_encounter_with_same_pokemon(area: &EncounterSet, index: usize) -> bool {
    let pokemon = area.encounters[index].pokemon;
    area.encounters
        .iter()
        .enumerate()
        .any(|(other, encounter)| other != index && encounter.pokemon == pokemon)
}

fn enhance_random_encounters_oras<H: RomHandler + ?Sized>(
    handler: &mut H,
    collapsed: &mut [EncounterSet],
    settings: &Settings,
) -> Result<()> {
    let options = WildOptions::from_settings(settings);
    let banned = wild_banned(handler, &options, false);
    let mut cache: BTreeMap<Type, Vec<PokemonId>> = BTreeMap::new();
    let mut random = handler.core().engine.random.clone();
    let all_indices: Vec<usize> = (0..collapsed.len()).collect();
    let zones = zones_to_areas(collapsed, &all_indices);
    for zone_areas in zones.values() {
        let mut non_rock_smash: Vec<usize> = Vec::new();
        let mut to_randomize: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for &area_index in zone_areas {
            if collapsed[area_index].display_name_contains("Rock Smash") {
                if options.type_themed {
                    let snapshot = collapsed[area_index].clone();
                    let possible =
                        find_themed_pool(handler, &options, &banned, &mut cache, &snapshot, 1)?;
                    for index in 0..snapshot.encounters.len() {
                        let picked = pick_real(handler, &mut random, &possible)?;
                        assign(handler, &mut collapsed[area_index], index, picked)?;
                    }
                } else {
                    for index in 0..collapsed[area_index].encounters.len() {
                        randomize_oras_encounter(
                            handler, &options, &banned, &mut cache, collapsed, area_index, index,
                        )?;
                    }
                }
            } else {
                to_randomize.insert(
                    non_rock_smash.len(),
                    (0..collapsed[area_index].encounters.len()).collect(),
                );
                non_rock_smash.push(area_index);
            }
        }
        let mut crash_threshold = zone_threshold(handler, collapsed, zone_areas);
        while crash_threshold < 18 && !to_randomize.is_empty() {
            let keys: Vec<usize> = to_randomize.keys().copied().collect();
            let area_key = keys[random_index(&mut random, keys.len())?];
            let indices = to_randomize
                .get_mut(&area_key)
                .ok_or(RandomizationError::NullPointer("encounterIndices"))?;
            let position = random_index(&mut random, indices.len())?;
            let encounter_index = indices[position];
            let area_index = non_rock_smash[area_key];
            randomize_oras_encounter(
                handler,
                &options,
                &banned,
                &mut cache,
                collapsed,
                area_index,
                encounter_index,
            )?;
            crash_threshold = zone_threshold(handler, collapsed, zone_areas);
            let indices = to_randomize
                .get_mut(&area_key)
                .ok_or(RandomizationError::NullPointer("encounterIndices"))?;
            indices.remove(position);
            if indices.is_empty() {
                to_randomize.remove(&area_key);
            }
        }
    }
    Ok(())
}

fn zone_threshold<H: RomHandler + ?Sized>(
    handler: &H,
    collapsed: &[EncounterSet],
    zone_areas: &[usize],
) -> usize {
    let refs: Vec<&EncounterSet> = zone_areas.iter().map(|&area| &collapsed[area]).collect();
    dex_nav_crash_threshold(handler, &refs)
}

fn randomize_oras_encounter<H: RomHandler + ?Sized>(
    handler: &mut H,
    options: &WildOptions,
    banned: &[PokemonId],
    cache: &mut BTreeMap<Type, Vec<PokemonId>>,
    collapsed: &mut [EncounterSet],
    area_index: usize,
    index: usize,
) -> Result<()> {
    let mut random = handler.core().engine.random.clone();
    let area_snapshot = collapsed[area_index].clone();
    if options.catch_em_all {
        if another_encounter_with_same_pokemon(&area_snapshot, index) {
            let picked = pick_entirely_random_pokemon(
                handler,
                options.allow_alt_formes,
                options.no_legendaries,
                &area_snapshot,
                banned,
            )?;
            assign(handler, &mut collapsed[area_index], index, picked)?;
        }
    } else if options.type_themed {
        let theme = type_for_area(handler, &area_snapshot).unwrap_or_else(|| random_type(handler));
        let typed = cache.entry(theme).or_insert_with(|| {
            let mut typed = pokemon_of_type_choice(
                handler,
                theme,
                options.no_legendaries,
                options.allow_alt_formes,
            );
            remove_all(&mut typed, banned);
            typed
        });
        let mut possible = typed.clone();
        remove_all(&mut possible, &area_snapshot.banned_pokemon);
        if possible.is_empty() {
            possible = fallback_pool(handler, options);
        }
        let picked = pick_real(handler, &mut random, &possible)?;
        assign(handler, &mut collapsed[area_index], index, picked)?;
    } else if options.use_power_levels {
        let mut allowed = base_pool(handler, options, false);
        remove_all(&mut allowed, banned);
        remove_all(&mut allowed, &area_snapshot.banned_pokemon);
        let current = area_snapshot.encounters[index].pokemon;
        let picked = pick_wild_power_lvl_replacement(handler, &allowed, current, false, None, 100)?;
        assign(handler, &mut collapsed[area_index], index, picked)?;
    } else {
        let picked = pick_entirely_random_pokemon(
            handler,
            options.allow_alt_formes,
            options.no_legendaries,
            &area_snapshot,
            banned,
        )?;
        assign(handler, &mut collapsed[area_index], index, picked)?;
    }
    Ok(())
}
