use std::collections::BTreeMap;

use randomon_data::{Aura, PokemonId, StaticEncounter, TotemPokemon};
use randomon_settings::{
    AbilitiesMod, AllyPokemonMod, AuraMod, Settings, StaticPokemonMod, TotemPokemonMod,
};

use crate::engine::pool::{check_pokemon_restrictions, non_cosmetic};
use crate::engine::util::{modified_level, pick, random_index, remove_all, remove_first, take};
use crate::error::{RandomizationError, Result};
use crate::handler::RomHandler;

fn clone_static_encounter(old: &StaticEncounter) -> StaticEncounter {
    StaticEncounter {
        pkmn: old.pkmn,
        level: old.level,
        max_level: old.max_level,
        held_item: old.held_item,
        is_egg: old.is_egg,
        reset_moves: true,
        linked_encounters: old
            .linked_encounters
            .iter()
            .map(|linked| StaticEncounter {
                pkmn: linked.pkmn,
                level: linked.level,
                max_level: linked.max_level,
                held_item: linked.held_item,
                is_egg: linked.is_egg,
                reset_moves: true,
                ..StaticEncounter::default()
            })
            .collect(),
        ..StaticEncounter::default()
    }
}

fn set_pokemon_and_forme_for_static_encounter<H: RomHandler + ?Sized>(
    handler: &H,
    encounter: &mut StaticEncounter,
    pokemon: PokemonId,
) -> Result<()> {
    let data = &handler.core().data;
    let mut random = handler.core().engine.random.clone();
    let pk = data.pk(pokemon);
    let mut check_cosmetics = true;
    let mut new_pokemon = pokemon;
    let mut new_forme = 0;
    if pk.forme_number > 0 {
        new_forme = pk.forme_number;
        new_pokemon = pk
            .base_forme
            .ok_or(RandomizationError::NullPointer("baseForme"))?;
        check_cosmetics = false;
    }
    if check_cosmetics && pk.cosmetic_forms > 0 {
        new_forme =
            pk.cosmetic_form_number(random_index(&mut random, pk.cosmetic_forms as usize)? as i32);
    } else if !check_cosmetics && pk.cosmetic_forms > 0 {
        new_forme +=
            pk.cosmetic_form_number(random_index(&mut random, pk.cosmetic_forms as usize)? as i32);
    }
    encounter.pkmn = new_pokemon;
    encounter.forme = new_forme;
    for linked in &mut encounter.linked_encounters {
        linked.pkmn = new_pokemon;
        linked.forme = new_forme;
    }
    Ok(())
}

fn set_forme_for_static_encounter<H: RomHandler + ?Sized>(
    handler: &H,
    encounter: &mut StaticEncounter,
    pokemon: PokemonId,
) -> Result<()> {
    let data = &handler.core().data;
    let mut random = handler.core().engine.random.clone();
    let pk = data.pk(pokemon);
    let mut check_cosmetics = true;
    encounter.forme = 0;
    if pk.forme_number > 0 {
        encounter.forme = pk.forme_number;
        encounter.pkmn = pk
            .base_forme
            .ok_or(RandomizationError::NullPointer("baseForme"))?;
        check_cosmetics = false;
    }
    let shown = data.pk(encounter.pkmn);
    if check_cosmetics && shown.cosmetic_forms > 0 {
        encounter.forme = shown
            .cosmetic_form_number(random_index(&mut random, shown.cosmetic_forms as usize)? as i32);
    } else if !check_cosmetics && pk.cosmetic_forms > 0 {
        encounter.forme +=
            pk.cosmetic_form_number(random_index(&mut random, pk.cosmetic_forms as usize)? as i32);
    }
    Ok(())
}

fn can_mega_evolve<H: RomHandler + ?Sized>(handler: &H, encounter: &StaticEncounter) -> bool {
    if encounter.held_item == 0 {
        return false;
    }
    let data = &handler.core().data;
    data.pk(encounter.pkmn)
        .mega_evolutions_from
        .iter()
        .any(|&mega| data.mega(mega).argument == encounter.held_item)
}

fn restricted_pokemon<H: RomHandler + ?Sized>(
    handler: &H,
    full: &[PokemonId],
    left: &mut Vec<PokemonId>,
    old: &StaticEncounter,
) -> Result<PokemonId> {
    let mut restricted: Vec<PokemonId> = left
        .iter()
        .copied()
        .filter(|pk| old.restricted_list.contains(pk))
        .collect();
    if restricted.is_empty() {
        restricted = full
            .iter()
            .copied()
            .filter(|pk| old.restricted_list.contains(pk))
            .collect();
    }
    let mut random = handler.core().engine.random.clone();
    let picked = take(&mut random, &mut restricted)?;
    remove_first(left, &picked);
    Ok(picked)
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

fn random_mega_stone<H: RomHandler + ?Sized>(handler: &H, pokemon: PokemonId) -> Result<i32> {
    let data = &handler.core().data;
    let megas = &data.pk(pokemon).mega_evolutions_from;
    let mut random = handler.core().engine.random.clone();
    Ok(data.mega(pick(&mut random, megas)?).argument)
}

fn mega_evo_pokemon<H: RomHandler + ?Sized>(
    handler: &H,
    full: &[PokemonId],
    left: &mut Vec<PokemonId>,
    encounter: &mut StaticEncounter,
) -> Result<PokemonId> {
    let sources = mega_evolution_sources(handler);
    let mut available: Vec<PokemonId> = sources
        .iter()
        .copied()
        .filter(|pk| left.contains(pk))
        .collect();
    if available.is_empty() {
        available = sources
            .iter()
            .copied()
            .filter(|pk| full.contains(pk))
            .collect();
    }
    let mut random = handler.core().engine.random.clone();
    let picked = take(&mut random, &mut available)?;
    remove_first(left, &picked);
    encounter.held_item = random_mega_stone(handler, picked)?;
    Ok(picked)
}

fn pick_static_power_lvl_replacement<H: RomHandler + ?Sized>(
    handler: &H,
    pool: &[PokemonId],
    current: PokemonId,
    ban_same: bool,
    limit_bst: bool,
) -> Result<PokemonId> {
    let data = &handler.core().data;
    let current_bst = data.pk(current).bst_for_power_levels();
    let mut min_target = if limit_bst {
        current_bst - current_bst / 5
    } else {
        current_bst - current_bst / 10
    };
    let mut max_target = if limit_bst {
        current_bst
    } else {
        current_bst + current_bst / 10
    };
    let mut can_pick: Vec<PokemonId> = Vec::new();
    let mut expand_rounds = 0;
    while can_pick.is_empty() || (can_pick.len() < 3 && expand_rounds < 3) {
        if pool.is_empty() && expand_rounds > 10_000 {
            return Err(RandomizationError::EmptyChoice);
        }
        for &pokemon in pool {
            let bst = data.pk(pokemon).bst_for_power_levels();
            if bst >= min_target
                && bst <= max_target
                && (!ban_same || pokemon != current)
                && !can_pick.contains(&pokemon)
            {
                can_pick.push(pokemon);
            }
        }
        min_target -= current_bst / 20;
        max_target += current_bst / 20;
        expand_rounds += 1;
    }
    let mut random = handler.core().engine.random.clone();
    pick(&mut random, &can_pick)
}

fn static_banned<H: RomHandler + ?Sized>(
    handler: &H,
    settings: &Settings,
    include_player_formes: bool,
) -> Vec<PokemonId> {
    let mut banned = handler.banned_for_static_pokemon();
    if include_player_formes {
        banned.extend(handler.get_banned_formes_for_player_pokemon());
    }
    if settings.abilities_mod != AbilitiesMod::Randomize {
        banned.extend(handler.get_ability_dependent_formes());
    }
    if settings.ban_irregular_alt_formes {
        banned.extend(handler.get_irregular_formes());
    }
    banned
}

pub fn randomize_static_pokemon<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let swap_legendaries = settings.static_pokemon_mod == StaticPokemonMod::RandomMatching;
    let similar_strength = settings.static_pokemon_mod == StaticPokemonMod::SimilarStrength;
    let limit_main_game_legendaries = settings.limit_main_game_legendaries;
    let limit600 = settings.limit600;
    let allow_alt_formes = settings.allow_static_alt_formes;
    let level_modifier = if settings.static_level_modified {
        settings.static_level_modifier
    } else {
        0
    };
    check_pokemon_restrictions(handler);
    let current = handler.get_static_pokemon();
    let mut replacements: Vec<StaticEncounter> = Vec::new();
    let banned = static_banned(handler, settings, true);
    let really_swap_megas = handler.force_swap_static_mega_evos() || settings.swap_static_mega_evos;
    let mut music_changes: BTreeMap<i32, i32> = BTreeMap::new();
    let change_music = if settings.correct_static_music {
        handler.get_special_music_statics()
    } else {
        Vec::new()
    };
    let mut random = handler.core().engine.random.clone();
    let pool = handler.core().engine.pool.clone();
    if swap_legendaries {
        let mut legendaries_left = pool.only_legendary.clone();
        if allow_alt_formes {
            legendaries_left.extend(pool.only_legendary_alts.iter().copied());
            legendaries_left = non_cosmetic(handler, &legendaries_left);
        }
        let mut nonlegs_left = pool.no_legendary.clone();
        if allow_alt_formes {
            nonlegs_left.extend(pool.no_legendary_alts.iter().copied());
            nonlegs_left = non_cosmetic(handler, &nonlegs_left);
        }
        let mut ultra_beasts_left = pool.ultra_beasts.clone();
        remove_all(&mut legendaries_left, &banned);
        remove_all(&mut nonlegs_left, &banned);
        remove_all(&mut ultra_beasts_left, &banned);
        let legendaries_pool = legendaries_left.clone();
        let nonlegs_pool = nonlegs_left.clone();
        let ultra_beasts_pool = ultra_beasts_left.clone();
        for old in &current {
            let mut new_static = clone_static_encounter(old);
            let new_pokemon;
            if handler.core().data.is_legendary(old.pkmn) {
                new_pokemon = if really_swap_megas && can_mega_evolve(handler, old) {
                    mega_evo_pokemon(
                        handler,
                        &pool.only_legendary,
                        &mut legendaries_left,
                        &mut new_static,
                    )?
                } else if old.restricted_pool {
                    restricted_pokemon(handler, &legendaries_pool, &mut legendaries_left, old)?
                } else {
                    take(&mut random, &mut legendaries_left)?
                };
                set_pokemon_and_forme_for_static_encounter(handler, &mut new_static, new_pokemon)?;
                if legendaries_left.is_empty() {
                    legendaries_left.extend(legendaries_pool.iter().copied());
                }
            } else if pool.ultra_beasts.contains(&old.pkmn) {
                new_pokemon = if old.restricted_pool {
                    restricted_pokemon(handler, &ultra_beasts_pool, &mut ultra_beasts_left, old)?
                } else {
                    take(&mut random, &mut ultra_beasts_left)?
                };
                set_pokemon_and_forme_for_static_encounter(handler, &mut new_static, new_pokemon)?;
                if ultra_beasts_left.is_empty() {
                    ultra_beasts_left.extend(ultra_beasts_pool.iter().copied());
                }
            } else {
                new_pokemon = if really_swap_megas && can_mega_evolve(handler, old) {
                    mega_evo_pokemon(
                        handler,
                        &pool.no_legendary,
                        &mut nonlegs_left,
                        &mut new_static,
                    )?
                } else if old.restricted_pool {
                    restricted_pokemon(handler, &nonlegs_pool, &mut nonlegs_left, old)?
                } else {
                    take(&mut random, &mut nonlegs_left)?
                };
                set_pokemon_and_forme_for_static_encounter(handler, &mut new_static, new_pokemon)?;
                if nonlegs_left.is_empty() {
                    nonlegs_left.extend(nonlegs_pool.iter().copied());
                }
            }
            replacements.push(new_static);
            let old_number = handler.core().data.pk(old.pkmn).number;
            if change_music.contains(&old_number) {
                music_changes.insert(old_number, handler.core().data.pk(new_pokemon).number);
            }
        }
    } else if similar_strength {
        let incl_formes_without_cosmetics = non_cosmetic(handler, &pool.main_incl_formes);
        let mut pokemon_left = if allow_alt_formes {
            incl_formes_without_cosmetics
        } else {
            pool.main.clone()
        };
        remove_all(&mut pokemon_left, &banned);
        let pokemon_pool = pokemon_left.clone();
        let main_game_legendaries = handler.get_main_game_legendaries();
        for old in &current {
            let mut new_static = clone_static_encounter(old);
            let old_pokemon = if old.forme > 0 {
                handler.get_alt_forme_of_pokemon(old.pkmn, old.forme)
            } else {
                old.pkmn
            };
            let data = &handler.core().data;
            let old_bst = data.pk(old_pokemon).bst_for_power_levels();
            let new_pokemon;
            if old_bst >= 600 && limit600 {
                new_pokemon = if really_swap_megas && can_mega_evolve(handler, old) {
                    mega_evo_pokemon(handler, &pool.main, &mut pokemon_left, &mut new_static)?
                } else if old.restricted_pool {
                    restricted_pokemon(handler, &pokemon_pool, &mut pokemon_left, old)?
                } else {
                    take(&mut random, &mut pokemon_left)?
                };
                set_pokemon_and_forme_for_static_encounter(handler, &mut new_static, new_pokemon)?;
            } else {
                let base_number = match data.pk(old_pokemon).base_forme {
                    None => data.pk(old_pokemon).number,
                    Some(base) => data.pk(base).number,
                };
                let limit_bst =
                    limit_main_game_legendaries && main_game_legendaries.contains(&base_number);
                if really_swap_megas && can_mega_evolve(handler, old) {
                    let sources = mega_evolution_sources(handler);
                    let mut available: Vec<PokemonId> = sources
                        .iter()
                        .copied()
                        .filter(|pk| pokemon_left.contains(pk))
                        .collect();
                    if available.is_empty() {
                        available = sources
                            .iter()
                            .copied()
                            .filter(|pk| pool.main.contains(pk))
                            .collect();
                    }
                    new_pokemon = pick_static_power_lvl_replacement(
                        handler,
                        &available,
                        old_pokemon,
                        true,
                        limit_bst,
                    )?;
                    new_static.held_item = random_mega_stone(handler, new_pokemon)?;
                } else if old.restricted_pool {
                    let mut restricted: Vec<PokemonId> = pokemon_left
                        .iter()
                        .copied()
                        .filter(|pk| old.restricted_list.contains(pk))
                        .collect();
                    if restricted.is_empty() {
                        restricted = pokemon_pool
                            .iter()
                            .copied()
                            .filter(|pk| old.restricted_list.contains(pk))
                            .collect();
                    }
                    new_pokemon = pick_static_power_lvl_replacement(
                        handler,
                        &restricted,
                        old_pokemon,
                        false,
                        limit_bst,
                    )?;
                } else {
                    new_pokemon = pick_static_power_lvl_replacement(
                        handler,
                        &pokemon_left,
                        old_pokemon,
                        true,
                        limit_bst,
                    )?;
                }
                remove_first(&mut pokemon_left, &new_pokemon);
                set_pokemon_and_forme_for_static_encounter(handler, &mut new_static, new_pokemon)?;
            }
            if pokemon_left.is_empty() {
                pokemon_left.extend(pokemon_pool.iter().copied());
            }
            replacements.push(new_static);
            let old_number = handler.core().data.pk(old.pkmn).number;
            if change_music.contains(&old_number) {
                music_changes.insert(old_number, handler.core().data.pk(new_pokemon).number);
            }
        }
    } else {
        let incl_formes_without_cosmetics = non_cosmetic(handler, &pool.main_incl_formes);
        let mut pokemon_left = if allow_alt_formes {
            incl_formes_without_cosmetics
        } else {
            pool.main.clone()
        };
        remove_all(&mut pokemon_left, &banned);
        let pokemon_pool = pokemon_left.clone();
        for old in &current {
            let mut new_static = clone_static_encounter(old);
            let new_pokemon = if really_swap_megas && can_mega_evolve(handler, old) {
                mega_evo_pokemon(handler, &pool.main, &mut pokemon_left, &mut new_static)?
            } else if old.restricted_pool {
                restricted_pokemon(handler, &pokemon_pool, &mut pokemon_left, old)?
            } else {
                take(&mut random, &mut pokemon_left)?
            };
            remove_first(&mut pokemon_left, &new_pokemon);
            set_pokemon_and_forme_for_static_encounter(handler, &mut new_static, new_pokemon)?;
            if pokemon_left.is_empty() {
                pokemon_left.extend(pokemon_pool.iter().copied());
            }
            replacements.push(new_static);
            let old_number = handler.core().data.pk(old.pkmn).number;
            if change_music.contains(&old_number) {
                music_changes.insert(old_number, handler.core().data.pk(new_pokemon).number);
            }
        }
    }
    if level_modifier != 0 {
        for encounter in &mut replacements {
            if !encounter.is_egg {
                encounter.level = modified_level(encounter.level, level_modifier);
                encounter.max_level = modified_level(encounter.max_level, level_modifier);
                for linked in &mut encounter.linked_encounters {
                    if !linked.is_egg {
                        linked.level = modified_level(linked.level, level_modifier);
                        linked.max_level = modified_level(linked.max_level, level_modifier);
                    }
                }
            }
        }
    }
    if !music_changes.is_empty() {
        handler.apply_correct_static_music(&music_changes);
    }
    handler.set_static_pokemon(&replacements)?;
    Ok(())
}

pub fn only_change_static_levels<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let level_modifier = settings.static_level_modifier;
    let mut current = handler.get_static_pokemon();
    for encounter in &mut current {
        if !encounter.is_egg {
            encounter.level = modified_level(encounter.level, level_modifier);
            for linked in &mut encounter.linked_encounters {
                if !linked.is_egg {
                    linked.level = modified_level(linked.level, level_modifier);
                }
            }
        }
        let pokemon = encounter.pkmn;
        set_pokemon_and_forme_for_static_encounter(handler, encounter, pokemon)?;
    }
    handler.set_static_pokemon(&current)?;
    Ok(())
}

pub fn randomize_totem_pokemon<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let randomize_totem = matches!(
        settings.totem_pokemon_mod,
        TotemPokemonMod::Random | TotemPokemonMod::SimilarStrength
    );
    let randomize_allies = matches!(
        settings.ally_pokemon_mod,
        AllyPokemonMod::Random | AllyPokemonMod::SimilarStrength
    );
    let randomize_auras = matches!(settings.aura_mod, AuraMod::Random | AuraMod::SameStrength);
    let similar_totem = settings.totem_pokemon_mod == TotemPokemonMod::SimilarStrength;
    let similar_allies = settings.ally_pokemon_mod == AllyPokemonMod::SimilarStrength;
    let similar_auras = settings.aura_mod == AuraMod::SameStrength;
    let level_modifier = if settings.totem_levels_modified {
        settings.totem_level_modifier
    } else {
        0
    };
    let allow_alt_formes = settings.allow_totem_alt_formes;
    check_pokemon_restrictions(handler);
    let current = handler.get_totem_pokemon();
    let banned = static_banned(handler, settings, false);
    let pool = handler.core().engine.pool.clone();
    let without_cosmetics = non_cosmetic(handler, &pool.main_incl_formes);
    let refill_source = if allow_alt_formes {
        without_cosmetics
    } else {
        pool.main.clone()
    };
    let mut pokemon_left = refill_source.clone();
    remove_all(&mut pokemon_left, &banned);
    let mut random = handler.core().engine.random.clone();
    let mut replacements: Vec<TotemPokemon> = Vec::new();
    for old in &current {
        let mut new_totem = TotemPokemon {
            encounter: StaticEncounter {
                held_item: old.encounter.held_item,
                ..StaticEncounter::default()
            },
            aura: old.aura,
            ally1_offset: 0,
            ally2_offset: 0,
            allies: BTreeMap::new(),
            unused: false,
        };
        if randomize_totem {
            let old_pokemon = if old.encounter.forme > 0 {
                handler.get_alt_forme_of_pokemon(old.encounter.pkmn, old.encounter.forme)
            } else {
                old.encounter.pkmn
            };
            let new_pokemon = if similar_totem {
                pick_static_power_lvl_replacement(handler, &pokemon_left, old_pokemon, true, false)?
            } else {
                take(&mut random, &mut pokemon_left)?
            };
            remove_first(&mut pokemon_left, &new_pokemon);
            new_totem.encounter.pkmn = new_pokemon;
            set_forme_for_static_encounter(handler, &mut new_totem.encounter, new_pokemon)?;
            new_totem.encounter.reset_moves = true;
            new_totem.encounter.level = old.encounter.level;
            if level_modifier != 0 {
                new_totem.encounter.level =
                    modified_level(new_totem.encounter.level, level_modifier);
            }
            if pokemon_left.is_empty() {
                pokemon_left.extend(refill_source.iter().copied());
                remove_all(&mut pokemon_left, &banned);
            }
        } else {
            new_totem.encounter.pkmn = old.encounter.pkmn;
            new_totem.encounter.level = old.encounter.level;
            if level_modifier != 0 {
                new_totem.encounter.level =
                    modified_level(new_totem.encounter.level, level_modifier);
            }
            let pokemon = new_totem.encounter.pkmn;
            set_forme_for_static_encounter(handler, &mut new_totem.encounter, pokemon)?;
        }
        if randomize_allies {
            for (&ally_index, old_ally) in &old.allies {
                let old_ally_pokemon = if old_ally.forme > 0 {
                    handler.get_alt_forme_of_pokemon(old_ally.pkmn, old_ally.forme)
                } else {
                    old_ally.pkmn
                };
                let new_ally_pokemon = if similar_allies {
                    pick_static_power_lvl_replacement(
                        handler,
                        &pokemon_left,
                        old_ally_pokemon,
                        true,
                        false,
                    )?
                } else {
                    take(&mut random, &mut pokemon_left)?
                };
                remove_first(&mut pokemon_left, &new_ally_pokemon);
                let mut new_ally = StaticEncounter::new(new_ally_pokemon);
                set_forme_for_static_encounter(handler, &mut new_ally, new_ally_pokemon)?;
                new_ally.reset_moves = true;
                new_ally.level = old_ally.level;
                if level_modifier != 0 {
                    new_ally.level = modified_level(new_ally.level, level_modifier);
                }
                new_totem.allies.insert(ally_index, new_ally);
                if pokemon_left.is_empty() {
                    pokemon_left.extend(refill_source.iter().copied());
                    remove_all(&mut pokemon_left, &banned);
                }
            }
        } else {
            new_totem.allies = old.allies.clone();
            for ally in new_totem.allies.values_mut() {
                if level_modifier != 0 {
                    ally.level = modified_level(ally.level, level_modifier);
                    let pokemon = ally.pkmn;
                    set_forme_for_static_encounter(handler, ally, pokemon)?;
                }
            }
        }
        if randomize_auras {
            new_totem.aura = if similar_auras {
                Aura::random_similar_strength(&mut random, old.aura)
            } else {
                Aura::random(&mut random)
            };
        } else {
            new_totem.aura = old.aura;
        }
        if settings.randomize_totem_held_items && old.encounter.held_item != 0 {
            let consumables = handler.get_all_consumable_held_items();
            new_totem.encounter.held_item = pick(&mut random, &consumables)?;
        }
        replacements.push(new_totem);
    }
    handler.set_totem_pokemon(&replacements)
}
