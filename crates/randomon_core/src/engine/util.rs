use randomon_java::JavaRng;

use crate::error::{RandomizationError, Result};

pub fn random_index(random: &mut impl JavaRng, len: usize) -> Result<usize> {
    if len == 0 {
        return Err(RandomizationError::EmptyChoice);
    }
    Ok(random.next_int_bounded(len as i32) as usize)
}

pub fn pick<T: Copy>(random: &mut impl JavaRng, list: &[T]) -> Result<T> {
    let index = random_index(random, list.len())?;
    Ok(list[index])
}

pub fn take<T>(random: &mut impl JavaRng, list: &mut Vec<T>) -> Result<T> {
    let index = random_index(random, list.len())?;
    Ok(list.remove(index))
}

pub fn remove_all<T: PartialEq>(list: &mut Vec<T>, others: &[T]) {
    list.retain(|item| !others.contains(item));
}

pub fn retain_all<T: PartialEq>(list: &mut Vec<T>, keep: &[T]) {
    list.retain(|item| keep.contains(item));
}

pub fn remove_first<T: PartialEq>(list: &mut Vec<T>, item: &T) -> bool {
    match list.iter().position(|candidate| candidate == item) {
        Some(position) => {
            list.remove(position);
            true
        }
        None => false,
    }
}

pub fn distinct<T: PartialEq + Copy>(list: &[T]) -> Vec<T> {
    let mut result = Vec::with_capacity(list.len());
    for &item in list {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

pub fn contains_all<T: PartialEq>(list: &[T], others: &[T]) -> bool {
    others.iter().all(|item| list.contains(item))
}

pub fn modified_level(level: i32, level_modifier: i32) -> i32 {
    let scaled = f64::from(level) * (1.0 + f64::from(level_modifier) / 100.0);
    100.min(randomon_java::math::round_double(scaled) as i32)
}

pub fn rotate_check_value(check_value: i32, values: &[i32]) -> i32 {
    values
        .iter()
        .fold(check_value, |value, &next| value.rotate_left(3) ^ next)
}
