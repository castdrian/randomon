use crate::error::Result;
use crate::formats::flatbuffer::{build_table_list, parse_table_list, scalar_table};

pub const FILE: &str = "bin/script_event_data/field_trade.bin";
const CONTEXT: &str = "EncounterTradeArchive";

scalar_table! {
    pub struct TradeEncounter {
        0 => form: u8,
        1 => dynamax_level: u8,
        2 => ball_item: i32,
        3 => field_03: i32,
        4 => hash_0: u64,
        5 => can_gigantamax: bool,
        6 => held_item: i32,
        7 => level: u8,
        8 => species: i32,
        9 => hash_1: u64,
        10 => trainer_id: i32,
        11 => memory: u8,
        12 => text_variable: u16,
        13 => feeling: u8,
        14 => intensity: u8,
        15 => hash_2: u64,
        16 => ot_gender: u8,
        17 => required_form: u8,
        18 => required_species: i32,
        19 => required_nature: i32,
        20 => unknown_requirement: u8,
        21 => shiny_lock: i32,
        22 => nature: i32,
        23 => gender: i8,
        24 => iv_speed: i8,
        25 => iv_attack: i8,
        26 => iv_defense: i8,
        27 => iv_hp: i8,
        28 => iv_special_attack: i8,
        29 => iv_special_defense: i8,
        30 => ability_number: u8,
        31 => relearn_1: u16,
        32 => relearn_2: u16,
        33 => relearn_3: u16,
        34 => relearn_4: u16,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TradeEncounterArchive {
    pub trades: Vec<TradeEncounter>,
}

impl TradeEncounterArchive {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(TradeEncounterArchive {
            trades: parse_table_list(data, CONTEXT, TradeEncounter::read)?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        build_table_list(self.trades.iter().map(TradeEncounter::build).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_archive_round_trips() {
        let archive = TradeEncounterArchive {
            trades: vec![TradeEncounter {
                dynamax_level: 1,
                ball_item: 4,
                level: 18,
                species: 52,
                trainer_id: 263_455,
                required_form: 2,
                required_species: 52,
                required_nature: 25,
                shiny_lock: 2,
                relearn_1: 387,
                ..TradeEncounter::default()
            }],
        };
        assert_eq!(
            TradeEncounterArchive::parse(&archive.to_bytes()),
            Ok(archive)
        );
    }
}
