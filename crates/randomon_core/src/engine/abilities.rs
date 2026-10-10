use randomon_data::PokemonId;
use randomon_data::constants::{abilities, global};
use randomon_java::JavaRng;
use randomon_settings::Settings;

use crate::engine::copy_up::{CopyUpActions, copy_up_evolutions_helper};
use crate::error::Result;
use crate::handler::RomHandler;

const GEN3_AIR_LOCK_INDEX: i32 = 77;

struct AbilityPicker {
    max_ability: i32,
    banned: Vec<i32>,
    use_variations: bool,
    ensure_two: bool,
    has_dw_abilities: bool,
}

fn pick_random_ability_variation<H: RomHandler + ?Sized>(
    handler: &H,
    selected: i32,
    already_set: &[i32],
) -> Result<i32> {
    let mut random = handler.core().engine.random.clone();
    let mut new_ability = selected;
    loop {
        let variations = handler.get_ability_variations();
        if let Some(options) = variations.get(&selected) {
            new_ability = crate::engine::util::pick(&mut random, options)?;
        }
        if !already_set.contains(&new_ability) {
            return Ok(new_ability);
        }
    }
}

fn pick_random_ability<H: RomHandler + ?Sized>(
    handler: &H,
    picker: &AbilityPicker,
    already_set: &[i32],
) -> Result<i32> {
    let mut random = handler.core().engine.random.clone();
    loop {
        let new_ability = random.next_int_bounded(picker.max_ability) + 1;
        if picker.banned.contains(&new_ability) || already_set.contains(&new_ability) {
            continue;
        }
        if picker.use_variations {
            return pick_random_ability_variation(handler, new_ability, already_set);
        }
        return Ok(new_ability);
    }
}

fn has_wonder_guard<H: RomHandler + ?Sized>(handler: &H, pokemon: PokemonId) -> bool {
    let pk = handler.core().data.pk(pokemon);
    pk.ability1 == abilities::WONDER_GUARD
        || pk.ability2 == abilities::WONDER_GUARD
        || pk.ability3 == abilities::WONDER_GUARD
}

fn randomize_pokemon_abilities<H: RomHandler + ?Sized>(
    handler: &mut H,
    picker: &AbilityPicker,
    pokemon: PokemonId,
) -> Result<()> {
    if has_wonder_guard(handler, pokemon) {
        return Ok(());
    }
    let first = pick_random_ability(handler, picker, &[])?;
    handler.core_mut().data.pk_mut(pokemon).ability1 = first;
    let mut random = handler.core().engine.random.clone();
    let second = if picker.ensure_two || random.next_double() < 0.5 {
        pick_random_ability(handler, picker, &[first])?
    } else {
        0
    };
    handler.core_mut().data.pk_mut(pokemon).ability2 = second;
    if picker.has_dw_abilities {
        let third = pick_random_ability(handler, picker, &[first, second])?;
        handler.core_mut().data.pk_mut(pokemon).ability3 = third;
    }
    Ok(())
}

pub fn randomize_abilities<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    if handler.abilities_per_pokemon() == 0 {
        return Ok(());
    }
    let mut banned = handler.get_useless_abilities();
    if !settings.allow_wonder_guard {
        banned.push(abilities::WONDER_GUARD);
    }
    if settings.ban_trapping_abilities {
        banned.extend_from_slice(global::BATTLE_TRAPPING_ABILITIES);
    }
    if settings.ban_negative_abilities {
        banned.extend_from_slice(global::NEGATIVE_ABILITIES);
    }
    if settings.ban_bad_abilities {
        banned.extend_from_slice(global::BAD_ABILITIES);
        if !settings.double_battle_mode {
            banned.extend_from_slice(global::DOUBLE_BATTLE_ABILITIES);
        }
    }
    if settings.weigh_duplicate_abilities_together {
        banned.extend_from_slice(global::DUPLICATE_ABILITIES);
        if handler.generation_of_pokemon() == 3 {
            banned.push(GEN3_AIR_LOCK_INDEX);
        }
    }
    let picker = AbilityPicker {
        max_ability: handler.highest_ability_index(),
        banned,
        use_variations: settings.weigh_duplicate_abilities_together,
        ensure_two: settings.ensure_two_abilities,
        has_dw_abilities: handler.abilities_per_pokemon() == 3,
    };
    if settings.abilities_follow_evolutions {
        copy_up_evolutions_helper(
            handler,
            &mut (),
            CopyUpActions {
                base: &mut |h: &mut H, _: &mut (), pk| randomize_pokemon_abilities(h, &picker, pk),
                evolved: &mut |h: &mut H, _: &mut (), from, to, _| {
                    if !has_wonder_guard(h, to) {
                        let data = &mut h.core_mut().data;
                        let (first, second, third) = {
                            let source = data.pk(from);
                            (source.ability1, source.ability2, source.ability3)
                        };
                        let target = data.pk_mut(to);
                        target.ability1 = first;
                        target.ability2 = second;
                        target.ability3 = third;
                    }
                    Ok(())
                },
                split: None,
                copy_split_evos: false,
            },
        )?;
    } else {
        for pokemon in handler.get_pokemon_incl_formes().into_iter().flatten() {
            randomize_pokemon_abilities(handler, &picker, pokemon)?;
        }
    }
    let all = handler.get_pokemon_incl_formes();
    let data = &mut handler.core_mut().data;
    for pokemon in all.into_iter().flatten() {
        if data.pk(pokemon).actually_cosmetic
            && let Some(base) = data.pk(pokemon).base_forme
        {
            let (first, second, third) = {
                let source = data.pk(base);
                (source.ability1, source.ability2, source.ability3)
            };
            let target = data.pk_mut(pokemon);
            target.ability1 = first;
            target.ability2 = second;
            target.ability3 = third;
        }
    }
    if settings.abilities_follow_mega_evolutions {
        for mega in handler.get_mega_evolutions() {
            let data = &mut handler.core_mut().data;
            let (from, to) = (data.mega(mega).from, data.mega(mega).to);
            if data.pk(from).mega_evolutions_from.len() > 1 {
                continue;
            }
            let (first, second, third) = {
                let source = data.pk(from);
                (source.ability1, source.ability2, source.ability3)
            };
            let target = data.pk_mut(to);
            target.ability1 = first;
            target.ability2 = second;
            target.ability3 = third;
        }
    }
    Ok(())
}
