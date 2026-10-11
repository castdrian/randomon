use randomon_data::PokemonId;
use randomon_settings::{AbilitiesMod, Settings};

use crate::engine::choose::first_non_empty;
use crate::engine::pool::{pool_choice, two_evo_candidates};
use crate::engine::util::pick;
use crate::error::Result;
use crate::handler::RomHandler;

fn starter_banned_list<H: RomHandler + ?Sized>(handler: &H, settings: &Settings) -> Vec<PokemonId> {
    let mut banned = handler.get_banned_formes_for_player_pokemon();
    if settings.abilities_mod == AbilitiesMod::Unchanged {
        banned.extend(handler.get_ability_dependent_formes());
    }
    if settings.ban_irregular_alt_formes {
        banned.extend(handler.get_irregular_formes());
    }
    banned
}

fn pick_starter<H: RomHandler + ?Sized>(
    handler: &H,
    candidates: &[PokemonId],
    picked: &[PokemonId],
    banned: &[PokemonId],
) -> Result<PokemonId> {
    let data = &handler.core().data;
    let usable: Vec<PokemonId> = candidates
        .iter()
        .copied()
        .filter(|&pokemon| data.exists(pokemon) && !data.pk(pokemon).actually_cosmetic)
        .collect();
    let fresh: Vec<PokemonId> = usable
        .iter()
        .copied()
        .filter(|pokemon| !picked.contains(pokemon))
        .collect();
    let allowed: Vec<PokemonId> = fresh
        .iter()
        .copied()
        .filter(|pokemon| !banned.contains(pokemon))
        .collect();
    let choices = first_non_empty(&[&allowed, &fresh, &usable, candidates]);
    let mut random = handler.core().engine.random.clone();
    pick(&mut random, &choices)
}

fn random_starter<H: RomHandler + ?Sized>(
    handler: &mut H,
    allow_alt_formes: bool,
    picked: &[PokemonId],
    banned: &[PokemonId],
) -> Result<PokemonId> {
    let candidates = pool_choice(handler, allow_alt_formes);
    pick_starter(handler, &candidates, picked, banned)
}

fn random_two_evo_starter<H: RomHandler + ?Sized>(
    handler: &mut H,
    allow_alt_formes: bool,
    picked: &[PokemonId],
    banned: &[PokemonId],
) -> Result<PokemonId> {
    let candidates = two_evo_candidates(handler, allow_alt_formes);
    pick_starter(handler, &candidates, picked, banned)
}

fn custom_or_random<H: RomHandler + ?Sized>(
    handler: &mut H,
    custom: i32,
    rom_pokemon: &[Option<PokemonId>],
    allow_alt_formes: bool,
    picked: &[PokemonId],
    banned: &[PokemonId],
) -> Result<PokemonId> {
    let chosen = usize::try_from(custom - 1)
        .ok()
        .filter(|&index| index > 0)
        .and_then(|index| rom_pokemon.get(index).copied().flatten());
    match chosen {
        Some(pokemon) => Ok(pokemon),
        None => random_starter(handler, allow_alt_formes, picked, banned),
    }
}

pub fn custom_starters<H: RomHandler + ?Sized>(handler: &mut H, settings: &Settings) -> Result<()> {
    let custom = settings.custom_starters;
    let allow_alt_formes = settings.allow_starter_alt_formes;
    let rom_pokemon: Vec<Option<PokemonId>> = {
        let data = &handler.core().data;
        handler
            .get_pokemon_incl_formes()
            .into_iter()
            .filter(|pokemon| pokemon.is_none_or(|pokemon| !data.pk(pokemon).actually_cosmetic))
            .collect()
    };
    let mut banned = starter_banned_list(handler, settings);
    let mut picked: Vec<PokemonId> = Vec::new();
    for &choice in &custom {
        let chosen = usize::try_from(choice - 1)
            .ok()
            .filter(|&index| index > 0)
            .and_then(|index| rom_pokemon.get(index).copied().flatten());
        banned.extend(chosen);
    }
    let first = custom_or_random(
        handler,
        custom[0],
        &rom_pokemon,
        allow_alt_formes,
        &picked,
        &banned,
    )?;
    picked.push(first);
    let second = custom_or_random(
        handler,
        custom[1],
        &rom_pokemon,
        allow_alt_formes,
        &picked,
        &banned,
    )?;
    picked.push(second);
    if !handler.is_yellow() {
        let third = custom_or_random(
            handler,
            custom[2],
            &rom_pokemon,
            allow_alt_formes,
            &picked,
            &banned,
        )?;
        picked.push(third);
        for _ in 3..handler.starter_count() {
            let pokemon = random_two_evo_starter(handler, allow_alt_formes, &picked, &[])?;
            picked.push(pokemon);
        }
    }
    handler.core_mut().engine.picked_starters = picked.clone();
    handler.set_starters(&picked)?;
    Ok(())
}

pub fn randomize_starters<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let allow_alt_formes = settings.allow_starter_alt_formes;
    let count = handler.starter_count();
    let banned = starter_banned_list(handler, settings);
    let mut picked: Vec<PokemonId> = Vec::new();
    for _ in 0..count {
        let pokemon = random_starter(handler, allow_alt_formes, &picked, &banned)?;
        picked.push(pokemon);
    }
    handler.core_mut().engine.picked_starters = picked.clone();
    handler.set_starters(&picked)?;
    Ok(())
}

pub fn randomize_basic_two_evos_starters<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let allow_alt_formes = settings.allow_starter_alt_formes;
    let count = handler.starter_count();
    let banned = starter_banned_list(handler, settings);
    let mut picked: Vec<PokemonId> = Vec::new();
    for _ in 0..count {
        let pokemon = random_two_evo_starter(handler, allow_alt_formes, &picked, &banned)?;
        picked.push(pokemon);
    }
    handler.core_mut().engine.picked_starters = picked.clone();
    handler.set_starters(&picked)?;
    Ok(())
}
