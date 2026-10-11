use std::collections::{BTreeMap, HashMap};

use randomon_data::text::{java_upper_case, utf16_length};
use randomon_java::JavaRng;
use randomon_settings::{CustomNames, Settings};

use crate::error::{RandomizationError, Result};
use crate::handler::{RomHandler, TrainerNameMode};

const REPEATED_TRAINER_NAMES: &[&str] =
    &["GRUNT", "EXECUTIVE", "SHADOW", "ADMIN", "GOON", "EMPLOYEE"];

fn custom_names(settings: &Settings) -> CustomNames {
    settings
        .custom_names
        .clone()
        .unwrap_or_else(CustomNames::default_names)
}

struct NamePool {
    all: Vec<String>,
    by_length: BTreeMap<i32, Vec<String>>,
}

fn name_pool<H: RomHandler + ?Sized>(
    handler: &H,
    names: &[String],
    length_limit: Option<i32>,
) -> NamePool {
    let mut pool = NamePool {
        all: Vec::new(),
        by_length: BTreeMap::new(),
    };
    for name in names {
        let length = handler.internal_string_length(name);
        if length_limit.is_none_or(|limit| length <= limit) {
            pool.all.push(name.clone());
            pool.by_length.entry(length).or_default().push(name.clone());
        }
    }
    pool
}

pub fn randomize_trainer_names<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    if !handler.can_change_trainer_text() {
        return Ok(());
    }
    let names = custom_names(settings);
    let pools = [
        name_pool(handler, &names.trainer_names, Some(10)),
        name_pool(handler, &names.doubles_trainer_names, Some(10)),
    ];
    let current = handler.get_trainer_names();
    if current.is_empty() {
        return Ok(());
    }
    let mode = handler.trainer_name_mode();
    let max_length = handler.max_trainer_name_length();
    let total_max_length = handler.max_sum_of_trainer_name_lengths();
    let class_lengths = handler.get_tc_name_lengths_by_trainer();
    let mut random = handler.core().engine.cosmetic_random.clone();
    let mut success = false;
    let mut tries = 0;
    let mut new_names: Vec<String> = Vec::new();
    while !success && tries < 1_000 {
        success = true;
        let mut translation: HashMap<String, String> = HashMap::new();
        new_names.clear();
        let mut total_length: i32 = 0;
        for (index, name) in current.iter().enumerate() {
            let upper = java_upper_case(name);
            if let Some(translated) = translation
                .get(name)
                .filter(|_| !REPEATED_TRAINER_NAMES.contains(&upper.as_str()))
            {
                new_names.push(translated.clone());
                total_length =
                    total_length.wrapping_add(handler.internal_string_length(translated));
            } else {
                let pool = &pools[usize::from(name.contains('&'))];
                let own_length = handler.internal_string_length(name);
                let pick_from = if mode == TrainerNameMode::SameLength {
                    pool.by_length.get(&own_length)
                } else {
                    Some(&pool.all)
                };
                let mut change_to = name.clone();
                let mut change_length = own_length;
                if let Some(pick_from) = pick_from.filter(|list| !list.is_empty())
                    && own_length > 0
                {
                    let class_length = |position: usize| -> Result<i32> {
                        class_lengths
                            .get(position)
                            .copied()
                            .ok_or(RandomizationError::IndexOutOfBounds("tcNameLengths"))
                    };
                    let too_long = |length: i32| -> Result<bool> {
                        Ok((mode == TrainerNameMode::MaxLength && length > max_length)
                            || (mode == TrainerNameMode::MaxLengthWithClass
                                && length + class_length(index)? > max_length))
                    };
                    let mut inner_tries = 0;
                    change_to =
                        pick_from[random.next_int_bounded(pick_from.len() as i32) as usize].clone();
                    change_length = handler.internal_string_length(&change_to);
                    while too_long(change_length)? {
                        inner_tries += 1;
                        if inner_tries == 100 {
                            change_to = name.clone();
                            change_length = own_length;
                            break;
                        }
                        change_to = pick_from
                            [random.next_int_bounded(pick_from.len() as i32) as usize]
                            .clone();
                        change_length = handler.internal_string_length(&change_to);
                    }
                }
                translation.insert(name.clone(), change_to.clone());
                new_names.push(change_to);
                total_length = total_length.wrapping_add(change_length);
            }
            if total_length > total_max_length {
                success = false;
                tries += 1;
                break;
            }
        }
    }
    if success {
        handler.set_trainer_names(&new_names);
    }
    Ok(())
}

pub fn randomize_trainer_class_names<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    if !handler.can_change_trainer_text() {
        return Ok(());
    }
    let names = custom_names(settings);
    let pools = [
        name_pool(handler, &names.trainer_classes, None),
        name_pool(handler, &names.doubles_trainer_classes, None),
    ];
    let current = handler.get_trainer_class_names();
    let same_length = handler.fixed_trainer_class_names_length();
    let max_length = handler.max_trainer_class_name_length();
    let doubles = handler.get_doubles_trainer_classes();
    let mut random = handler.core().engine.cosmetic_random.clone();
    let mut translation: HashMap<String, String> = HashMap::new();
    let mut new_names: Vec<String> = Vec::new();
    for (index, name) in current.iter().enumerate() {
        if let Some(translated) = translation.get(name) {
            new_names.push(translated.clone());
            continue;
        }
        let pool = &pools[usize::from(doubles.contains(&(index as i32)))];
        let own_length = handler.internal_string_length(name);
        let pick_from = if same_length {
            pool.by_length.get(&own_length)
        } else {
            Some(&pool.all)
        };
        let mut change_to = name.clone();
        let fitting: Vec<&String> = pick_from
            .map(|list| {
                list.iter()
                    .filter(|candidate| utf16_length(candidate) as i64 <= i64::from(max_length))
                    .collect()
            })
            .unwrap_or_default();
        if !fitting.is_empty() {
            change_to = fitting[random.next_int_bounded(fitting.len() as i32) as usize].clone();
        }
        translation.insert(name.clone(), change_to.clone());
        new_names.push(change_to);
    }
    handler.set_trainer_class_names(&new_names);
    Ok(())
}
