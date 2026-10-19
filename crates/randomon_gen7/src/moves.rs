use randomon_core::{RandomizationError, Result};
use randomon_ctr::bytes::{read_long, read_word};
use randomon_data::constants::{global, moves};
use randomon_data::{
    CriticalChance, Move, MoveStatChange, StatChangeMoveType, StatChangeType, StatusMoveType,
    StatusType,
};
use randomon_formats::ctr::{Decompression, mini};
use randomon_java::math::round_double;

use crate::constants;
use crate::handler::{Gen7RomHandler, io};
use crate::pokemon::garc_file;

const MOVE_MINI_IDENTIFIER: &str = "WD";

fn signed(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset] as i8)
}

fn unsigned(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset])
}

fn stat_change_move_type(qualities: i32, target: i32) -> StatChangeMoveType {
    match qualities {
        constants::NO_DAMAGE_STAT_CHANGE_QUALITY
        | constants::NO_DAMAGE_STATUS_AND_STAT_CHANGE_QUALITY => match target {
            6 | 7 => StatChangeMoveType::NoDamageUser,
            2 => StatChangeMoveType::NoDamageAlly,
            8 => StatChangeMoveType::NoDamageAll,
            _ => StatChangeMoveType::NoDamageTarget,
        },
        constants::DAMAGE_TARGET_DEBUFF_QUALITY => StatChangeMoveType::DamageTarget,
        constants::DAMAGE_USER_BUFF_QUALITY => StatChangeMoveType::DamageUser,
        _ => StatChangeMoveType::NoneOrUnknown,
    }
}

fn hit_count(number: i32) -> Option<f64> {
    if global::NORMAL_MULTIHIT_MOVES.contains(&number) {
        Some(19.0 / 6.0)
    } else if global::DOUBLE_HIT_MOVES.contains(&number) {
        Some(2.0)
    } else if number == moves::TRIPLE_KICK {
        Some(2.71)
    } else {
        None
    }
}

fn is_trapping_effect(effect: i32) -> bool {
    matches!(
        effect,
        constants::NO_DAMAGE_TARGET_TRAPPING_EFFECT
            | constants::NO_DAMAGE_FIELD_TRAPPING_EFFECT
            | constants::DAMAGE_ADJACENT_FOES_TRAPPING_EFFECT
            | constants::DAMAGE_TARGET_TRAPPING_EFFECT
    )
}

fn apply_status(
    mv: &mut Move,
    data: &[u8],
    internal_status_type: i32,
    qualities: i32,
) -> Result<()> {
    if internal_status_type >= 7 {
        return Ok(());
    }
    mv.status_type = StatusType::from_ordinal(internal_status_type as usize)
        .ok_or(RandomizationError::IndexOutOfBounds("StatusType.values()"))?;
    if mv.status_type == StatusType::Poison
        && (mv.number == moves::TOXIC || mv.number == moves::POISON_FANG)
    {
        mv.status_type = StatusType::ToxicPoison;
    }
    mv.status_percent_chance = f64::from(unsigned(data, 10));
    match qualities {
        constants::NO_DAMAGE_STATUS_QUALITY
        | constants::NO_DAMAGE_STATUS_AND_STAT_CHANGE_QUALITY => {
            mv.status_move_type = StatusMoveType::NoDamage;
        }
        constants::DAMAGE_STATUS_QUALITY => mv.status_move_type = StatusMoveType::Damage,
        _ => {}
    }
    Ok(())
}

fn stat_changes(data: &[u8]) -> Result<[MoveStatChange; 3]> {
    let mut changes = [MoveStatChange::default(); 3];
    for (index, change) in changes.iter_mut().enumerate() {
        let kind_index = signed(data, 21 + index);
        change.kind = usize::try_from(kind_index)
            .ok()
            .and_then(StatChangeType::from_ordinal)
            .ok_or(RandomizationError::IndexOutOfBounds(
                "StatChangeType.values()",
            ))?;
        change.stages = signed(data, 24 + index);
        change.percent_chance = f64::from(signed(data, 27 + index));
    }
    Ok(changes)
}

pub fn parse_move(number: i32, name: String, data: &[u8]) -> Result<Move> {
    let category = constants::MOVE_CATEGORY_INDICES
        .get(data[2] as usize)
        .copied()
        .ok_or(RandomizationError::IndexOutOfBounds("moveCategoryIndices"))?;
    let mut mv = Move {
        name,
        number,
        internal_id: number,
        effect_index: read_word(data, 16),
        hitratio: f64::from(unsigned(data, 4)),
        power: unsigned(data, 3),
        pp: unsigned(data, 5),
        kind: constants::type_from_byte(data[0]),
        flinch_percent_chance: f64::from(unsigned(data, 15)),
        target: unsigned(data, 20),
        category,
        priority: signed(data, 6),
        ..Move::default()
    };
    let crit_stages = unsigned(data, 14);
    if crit_stages == 6 {
        mv.critical_chance = CriticalChance::Guaranteed;
    } else if crit_stages > 0 {
        mv.critical_chance = CriticalChance::Increased;
    }
    let internal_status_type = read_word(data, 8);
    let flags = read_long(data, 36);
    mv.makes_contact = flags & 0x001 != 0;
    mv.is_charge_move = flags & 0x002 != 0;
    mv.is_recharge_move = flags & 0x004 != 0;
    mv.is_punch_move = flags & 0x080 != 0;
    mv.is_sound_move = flags & 0x100 != 0;
    mv.is_trap_move = internal_status_type == 8 || is_trapping_effect(mv.effect_index);
    let qualities = signed(data, 1);
    let recoil_or_absorb = signed(data, 18);
    if qualities == constants::DAMAGE_ABSORB_QUALITY {
        mv.absorb_percent = recoil_or_absorb;
    } else {
        mv.recoil_percent = -recoil_or_absorb;
    }
    if let Some(count) = hit_count(number) {
        mv.hit_count = count;
    }
    mv.stat_change_move_type = stat_change_move_type(qualities, mv.target);
    mv.stat_changes = stat_changes(data)?;
    apply_status(&mut mv, data, internal_status_type, qualities)?;
    Ok(mv)
}

impl Gen7RomHandler {
    fn move_records(&self) -> Result<Vec<Vec<u8>>> {
        let packed = garc_file(&self.move_garc, 0)?;
        mini::unpack(packed, MOVE_MINI_IDENTIFIER)
            .map_err(io)?
            .ok_or(RandomizationError::NullPointer("Mini.UnpackMini"))
    }

    pub(crate) fn load_moves(&mut self) -> Result<()> {
        self.move_garc = self.read_garc(self.file("MoveData"), Decompression::Skip)?;
        let count = constants::move_count(self.rom_type);
        let names = self.strings(false, self.entry.int("MoveNamesTextOffset"))?;
        let records = self.move_records()?;
        self.core.data.reserve_moves(count as usize + 1);
        for number in 1..=count {
            let data = records
                .get(number as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("move data"))?;
            let name = names
                .get(number as usize)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("move names"))?;
            let mv = parse_move(number, name, data)?;
            if number == moves::SWIFT {
                self.core.engine.perfect_accuracy = mv.hitratio as i32;
            }
            self.core.data.create_move(mv);
        }
        Ok(())
    }

    pub(crate) fn save_moves(&mut self) -> Result<()> {
        let count = constants::move_count(self.rom_type);
        let mut records = self.move_records()?;
        for number in 1..=count {
            let mv = self
                .core
                .data
                .mv(randomon_data::MoveId::from_index(number as usize));
            let mut hitratio = round_double(mv.hitratio) as i32;
            if hitratio < 0 {
                hitratio = 0;
            }
            if hitratio > 101 {
                hitratio = 100;
            }
            let data = records
                .get_mut(number as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("movesData"))?;
            data[2] = constants::move_category_to_byte(mv.category);
            data[3] = mv.power as u8;
            data[0] = constants::type_to_byte(mv.kind);
            data[4] = hitratio as u8;
            data[5] = mv.pp as u8;
        }
        let packed = mini::pack(&records, MOVE_MINI_IDENTIFIER).map_err(io)?;
        self.move_garc.set_file(0, packed).map_err(io)?;
        self.write_garc(self.file("MoveData"), &self.move_garc)
    }
}
