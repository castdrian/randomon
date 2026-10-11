mod grid;

use randomon_core::{HandlerRegistry, RandomizationOptions};
use randomon_gen3::{Gen3Factory, synthetic};

const ENTRIES: &[&str] = &["Emerald (U)", "Fire Red (U) 1.1", "Ruby (U)"];
const ROUNDS_ENV: &str = "RANDOMON_GEN3_CORRUPTION_ROUNDS";
const DEFAULT_ROUNDS: usize = 6;
const BYTES_PER_CORRUPTION: usize = 20_000;

struct Noise(u64);

impl Noise {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

fn rounds() -> usize {
    std::env::var(ROUNDS_ENV)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(DEFAULT_ROUNDS)
}

fn corrupt(rom: &mut [u8], noise: &mut Noise) {
    let intensity = 1 + noise.below(BYTES_PER_CORRUPTION);
    for _ in 0..intensity {
        let offset = 0x200 + noise.below(rom.len() - 0x200);
        rom[offset] = noise.next() as u8;
    }
}

#[test]
fn corrupted_roms_never_panic() {
    let registry = HandlerRegistry::new().with(Gen3Factory);
    let options = RandomizationOptions::default();
    let mut noise = Noise(0x5EED_1234_ABCD_0001);
    let mut panics = Vec::new();
    for entry in ENTRIES {
        let clean = synthetic::build(entry);
        let mut cases = grid::settings_grid(entry);
        cases.extend(grid::fuzz_grid(entry, 8));
        for round in 0..rounds() {
            let mut rom = clean.clone();
            corrupt(&mut rom, &mut noise);
            let (case, settings) = &cases[(round * 7) % cases.len()];
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = registry.randomize(
                    &rom,
                    settings,
                    grid::SEEDS[round % grid::SEEDS.len()],
                    &options,
                );
            }));
            if outcome.is_err() {
                panics.push(format!("{entry} round {round} ({case})"));
            }
        }
    }
    assert!(panics.is_empty(), "panicked on: {}", panics.join(", "));
}
