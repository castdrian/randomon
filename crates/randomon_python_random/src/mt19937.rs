const STATE_SIZE: usize = 624;
const SHIFT_SIZE: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;
const INIT_GENRAND_SEED: u32 = 19_650_218;

#[derive(Clone, PartialEq, Eq)]
pub struct Mt19937 {
    state: [u32; STATE_SIZE],
    index: usize,
}

impl std::fmt::Debug for Mt19937 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Mt19937")
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

impl Mt19937 {
    pub fn from_genrand_seed(seed: u32) -> Self {
        let mut state = [0u32; STATE_SIZE];
        state[0] = seed;
        for i in 1..STATE_SIZE {
            let previous = state[i - 1];
            state[i] = 1_812_433_253u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(i as u32);
        }
        Self {
            state,
            index: STATE_SIZE,
        }
    }

    pub fn from_key(key: &[u32]) -> Self {
        let mut generator = Self::from_genrand_seed(INIT_GENRAND_SEED);
        let state = &mut generator.state;
        let key_length = key.len().max(1);
        let mut i = 1usize;
        let mut j = 0usize;
        for _ in 0..STATE_SIZE.max(key_length) {
            let previous = state[i - 1];
            let key_word = key.get(j).copied().unwrap_or(0);
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key_word)
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= STATE_SIZE {
                state[0] = state[STATE_SIZE - 1];
                i = 1;
            }
            if j >= key_length {
                j = 0;
            }
        }
        for _ in 0..STATE_SIZE - 1 {
            let previous = state[i - 1];
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= STATE_SIZE {
                state[0] = state[STATE_SIZE - 1];
                i = 1;
            }
        }
        state[0] = 0x8000_0000;
        generator
    }

    fn twist(&mut self) {
        let state = &mut self.state;
        for k in 0..STATE_SIZE {
            let y = (state[k] & UPPER_MASK) | (state[(k + 1) % STATE_SIZE] & LOWER_MASK);
            let magnitude = if y & 1 == 1 { MATRIX_A } else { 0 };
            state[k] = state[(k + SHIFT_SIZE) % STATE_SIZE] ^ (y >> 1) ^ magnitude;
        }
        self.index = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.index >= STATE_SIZE {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }
}
