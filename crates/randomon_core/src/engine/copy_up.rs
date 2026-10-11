use std::collections::BTreeSet;

use randomon_data::{EvolutionId, GameData, PokemonId};

use crate::error::{RandomizationError, Result};
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

const CHAIN_LIMIT: usize = 100_000;

fn first_evolution_to(data: &GameData, pokemon: PokemonId) -> Result<EvolutionId> {
    data.evolutions_to(pokemon)
        .first()
        .copied()
        .ok_or(RandomizationError::IndexOutOfBounds("evolutionsTo.get(0)"))
}

enum ChainWalk {
    Reached(Vec<EvolutionId>, EvolutionId),
    Cycle(PokemonId),
}

fn walk_to_flagged_ancestor(data: &GameData, pokemon: PokemonId) -> Result<ChainWalk> {
    let mut stack: Vec<EvolutionId> = Vec::new();
    let mut visited = vec![pokemon];
    let mut evo = first_evolution_to(data, pokemon)?;
    loop {
        let from = data.evo(evo).from;
        if data.pk(from).temporary_flag {
            return Ok(ChainWalk::Reached(stack, evo));
        }
        if visited.contains(&from) {
            return Ok(ChainWalk::Cycle(from));
        }
        visited.push(from);
        stack.push(evo);
        if stack.len() > CHAIN_LIMIT {
            return Err(RandomizationError::randomization(
                "evolution chain does not reach a base Pokemon",
            ));
        }
        evo = first_evolution_to(data, from)?;
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
        if handler.core().data.pk(pokemon).temporary_flag {
            continue;
        }
        let (mut stack, evo) = loop {
            match walk_to_flagged_ancestor(&handler.core().data, pokemon)? {
                ChainWalk::Reached(stack, evo) => break (stack, evo),
                ChainWalk::Cycle(cycle_start) => {
                    base(handler, context, cycle_start)?;
                    handler.core_mut().data.pk_mut(cycle_start).temporary_flag = true;
                }
            }
        };
        let (from, to) = {
            let evolution = handler.core().data.evo(evo);
            (evolution.from, evolution.to)
        };
        let to_is_final = !middles.contains(&to);
        match split.as_mut() {
            Some(split_action) if copy_split_evos && splits.contains(&to) => {
                split_action(handler, context, from, to, to_is_final)?
            }
            _ => evolved(handler, context, from, to, to_is_final)?,
        }
        handler.core_mut().data.pk_mut(to).temporary_flag = true;
        while let Some(evo) = stack.pop() {
            let (from, to) = {
                let evolution = handler.core().data.evo(evo);
                (evolution.from, evolution.to)
            };
            let to_is_final = !middles.contains(&to);
            match split.as_mut() {
                Some(split_action) if copy_split_evos && splits.contains(&pokemon) => {
                    split_action(handler, context, from, to, to_is_final)?
                }
                _ => evolved(handler, context, from, to, to_is_final)?,
            }
            handler.core_mut().data.pk_mut(to).temporary_flag = true;
        }
    }
    Ok(())
}
