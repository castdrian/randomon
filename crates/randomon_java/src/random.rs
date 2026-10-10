use crate::strict_math;

const MULTIPLIER: i64 = 0x5_DEEC_E66D;
const ADDEND: i64 = 0xB;
const MASK: i64 = (1 << 48) - 1;
const DOUBLE_UNIT: f64 = 1.0 / (1u64 << 53) as f64;
const FLOAT_UNIT: f32 = 1.0 / (1u32 << 24) as f32;

pub trait JavaRng {
    fn set_seed(&mut self, seed: i64);
    fn next_int(&mut self) -> i32;
    fn next_int_bounded(&mut self, bound: i32) -> i32;
    fn next_long(&mut self) -> i64;
    fn next_boolean(&mut self) -> bool;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
    fn next_gaussian(&mut self) -> f64;
    fn next_bytes(&mut self, bytes: &mut [i8]);
}

impl<R: JavaRng + ?Sized> JavaRng for &mut R {
    fn set_seed(&mut self, seed: i64) {
        (**self).set_seed(seed)
    }
    fn next_int(&mut self) -> i32 {
        (**self).next_int()
    }
    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        (**self).next_int_bounded(bound)
    }
    fn next_long(&mut self) -> i64 {
        (**self).next_long()
    }
    fn next_boolean(&mut self) -> bool {
        (**self).next_boolean()
    }
    fn next_float(&mut self) -> f32 {
        (**self).next_float()
    }
    fn next_double(&mut self) -> f64 {
        (**self).next_double()
    }
    fn next_gaussian(&mut self) -> f64 {
        (**self).next_gaussian()
    }
    fn next_bytes(&mut self, bytes: &mut [i8]) {
        (**self).next_bytes(bytes)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JavaRandom {
    seed: i64,
    next_next_gaussian: Option<f64>,
}

impl JavaRandom {
    pub fn new(seed: i64) -> Self {
        Self {
            seed: initial_scramble(seed),
            next_next_gaussian: None,
        }
    }

    pub fn from_entropy() -> Self {
        Self::new(crate::entropy::random_i64())
    }

    pub fn internal_seed(&self) -> i64 {
        self.seed
    }

    pub fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        (self.seed >> (48 - bits)) as i32
    }
}

fn initial_scramble(seed: i64) -> i64 {
    (seed ^ MULTIPLIER) & MASK
}

pub fn next_bytes_from_ints(bytes: &mut [i8], mut next_int: impl FnMut() -> i32) {
    for chunk in bytes.chunks_mut(4) {
        let mut rnd = next_int();
        for byte in chunk {
            *byte = rnd as i8;
            rnd >>= 8;
        }
    }
}

impl JavaRng for JavaRandom {
    fn set_seed(&mut self, seed: i64) {
        self.seed = initial_scramble(seed);
        self.next_next_gaussian = None;
    }

    fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let mut r = self.next(31);
        let m = bound - 1;
        if (bound & m) == 0 {
            return ((bound as i64 * r as i64) >> 31) as i32;
        }
        let mut u = r;
        loop {
            r = u % bound;
            if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                return r;
            }
            u = self.next(31);
        }
    }

    fn next_long(&mut self) -> i64 {
        let high = (self.next(32) as i64) << 32;
        high.wrapping_add(self.next(32) as i64)
    }

    fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    fn next_float(&mut self) -> f32 {
        self.next(24) as f32 * FLOAT_UNIT
    }

    fn next_double(&mut self) -> f64 {
        let high = (self.next(26) as i64) << 27;
        (high + self.next(27) as i64) as f64 * DOUBLE_UNIT
    }

    fn next_gaussian(&mut self) -> f64 {
        if let Some(cached) = self.next_next_gaussian.take() {
            return cached;
        }
        loop {
            let v1 = 2.0 * self.next_double() - 1.0;
            let v2 = 2.0 * self.next_double() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                let multiplier = strict_math::sqrt(-2.0 * strict_math::log(s) / s);
                self.next_next_gaussian = Some(v2 * multiplier);
                return v1 * multiplier;
            }
        }
    }

    fn next_bytes(&mut self, bytes: &mut [i8]) {
        next_bytes_from_ints(bytes, || self.next_int());
    }
}
