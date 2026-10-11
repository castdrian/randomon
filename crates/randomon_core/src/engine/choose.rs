use randomon_data::{GameData, PokemonId};
use randomon_java::JavaRng;

use crate::engine::util::pick;
use crate::error::Result;

pub struct StrengthWindow {
    pub target_bst: i32,
    pub enough: usize,
    pub extra_rounds: usize,
}

impl StrengthWindow {
    pub fn new(target_bst: i32, enough: usize, extra_rounds: usize) -> Self {
        Self {
            target_bst,
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
        let Some(widest) = strength
            .iter()
            .map(|&(_, bst)| (bst - self.target_bst).abs())
            .max()
        else {
            return Vec::new();
        };
        let step = (self.target_bst / 20).max(1);
        let mut reach = (self.target_bst / 10).max(0);
        let mut rounds = 0;
        loop {
            let found: Vec<PokemonId> = strength
                .iter()
                .filter(|&&(_, bst)| (bst - self.target_bst).abs() <= reach)
                .map(|&(pokemon, _)| pokemon)
                .collect();
            rounds += 1;
            let settled =
                !found.is_empty() && (found.len() >= self.enough || rounds > self.extra_rounds);
            if settled || reach >= widest {
                return found;
            }
            reach += step;
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

pub fn first_non_empty<T: Clone>(lists: &[&[T]]) -> Vec<T> {
    lists
        .iter()
        .find(|list| !list.is_empty())
        .map(|list| list.to_vec())
        .unwrap_or_default()
}
