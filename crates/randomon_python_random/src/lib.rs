mod mt19937;

pub use mt19937::Mt19937;

const RANDOM_UNIT: f64 = 1.0 / 9_007_199_254_740_992.0;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PythonError {
    #[error("ValueError: {0}")]
    ValueError(String),
    #[error("IndexError: {0}")]
    IndexError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRandom {
    generator: Mt19937,
}

pub fn seed_key_from_magnitude(magnitude: u128) -> Vec<u32> {
    if magnitude == 0 {
        return vec![0];
    }
    let mut key = Vec::new();
    let mut rest = magnitude;
    while rest != 0 {
        key.push(rest as u32);
        rest >>= 32;
    }
    key
}

impl PythonRandom {
    pub fn new(seed: i128) -> Self {
        Self::from_key(&seed_key_from_magnitude(seed.unsigned_abs()))
    }

    pub fn from_u128(seed: u128) -> Self {
        Self::from_key(&seed_key_from_magnitude(seed))
    }

    pub fn from_key(key: &[u32]) -> Self {
        Self {
            generator: Mt19937::from_key(key),
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.generator.next_u32()
    }

    pub fn random(&mut self) -> f64 {
        let high = f64::from(self.next_u32() >> 5);
        let low = f64::from(self.next_u32() >> 6);
        (high * 67_108_864.0 + low) * RANDOM_UNIT
    }

    pub fn getrandbits_words(&mut self, bits: u32) -> Vec<u32> {
        let mut words = Vec::with_capacity(bits.div_ceil(32) as usize);
        let mut remaining = bits;
        while remaining > 0 {
            let word = self.next_u32();
            if remaining < 32 {
                words.push(word >> (32 - remaining));
                remaining = 0;
            } else {
                words.push(word);
                remaining -= 32;
            }
        }
        words
    }

    pub fn getrandbits(&mut self, bits: u32) -> u64 {
        assert!(
            bits <= 64,
            "getrandbits beyond 64 bits needs getrandbits_words"
        );
        self.getrandbits_words(bits)
            .iter()
            .enumerate()
            .fold(0u64, |value, (index, &word)| {
                value | (u64::from(word) << (32 * index))
            })
    }

    pub fn randbelow(&mut self, bound: u64) -> u64 {
        let bits = u64::BITS - bound.leading_zeros();
        let mut value = self.getrandbits(bits);
        while value >= bound {
            value = self.getrandbits(bits);
        }
        value
    }

    pub fn randrange(&mut self, start: i64, stop: i64) -> Result<i64, PythonError> {
        let width = i128::from(stop) - i128::from(start);
        if width <= 0 {
            return Err(PythonError::ValueError(format!(
                "empty range in randrange({start}, {stop})"
            )));
        }
        let offset = self.randbelow(width as u64);
        Ok((i128::from(start) + i128::from(offset)) as i64)
    }

    pub fn randrange_to(&mut self, stop: i64) -> Result<i64, PythonError> {
        if stop <= 0 {
            return Err(PythonError::ValueError(format!(
                "empty range in randrange({stop})"
            )));
        }
        Ok(self.randbelow(stop as u64) as i64)
    }

    pub fn randint(&mut self, low: i64, high: i64) -> Result<i64, PythonError> {
        self.randrange(low, high + 1)
    }

    pub fn choice_index(&mut self, length: usize) -> Result<usize, PythonError> {
        if length == 0 {
            return Err(PythonError::IndexError(
                "Cannot choose from an empty sequence".into(),
            ));
        }
        Ok(self.randbelow(length as u64) as usize)
    }

    pub fn choice<'a, T>(&mut self, sequence: &'a [T]) -> Result<&'a T, PythonError> {
        let index = self.choice_index(sequence.len())?;
        Ok(&sequence[index])
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.randbelow(i as u64 + 1) as usize;
            items.swap(i, j);
        }
    }

    pub fn sample_indices(
        &mut self,
        population: usize,
        count: usize,
    ) -> Result<Vec<usize>, PythonError> {
        if count > population {
            return Err(PythonError::ValueError(
                "Sample larger than population or is negative".into(),
            ));
        }
        if population <= sample_set_threshold(count) {
            Ok(self.sample_by_pool(population, count))
        } else {
            Ok(self.sample_by_set(population, count))
        }
    }

    fn sample_by_pool(&mut self, population: usize, count: usize) -> Vec<usize> {
        let mut pool: Vec<usize> = (0..population).collect();
        (0..count)
            .map(|i| {
                let j = self.randbelow((population - i) as u64) as usize;
                let picked = pool[j];
                pool[j] = pool[population - i - 1];
                picked
            })
            .collect()
    }

    fn sample_by_set(&mut self, population: usize, count: usize) -> Vec<usize> {
        let mut selected = std::collections::HashSet::with_capacity(count);
        (0..count)
            .map(|_| {
                let mut j = self.randbelow(population as u64) as usize;
                while selected.contains(&j) {
                    j = self.randbelow(population as u64) as usize;
                }
                selected.insert(j);
                j
            })
            .collect()
    }

    pub fn sample<T: Clone>(
        &mut self,
        population: &[T],
        count: usize,
    ) -> Result<Vec<T>, PythonError> {
        Ok(self
            .sample_indices(population.len(), count)?
            .into_iter()
            .map(|index| population[index].clone())
            .collect())
    }

    pub fn sample_range(
        &mut self,
        start: i64,
        stop: i64,
        count: usize,
    ) -> Result<Vec<i64>, PythonError> {
        let length = usize::try_from((stop - start).max(0)).unwrap_or(0);
        Ok(self
            .sample_indices(length, count)?
            .into_iter()
            .map(|index| start + index as i64)
            .collect())
    }

    pub fn uniform(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.random()
    }
}

pub fn sample_set_threshold(count: usize) -> usize {
    let mut threshold = 21usize;
    if count > 5 {
        let exponent = ((count * 3) as f64).ln() / 4f64.ln();
        threshold += 4usize.pow(exponent.ceil() as u32);
    }
    threshold
}
