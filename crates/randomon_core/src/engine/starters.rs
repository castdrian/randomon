use randomon_data::PokemonId;
use randomon_settings::{AbilitiesMod, Settings};

use crate::engine::pool::{random_2_evos_pokemon, random_pokemon_choice};
use crate::error::{RandomizationError, Result};
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

fn random_starter<H: RomHandler + ?Sized>(
    handler: &mut H,
    allow_alt_formes: bool,
    picked: &[PokemonId],
    banned: &[PokemonId],
) -> Result<PokemonId> {
    let mut pokemon = random_pokemon_choice(handler, allow_alt_formes)?;
    while picked.contains(&pokemon)
        || banned.contains(&pokemon)
        || handler.core().data.pk(pokemon).actually_cosmetic
    {
        pokemon = random_pokemon_choice(handler, allow_alt_formes)?;
    }
    Ok(pokemon)
}

fn custom_or_random<H: RomHandler + ?Sized>(
    handler: &mut H,
    custom: i32,
    rom_pokemon: &[Option<PokemonId>],
    allow_alt_formes: bool,
    picked: &[PokemonId],
    banned: &[PokemonId],
) -> Result<PokemonId> {
    if custom - 1 == 0 {
        random_starter(handler, allow_alt_formes, picked, banned)
    } else {
        rom_pokemon
            .get((custom - 1) as usize)
            .copied()
            .flatten()
            .ok_or(RandomizationError::IndexOutOfBounds("custom starter"))
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
        if choice - 1 != 0 {
            let pokemon = rom_pokemon
                .get((choice - 1) as usize)
                .copied()
                .ok_or(RandomizationError::IndexOutOfBounds("custom starter"))?;
            if let Some(pokemon) = pokemon {
                banned.push(pokemon);
            }
        }
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
            let mut pokemon = random_2_evos_pokemon(handler, allow_alt_formes)?;
            while picked.contains(&pokemon) {
                pokemon = random_2_evos_pokemon(handler, allow_alt_formes)?;
            }
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
        let mut pokemon = random_2_evos_pokemon(handler, allow_alt_formes)?;
        while picked.contains(&pokemon) || banned.contains(&pokemon) {
            pokemon = random_2_evos_pokemon(handler, allow_alt_formes)?;
        }
        picked.push(pokemon);
    }
    handler.core_mut().engine.picked_starters = picked.clone();
    handler.set_starters(&picked)?;
    Ok(())
}
