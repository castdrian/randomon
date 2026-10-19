use crate::error::Result;
use crate::formats::flatbuffer::{TableRef, scalar_table};

scalar_table! {
    pub struct Move {
        0 => version: u32,
        1 => move_id: u32,
        2 => can_use_move: bool,
        3 => move_type: u8,
        4 => quality: u8,
        5 => category: u8,
        6 => power: u8,
        7 => accuracy: u8,
        8 => pp: u8,
        9 => priority: i8,
        10 => hit_max: u8,
        11 => hit_min: u8,
        12 => inflict: u16,
        13 => inflict_percent: u8,
        14 => inflict_count: u8,
        15 => turn_min: u8,
        16 => turn_max: u8,
        17 => crit_stage: i8,
        18 => flinch: u8,
        19 => effect_sequence: u16,
        20 => recoil: i8,
        21 => healing: i8,
        22 => target: u8,
        23 => stat_1: u8,
        24 => stat_2: u8,
        25 => stat_3: u8,
        26 => stat_1_stage: i8,
        27 => stat_2_stage: i8,
        28 => stat_3_stage: i8,
        29 => stat_1_percent: u8,
        30 => stat_2_percent: u8,
        31 => stat_3_percent: u8,
        32 => gigantamax_power: u8,
        33 => makes_contact: bool,
        34 => charge: bool,
        35 => recharge: bool,
        36 => protect: bool,
        37 => reflectable: bool,
        38 => snatch: bool,
        39 => mirror: bool,
        40 => punch: bool,
        41 => sound: bool,
        42 => gravity: bool,
        43 => defrost: bool,
        44 => distance_triple: bool,
        45 => heal: bool,
        46 => ignore_substitute: bool,
        47 => fail_sky_battle: bool,
        48 => animate_ally: bool,
        49 => dance: bool,
        50 => metronome: bool,
    }
}

pub mod category {
    pub const STATUS: u8 = 0;
    pub const PHYSICAL: u8 = 1;
    pub const SPECIAL: u8 = 2;
}

impl Move {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Move::read(&TableRef::root(data)?)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.build().finish()
    }

    pub fn is_damaging(&self) -> bool {
        self.category != category::STATUS
    }
}

pub fn file_name(index: usize) -> String {
    format!("waza{index:04}.wazabin")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tackle() -> Move {
        Move {
            version: 5,
            move_id: 33,
            can_use_move: true,
            move_type: 0,
            category: category::PHYSICAL,
            power: 40,
            accuracy: 100,
            pp: 35,
            target: 6,
            makes_contact: true,
            protect: true,
            mirror: true,
            ..Move::default()
        }
    }

    #[test]
    fn move_round_trips() {
        let bytes = tackle().to_bytes();
        assert_eq!(Move::parse(&bytes), Ok(tackle()));
        assert!(tackle().is_damaging());
    }

    #[test]
    fn previously_absent_fields_can_be_written() {
        let stub = Move {
            move_id: 700,
            ..Move::default()
        };
        let mut parsed = Move::parse(&stub.to_bytes()).expect("parse");
        parsed.move_type = 9;
        parsed.power = 90;
        parsed.priority = -1;
        let reparsed = Move::parse(&parsed.to_bytes()).expect("parse");
        assert_eq!(reparsed.move_type, 9);
        assert_eq!(reparsed.power, 90);
        assert_eq!(reparsed.priority, -1);
    }

    #[test]
    fn file_names_are_zero_padded() {
        assert_eq!(file_name(7), "waza0007.wazabin");
        assert_eq!(file_name(826), "waza0826.wazabin");
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(Move::parse(&[1, 2, 3]).is_err());
    }
}
