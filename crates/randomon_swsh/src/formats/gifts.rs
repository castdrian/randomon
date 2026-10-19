use crate::error::Result;
use crate::formats::flatbuffer::{build_table_list, parse_table_list, scalar_table};

pub const FILE: &str = "bin/script_event_data/add_poke.bin";
pub const STARTER_SPECIES: [i32; 3] = [810, 813, 816];
const CONTEXT: &str = "EncounterGiftArchive";

scalar_table! {
    pub struct GiftEncounter {
        0 => is_egg: i32,
        1 => form: u8,
        2 => dynamax_level: u8,
        3 => ball_item: i32,
        4 => field_04: u8,
        5 => hash_1: u64,
        6 => can_gigantamax: bool,
        7 => held_item: i32,
        8 => level: u8,
        9 => species: i32,
        10 => field_0a: u8,
        11 => memory_code: u8,
        12 => memory_data: u16,
        13 => memory_feel: u8,
        14 => memory_level: u8,
        15 => ot_name_id: u64,
        16 => ot_gender: i32,
        17 => shiny_lock: i32,
        18 => nature: i32,
        19 => gender: u8,
        20 => iv_speed: i8,
        21 => iv_attack: i8,
        22 => iv_defense: i8,
        23 => iv_hp: i8,
        24 => iv_special_attack: i8,
        25 => iv_special_defense: i8,
        26 => ability: i32,
        27 => special_move: i32,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GiftEncounterArchive {
    pub gifts: Vec<GiftEncounter>,
}

impl GiftEncounterArchive {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(GiftEncounterArchive {
            gifts: parse_table_list(data, CONTEXT, GiftEncounter::read)?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        build_table_list(self.gifts.iter().map(GiftEncounter::build).collect())
    }

    pub fn starter_indices(&self) -> Vec<usize> {
        STARTER_SPECIES
            .iter()
            .filter_map(|&species| self.gifts.iter().position(|gift| gift.species == species))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn starter(species: i32) -> GiftEncounter {
        GiftEncounter {
            ball_item: 4,
            level: 5,
            species,
            ot_name_id: 0xCBF2_9CE4_8422_2645,
            ot_gender: 2,
            shiny_lock: 2,
            nature: 25,
            iv_hp: -1,
            ..GiftEncounter::default()
        }
    }

    #[test]
    fn gift_archive_round_trips_and_finds_starters() {
        let archive = GiftEncounterArchive {
            gifts: vec![starter(810), starter(772), starter(813), starter(816)],
        };
        let parsed = GiftEncounterArchive::parse(&archive.to_bytes()).expect("parse");
        assert_eq!(parsed, archive);
        assert_eq!(parsed.starter_indices(), vec![0, 2, 3]);
    }
}
