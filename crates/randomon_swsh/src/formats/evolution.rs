use crate::error::{Result, invalid};

pub const ENTRY_SIZE: usize = 8;
pub const MAX_EVOLUTIONS: usize = 9;
pub const FILE_SIZE: usize = ENTRY_SIZE * MAX_EVOLUTIONS;
const CONTEXT: &str = "evolution file";

pub mod method {
    pub const NONE: u16 = 0;
    pub const LEVEL_UP_FRIENDSHIP: u16 = 1;
    pub const LEVEL_UP_FRIENDSHIP_MORNING: u16 = 2;
    pub const LEVEL_UP_FRIENDSHIP_NIGHT: u16 = 3;
    pub const LEVEL_UP: u16 = 4;
    pub const TRADE: u16 = 5;
    pub const TRADE_HELD_ITEM: u16 = 6;
    pub const TRADE_SHELMET_KARRABLAST: u16 = 7;
    pub const USE_ITEM: u16 = 8;
    pub const LEVEL_UP_ATTACK_GREATER: u16 = 9;
    pub const LEVEL_UP_ATTACK_EQUALS_DEFENSE: u16 = 10;
    pub const LEVEL_UP_DEFENSE_GREATER: u16 = 11;
    pub const LEVEL_UP_ENCRYPTION_LOW: u16 = 12;
    pub const LEVEL_UP_ENCRYPTION_HIGH: u16 = 13;
    pub const LEVEL_UP_NINJASK: u16 = 14;
    pub const LEVEL_UP_SHEDINJA: u16 = 15;
    pub const LEVEL_UP_BEAUTY: u16 = 16;
    pub const USE_ITEM_MALE: u16 = 17;
    pub const USE_ITEM_FEMALE: u16 = 18;
    pub const LEVEL_UP_HELD_ITEM_DAY: u16 = 19;
    pub const LEVEL_UP_HELD_ITEM_NIGHT: u16 = 20;
    pub const LEVEL_UP_KNOWS_MOVE: u16 = 21;
    pub const LEVEL_UP_WITH_TEAMMATE: u16 = 22;
    pub const LEVEL_UP_MALE: u16 = 23;
    pub const LEVEL_UP_FEMALE: u16 = 24;
    pub const LEVEL_UP_ELECTRIC_FIELD: u16 = 25;
    pub const LEVEL_UP_MOSSY_ROCK: u16 = 26;
    pub const LEVEL_UP_ICY_ROCK: u16 = 27;
    pub const LEVEL_UP_INVERTED: u16 = 28;
    pub const LEVEL_UP_AFFECTION_MOVE_TYPE: u16 = 29;
    pub const LEVEL_UP_MOVE_TYPE: u16 = 30;
    pub const LEVEL_UP_WEATHER: u16 = 31;
    pub const LEVEL_UP_MORNING: u16 = 32;
    pub const LEVEL_UP_NIGHT: u16 = 33;
    pub const LEVEL_UP_FORM_FEMALE: u16 = 34;
    pub const LEVEL_UP_VERSION: u16 = 36;
    pub const LEVEL_UP_VERSION_DAY: u16 = 37;
    pub const LEVEL_UP_VERSION_NIGHT: u16 = 38;
    pub const LEVEL_UP_SUMMIT: u16 = 39;
    pub const LEVEL_UP_DUSK: u16 = 40;
    pub const LEVEL_UP_WORMHOLE: u16 = 41;
    pub const USE_ITEM_WORMHOLE: u16 = 42;
    pub const CRITICAL_HITS_IN_BATTLE: u16 = 43;
    pub const HIT_POINTS_LOST_IN_BATTLE: u16 = 44;
    pub const SPIN: u16 = 45;
    pub const LEVEL_UP_NATURE_AMPED: u16 = 46;
    pub const LEVEL_UP_NATURE_LOW_KEY: u16 = 47;
    pub const TOWER_OF_DARKNESS: u16 = 48;
    pub const TOWER_OF_WATERS: u16 = 49;

    pub fn uses_level(method: u16) -> bool {
        !matches!(
            method,
            NONE | TRADE
                | TRADE_HELD_ITEM
                | TRADE_SHELMET_KARRABLAST
                | USE_ITEM
                | USE_ITEM_MALE
                | USE_ITEM_FEMALE
                | USE_ITEM_WORMHOLE
                | CRITICAL_HITS_IN_BATTLE
                | HIT_POINTS_LOST_IN_BATTLE
                | SPIN
                | TOWER_OF_DARKNESS
                | TOWER_OF_WATERS
        )
    }

    pub fn is_trade(method: u16) -> bool {
        matches!(method, TRADE | TRADE_HELD_ITEM | TRADE_SHELMET_KARRABLAST)
    }

    pub fn argument_is_item(method: u16) -> bool {
        matches!(
            method,
            TRADE_HELD_ITEM
                | USE_ITEM
                | USE_ITEM_MALE
                | USE_ITEM_FEMALE
                | LEVEL_UP_HELD_ITEM_DAY
                | LEVEL_UP_HELD_ITEM_NIGHT
                | USE_ITEM_WORMHOLE
        )
    }

    pub fn argument_is_move(method: u16) -> bool {
        method == LEVEL_UP_KNOWS_MOVE
    }

    pub fn argument_is_species(method: u16) -> bool {
        method == LEVEL_UP_WITH_TEAMMATE
    }

    pub fn argument_is_type(method: u16) -> bool {
        matches!(method, LEVEL_UP_AFFECTION_MOVE_TYPE | LEVEL_UP_MOVE_TYPE)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Evolution {
    pub method: u16,
    pub argument: u16,
    pub species: u16,
    pub form: i8,
    pub level: u8,
}

impl Evolution {
    fn parse(entry: &[u8; ENTRY_SIZE]) -> Self {
        Evolution {
            method: u16::from_le_bytes([entry[0], entry[1]]),
            argument: u16::from_le_bytes([entry[2], entry[3]]),
            species: u16::from_le_bytes([entry[4], entry[5]]),
            form: i8::from_le_bytes([entry[6]]),
            level: entry[7],
        }
    }

    fn write(&self, entry: &mut [u8; ENTRY_SIZE]) {
        entry[0..2].copy_from_slice(&self.method.to_le_bytes());
        entry[2..4].copy_from_slice(&self.argument.to_le_bytes());
        entry[4..6].copy_from_slice(&self.species.to_le_bytes());
        entry[6..7].copy_from_slice(&self.form.to_le_bytes());
        entry[7] = self.level;
    }

    fn is_empty(&self) -> bool {
        *self == Evolution::default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvolutionSet {
    pub evolutions: Vec<Evolution>,
}

impl EvolutionSet {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() != FILE_SIZE {
            return Err(invalid(
                CONTEXT,
                format!("expected {FILE_SIZE} bytes, got {}", data.len()),
            ));
        }
        let entries: Vec<Evolution> = data
            .as_chunks::<ENTRY_SIZE>()
            .0
            .iter()
            .map(Evolution::parse)
            .collect();
        let used = entries
            .iter()
            .rposition(|entry| !entry.is_empty())
            .map_or(0, |index| index + 1);
        Ok(EvolutionSet {
            evolutions: entries[..used].to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        if self.evolutions.len() > MAX_EVOLUTIONS {
            return Err(invalid(
                CONTEXT,
                format!(
                    "{} evolutions exceed the limit of {MAX_EVOLUTIONS}",
                    self.evolutions.len()
                ),
            ));
        }
        let mut data = vec![0u8; FILE_SIZE];
        for (entry, evolution) in data
            .as_chunks_mut::<ENTRY_SIZE>()
            .0
            .iter_mut()
            .zip(&self.evolutions)
        {
            evolution.write(entry);
        }
        Ok(data)
    }

    pub fn active(&self) -> impl Iterator<Item = &Evolution> {
        self.evolutions
            .iter()
            .filter(|evolution| evolution.method != method::NONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> EvolutionSet {
        EvolutionSet {
            evolutions: vec![
                Evolution {
                    method: method::LEVEL_UP,
                    argument: 0,
                    species: 2,
                    form: 0,
                    level: 16,
                },
                Evolution {
                    method: method::USE_ITEM,
                    argument: 82,
                    species: 136,
                    form: -1,
                    level: 0,
                },
            ],
        }
    }

    #[test]
    fn evolution_set_round_trips() {
        let bytes = sample().to_bytes().expect("write");
        assert_eq!(bytes.len(), FILE_SIZE);
        assert_eq!(bytes[..8], [4, 0, 0, 0, 2, 0, 0, 16]);
        assert_eq!(bytes[14], 0xFF);
        assert_eq!(EvolutionSet::parse(&bytes), Ok(sample()));
        assert_eq!(sample().active().count(), 2);
    }

    #[test]
    fn gaps_before_used_entries_are_preserved() {
        let mut bytes = vec![0u8; FILE_SIZE];
        bytes[16] = 4;
        bytes[23] = 30;
        let set = EvolutionSet::parse(&bytes).expect("parse");
        assert_eq!(set.evolutions.len(), 3);
        assert_eq!(set.active().count(), 1);
        assert_eq!(set.to_bytes().expect("write"), bytes);
    }

    #[test]
    fn invalid_sizes_are_rejected() {
        assert!(EvolutionSet::parse(&[0; 8]).is_err());
        let set = EvolutionSet {
            evolutions: vec![Evolution::default(); MAX_EVOLUTIONS + 1],
        };
        assert!(set.to_bytes().is_err());
    }

    #[test]
    fn method_helpers_classify_arguments() {
        assert!(method::uses_level(method::LEVEL_UP));
        assert!(!method::uses_level(method::USE_ITEM));
        assert!(method::is_trade(method::TRADE_HELD_ITEM));
        assert!(method::argument_is_item(method::LEVEL_UP_HELD_ITEM_NIGHT));
        assert!(method::argument_is_move(method::LEVEL_UP_KNOWS_MOVE));
        assert!(method::argument_is_species(method::LEVEL_UP_WITH_TEAMMATE));
        assert!(method::argument_is_type(method::LEVEL_UP_MOVE_TYPE));
    }
}
