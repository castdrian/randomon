use crate::bytes::{array_at, bytes_at, u8_at, u16_at, u32_at};
use crate::error::{Result, invalid};
use crate::formats::common::Stats;

pub const TRAINER_DATA_SIZE: usize = 0x14;
pub const TRAINER_POKEMON_SIZE: usize = 0x20;
pub const TRAINER_CLASS_SIZE: usize = 0x118;
pub const MAX_PARTY_SIZE: usize = 6;
const DATA_CONTEXT: &str = "trainer_data";
const POKE_CONTEXT: &str = "trainer_poke";
const CLASS_CONTEXT: &str = "trainer_type";

const GENDER_MASK: u8 = 0x03;
const ABILITY_SHIFT: u8 = 4;
const ABILITY_MASK: u8 = 0x30;
const IV_BITS: u32 = 5;
const IV_MASK: u32 = 0x1F;
const SHINY_BIT: u32 = 1 << 30;
const CAN_DYNAMAX_BIT: u32 = 1 << 31;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrainerData {
    pub class: u16,
    pub battle_mode: u8,
    pub party_count: u8,
    pub items: [u16; 4],
    pub ai_flags: u32,
    pub heal: u8,
    pub money: u8,
    pub gift: u16,
}

impl TrainerData {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() != TRAINER_DATA_SIZE {
            return Err(invalid(
                DATA_CONTEXT,
                format!("expected {TRAINER_DATA_SIZE} bytes, got {}", data.len()),
            ));
        }
        let mut items = [0u16; 4];
        for (index, item) in items.iter_mut().enumerate() {
            *item = u16_at(data, 4 + index * 2, DATA_CONTEXT)?;
        }
        Ok(TrainerData {
            class: u16_at(data, 0, DATA_CONTEXT)?,
            battle_mode: u8_at(data, 2, DATA_CONTEXT)?,
            party_count: u8_at(data, 3, DATA_CONTEXT)?,
            items,
            ai_flags: u32_at(data, 0x0C, DATA_CONTEXT)?,
            heal: u8_at(data, 0x10, DATA_CONTEXT)?,
            money: u8_at(data, 0x11, DATA_CONTEXT)?,
            gift: u16_at(data, 0x12, DATA_CONTEXT)?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(TRAINER_DATA_SIZE);
        data.extend_from_slice(&self.class.to_le_bytes());
        data.push(self.battle_mode);
        data.push(self.party_count);
        for item in &self.items {
            data.extend_from_slice(&item.to_le_bytes());
        }
        data.extend_from_slice(&self.ai_flags.to_le_bytes());
        data.push(self.heal);
        data.push(self.money);
        data.extend_from_slice(&self.gift.to_le_bytes());
        data
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrainerPokemon {
    pub gender: u8,
    pub ability_slot: u8,
    pub other_flags: u8,
    pub nature: u8,
    pub evs: Stats<u8>,
    pub dynamax_level: u8,
    pub can_gigantamax: bool,
    pub level: u16,
    pub species: u16,
    pub form: u16,
    pub held_item: u16,
    pub moves: [u16; 4],
    pub padding: u16,
    pub ivs: Stats<u8>,
    pub shiny: bool,
    pub can_dynamax: bool,
}

fn unpack_ivs(packed: u32) -> Stats<u8> {
    let iv = |slot: u32| ((packed >> (slot * IV_BITS)) & IV_MASK) as u8;
    Stats::from_hp_atk_def_spe_spa_spd([iv(0), iv(1), iv(2), iv(3), iv(4), iv(5)])
}

fn pack_ivs(ivs: &Stats<u8>) -> u32 {
    ivs.to_hp_atk_def_spe_spa_spd()
        .iter()
        .enumerate()
        .fold(0u32, |packed, (slot, &value)| {
            packed | ((u32::from(value.min(31)) & IV_MASK) << (slot as u32 * IV_BITS))
        })
}

impl TrainerPokemon {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() != TRAINER_POKEMON_SIZE {
            return Err(invalid(
                POKE_CONTEXT,
                format!("expected {TRAINER_POKEMON_SIZE} bytes, got {}", data.len()),
            ));
        }
        let flags = u8_at(data, 0, POKE_CONTEXT)?;
        let [hp, attack, defense, special_attack, special_defense, speed] =
            array_at::<6>(data, 2, POKE_CONTEXT)?;
        let mut moves = [0u16; 4];
        for (index, move_id) in moves.iter_mut().enumerate() {
            *move_id = u16_at(data, 0x12 + index * 2, POKE_CONTEXT)?;
        }
        let packed_ivs = u32_at(data, 0x1C, POKE_CONTEXT)?;
        let can_gigantamax = match u8_at(data, 0x09, POKE_CONTEXT)? {
            0 => false,
            1 => true,
            other => return Err(invalid(POKE_CONTEXT, format!("gigantamax flag {other}"))),
        };
        Ok(TrainerPokemon {
            gender: flags & GENDER_MASK,
            ability_slot: (flags & ABILITY_MASK) >> ABILITY_SHIFT,
            other_flags: flags & !(GENDER_MASK | ABILITY_MASK),
            nature: u8_at(data, 1, POKE_CONTEXT)?,
            evs: Stats {
                hp,
                attack,
                defense,
                speed,
                special_attack,
                special_defense,
            },
            dynamax_level: u8_at(data, 0x08, POKE_CONTEXT)?,
            can_gigantamax,
            level: u16_at(data, 0x0A, POKE_CONTEXT)?,
            species: u16_at(data, 0x0C, POKE_CONTEXT)?,
            form: u16_at(data, 0x0E, POKE_CONTEXT)?,
            held_item: u16_at(data, 0x10, POKE_CONTEXT)?,
            moves,
            padding: u16_at(data, 0x1A, POKE_CONTEXT)?,
            ivs: unpack_ivs(packed_ivs),
            shiny: packed_ivs & SHINY_BIT != 0,
            can_dynamax: packed_ivs & CAN_DYNAMAX_BIT != 0,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let flags = (self.gender & GENDER_MASK)
            | ((self.ability_slot << ABILITY_SHIFT) & ABILITY_MASK)
            | (self.other_flags & !(GENDER_MASK | ABILITY_MASK));
        let mut packed_ivs = pack_ivs(&self.ivs);
        if self.shiny {
            packed_ivs |= SHINY_BIT;
        }
        if self.can_dynamax {
            packed_ivs |= CAN_DYNAMAX_BIT;
        }
        let mut data = Vec::with_capacity(TRAINER_POKEMON_SIZE);
        data.push(flags);
        data.push(self.nature);
        data.extend_from_slice(&[
            self.evs.hp,
            self.evs.attack,
            self.evs.defense,
            self.evs.special_attack,
            self.evs.special_defense,
            self.evs.speed,
        ]);
        data.push(self.dynamax_level);
        data.push(u8::from(self.can_gigantamax));
        for value in [self.level, self.species, self.form, self.held_item] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        for move_id in &self.moves {
            data.extend_from_slice(&move_id.to_le_bytes());
        }
        data.extend_from_slice(&self.padding.to_le_bytes());
        data.extend_from_slice(&packed_ivs.to_le_bytes());
        data
    }
}

pub fn parse_team(data: &[u8]) -> Result<Vec<TrainerPokemon>> {
    if !data.len().is_multiple_of(TRAINER_POKEMON_SIZE) {
        return Err(invalid(
            POKE_CONTEXT,
            format!(
                "{} bytes is not a whole number of party members",
                data.len()
            ),
        ));
    }
    data.as_chunks::<TRAINER_POKEMON_SIZE>()
        .0
        .iter()
        .map(|member| TrainerPokemon::parse(member))
        .collect()
}

pub fn write_team(team: &[TrainerPokemon]) -> Vec<u8> {
    team.iter().flat_map(TrainerPokemon::to_bytes).collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trainer {
    pub data: TrainerData,
    pub team: Vec<TrainerPokemon>,
}

impl Trainer {
    pub fn parse(data: &[u8], team: &[u8]) -> Result<Self> {
        Ok(Trainer {
            data: TrainerData::parse(data)?,
            team: parse_team(team)?,
        })
    }

    pub fn to_bytes(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        if self.team.len() > MAX_PARTY_SIZE {
            return Err(invalid(
                DATA_CONTEXT,
                format!("party of {} exceeds {MAX_PARTY_SIZE}", self.team.len()),
            ));
        }
        let mut data = self.data;
        data.party_count = self.team.len() as u8;
        Ok((data.to_bytes(), write_team(&self.team)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerClass {
    pub raw: Vec<u8>,
}

impl TrainerClass {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() != TRAINER_CLASS_SIZE {
            return Err(invalid(
                CLASS_CONTEXT,
                format!("expected {TRAINER_CLASS_SIZE} bytes, got {}", data.len()),
            ));
        }
        Ok(TrainerClass { raw: data.to_vec() })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.clone()
    }

    pub fn group(&self) -> u8 {
        self.raw.get(1).copied().unwrap_or(0)
    }

    pub fn ball_id(&self) -> u8 {
        self.raw.get(2).copied().unwrap_or(0)
    }

    pub fn set_ball_id(&mut self, ball: u8) {
        if let Some(byte) = self.raw.get_mut(2) {
            *byte = ball;
        }
    }

    pub fn model_hash(&self) -> Result<[u8; 0x10]> {
        array_at(&self.raw, 0x08, CLASS_CONTEXT)
    }

    pub fn presentation_path(&self) -> Result<String> {
        self.ascii_at(0x18)
    }

    pub fn battle_sequence_path(&self) -> Result<String> {
        self.ascii_at(0x88)
    }

    fn ascii_at(&self, offset: usize) -> Result<String> {
        let bytes = bytes_at(&self.raw, offset, 0x80, CLASS_CONTEXT)?;
        let end = bytes
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(bytes.len());
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }
}

pub fn data_file_name(index: usize) -> String {
    format!("trainer_data_{index:03}.bin")
}

pub fn poke_file_name(index: usize) -> String {
    format!("trainer_poke_{index:03}.bin")
}

pub fn class_file_name(index: usize) -> String {
    format!("trainer_type_{index:03}.bin")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pokemon() -> TrainerPokemon {
        TrainerPokemon {
            gender: 2,
            ability_slot: 1,
            other_flags: 0,
            nature: 12,
            evs: Stats {
                hp: 1,
                attack: 2,
                defense: 3,
                speed: 6,
                special_attack: 4,
                special_defense: 5,
            },
            dynamax_level: 3,
            can_gigantamax: true,
            level: 5,
            species: 819,
            form: 0,
            held_item: 0,
            moves: [33, 39, 0, 0],
            padding: 0,
            ivs: Stats {
                hp: 31,
                attack: 1,
                defense: 2,
                speed: 3,
                special_attack: 4,
                special_defense: 5,
            },
            shiny: true,
            can_dynamax: true,
        }
    }

    #[test]
    fn trainer_pokemon_round_trips_with_documented_layout() {
        let bytes = pokemon().to_bytes();
        assert_eq!(bytes.len(), TRAINER_POKEMON_SIZE);
        assert_eq!(bytes[0], 0x12);
        assert_eq!(bytes[2..8], [1, 2, 3, 4, 5, 6]);
        assert_eq!(u16::from_le_bytes([bytes[0x0C], bytes[0x0D]]), 819);
        let ivs = u32::from_le_bytes([bytes[0x1C], bytes[0x1D], bytes[0x1E], bytes[0x1F]]);
        assert_eq!(ivs & 0x1F, 31);
        assert_eq!((ivs >> 15) & 0x1F, 3);
        assert_eq!(ivs >> 30, 3);
        assert_eq!(TrainerPokemon::parse(&bytes), Ok(pokemon()));
    }

    #[test]
    fn trainer_data_round_trips() {
        let data = TrainerData {
            class: 3,
            battle_mode: 1,
            party_count: 2,
            items: [17, 0, 0, 26],
            ai_flags: 0x8000_0007,
            heal: 1,
            money: 20,
            gift: 0,
        };
        assert_eq!(TrainerData::parse(&data.to_bytes()), Ok(data));
        assert!(TrainerData::parse(&[0; 4]).is_err());
    }

    #[test]
    fn changing_party_size_updates_the_count() {
        let mut trainer = Trainer {
            data: TrainerData {
                party_count: 1,
                ..TrainerData::default()
            },
            team: vec![pokemon()],
        };
        trainer.team.push(pokemon());
        trainer.team.push(pokemon());
        let (data, team) = trainer.to_bytes().expect("write");
        let parsed = Trainer::parse(&data, &team).expect("parse");
        assert_eq!(parsed.data.party_count, 3);
        assert_eq!(parsed.team.len(), 3);
        trainer.team = vec![pokemon(); 7];
        assert!(trainer.to_bytes().is_err());
    }

    #[test]
    fn malformed_teams_are_rejected() {
        assert!(parse_team(&[0; 33]).is_err());
        let mut bytes = pokemon().to_bytes();
        bytes[0x09] = 7;
        assert!(TrainerPokemon::parse(&bytes).is_err());
        assert_eq!(parse_team(&[]), Ok(Vec::new()));
    }

    #[test]
    fn trainer_class_exposes_known_fields() {
        let mut raw = vec![0u8; TRAINER_CLASS_SIZE];
        raw[1] = 2;
        raw[2] = 4;
        raw[0x18..0x1C].copy_from_slice(b"bgm1");
        let mut class = TrainerClass::parse(&raw).expect("parse");
        assert_eq!(class.group(), 2);
        assert_eq!(class.ball_id(), 4);
        assert_eq!(class.presentation_path(), Ok("bgm1".to_string()));
        assert_eq!(class.battle_sequence_path(), Ok(String::new()));
        class.set_ball_id(1);
        assert_eq!(class.to_bytes()[2], 1);
        assert!(TrainerClass::parse(&raw[1..]).is_err());
    }

    #[test]
    fn file_names_match_the_romfs() {
        assert_eq!(data_file_name(1), "trainer_data_001.bin");
        assert_eq!(poke_file_name(42), "trainer_poke_042.bin");
        assert_eq!(class_file_name(218), "trainer_type_218.bin");
    }
}
