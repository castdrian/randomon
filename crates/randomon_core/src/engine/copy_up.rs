use std::collections::BTreeSet;

use randomon_data::{EvolutionId, GameData, PokemonId};

use crate::error::Result;
use crate::handler::RomHandler;

pub fn basic_pokemon(data: &GameData, all: &[Option<PokemonId>]) -> BTreeSet<PokemonId> {
    all.iter()
        .flatten()
        .copied()
        .filter(|&pokemon| data.evolutions_to(pokemon).is_empty())
        .collect()
}

pub fn split_evolutions(data: &GameData, all: &[Option<PokemonId>]) -> BTreeSet<PokemonId> {
    all.iter()
        .flatten()
        .copied()
        .filter(|&pokemon| {
            data.evolutions_to(pokemon)
                .first()
                .is_some_and(|&evo| !data.evo(evo).carry_stats)
        })
        .collect()
}

pub fn middle_evolutions(
    data: &GameData,
    all: &[Option<PokemonId>],
    include_split_evos: bool,
) -> BTreeSet<PokemonId> {
    all.iter()
        .flatten()
        .copied()
        .filter(|&pokemon| {
            let to = data.evolutions_to(pokemon);
            to.len() == 1
                && !data.evolutions_from(pokemon).is_empty()
                && (data.evo(to[0]).carry_stats || include_split_evos)
        })
        .collect()
}

pub fn final_evolutions(
    data: &GameData,
    all: &[Option<PokemonId>],
    include_split_evos: bool,
) -> BTreeSet<PokemonId> {
    all.iter()
        .flatten()
        .copied()
        .filter(|&pokemon| {
            let to = data.evolutions_to(pokemon);
            to.len() == 1
                && data.evolutions_from(pokemon).is_empty()
                && (data.evo(to[0]).carry_stats || include_split_evos)
        })
        .collect()
}

pub type BaseAction<'a, H, C> = dyn FnMut(&mut H, &mut C, PokemonId) -> Result<()> + 'a;
pub type EvolvedAction<'a, H, C> =
    dyn FnMut(&mut H, &mut C, PokemonId, PokemonId, bool) -> Result<()> + 'a;

pub struct CopyUpActions<'a, H: ?Sized, C> {
    pub base: &'a mut BaseAction<'a, H, C>,
    pub evolved: &'a mut EvolvedAction<'a, H, C>,
    pub split: Option<&'a mut EvolvedAction<'a, H, C>>,
    pub copy_split_evos: bool,
}

fn is_flagged<H: RomHandler + ?Sized>(handler: &H, pokemon: PokemonId) -> bool {
    handler.core().data.pk(pokemon).temporary_flag
}

fn flag<H: RomHandler + ?Sized>(handler: &mut H, pokemon: PokemonId) {
    handler.core_mut().data.pk_mut(pokemon).temporary_flag = true;
}

fn chain_to_flagged_ancestor<H: RomHandler + ?Sized, C>(
    handler: &mut H,
    context: &mut C,
    base: &mut BaseAction<'_, H, C>,
    pokemon: PokemonId,
) -> Result<Vec<EvolutionId>> {
    let mut chain: Vec<EvolutionId> = Vec::new();
    let mut on_path = BTreeSet::from([pokemon]);
    let mut current = pokemon;
    loop {
        let Some(evo) = handler.core().data.evolutions_to(current).first().copied() else {
            base(handler, context, current)?;
            flag(handler, current);
            return Ok(chain);
        };
        let from = handler.core().data.evo(evo).from;
        chain.push(evo);
        if is_flagged(handler, from) {
            return Ok(chain);
        }
        if !on_path.insert(from) {
            base(handler, context, from)?;
            flag(handler, from);
            return Ok(chain);
        }
        current = from;
    }
}

pub fn copy_up_evolutions_helper<H: RomHandler + ?Sized, C>(
    handler: &mut H,
    context: &mut C,
    actions: CopyUpActions<'_, H, C>,
) -> Result<()> {
    let CopyUpActions {
        base,
        evolved,
        mut split,
        copy_split_evos,
    } = actions;
    let all = handler.get_pokemon_incl_formes();
    let all_without_formes = handler.get_pokemon();
    for &pokemon in all.iter().flatten() {
        handler.core_mut().data.pk_mut(pokemon).temporary_flag = false;
    }
    let data = &handler.core().data;
    let basic = basic_pokemon(data, &all);
    let splits = split_evolutions(data, &all);
    let middles = middle_evolutions(data, &all_without_formes, copy_split_evos);
    for &pokemon in &basic {
        base(handler, context, pokemon)?;
        handler.core_mut().data.pk_mut(pokemon).temporary_flag = true;
    }
    if !copy_split_evos {
        for &pokemon in &splits {
            base(handler, context, pokemon)?;
            handler.core_mut().data.pk_mut(pokemon).temporary_flag = true;
        }
    }
    for &pokemon in all.iter().flatten() {
        if is_flagged(handler, pokemon) {
            continue;
        }
        let mut chain = chain_to_flagged_ancestor(handler, context, base, pokemon)?;
        let mut first_step = true;
        while let Some(evo) = chain.pop() {
            let (from, to) = {
                let evolution = handler.core().data.evo(evo);
                (evolution.from, evolution.to)
            };
            if is_flagged(handler, to) {
                first_step = false;
                continue;
            }
            let to_is_final = !middles.contains(&to);
            let split_target = if first_step { to } else { pokemon };
            first_step = false;
            match split.as_mut() {
                Some(split_action) if copy_split_evos && splits.contains(&split_target) => {
                    split_action(handler, context, from, to, to_is_final)?
                }
                _ => evolved(handler, context, from, to, to_is_final)?,
            }
            flag(handler, to);
        }
    }
    Ok(())
}
