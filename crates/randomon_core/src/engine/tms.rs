use randomon_data::constants::global;
use randomon_data::{MoveId, PokemonId, Type};
use randomon_java::JavaRng;
use randomon_settings::{MoveTutorsCompatibilityMod, Settings, TmsHmsCompatibilityMod};

use crate::engine::copy_up::{CopyUpActions, copy_up_evolutions_helper};
use crate::engine::util::{pick, remove_first, retain_all};
use crate::error::{RandomizationError, Result};
use crate::handler::{Compatibility, RomHandler};

struct MovePicks {
    picked: Vec<i32>,
}

fn pick_new_moves<H: RomHandler + ?Sized>(
    handler: &H,
    old_moves: &[i32],
    extra_unusable: &[i32],
    no_broken: bool,
    preserve_field: bool,
    good_damaging_percentage: f64,
) -> Result<MovePicks> {
    let hms = handler.get_hm_moves();
    let mut banned: Vec<i32> = if no_broken {
        handler.get_game_breaking_moves()
    } else {
        Vec::new()
    };
    banned.extend(handler.get_moves_banned_from_levelup());
    banned.extend(handler.get_illegal_moves());
    let field_moves = handler.get_field_moves();
    let mut preserved_field_count = 0;
    if preserve_field {
        let mut existing_field = old_moves.to_vec();
        retain_all(&mut existing_field, &field_moves);
        preserved_field_count = existing_field.len();
        banned.extend(existing_field);
    }
    let perfect_accuracy = handler.core().engine.perfect_accuracy;
    let data = &handler.core().data;
    let mut usable: Vec<MoveId> = Vec::new();
    let mut unusable_damaging: Vec<MoveId> = Vec::new();
    for id in data.move_ids().into_iter().skip(1) {
        let Some(id) = id else {
            return Err(RandomizationError::NullPointer("move list entry"));
        };
        let mv = data.mv(id);
        let unusable = global::banned_random_move(mv.number)
            || global::is_z_move(mv.number)
            || hms.contains(&mv.number)
            || banned.contains(&mv.number)
            || extra_unusable.contains(&mv.number);
        if unusable {
            continue;
        }
        if global::banned_for_damaging_move(mv.number) || !mv.is_good_damaging(perfect_accuracy) {
            unusable_damaging.push(id);
        }
        usable.push(id);
    }
    let mut usable_damaging: Vec<MoveId> = usable
        .iter()
        .copied()
        .filter(|id| !unusable_damaging.contains(id))
        .collect();
    let count = old_moves.len() as i32 - preserved_field_count as i32;
    let mut good_damaging_left =
        randomon_java::math::round_double(good_damaging_percentage * f64::from(count)) as i32;
    let mut random = handler.core().engine.random.clone();
    let mut picked: Vec<i32> = Vec::new();
    for _ in 0..count {
        let chosen = if good_damaging_left > 0 && !usable_damaging.is_empty() {
            pick(&mut random, &usable_damaging)?
        } else {
            pick(&mut random, &usable)?
        };
        picked.push(data.mv(chosen).number);
        remove_first(&mut usable, &chosen);
        remove_first(&mut usable_damaging, &chosen);
        good_damaging_left -= 1;
    }
    randomon_java::shuffle(&mut picked, &mut random);
    Ok(MovePicks { picked })
}

fn distribute(
    old_moves: &[i32],
    picks: MovePicks,
    preserve_field: bool,
    field_moves: &[i32],
) -> Result<Vec<i32>> {
    let mut picked = picks.picked.into_iter();
    old_moves
        .iter()
        .map(|&old| {
            if preserve_field && field_moves.contains(&old) {
                Ok(old)
            } else {
                picked
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("pickedMoves"))
            }
        })
        .collect()
}

pub fn randomize_tm_moves<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let good_damaging = if settings.tms_force_good_damaging {
        f64::from(settings.tms_good_damaging_percent) / 100.0
    } else {
        0.0
    };
    let tm_count = handler.get_tm_count() as usize;
    let old_tms = handler.get_tm_moves();
    let old_considered: Vec<i32> = old_tms.iter().copied().take(tm_count).collect();
    let picks = pick_new_moves(
        handler,
        &old_considered,
        &[],
        settings.block_broken_tm_moves,
        settings.keep_field_move_tms,
        good_damaging,
    )?;
    let field_moves = handler.get_field_moves();
    let new_tms = distribute(
        &old_considered,
        picks,
        settings.keep_field_move_tms,
        &field_moves,
    )?;
    handler.set_tm_moves(&new_tms)
}

pub fn randomize_move_tutor_moves<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    if !handler.has_move_tutors() {
        return Ok(());
    }
    let good_damaging = if settings.tutors_force_good_damaging {
        f64::from(settings.tutors_good_damaging_percent) / 100.0
    } else {
        0.0
    };
    let tms = handler.get_tm_moves();
    let old_tutors = handler.get_move_tutor_moves();
    let picks = pick_new_moves(
        handler,
        &old_tutors,
        &tms,
        settings.block_broken_tutor_moves,
        settings.keep_field_move_tutors,
        good_damaging,
    )?;
    let field_moves = handler.get_field_moves();
    let new_tutors = distribute(
        &old_tutors,
        picks,
        settings.keep_field_move_tutors,
        &field_moves,
    )?;
    handler.set_move_tutor_moves(&new_tutors)
}

fn compatibility_probability<H: RomHandler + ?Sized>(
    handler: &H,
    pokemon: PokemonId,
    move_number: i32,
    required_early: bool,
    prefer_same_type: bool,
) -> f64 {
    let data = &handler.core().data;
    let pk = data.pk(pokemon);
    let move_type = data.move_by_number(move_number).and_then(|mv| mv.kind);
    let mut probability: f64 = 0.5;
    if prefer_same_type {
        probability = if pk.primary_type == move_type
            || (pk.secondary_type.is_some() && pk.secondary_type == move_type)
        {
            0.9
        } else if move_type == Some(Type::Normal) {
            0.5
        } else {
            0.25
        };
    }
    if required_early {
        probability = (probability * 1.8).min(1.0);
    }
    probability
}

fn randomize_pokemon_move_compatibility<H: RomHandler + ?Sized>(
    handler: &H,
    pokemon: PokemonId,
    flags: &mut [bool],
    move_numbers: &[i32],
    prioritized: &[i32],
    prefer_same_type: bool,
) -> Result<()> {
    let mut random = handler.core().engine.random.clone();
    for (index, &number) in move_numbers.iter().enumerate() {
        let probability = compatibility_probability(
            handler,
            pokemon,
            number,
            prioritized.contains(&number),
            prefer_same_type,
        );
        let slot = flags
            .get_mut(index + 1)
            .ok_or(RandomizationError::IndexOutOfBounds("compatibility flags"))?;
        *slot = random.next_double() < probability;
    }
    Ok(())
}

fn copy_move_compatibility_up<H: RomHandler + ?Sized>(
    handler: &H,
    from: PokemonId,
    to: PokemonId,
    previous: &[bool],
    target: &mut [bool],
    move_numbers: &[i32],
    prefer_same_type: bool,
) -> Result<()> {
    let data = &handler.core().data;
    let mut random = handler.core().engine.random.clone();
    let source = data.pk(from);
    let evolved = data.pk(to);
    for (index, &number) in move_numbers.iter().enumerate() {
        let slot = index + 1;
        let had = *previous
            .get(slot)
            .ok_or(RandomizationError::IndexOutOfBounds("compatibility flags"))?;
        if !had {
            let move_type = data.move_by_number(number).and_then(|mv| mv.kind);
            let mut probability = 0.25;
            if prefer_same_type {
                probability = 0.1;
                let primary_new = evolved.primary_type.is_some()
                    && evolved.primary_type == move_type
                    && evolved.primary_type != source.primary_type
                    && evolved.primary_type != source.secondary_type;
                let secondary_new = evolved.secondary_type.is_some()
                    && evolved.secondary_type == move_type
                    && evolved.secondary_type != source.secondary_type
                    && evolved.secondary_type != source.primary_type;
                if evolved.primary_type.is_none() {
                    return Err(RandomizationError::NullPointer("primaryType"));
                }
                if primary_new || secondary_new {
                    probability = 0.9;
                }
            }
            target[slot] = random.next_double() < probability;
        } else {
            target[slot] = had;
        }
    }
    Ok(())
}

fn randomize_compatibility<H: RomHandler + ?Sized>(
    handler: &mut H,
    compat: &mut Compatibility,
    move_numbers: &[i32],
    prioritized: &[i32],
    prefer_same_type: bool,
    follow_evolutions: bool,
) -> Result<()> {
    if follow_evolutions {
        copy_up_evolutions_helper(
            handler,
            compat,
            CopyUpActions {
                base: &mut |h: &mut H, compat: &mut Compatibility, pk| {
                    let flags = compat
                        .get_mut(&pk)
                        .ok_or(RandomizationError::NullPointer("compat.get"))?;
                    randomize_pokemon_move_compatibility(
                        h,
                        pk,
                        flags,
                        move_numbers,
                        prioritized,
                        prefer_same_type,
                    )
                },
                evolved: &mut |h: &mut H, compat: &mut Compatibility, from, to, _| {
                    let previous = compat
                        .get(&from)
                        .cloned()
                        .ok_or(RandomizationError::NullPointer("compat.get"))?;
                    let target = compat
                        .get_mut(&to)
                        .ok_or(RandomizationError::NullPointer("compat.get"))?;
                    copy_move_compatibility_up(
                        h,
                        from,
                        to,
                        &previous,
                        target,
                        move_numbers,
                        prefer_same_type,
                    )
                },
                split: None,
                copy_split_evos: true,
            },
        )
    } else {
        for (&pokemon, flags) in compat.iter_mut() {
            randomize_pokemon_move_compatibility(
                handler,
                pokemon,
                flags,
                move_numbers,
                prioritized,
                prefer_same_type,
            )?;
        }
        Ok(())
    }
}

pub fn randomize_tmhm_compatibility<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let prefer_same_type =
        settings.tms_hms_compatibility_mod == TmsHmsCompatibilityMod::RandomPreferType;
    let required_early = handler.get_early_required_hm_moves();
    let mut compat = handler.get_tmhm_compatibility();
    let mut tm_hms = handler.get_tm_moves();
    tm_hms.extend(handler.get_hm_moves());
    randomize_compatibility(
        handler,
        &mut compat,
        &tm_hms,
        &required_early,
        prefer_same_type,
        settings.tms_follow_evolutions,
    )?;
    handler.set_tmhm_compatibility(&compat);
    Ok(())
}

pub fn randomize_move_tutor_compatibility<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let prefer_same_type =
        settings.move_tutors_compatibility_mod == MoveTutorsCompatibilityMod::RandomPreferType;
    if !handler.has_move_tutors() {
        return Ok(());
    }
    let mut compat = handler.get_move_tutor_compatibility();
    let tutors = handler.get_move_tutor_moves();
    randomize_compatibility(
        handler,
        &mut compat,
        &tutors,
        &[],
        prefer_same_type,
        settings.tutor_follow_evolutions,
    )?;
    handler.set_move_tutor_compatibility(&compat);
    Ok(())
}

fn set_all_flags(compat: &mut Compatibility, start: usize) {
    for flags in compat.values_mut() {
        for flag in flags.iter_mut().skip(start) {
            *flag = true;
        }
    }
}

pub fn full_tmhm_compatibility<H: RomHandler + ?Sized>(handler: &mut H) {
    let mut compat = handler.get_tmhm_compatibility();
    set_all_flags(&mut compat, 1);
    handler.set_tmhm_compatibility(&compat);
}

pub fn full_hm_compatibility<H: RomHandler + ?Sized>(handler: &mut H) {
    let mut compat = handler.get_tmhm_compatibility();
    let tm_count = handler.get_tm_count() as usize;
    set_all_flags(&mut compat, tm_count + 1);
    handler.set_tmhm_compatibility(&compat);
}

pub fn full_move_tutor_compatibility<H: RomHandler + ?Sized>(handler: &mut H) {
    if !handler.has_move_tutors() {
        return;
    }
    let mut compat = handler.get_move_tutor_compatibility();
    set_all_flags(&mut compat, 1);
    handler.set_move_tutor_compatibility(&compat);
}

fn ensure_sanity(
    compat: &mut Compatibility,
    movesets: &crate::handler::Movesets,
    data: &randomon_data::GameData,
    moves: &[i32],
) {
    for (&pokemon, flags) in compat.iter_mut() {
        let Some(moveset) = movesets.get(&data.pk(pokemon).number) else {
            continue;
        };
        for learnt in moveset {
            if let Some(index) = moves
                .iter()
                .position(|&number| number == learnt.move_number)
            {
                flags[index + 1] = true;
            }
        }
    }
}

pub fn ensure_tm_compat_sanity<H: RomHandler + ?Sized>(handler: &mut H) {
    let mut compat = handler.get_tmhm_compatibility();
    let movesets = handler.get_moves_learnt();
    let tm_moves = handler.get_tm_moves();
    ensure_sanity(&mut compat, &movesets, &handler.core().data, &tm_moves);
    handler.set_tmhm_compatibility(&compat);
}

pub fn ensure_move_tutor_compat_sanity<H: RomHandler + ?Sized>(handler: &mut H) {
    if !handler.has_move_tutors() {
        return;
    }
    let mut compat = handler.get_move_tutor_compatibility();
    let movesets = handler.get_moves_learnt();
    let tutor_moves = handler.get_move_tutor_moves();
    ensure_sanity(&mut compat, &movesets, &handler.core().data, &tutor_moves);
    handler.set_move_tutor_compatibility(&compat);
}

fn propagate_up_evolutions<H: RomHandler + ?Sized>(
    handler: &mut H,
    compat: &mut Compatibility,
) -> Result<()> {
    copy_up_evolutions_helper(
        handler,
        compat,
        CopyUpActions {
            base: &mut |_: &mut H, _: &mut Compatibility, _| Ok(()),
            evolved: &mut |_: &mut H, compat: &mut Compatibility, from, to, _| {
                let previous = compat
                    .get(&from)
                    .cloned()
                    .ok_or(RandomizationError::NullPointer("compat.get"))?;
                let target = compat
                    .get_mut(&to)
                    .ok_or(RandomizationError::NullPointer("compat.get"))?;
                for index in 1..target.len() {
                    target[index] |= previous[index];
                }
                Ok(())
            },
            split: None,
            copy_split_evos: true,
        },
    )
}

pub fn ensure_tm_evolution_sanity<H: RomHandler + ?Sized>(handler: &mut H) -> Result<()> {
    let mut compat = handler.get_tmhm_compatibility();
    propagate_up_evolutions(handler, &mut compat)?;
    handler.set_tmhm_compatibility(&compat);
    Ok(())
}

pub fn ensure_move_tutor_evolution_sanity<H: RomHandler + ?Sized>(handler: &mut H) -> Result<()> {
    if !handler.has_move_tutors() {
        return Ok(());
    }
    let mut compat = handler.get_move_tutor_compatibility();
    propagate_up_evolutions(handler, &mut compat)?;
    handler.set_move_tutor_compatibility(&compat);
    Ok(())
}

fn copy_to_cosmetic(compat: &mut Compatibility, data: &randomon_data::GameData) {
    let keys: Vec<PokemonId> = compat.keys().copied().collect();
    for pokemon in keys {
        let pk = data.pk(pokemon);
        if pk.actually_cosmetic
            && let Some(base) = pk.base_forme
            && let Some(base_flags) = compat.get(&base).cloned()
            && let Some(flags) = compat.get_mut(&pokemon)
        {
            let length = flags.len();
            flags[1..].copy_from_slice(&base_flags[1..length]);
        }
    }
}

pub fn copy_tm_compatibility_to_cosmetic_formes<H: RomHandler + ?Sized>(handler: &mut H) {
    let mut compat = handler.get_tmhm_compatibility();
    copy_to_cosmetic(&mut compat, &handler.core().data);
    handler.set_tmhm_compatibility(&compat);
}

pub fn copy_move_tutor_compatibility_to_cosmetic_formes<H: RomHandler + ?Sized>(handler: &mut H) {
    let mut compat = handler.get_move_tutor_compatibility();
    copy_to_cosmetic(&mut compat, &handler.core().data);
    handler.set_move_tutor_compatibility(&compat);
}
