use std::collections::BTreeSet;

use randomon_data::constants::species;
use randomon_data::{PokemonId, Type};
use randomon_settings::{GenRestrictions, Settings};

use crate::engine::choose::first_non_empty;
use crate::engine::evolutions::evolutionary_relatives;
use crate::engine::util::pick;
use crate::error::Result;
use crate::handler::RomHandler;
use crate::state::PokemonPool;

const GEN2_POKEMON_COUNT: usize = 251;
const GEN3_POKEMON_COUNT: usize = 386;
const GEN4_POKEMON_COUNT: usize = 493;
const GEN5_POKEMON_COUNT: usize = 649;
const GEN6_POKEMON_COUNT: usize = 721;

pub fn without_null(list: Vec<Option<PokemonId>>) -> Vec<PokemonId> {
    list.into_iter().skip(1).flatten().collect()
}

pub fn all_pokemon_without_null<H: RomHandler + ?Sized>(handler: &H) -> Vec<PokemonId> {
    without_null(handler.get_pokemon())
}

pub fn all_pokemon_incl_formes_without_null<H: RomHandler + ?Sized>(handler: &H) -> Vec<PokemonId> {
    without_null(handler.get_pokemon_incl_formes())
}

fn add_pokes_from_range(pool: &mut Vec<PokemonId>, all: &[Option<PokemonId>], min: i32, max: i32) {
    for number in min..=max {
        if let Some(Some(pokemon)) = all.get(number as usize)
            && !pool.contains(pokemon)
        {
            pool.push(*pokemon);
        }
    }
}

fn add_evolutionary_relatives<H: RomHandler + ?Sized>(handler: &H, pool: &mut Vec<PokemonId>) {
    let mut new_pokemon = BTreeSet::new();
    for &pokemon in pool.iter() {
        for relative in evolutionary_relatives(&handler.core().data, pokemon) {
            if !pool.contains(&relative) {
                new_pokemon.insert(relative);
            }
        }
    }
    pool.extend(new_pokemon);
}

fn add_all_pokes_incl_formes<H: RomHandler + ?Sized>(
    handler: &H,
    pool: &[PokemonId],
    incl_formes: &mut Vec<PokemonId>,
) {
    let alt_formes = handler.get_alt_formes();
    let data = &handler.core().data;
    for &current in pool {
        if !incl_formes.contains(&current) {
            incl_formes.push(current);
        }
        let current_number = data.pk(current).number;
        for &alt in &alt_formes {
            if data
                .pk(alt)
                .base_forme
                .is_some_and(|base| data.pk(base).number == current_number)
            {
                incl_formes.push(alt);
            }
        }
    }
}

fn restricted_pool<H: RomHandler + ?Sized>(
    handler: &H,
    restrictions: GenRestrictions,
    is_sm: bool,
) -> Vec<PokemonId> {
    let all = handler.get_pokemon();
    let max_gen7 = if is_sm {
        species::MARSHADOW
    } else {
        species::ZERAORA
    };
    let ranges = [
        (restrictions.allow_gen1, 0, species::BULBASAUR, species::MEW),
        (
            restrictions.allow_gen2,
            GEN2_POKEMON_COUNT,
            species::CHIKORITA,
            species::CELEBI,
        ),
        (
            restrictions.allow_gen3,
            GEN3_POKEMON_COUNT,
            species::TREECKO,
            species::DEOXYS,
        ),
        (
            restrictions.allow_gen4,
            GEN4_POKEMON_COUNT,
            species::TURTWIG,
            species::ARCEUS,
        ),
        (
            restrictions.allow_gen5,
            GEN5_POKEMON_COUNT,
            species::VICTINI,
            species::GENESECT,
        ),
        (
            restrictions.allow_gen6,
            GEN6_POKEMON_COUNT,
            species::CHESPIN,
            species::VOLCANION,
        ),
        (
            restrictions.allow_gen7,
            max_gen7 as usize,
            species::ROWLET,
            max_gen7,
        ),
    ];
    let mut pool = Vec::new();
    for (allowed, previous_count, first, last) in ranges {
        if allowed && all.len() > previous_count {
            add_pokes_from_range(&mut pool, &all, first, last);
        }
    }
    pool
}

pub fn set_pokemon_pool<H: RomHandler + ?Sized>(handler: &mut H, settings: Option<&Settings>) {
    let restrictions = settings
        .filter(|settings| settings.limit_pokemon)
        .and_then(|settings| settings.current_restrictions);
    let is_sm = handler.core().engine.is_sm;
    let mut pool = PokemonPool {
        main: all_pokemon_without_null(handler),
        main_incl_formes: all_pokemon_incl_formes_without_null(handler),
        alt_formes: handler.get_alt_formes(),
        mega_evolutions: handler.get_mega_evolutions(),
        ..PokemonPool::default()
    };
    if let Some(restrictions) = restrictions
        && !restricted_pool(handler, restrictions, is_sm).is_empty()
    {
        pool.main = Vec::new();
        pool.main_incl_formes = Vec::new();
        pool.mega_evolutions = Vec::new();
        let all = handler.get_pokemon();
        if restrictions.allow_gen1 {
            add_pokes_from_range(&mut pool.main, &all, species::BULBASAUR, species::MEW);
        }
        if restrictions.allow_gen2 && all.len() > GEN2_POKEMON_COUNT {
            add_pokes_from_range(&mut pool.main, &all, species::CHIKORITA, species::CELEBI);
        }
        if restrictions.allow_gen3 && all.len() > GEN3_POKEMON_COUNT {
            add_pokes_from_range(&mut pool.main, &all, species::TREECKO, species::DEOXYS);
        }
        if restrictions.allow_gen4 && all.len() > GEN4_POKEMON_COUNT {
            add_pokes_from_range(&mut pool.main, &all, species::TURTWIG, species::ARCEUS);
        }
        if restrictions.allow_gen5 && all.len() > GEN5_POKEMON_COUNT {
            add_pokes_from_range(&mut pool.main, &all, species::VICTINI, species::GENESECT);
        }
        if restrictions.allow_gen6 && all.len() > GEN6_POKEMON_COUNT {
            add_pokes_from_range(&mut pool.main, &all, species::CHESPIN, species::VOLCANION);
        }
        let max_gen7 = if is_sm {
            species::MARSHADOW
        } else {
            species::ZERAORA
        };
        if restrictions.allow_gen7 && all.len() > max_gen7 as usize {
            add_pokes_from_range(&mut pool.main, &all, species::ROWLET, max_gen7);
        }
        if restrictions.allow_evolutionary_relatives {
            add_evolutionary_relatives(handler, &mut pool.main);
        }
        let mut incl_formes = Vec::new();
        add_all_pokes_incl_formes(handler, &pool.main, &mut incl_formes);
        pool.main_incl_formes = incl_formes;
        let data = &handler.core().data;
        for mega in handler.get_mega_evolutions() {
            if pool.main_incl_formes.contains(&data.mega(mega).to) {
                pool.mega_evolutions.push(mega);
            }
        }
    }
    let data = &handler.core().data;
    for &pokemon in &pool.main {
        if data.is_legendary(pokemon) {
            pool.only_legendary.push(pokemon);
        } else if data.pk(pokemon).is_ultra_beast() {
            pool.ultra_beasts.push(pokemon);
        } else {
            pool.no_legendary.push(pokemon);
        }
    }
    for &pokemon in &pool.main_incl_formes {
        if data.is_legendary(pokemon) {
            pool.only_legendary_incl_formes.push(pokemon);
        } else if !pool.ultra_beasts.contains(&pokemon) {
            pool.no_legendary_incl_formes.push(pokemon);
        }
    }
    for &forme in &pool.alt_formes {
        if data.is_legendary(forme) {
            pool.only_legendary_alts.push(forme);
        } else {
            pool.no_legendary_alts.push(forme);
        }
    }
    let engine = &mut handler.core_mut().engine;
    engine.restrictions_set = true;
    engine.pool = pool;
}

pub fn check_pokemon_restrictions<H: RomHandler + ?Sized>(handler: &mut H) {
    if !handler.core().engine.restrictions_set {
        handler.set_pokemon_pool(None);
    }
}

fn random_from<H: RomHandler + ?Sized>(
    handler: &mut H,
    list: impl Fn(&PokemonPool) -> &Vec<PokemonId>,
) -> Result<PokemonId> {
    check_pokemon_restrictions(handler);
    let mut random = handler.core().engine.random.clone();
    let pool = &handler.core().engine.pool;
    let choices = first_non_empty(&[list(pool), &pool.main_incl_formes, &pool.main]);
    pick(&mut random, &choices)
}

pub fn random_pokemon<H: RomHandler + ?Sized>(handler: &mut H) -> Result<PokemonId> {
    random_from(handler, |pool| &pool.main)
}

pub fn random_pokemon_incl_formes<H: RomHandler + ?Sized>(handler: &mut H) -> Result<PokemonId> {
    random_from(handler, |pool| &pool.main_incl_formes)
}

pub fn random_non_legendary_pokemon<H: RomHandler + ?Sized>(handler: &mut H) -> Result<PokemonId> {
    random_from(handler, |pool| &pool.no_legendary)
}

pub fn random_non_legendary_pokemon_incl_formes<H: RomHandler + ?Sized>(
    handler: &mut H,
) -> Result<PokemonId> {
    random_from(handler, |pool| &pool.no_legendary_incl_formes)
}

pub fn random_legendary_pokemon<H: RomHandler + ?Sized>(handler: &mut H) -> Result<PokemonId> {
    random_from(handler, |pool| &pool.only_legendary)
}

pub fn random_pokemon_choice<H: RomHandler + ?Sized>(
    handler: &mut H,
    include_formes: bool,
) -> Result<PokemonId> {
    if include_formes {
        random_pokemon_incl_formes(handler)
    } else {
        random_pokemon(handler)
    }
}

pub fn pool_choice<H: RomHandler + ?Sized>(
    handler: &mut H,
    include_formes: bool,
) -> Vec<PokemonId> {
    check_pokemon_restrictions(handler);
    let pool = &handler.core().engine.pool;
    if include_formes {
        first_non_empty(&[&pool.main_incl_formes, &pool.main])
    } else {
        pool.main.clone()
    }
}

pub fn two_evo_candidates<H: RomHandler + ?Sized>(
    handler: &mut H,
    allow_alt_formes: bool,
) -> Vec<PokemonId> {
    ensure_two_evo_pokes(handler, allow_alt_formes);
    handler
        .core()
        .engine
        .two_evo_pokes
        .clone()
        .unwrap_or_default()
}

pub fn random_2_evos_pokemon<H: RomHandler + ?Sized>(
    handler: &mut H,
    allow_alt_formes: bool,
) -> Result<PokemonId> {
    ensure_two_evo_pokes(handler, allow_alt_formes);
    let mut random = handler.core().engine.random.clone();
    pick(
        &mut random,
        handler
            .core()
            .engine
            .two_evo_pokes
            .as_deref()
            .unwrap_or_default(),
    )
}

fn ensure_two_evo_pokes<H: RomHandler + ?Sized>(handler: &mut H, allow_alt_formes: bool) {
    if handler.core().engine.two_evo_pokes.is_none() {
        let all = if allow_alt_formes {
            let data = &handler.core().data;
            handler
                .get_pokemon_incl_formes()
                .into_iter()
                .filter(|pokemon| pokemon.is_none_or(|pokemon| !data.pk(pokemon).actually_cosmetic))
                .collect()
        } else {
            handler.get_pokemon()
        };
        let data = &handler.core().data;
        let all: Vec<PokemonId> = all.into_iter().flatten().collect();
        let evolves = |pokemon: &PokemonId| !data.evolutions_from(*pokemon).is_empty();
        let is_basic = |pokemon: &PokemonId| data.evolutions_to(*pokemon).is_empty();
        let has_second_stage = |pokemon: &PokemonId| {
            data.evolution_targets(*pokemon)
                .iter()
                .any(|target| !data.evolutions_from(*target).is_empty())
        };
        let two_stages: Vec<PokemonId> = all
            .iter()
            .copied()
            .filter(|pokemon| is_basic(pokemon) && evolves(pokemon) && has_second_stage(pokemon))
            .collect();
        let basic_evolvers: Vec<PokemonId> = all
            .iter()
            .copied()
            .filter(|pokemon| is_basic(pokemon) && evolves(pokemon))
            .collect();
        let evolvers: Vec<PokemonId> = all.iter().copied().filter(evolves).collect();
        let two_evo = first_non_empty(&[&two_stages, &basic_evolvers, &evolvers, &all]);
        handler.core_mut().engine.two_evo_pokes = Some(two_evo);
    }
}

pub fn pokemon_of_type<H: RomHandler + ?Sized>(
    handler: &H,
    kind: Type,
    no_legendaries: bool,
) -> Vec<PokemonId> {
    typed_pokemon(
        handler,
        &handler.core().engine.pool.main,
        kind,
        no_legendaries,
    )
}

pub fn pokemon_of_type_incl_formes<H: RomHandler + ?Sized>(
    handler: &H,
    kind: Type,
    no_legendaries: bool,
) -> Vec<PokemonId> {
    typed_pokemon(
        handler,
        &handler.core().engine.pool.main_incl_formes,
        kind,
        no_legendaries,
    )
}

pub fn pokemon_of_type_choice<H: RomHandler + ?Sized>(
    handler: &H,
    kind: Type,
    no_legendaries: bool,
    include_formes: bool,
) -> Vec<PokemonId> {
    if include_formes {
        pokemon_of_type_incl_formes(handler, kind, no_legendaries)
    } else {
        pokemon_of_type(handler, kind, no_legendaries)
    }
}

fn typed_pokemon<H: RomHandler + ?Sized>(
    handler: &H,
    list: &[PokemonId],
    kind: Type,
    no_legendaries: bool,
) -> Vec<PokemonId> {
    let data = &handler.core().data;
    list.iter()
        .copied()
        .filter(|&pokemon| {
            let pk = data.pk(pokemon);
            (!no_legendaries || !data.is_legendary(pokemon))
                && !pk.actually_cosmetic
                && (pk.primary_type == Some(kind) || pk.secondary_type == Some(kind))
        })
        .collect()
}

pub fn non_cosmetic<H: RomHandler + ?Sized>(handler: &H, list: &[PokemonId]) -> Vec<PokemonId> {
    let data = &handler.core().data;
    list.iter()
        .copied()
        .filter(|&pokemon| !data.pk(pokemon).actually_cosmetic)
        .collect()
}

pub fn ability_dependent_formes<H: RomHandler + ?Sized>(handler: &H) -> Vec<PokemonId> {
    let data = &handler.core().data;
    handler
        .core()
        .engine
        .pool
        .main_incl_formes
        .iter()
        .copied()
        .filter(|&pokemon| {
            let pk = data.pk(pokemon);
            pk.base_forme.is_some_and(|base| {
                let base_number = data.pk(base).number;
                base_number == species::CASTFORM
                    || (base_number == species::DARMANITAN && pk.forme_number == 1)
                    || base_number == species::AEGISLASH
                    || base_number == species::WISHIWASHI
            })
        })
        .collect()
}

pub fn banned_formes_for_player_pokemon<H: RomHandler + ?Sized>(handler: &H) -> Vec<PokemonId> {
    let data = &handler.core().data;
    handler
        .core()
        .engine
        .pool
        .main_incl_formes
        .iter()
        .copied()
        .filter(|&pokemon| {
            data.pk(pokemon).base_forme.is_some_and(|base| {
                let base_number = data.pk(base).number;
                base_number == species::GIRATINA || base_number == species::SHAYMIN
            })
        })
        .collect()
}

pub fn find_pokemon_in_pool_with_species_id(
    handler: &(impl RomHandler + ?Sized),
    pool: &[PokemonId],
    species_id: i32,
) -> Option<PokemonId> {
    let data = &handler.core().data;
    pool.iter()
        .copied()
        .find(|&pokemon| data.pk(pokemon).number == species_id)
}
