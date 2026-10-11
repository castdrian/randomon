use randomon_data::{GameData, PokemonId};
use randomon_java::JavaRng;

use crate::engine::util::pick;
use crate::error::Result;

pub struct StrengthWindow {
    low: i32,
    high: i32,
    step: i32,
    enough: usize,
    extra_rounds: usize,
}

impl StrengthWindow {
    pub fn around(target_bst: i32, enough: usize, extra_rounds: usize) -> Self {
        Self {
            low: target_bst - target_bst / 10,
            high: target_bst + target_bst / 10,
            step: (target_bst / 20).max(1),
            enough,
            extra_rounds,
        }
    }

    pub fn at_most(target_bst: i32, enough: usize, extra_rounds: usize) -> Self {
        Self {
            low: target_bst - target_bst / 5,
            high: target_bst,
            step: (target_bst / 20).max(1),
            enough,
            extra_rounds,
        }
    }

    pub fn candidates(
        &self,
        data: &GameData,
        pool: &[PokemonId],
        accept: impl Fn(PokemonId) -> bool,
    ) -> Vec<PokemonId> {
        let strength: Vec<(PokemonId, i32)> = pool
            .iter()
            .copied()
            .filter(|&pokemon| data.exists(pokemon) && accept(pokemon))
            .map(|pokemon| (pokemon, data.pk(pokemon).bst_for_power_levels()))
            .collect();
        let (Some(weakest), Some(strongest)) = (
            strength.iter().map(|&(_, bst)| bst).min(),
            strength.iter().map(|&(_, bst)| bst).max(),
        ) else {
            return Vec::new();
        };
        let (mut low, mut high) = (self.low, self.high);
        let mut rounds = 0;
        loop {
            let mut found: Vec<PokemonId> = Vec::new();
            for &(pokemon, bst) in &strength {
                if bst >= low && bst <= high && !found.contains(&pokemon) {
                    found.push(pokemon);
                }
            }
            rounds += 1;
            let settled =
                !found.is_empty() && (found.len() >= self.enough || rounds > self.extra_rounds);
            if settled || (low <= weakest && high >= strongest) {
                return found;
            }
            low -= self.step;
            high += self.step;
        }
    }
}

pub fn pick_unused<T: Copy + PartialEq>(
    random: &mut impl JavaRng,
    pool: &[T],
    used: &[T],
) -> Result<T> {
    let unused: Vec<T> = pool
        .iter()
        .copied()
        .filter(|item| !used.contains(item))
        .collect();
    if unused.is_empty() {
        pick(random, pool)
    } else {
        pick(random, &unused)
    }
}

pub fn take_refilling<T: Copy>(
    random: &mut impl JavaRng,
    left: &mut Vec<T>,
    refill: &[T],
) -> Option<T> {
    if left.is_empty() {
        left.extend_from_slice(refill);
    }
    if left.is_empty() {
        return None;
    }
    let index = random.next_int_bounded(left.len() as i32) as usize;
    Some(left.remove(index))
}

pub fn first_non_empty<T: Clone>(lists: &[&[T]]) -> Vec<T> {
    lists
        .iter()
        .find(|list| !list.is_empty())
        .map(|list| list.to_vec())
        .unwrap_or_default()
}
