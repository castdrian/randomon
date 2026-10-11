use randomon_settings::JavaEnum;

pub struct Dice {
    state: u64,
}

impl Dice {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        }
    }

    pub fn roll(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    pub fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 { 0 } else { self.roll() % bound }
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    pub fn coin(&mut self) -> bool {
        self.chance(50)
    }

    pub fn range(&mut self, low: i32, high: i32) -> i32 {
        let span = (i64::from(high) - i64::from(low) + 1).max(1) as u64;
        low + self.below(span) as i32
    }

    pub fn pick<T: Copy>(&mut self, options: &[T]) -> T {
        options[self.below(options.len() as u64) as usize]
    }

    pub fn variant<E: JavaEnum>(&mut self) -> E {
        self.pick(E::ALL)
    }

    pub fn seed(&mut self) -> i64 {
        (self.roll() >> 16) as i64
    }
}
