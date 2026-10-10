use randomon_data::{ExpCurve, PokemonId, stat};
use randomon_settings::{ExpCurveMod, Settings};

use crate::engine::copy_up::{CopyUpActions, copy_up_evolutions_helper};
use crate::error::Result;
use crate::handler::RomHandler;

fn shuffle_one<H: RomHandler + ?Sized>(handler: &mut H, pokemon: PokemonId) {
    let mut random = handler.core().engine.random.clone();
    handler
        .core_mut()
        .data
        .pk_mut(pokemon)
        .shuffle_stats(&mut random);
}

fn copy_shuffled_up<H: RomHandler + ?Sized>(handler: &mut H, from: PokemonId, to: PokemonId) {
    let data = &mut handler.core_mut().data;
    let order = data.pk(from).shuffled_stats_order.clone();
    data.pk_mut(to).copy_shuffled_stats_up_evolution(&order);
}

fn randomize_one<H: RomHandler + ?Sized>(handler: &mut H, pokemon: PokemonId) {
    let mut random = handler.core().engine.random.clone();
    handler
        .core_mut()
        .data
        .pk_mut(pokemon)
        .randomize_stats_within_bst(&mut random);
}

fn assign_new_stats<H: RomHandler + ?Sized>(handler: &mut H, from: PokemonId, to: PokemonId) {
    let mut random = handler.core().engine.random.clone();
    let data = &mut handler.core_mut().data;
    let source = data.pk(from).stats();
    data.pk_mut(to)
        .assign_new_stats_for_evolution(&source, &mut random);
}

fn copy_randomized_up<H: RomHandler + ?Sized>(handler: &mut H, from: PokemonId, to: PokemonId) {
    let data = &mut handler.core_mut().data;
    let source = data.pk(from).stats();
    data.pk_mut(to).copy_randomized_stats_up_evolution(&source);
}

fn copy_base_stats_to_cosmetic_formes<H: RomHandler + ?Sized>(handler: &mut H) {
    let all = handler.get_pokemon_incl_formes();
    let data = &mut handler.core_mut().data;
    for pokemon in all.into_iter().flatten() {
        if data.pk(pokemon).actually_cosmetic
            && let Some(base) = data.pk(pokemon).base_forme
        {
            let stats = data.pk(base).stats();
            data.pk_mut(pokemon).copy_base_forme_base_stats(&stats);
        }
    }
}

fn single_mega_sources<H: RomHandler + ?Sized>(handler: &H) -> Vec<(PokemonId, PokemonId, bool)> {
    let data = &handler.core().data;
    handler
        .get_mega_evolutions()
        .into_iter()
        .map(|mega| {
            let mega = data.mega(mega);
            (
                mega.from,
                mega.to,
                data.pk(mega.from).mega_evolutions_from.len() > 1,
            )
        })
        .collect()
}

pub fn shuffle_pokemon_stats<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    if settings.base_stats_follow_evolutions {
        copy_up_evolutions_helper(
            handler,
            &mut (),
            CopyUpActions {
                base: &mut |h: &mut H, _: &mut (), pk| {
                    shuffle_one(h, pk);
                    Ok(())
                },
                evolved: &mut |h: &mut H, _: &mut (), from, to, _| {
                    copy_shuffled_up(h, from, to);
                    Ok(())
                },
                split: None,
                copy_split_evos: false,
            },
        )?;
    } else {
        for pokemon in handler.get_pokemon_incl_formes().into_iter().flatten() {
            shuffle_one(handler, pokemon);
        }
    }
    copy_base_stats_to_cosmetic_formes(handler);
    if settings.base_stats_follow_mega_evolutions {
        for (from, to, has_several) in single_mega_sources(handler) {
            if !has_several {
                copy_shuffled_up(handler, from, to);
            }
        }
    }
    Ok(())
}

pub fn randomize_pokemon_stats<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let assign_randomly = settings.assign_evo_stats_randomly;
    if settings.base_stats_follow_evolutions {
        copy_up_evolutions_helper(
            handler,
            &mut (),
            CopyUpActions {
                base: &mut |h: &mut H, _: &mut (), pk| {
                    randomize_one(h, pk);
                    Ok(())
                },
                evolved: &mut |h: &mut H, _: &mut (), from, to, _| {
                    if assign_randomly {
                        assign_new_stats(h, from, to);
                    } else {
                        copy_randomized_up(h, from, to);
                    }
                    Ok(())
                },
                split: Some(&mut |h: &mut H, _: &mut (), from, to, _| {
                    assign_new_stats(h, from, to);
                    Ok(())
                }),
                copy_split_evos: true,
            },
        )?;
    } else {
        for pokemon in handler.get_pokemon_incl_formes().into_iter().flatten() {
            randomize_one(handler, pokemon);
        }
    }
    copy_base_stats_to_cosmetic_formes(handler);
    if settings.base_stats_follow_mega_evolutions {
        for (from, to, has_several) in single_mega_sources(handler) {
            if has_several || assign_randomly {
                assign_new_stats(handler, from, to);
            } else {
                copy_randomized_up(handler, from, to);
            }
        }
    }
    Ok(())
}

pub fn update_pokemon_stats<H: RomHandler + ?Sized>(handler: &mut H, settings: &Settings) {
    let generation = settings.update_base_stats_to_generation;
    let pokes = handler.get_pokemon_incl_formes();
    let is_gen1 = handler.generation_of_pokemon() == 1;
    for target_generation in 6..=generation {
        let changes = handler.get_updated_pokemon_stats(target_generation);
        for (index, slot) in pokes.iter().enumerate().skip(1) {
            let Some(change) = changes.get(&(index as i32)) else {
                continue;
            };
            let Some(pokemon) = slot else {
                continue;
            };
            let pk = handler.core_mut().data.pk_mut(*pokemon);
            let mut values = change.values.iter().copied();
            if change.stat & stat::HP != 0 {
                pk.hp = values.next().unwrap_or(pk.hp);
            }
            if change.stat & stat::ATK != 0 {
                pk.attack = values.next().unwrap_or(pk.attack);
            }
            if change.stat & stat::DEF != 0 {
                pk.defense = values.next().unwrap_or(pk.defense);
            }
            if change.stat & stat::SPATK != 0 {
                let value = values.next();
                if !is_gen1 {
                    pk.spatk = value.unwrap_or(pk.spatk);
                }
            }
            if change.stat & stat::SPDEF != 0 {
                let value = values.next();
                if !is_gen1 {
                    pk.spdef = value.unwrap_or(pk.spdef);
                }
            }
            if change.stat & stat::SPEED != 0 {
                pk.speed = values.next().unwrap_or(pk.speed);
            }
            if change.stat & stat::SPECIAL != 0 {
                pk.special = values.next().unwrap_or(pk.special);
            }
        }
    }
}

pub fn standardize_exp_curves<H: RomHandler + ?Sized>(handler: &mut H, settings: &Settings) {
    let curve = settings.selected_exp_curve;
    let pokes = handler.get_pokemon_incl_formes();
    let data = &mut handler.core_mut().data;
    for pokemon in pokes.into_iter().flatten() {
        let new_curve = match settings.exp_curve_mod {
            ExpCurveMod::Legendaries if data.is_legendary(pokemon) => Some(ExpCurve::Slow),
            ExpCurveMod::StrongLegendaries if data.is_strong_legendary(pokemon) => {
                Some(ExpCurve::Slow)
            }
            _ => curve,
        };
        data.pk_mut(pokemon).growth_curve = new_curve;
    }
}

pub fn change_catch_rates<H: RomHandler + ?Sized>(handler: &mut H, settings: &Settings) {
    let level = settings.minimum_catch_rate_level;
    if level == 5 {
        handler.enable_guaranteed_pokemon_catching();
        return;
    }
    let (normal, legendary) = match level {
        2 => (128, 64),
        3 => (200, 100),
        4 => (255, 255),
        _ => (75, 37),
    };
    handler.minimum_catch_rate(normal, legendary);
}

pub fn minimum_catch_rate<H: RomHandler + ?Sized>(
    handler: &mut H,
    rate_non_legendary: i32,
    rate_legendary: i32,
) {
    let pokes = handler.get_pokemon_incl_formes();
    let data = &mut handler.core_mut().data;
    for pokemon in pokes.into_iter().flatten() {
        let minimum = if data.is_legendary(pokemon) {
            rate_legendary
        } else {
            rate_non_legendary
        };
        let pk = data.pk_mut(pokemon);
        pk.catch_rate = pk.catch_rate.max(minimum);
    }
}
