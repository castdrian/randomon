use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::entropy;
use crate::random::{JavaRandom, JavaRng};

#[derive(Debug, Clone, PartialEq)]
pub struct RandomSource {
    source: JavaRandom,
    cosmetic_source: JavaRandom,
    calls: i32,
    cosmetic_calls: i32,
}

impl Default for RandomSource {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomSource {
    pub fn new() -> Self {
        let mut created = Self::with_seed(0);
        created.seed(entropy::random_i64());
        created
    }

    pub fn with_seed(seed: i64) -> Self {
        Self {
            source: JavaRandom::new(seed),
            cosmetic_source: JavaRandom::new(seed),
            calls: 0,
            cosmetic_calls: 0,
        }
    }

    pub fn reset(&mut self) {
        self.source = JavaRandom::from_entropy();
        self.cosmetic_source = JavaRandom::from_entropy();
        self.calls = 0;
        self.cosmetic_calls = 0;
    }

    pub fn seed(&mut self, seed: i64) {
        self.source.set_seed(seed);
        self.cosmetic_source.set_seed(seed);
        self.calls = 0;
        self.cosmetic_calls = 0;
    }

    fn count_call(&mut self) -> &mut JavaRandom {
        self.calls = self.calls.wrapping_add(1);
        &mut self.source
    }

    pub fn random(&mut self) -> f64 {
        self.count_call().next_double()
    }

    pub fn next_int_bounded(&mut self, size: i32) -> i32 {
        self.count_call().next_int_bounded(size)
    }

    pub fn next_int_cosmetic(&mut self, size: i32) -> i32 {
        self.cosmetic_calls = self.cosmetic_calls.wrapping_add(1);
        self.cosmetic_source.next_int_bounded(size)
    }

    pub fn next_bytes(&mut self, bytes: &mut [i8]) {
        self.count_call().next_bytes(bytes)
    }

    pub fn next_int(&mut self) -> i32 {
        self.count_call().next_int()
    }

    pub fn next_long(&mut self) -> i64 {
        self.count_call().next_long()
    }

    pub fn next_boolean(&mut self) -> bool {
        self.count_call().next_boolean()
    }

    pub fn next_float(&mut self) -> f32 {
        self.count_call().next_float()
    }

    pub fn next_double(&mut self) -> f64 {
        self.count_call().next_double()
    }

    pub fn next_gaussian(&mut self) -> f64 {
        self.count_call().next_gaussian()
    }

    pub fn pick_seed() -> i64 {
        Self::seed_from_bytes(entropy::secure_seed_bytes::<6>())
    }

    pub fn seed_from_bytes(bytes: [u8; 6]) -> i64 {
        bytes.iter().enumerate().fold(0i64, |value, (index, byte)| {
            value | (i64::from(*byte) << (8 * index))
        })
    }

    pub fn calls_since_seed(&self) -> i32 {
        self.calls.wrapping_add(self.cosmetic_calls)
    }

    pub fn instance(&mut self) -> RandomSourceInstance<'_> {
        RandomSourceInstance { source: self }
    }

    pub fn cosmetic_instance(&mut self) -> CosmeticRandomSourceInstance<'_> {
        CosmeticRandomSourceInstance { source: self }
    }

    pub fn into_shared(self) -> SharedRandomSource {
        SharedRandomSource {
            inner: Arc::new(Mutex::new(self)),
        }
    }
}

macro_rules! forward_main_rng {
    ($target:ty, $access:ident) => {
        impl JavaRng for $target {
            fn set_seed(&mut self, seed: i64) {
                self.$access().seed(seed)
            }
            fn next_int(&mut self) -> i32 {
                self.$access().next_int()
            }
            fn next_int_bounded(&mut self, bound: i32) -> i32 {
                self.$access().next_int_bounded(bound)
            }
            fn next_long(&mut self) -> i64 {
                self.$access().next_long()
            }
            fn next_boolean(&mut self) -> bool {
                self.$access().next_boolean()
            }
            fn next_float(&mut self) -> f32 {
                self.$access().next_float()
            }
            fn next_double(&mut self) -> f64 {
                self.$access().next_double()
            }
            fn next_gaussian(&mut self) -> f64 {
                self.$access().next_gaussian()
            }
            fn next_bytes(&mut self, bytes: &mut [i8]) {
                self.$access().next_bytes(bytes)
            }
        }
    };
}

macro_rules! forward_cosmetic_rng {
    ($target:ty, $access:ident) => {
        impl JavaRng for $target {
            fn set_seed(&mut self, seed: i64) {
                self.$access().seed(seed)
            }
            fn next_int(&mut self) -> i32 {
                0
            }
            fn next_int_bounded(&mut self, bound: i32) -> i32 {
                self.$access().next_int_cosmetic(bound)
            }
            fn next_long(&mut self) -> i64 {
                0
            }
            fn next_boolean(&mut self) -> bool {
                false
            }
            fn next_float(&mut self) -> f32 {
                0.0
            }
            fn next_double(&mut self) -> f64 {
                0.0
            }
            fn next_gaussian(&mut self) -> f64 {
                0.0
            }
            fn next_bytes(&mut self, _bytes: &mut [i8]) {}
        }
    };
}

pub struct RandomSourceInstance<'a> {
    source: &'a mut RandomSource,
}

impl RandomSourceInstance<'_> {
    fn target(&mut self) -> &mut RandomSource {
        &mut *self.source
    }
}

forward_main_rng!(RandomSourceInstance<'_>, target);

pub struct CosmeticRandomSourceInstance<'a> {
    source: &'a mut RandomSource,
}

impl CosmeticRandomSourceInstance<'_> {
    fn target(&mut self) -> &mut RandomSource {
        &mut *self.source
    }
}

forward_cosmetic_rng!(CosmeticRandomSourceInstance<'_>, target);

#[derive(Debug, Clone)]
pub struct SharedRandomSource {
    inner: Arc<Mutex<RandomSource>>,
}

impl Default for SharedRandomSource {
    fn default() -> Self {
        RandomSource::new().into_shared()
    }
}

impl SharedRandomSource {
    pub fn lock(&self) -> MutexGuard<'_, RandomSource> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn instance(&self) -> SharedRandomSourceInstance {
        SharedRandomSourceInstance {
            shared: self.clone(),
        }
    }

    pub fn cosmetic_instance(&self) -> SharedCosmeticRandomSourceInstance {
        SharedCosmeticRandomSourceInstance {
            shared: self.clone(),
        }
    }

    pub fn seed(&self, seed: i64) {
        self.lock().seed(seed)
    }

    pub fn reset(&self) {
        self.lock().reset()
    }

    pub fn calls_since_seed(&self) -> i32 {
        self.lock().calls_since_seed()
    }
}

#[derive(Debug, Clone)]
pub struct SharedRandomSourceInstance {
    shared: SharedRandomSource,
}

impl SharedRandomSourceInstance {
    fn target(&mut self) -> MutexGuard<'_, RandomSource> {
        self.shared.lock()
    }
}

forward_main_rng!(SharedRandomSourceInstance, target);

#[derive(Debug, Clone)]
pub struct SharedCosmeticRandomSourceInstance {
    shared: SharedRandomSource,
}

impl SharedCosmeticRandomSourceInstance {
    fn target(&mut self) -> MutexGuard<'_, RandomSource> {
        self.shared.lock()
    }
}

forward_cosmetic_rng!(SharedCosmeticRandomSourceInstance, target);
