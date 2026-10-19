use crate::error::Result;
use crate::formats::flatbuffer::{build_table_list, parse_table_list, scalar_table};

pub const FILE: &str = "bin/appli/chika/data_table/underground_exploration_poke.bin";
const CONTEXT: &str = "EncounterUndergroundArchive";

scalar_table! {
    pub struct UndergroundEncounter {
        0 => is_single_capture: bool,
        1 => single_capture_flag_block: u64,
        2 => field_02: u8,
        3 => form: u8,
        4 => gigantamax_state: u32,
        5 => ball: u32,
        6 => index: u32,
        7 => level: u32,
        8 => species: i32,
        9 => ui_message_id: u64,
        10 => ot_gender: u32,
        11 => version: u8,
        12 => shiny: u32,
        13 => iv_speed: i8,
        14 => iv_attack: i8,
        15 => iv_defense: i8,
        16 => iv_hp: i8,
        17 => iv_special_attack: i8,
        18 => iv_special_defense: i8,
        19 => ability: u32,
        20 => is_story_progress_gated: bool,
        21 => move_0: u32,
        22 => move_1: u32,
        23 => move_2: u32,
        24 => move_3: u32,
    }
}

impl UndergroundEncounter {
    pub fn moves(&self) -> [u32; 4] {
        [self.move_0, self.move_1, self.move_2, self.move_3]
    }

    pub fn set_moves(&mut self, moves: [u32; 4]) {
        [self.move_0, self.move_1, self.move_2, self.move_3] = moves;
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UndergroundArchive {
    pub encounters: Vec<UndergroundEncounter>,
}

impl UndergroundArchive {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(UndergroundArchive {
            encounters: parse_table_list(data, CONTEXT, UndergroundEncounter::read)?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        build_table_list(
            self.encounters
                .iter()
                .map(UndergroundEncounter::build)
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn underground_archive_round_trips() {
        let mut encounter = UndergroundEncounter {
            single_capture_flag_block: 0xCBF2_9CE4_8422_2645,
            ball: 1,
            index: 4,
            level: 65,
            species: 2,
            ability: 4,
            ..UndergroundEncounter::default()
        };
        encounter.set_moves([520, 235, 76, 188]);
        let archive = UndergroundArchive {
            encounters: vec![encounter],
        };
        let parsed = UndergroundArchive::parse(&archive.to_bytes()).expect("parse");
        assert_eq!(parsed, archive);
        assert_eq!(parsed.encounters[0].moves(), [520, 235, 76, 188]);
    }
}
