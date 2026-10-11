use rand::seq::IndexedRandom;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha12Rng;

use crate::error::{BdspError, Result};

pub struct SeededRng {
    generator: ChaCha12Rng,
}

impl SeededRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            generator: ChaCha12Rng::seed_from_u64(seed),
        }
    }

    pub fn random(&mut self) -> f64 {
        self.generator.random()
    }

    pub fn choice<'a, T>(&mut self, pool: &'a [T]) -> Option<&'a T> {
        pool.choose(&mut self.generator)
    }

    pub fn sample<T: Clone>(&mut self, pool: &[T], amount: usize) -> Result<Vec<T>> {
        if amount > pool.len() {
            return Err(BdspError::EmptyPool(format!(
                "{amount} distinct picks from a pool of {}",
                pool.len()
            )));
        }
        Ok(pool.sample(&mut self.generator, amount).cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_same_sequence() {
        let mut first = SeededRng::from_seed(42);
        let mut second = SeededRng::from_seed(42);
        let pool: Vec<u16> = (1..=500).collect();
        assert_eq!(
            first.sample(&pool, 20).unwrap(),
            second.sample(&pool, 20).unwrap()
        );
        assert_eq!(first.choice(&pool), second.choice(&pool));
        assert_eq!(first.random().to_bits(), second.random().to_bits());
    }

    #[test]
    fn samples_are_distinct() {
        let mut rng = SeededRng::from_seed(7);
        let pool: Vec<u16> = (1..=100).collect();
        let mut picks = rng.sample(&pool, 100).unwrap();
        picks.sort_unstable();
        picks.dedup();
        assert_eq!(picks.len(), 100);
    }

    #[test]
    fn oversized_samples_are_errors() {
        let mut rng = SeededRng::from_seed(1);
        assert!(rng.sample(&[1, 2, 3], 4).is_err());
    }
}
