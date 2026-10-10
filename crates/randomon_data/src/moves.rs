use crate::constants::global::MIN_DAMAGING_MOVE_POWER;
use crate::types::{
    CriticalChance, MoveCategory, StatChangeMoveType, StatChangeType, StatusMoveType, StatusType,
    Type,
};

#[derive(Clone, Copy, Debug)]
pub struct MoveStatChange {
    pub kind: StatChangeType,
    pub stages: i32,
    pub percent_chance: f64,
}

impl PartialEq for MoveStatChange {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.stages == other.stages
            && self.percent_chance == other.percent_chance
    }
}

impl Default for MoveStatChange {
    fn default() -> Self {
        Self {
            kind: StatChangeType::None,
            stages: 0,
            percent_chance: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Move {
    pub name: String,
    pub number: i32,
    pub internal_id: i32,
    pub power: i32,
    pub pp: i32,
    pub hitratio: f64,
    pub kind: Option<Type>,
    pub category: MoveCategory,
    pub stat_change_move_type: StatChangeMoveType,
    pub stat_changes: [MoveStatChange; 3],
    pub status_move_type: StatusMoveType,
    pub status_type: StatusType,
    pub critical_chance: CriticalChance,
    pub status_percent_chance: f64,
    pub flinch_percent_chance: f64,
    pub recoil_percent: i32,
    pub absorb_percent: i32,
    pub priority: i32,
    pub makes_contact: bool,
    pub is_charge_move: bool,
    pub is_recharge_move: bool,
    pub is_punch_move: bool,
    pub is_sound_move: bool,
    pub is_trap_move: bool,
    pub effect_index: i32,
    pub target: i32,
    pub hit_count: f64,
}

impl Default for Move {
    fn default() -> Self {
        Self {
            name: String::new(),
            number: 0,
            internal_id: 0,
            power: 0,
            pp: 0,
            hitratio: 0.0,
            kind: None,
            category: MoveCategory::Physical,
            stat_change_move_type: StatChangeMoveType::NoneOrUnknown,
            stat_changes: [MoveStatChange::default(); 3],
            status_move_type: StatusMoveType::NoneOrUnknown,
            status_type: StatusType::None,
            critical_chance: CriticalChance::Normal,
            status_percent_chance: 0.0,
            flinch_percent_chance: 0.0,
            recoil_percent: 0,
            absorb_percent: 0,
            priority: 0,
            makes_contact: false,
            is_charge_move: false,
            is_recharge_move: false,
            is_punch_move: false,
            is_sound_move: false,
            is_trap_move: false,
            effect_index: 0,
            target: 0,
            hit_count: 1.0,
        }
    }
}

impl Move {
    pub fn has_specific_stat_change(&self, kind: StatChangeType, is_positive: bool) -> bool {
        self.stat_changes
            .iter()
            .any(|change| change.kind == kind && (is_positive ^ (change.stages < 0)))
    }

    pub fn has_beneficial_stat_change(&self) -> bool {
        (self.stat_change_move_type == StatChangeMoveType::DamageTarget
            && self.stat_changes[0].stages < 0)
            || (self.stat_change_move_type == StatChangeMoveType::DamageUser
                && self.stat_changes[0].stages > 0)
    }

    pub fn effective_power(&self) -> f64 {
        f64::from(self.power) * self.hit_count
    }

    pub fn is_good_damaging(&self, perfect_accuracy: i32) -> bool {
        let power = self.effective_power();
        power >= f64::from(2 * MIN_DAMAGING_MOVE_POWER)
            || (power >= f64::from(MIN_DAMAGING_MOVE_POWER)
                && (self.hitratio >= 90.0 || self.hitratio == f64::from(perfect_accuracy)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveLearnt {
    pub move_number: i32,
    pub level: i32,
}

pub fn moves_at_level(movepool: &[MoveLearnt], level: i32, empty_value: i32) -> [i32; 4] {
    let mut current = [empty_value; 4];
    if empty_value == 0 {
        current = [0; 4];
    }
    let mut count = 0;
    for learnt in movepool {
        if learnt.level > level {
            break;
        }
        if current[..count].contains(&learnt.move_number) {
            continue;
        }
        if count == 4 {
            current.copy_within(1..4, 0);
            current[3] = learnt.move_number;
        } else {
            current[count] = learnt.move_number;
            count += 1;
        }
    }
    current
}
