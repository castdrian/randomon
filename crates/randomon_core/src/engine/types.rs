use randomon_data::{PokemonId, Type};
use randomon_settings::{Settings, TypesMod};

use crate::engine::copy_up::{CopyUpActions, copy_up_evolutions_helper};
use crate::error::Result;
use crate::handler::RomHandler;

pub fn random_type<H: RomHandler + ?Sized>(handler: &H) -> Type {
    let mut random = handler.core().engine.random.clone();
    let mut kind = Type::random_type(&mut random);
    while !handler.type_in_game(kind) {
        kind = Type::random_type(&mut random);
    }
    kind
}

fn random_type_other_than<H: RomHandler + ?Sized>(handler: &H, excluded: Option<Type>) -> Type {
    let mut kind = random_type(handler);
    while Some(kind) == excluded {
        kind = random_type(handler);
    }
    kind
}

fn next_double<H: RomHandler + ?Sized>(handler: &H) -> f64 {
    use randomon_java::JavaRng;
    handler.core().engine.random.clone().next_double()
}

fn randomize_base_types<H: RomHandler + ?Sized>(
    handler: &mut H,
    pokemon: PokemonId,
    dual_type_only: bool,
) {
    let primary = random_type(handler);
    handler.core_mut().data.pk_mut(pokemon).primary_type = Some(primary);
    handler.core_mut().data.pk_mut(pokemon).secondary_type = None;
    let data = &handler.core().data;
    let single_carrying_evolution = data.evolutions_from(pokemon).len() == 1
        && data.evo(data.evolutions_from(pokemon)[0]).carry_stats;
    let chance = if single_carrying_evolution { 0.35 } else { 0.5 };
    if next_double(handler) < chance || dual_type_only {
        let secondary = random_type_other_than(handler, Some(primary));
        handler.core_mut().data.pk_mut(pokemon).secondary_type = Some(secondary);
    }
}

fn copy_types_up<H: RomHandler + ?Sized>(
    handler: &mut H,
    from: PokemonId,
    to: PokemonId,
    to_is_final: bool,
    dual_type_only: bool,
) {
    let data = &mut handler.core_mut().data;
    let (primary, secondary) = {
        let source = data.pk(from);
        (source.primary_type, source.secondary_type)
    };
    data.pk_mut(to).primary_type = primary;
    data.pk_mut(to).secondary_type = secondary;
    if secondary.is_none() {
        let chance = if to_is_final { 0.25 } else { 0.15 };
        if next_double(handler) < chance || dual_type_only {
            let new_secondary = random_type_other_than(handler, primary);
            handler.core_mut().data.pk_mut(to).secondary_type = Some(new_secondary);
        }
    }
}

pub fn randomize_pokemon_types<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let evolution_sanity = settings.types_mod == TypesMod::RandomFollowEvolutions;
    let dual_type_only = settings.dual_type_only;
    let all = handler.get_pokemon_incl_formes();
    if evolution_sanity {
        copy_up_evolutions_helper(
            handler,
            &mut (),
            CopyUpActions {
                base: &mut |h: &mut H, _: &mut (), pk| {
                    randomize_base_types(h, pk, dual_type_only);
                    Ok(())
                },
                evolved: &mut |h: &mut H, _: &mut (), from, to, to_is_final| {
                    copy_types_up(h, from, to, to_is_final, dual_type_only);
                    Ok(())
                },
                split: None,
                copy_split_evos: false,
            },
        )?;
    } else {
        for pokemon in all.iter().flatten().copied() {
            let primary = random_type(handler);
            handler.core_mut().data.pk_mut(pokemon).primary_type = Some(primary);
            handler.core_mut().data.pk_mut(pokemon).secondary_type = None;
            if next_double(handler) < 0.5 || settings.dual_type_only {
                let secondary = random_type_other_than(handler, Some(primary));
                handler.core_mut().data.pk_mut(pokemon).secondary_type = Some(secondary);
            }
        }
    }
    let data = &mut handler.core_mut().data;
    for pokemon in all.iter().flatten().copied() {
        if data.pk(pokemon).actually_cosmetic
            && let Some(base) = data.pk(pokemon).base_forme
        {
            let (primary, secondary) = (data.pk(base).primary_type, data.pk(base).secondary_type);
            data.pk_mut(pokemon).primary_type = primary;
            data.pk_mut(pokemon).secondary_type = secondary;
        }
    }
    if settings.types_follow_mega_evolutions {
        let megas = handler.get_mega_evolutions();
        for mega in megas {
            let (from, to) = {
                let mega = handler.core().data.mega(mega);
                (mega.from, mega.to)
            };
            if handler.core().data.pk(from).mega_evolutions_from.len() > 1 {
                continue;
            }
            let data = &mut handler.core_mut().data;
            let (primary, secondary) = (data.pk(from).primary_type, data.pk(from).secondary_type);
            data.pk_mut(to).primary_type = primary;
            data.pk_mut(to).secondary_type = secondary;
            if secondary.is_none() && next_double(handler) < 0.25 {
                let new_secondary = random_type_other_than(handler, primary);
                handler.core_mut().data.pk_mut(to).secondary_type = Some(new_secondary);
            }
        }
    }
    Ok(())
}
