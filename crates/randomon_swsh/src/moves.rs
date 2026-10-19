use randomon_data::constants::moves;
use randomon_gen7::constants::{move_category_to_byte, type_to_byte};
use randomon_gen7::parse_move;
use randomon_java::math::round_double;

use crate::constants;
use crate::formats::moves::Move as MoveRecord;
use crate::handler::SwshRomHandler;

const RECORD_LENGTH: usize = 40;
const NEVER_MISSES: i32 = 101;

fn flag_word(record: &MoveRecord) -> u32 {
    [
        record.makes_contact,
        record.charge,
        record.recharge,
        record.protect,
        record.reflectable,
        record.snatch,
        record.mirror,
        record.punch,
        record.sound,
        record.gravity,
        record.defrost,
        record.distance_triple,
        record.heal,
        record.ignore_substitute,
        record.fail_sky_battle,
        record.animate_ally,
        record.dance,
    ]
    .iter()
    .enumerate()
    .fold(0, |word, (bit, &set)| word | (u32::from(set) << bit))
}

fn packed_record(record: &MoveRecord) -> [u8; RECORD_LENGTH] {
    let mut data = [0u8; RECORD_LENGTH];
    data[0] = record.move_type;
    data[1] = record.quality;
    data[2] = record.category;
    data[3] = record.power;
    data[4] = record.accuracy;
    data[5] = record.pp;
    data[6] = record.priority as u8;
    data[7] = (record.hit_min & 0x0F) | (record.hit_max << 4);
    data[8..10].copy_from_slice(&record.inflict.to_le_bytes());
    data[10] = record.inflict_percent;
    data[11] = record.inflict_count;
    data[12] = record.turn_min;
    data[13] = record.turn_max;
    data[14] = record.crit_stage as u8;
    data[15] = record.flinch;
    data[16..18].copy_from_slice(&record.effect_sequence.to_le_bytes());
    data[18] = record.recoil as u8;
    data[19] = record.healing as u8;
    data[20] = record.target;
    data[21..24].copy_from_slice(&[record.stat_1, record.stat_2, record.stat_3]);
    data[24..27].copy_from_slice(&[
        record.stat_1_stage as u8,
        record.stat_2_stage as u8,
        record.stat_3_stage as u8,
    ]);
    data[27..30].copy_from_slice(&[
        record.stat_1_percent,
        record.stat_2_percent,
        record.stat_3_percent,
    ]);
    data[36..40].copy_from_slice(&flag_word(record).to_le_bytes());
    data
}

pub fn is_usable(number: i32, record: &MoveRecord) -> bool {
    number > 0 && record.can_use_move && !constants::is_max_move(number)
}

impl SwshRomHandler {
    pub(crate) fn load_moves(&mut self) {
        let names = self.text_lines(crate::TextRole::MoveNames);
        let records = self.files.data.moves.clone();
        self.core.data.reserve_moves(records.len());
        for (index, record) in records.iter().enumerate() {
            let number = index as i32;
            if !is_usable(number, record) {
                continue;
            }
            let name = names.get(index).cloned().unwrap_or_default();
            let Ok(mv) = parse_move(number, name, &packed_record(record)) else {
                continue;
            };
            if number == moves::SWIFT {
                self.core.engine.perfect_accuracy = mv.hitratio as i32;
            }
            self.core.data.create_move(mv);
        }
    }

    pub(crate) fn save_moves(&mut self) {
        for index in 0..self.files.data.moves.len() {
            let Some(mv) = self.core.data.move_by_number(index as i32) else {
                continue;
            };
            let rounded = round_double(mv.hitratio) as i32;
            let accuracy = if rounded > NEVER_MISSES {
                100
            } else {
                rounded.max(0)
            };
            let mut record = self.files.data.moves[index];
            record.move_type = type_to_byte(mv.kind);
            record.category = move_category_to_byte(mv.category);
            record.power = mv.power.clamp(0, 255) as u8;
            record.accuracy = accuracy as u8;
            record.pp = mv.pp.clamp(0, 255) as u8;
            if record != self.files.data.moves[index] {
                self.files.data.moves[index] = record;
            }
        }
    }
}
