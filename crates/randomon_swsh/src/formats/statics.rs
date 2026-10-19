use crate::error::Result;
use crate::formats::common::Stats;
use crate::formats::flatbuffer::{
    FieldMap, build_table_list, identity_field_map, parse_table_list, scalar_table,
};
use crate::version::DataVersion;

pub const FILE: &str = "bin/script_event_data/event_encount_data.bin";
const CONTEXT: &str = "EncounterStaticArchive";
const EXPANSION_ONLY_FIELD: usize = 12;

scalar_table! {
    pub struct StaticEncounter {
        0 => background_far_type_id: u64,
        1 => background_near_type_id: u64,
        2 => ev_speed: u8,
        3 => ev_attack: u8,
        4 => ev_defense: u8,
        5 => ev_hp: u8,
        6 => ev_special_attack: u8,
        7 => ev_special_defense: u8,
        8 => form: u8,
        9 => dynamax_level: u8,
        10 => field_0a: i32,
        11 => encounter_id: u64,
        12 => field_0c: u8,
        13 => can_gigantamax: bool,
        14 => held_item: i32,
        15 => level: u8,
        16 => scenario: i32,
        17 => species: i32,
        18 => shiny_lock: u32,
        19 => nature: u32,
        20 => gender: i8,
        21 => iv_speed: i8,
        22 => iv_attack: i8,
        23 => iv_defense: i8,
        24 => iv_hp: i8,
        25 => iv_special_attack: i8,
        26 => iv_special_defense: i8,
        27 => ability: i32,
        28 => move_0: i32,
        29 => move_1: i32,
        30 => move_2: i32,
        31 => move_3: i32,
    }
}

fn base_game_field(index: usize) -> Option<usize> {
    match index {
        EXPANSION_ONLY_FIELD => None,
        index if index < EXPANSION_ONLY_FIELD => Some(index),
        index => Some(index - 1),
    }
}

pub fn field_map(version: DataVersion) -> FieldMap {
    match version {
        DataVersion::Base => base_game_field,
        DataVersion::Expansion => identity_field_map,
    }
}

impl StaticEncounter {
    pub fn ivs(&self) -> Stats<i8> {
        Stats {
            hp: self.iv_hp,
            attack: self.iv_attack,
            defense: self.iv_defense,
            speed: self.iv_speed,
            special_attack: self.iv_special_attack,
            special_defense: self.iv_special_defense,
        }
    }

    pub fn set_ivs(&mut self, ivs: Stats<i8>) {
        self.iv_hp = ivs.hp;
        self.iv_attack = ivs.attack;
        self.iv_defense = ivs.defense;
        self.iv_speed = ivs.speed;
        self.iv_special_attack = ivs.special_attack;
        self.iv_special_defense = ivs.special_defense;
    }

    pub fn moves(&self) -> [i32; 4] {
        [self.move_0, self.move_1, self.move_2, self.move_3]
    }

    pub fn set_moves(&mut self, moves: [i32; 4]) {
        [self.move_0, self.move_1, self.move_2, self.move_3] = moves;
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StaticEncounterArchive {
    pub encounters: Vec<StaticEncounter>,
}

impl StaticEncounterArchive {
    pub fn parse(data: &[u8], version: DataVersion) -> Result<Self> {
        let map = field_map(version);
        Ok(StaticEncounterArchive {
            encounters: parse_table_list(data, CONTEXT, |table| {
                StaticEncounter::read_mapped(table, map)
            })?,
        })
    }

    pub fn to_bytes(&self, version: DataVersion) -> Result<Vec<u8>> {
        let map = field_map(version);
        let tables = self
            .encounters
            .iter()
            .map(|encounter| encounter.build_mapped(map))
            .collect::<Result<Vec<_>>>()?;
        Ok(build_table_list(tables))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::flatbuffer::TableRef;

    fn haxorus() -> StaticEncounter {
        StaticEncounter {
            background_far_type_id: 1,
            encounter_id: 0xD3E2_5289_3D9E_9F38,
            dynamax_level: 1,
            held_item: 0,
            level: 62,
            scenario: 12,
            species: 612,
            shiny_lock: 2,
            nature: 13,
            ability: 2,
            move_0: 200,
            move_1: 89,
            move_2: 231,
            move_3: 416,
            ..StaticEncounter::default()
        }
    }

    #[test]
    fn expansion_layout_round_trips_with_schema_indices() {
        let archive = StaticEncounterArchive {
            encounters: vec![StaticEncounter {
                field_0c: 1,
                ..haxorus()
            }],
        };
        let bytes = archive.to_bytes(DataVersion::Expansion).expect("write");
        let root = TableRef::root(&bytes).expect("root");
        let entry = root.tables(0).expect("tables")[0];
        assert_eq!(entry.scalar::<i32>(17), Ok(612));
        assert_eq!(
            StaticEncounterArchive::parse(&bytes, DataVersion::Expansion),
            Ok(archive)
        );
    }

    #[test]
    fn base_layout_shifts_fields_after_the_expansion_field() {
        let archive = StaticEncounterArchive {
            encounters: vec![haxorus()],
        };
        let bytes = archive.to_bytes(DataVersion::Base).expect("write");
        let root = TableRef::root(&bytes).expect("root");
        let entry = root.tables(0).expect("tables")[0];
        assert_eq!(entry.scalar::<i32>(16), Ok(612));
        assert_eq!(entry.scalar::<u8>(14), Ok(62));
        assert_eq!(
            StaticEncounterArchive::parse(&bytes, DataVersion::Base),
            Ok(archive)
        );
        let expansion_only = StaticEncounterArchive {
            encounters: vec![StaticEncounter {
                field_0c: 1,
                ..haxorus()
            }],
        };
        assert!(expansion_only.to_bytes(DataVersion::Base).is_err());
    }

    #[test]
    fn iv_and_move_helpers_use_named_stats() {
        let mut encounter = haxorus();
        encounter.set_ivs(Stats::uniform(31));
        encounter.set_moves([1, 2, 3, 4]);
        assert_eq!(encounter.ivs().speed, 31);
        assert_eq!(encounter.moves(), [1, 2, 3, 4]);
    }
}
