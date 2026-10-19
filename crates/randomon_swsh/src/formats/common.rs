#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Stats<T> {
    pub hp: T,
    pub attack: T,
    pub defense: T,
    pub speed: T,
    pub special_attack: T,
    pub special_defense: T,
}

impl<T: Copy> Stats<T> {
    pub fn uniform(value: T) -> Self {
        Stats {
            hp: value,
            attack: value,
            defense: value,
            speed: value,
            special_attack: value,
            special_defense: value,
        }
    }

    pub fn to_hp_atk_def_spe_spa_spd(&self) -> [T; 6] {
        [
            self.hp,
            self.attack,
            self.defense,
            self.speed,
            self.special_attack,
            self.special_defense,
        ]
    }

    pub fn from_hp_atk_def_spe_spa_spd(values: [T; 6]) -> Self {
        let [hp, attack, defense, speed, special_attack, special_defense] = values;
        Stats {
            hp,
            attack,
            defense,
            speed,
            special_attack,
            special_defense,
        }
    }
}

impl Stats<u8> {
    pub fn total(&self) -> u32 {
        self.to_hp_atk_def_spe_spa_spd()
            .iter()
            .map(|&value| u32::from(value))
            .sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitFlags<const N: usize>(pub [u8; N]);

impl<const N: usize> Default for BitFlags<N> {
    fn default() -> Self {
        BitFlags([0; N])
    }
}

impl<const N: usize> BitFlags<N> {
    pub const BITS: usize = N * 8;

    pub fn get(&self, index: usize) -> bool {
        self.0
            .get(index / 8)
            .is_some_and(|byte| byte & (1 << (index % 8)) != 0)
    }

    pub fn set(&mut self, index: usize, value: bool) {
        if let Some(byte) = self.0.get_mut(index / 8) {
            let mask = 1 << (index % 8);
            if value {
                *byte |= mask;
            } else {
                *byte &= !mask;
            }
        }
    }

    pub fn count(&self) -> usize {
        self.0.iter().map(|byte| byte.count_ones() as usize).sum()
    }

    pub fn set_indices(&self) -> Vec<usize> {
        (0..Self::BITS).filter(|&index| self.get(index)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_flags_get_and_set_least_significant_first() {
        let mut flags = BitFlags::<2>::default();
        flags.set(0, true);
        flags.set(9, true);
        assert_eq!(flags.0, [0x01, 0x02]);
        assert!(flags.get(9));
        assert!(!flags.get(8));
        assert!(!flags.get(100));
        flags.set(100, true);
        flags.set(0, false);
        assert_eq!(flags.0, [0x00, 0x02]);
        assert_eq!(flags.count(), 1);
        assert_eq!(flags.set_indices(), vec![9]);
    }

    #[test]
    fn stats_convert_between_named_and_array_order() {
        let stats = Stats::from_hp_atk_def_spe_spa_spd([1u8, 2, 3, 4, 5, 6]);
        assert_eq!(stats.speed, 4);
        assert_eq!(stats.to_hp_atk_def_spe_spa_spd(), [1, 2, 3, 4, 5, 6]);
        assert_eq!(stats.total(), 21);
        assert_eq!(Stats::uniform(31u8).special_defense, 31);
    }
}
