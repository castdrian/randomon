use randomon_data::constants::gen1::*;
use randomon_data::constants::{gb, global, moves as m};
use randomon_data::{
    CriticalChance, Move, MoveCategory, StatChangeMoveType, StatChangeType, StatusMoveType,
    StatusType,
};

pub fn apply_stat_changes_from_effect(mv: &mut Move) {
    let (kind, stages) = match mv.effect_index {
        NO_DAMAGE_ATK_PLUS_ONE_EFFECT => (StatChangeType::Attack, 1),
        NO_DAMAGE_DEF_PLUS_ONE_EFFECT => (StatChangeType::Defense, 1),
        NO_DAMAGE_SPECIAL_PLUS_ONE_EFFECT => (StatChangeType::Special, 1),
        NO_DAMAGE_EVASION_PLUS_ONE_EFFECT => (StatChangeType::Evasion, 1),
        NO_DAMAGE_ATK_MINUS_ONE_EFFECT | DAMAGE_ATK_MINUS_ONE_EFFECT => {
            (StatChangeType::Attack, -1)
        }
        NO_DAMAGE_DEF_MINUS_ONE_EFFECT | DAMAGE_DEF_MINUS_ONE_EFFECT => {
            (StatChangeType::Defense, -1)
        }
        NO_DAMAGE_SPE_MINUS_ONE_EFFECT | DAMAGE_SPE_MINUS_ONE_EFFECT => (StatChangeType::Speed, -1),
        NO_DAMAGE_ACCURACY_MINUS_ONE_EFFECT => (StatChangeType::Accuracy, -1),
        NO_DAMAGE_ATK_PLUS_TWO_EFFECT => (StatChangeType::Attack, 2),
        NO_DAMAGE_DEF_PLUS_TWO_EFFECT => (StatChangeType::Defense, 2),
        NO_DAMAGE_SPE_PLUS_TWO_EFFECT => (StatChangeType::Speed, 2),
        NO_DAMAGE_SPECIAL_PLUS_TWO_EFFECT => (StatChangeType::Special, 2),
        NO_DAMAGE_DEF_MINUS_TWO_EFFECT => (StatChangeType::Defense, -2),
        DAMAGE_SPECIAL_MINUS_ONE_EFFECT => (StatChangeType::Special, -1),
        _ => return,
    };
    mv.stat_changes[0].kind = kind;
    mv.stat_changes[0].stages = stages;
    match mv.effect_index {
        DAMAGE_ATK_MINUS_ONE_EFFECT
        | DAMAGE_DEF_MINUS_ONE_EFFECT
        | DAMAGE_SPE_MINUS_ONE_EFFECT
        | DAMAGE_SPECIAL_MINUS_ONE_EFFECT => {
            mv.stat_change_move_type = StatChangeMoveType::DamageTarget;
        }
        _ => {
            mv.stat_change_move_type = if mv.stat_changes[0].stages < 0 {
                StatChangeMoveType::NoDamageTarget
            } else {
                StatChangeMoveType::NoDamageUser
            };
        }
    }
    if mv.stat_change_move_type == StatChangeMoveType::DamageTarget {
        for change in &mut mv.stat_changes {
            if change.kind != StatChangeType::None {
                change.percent_chance = 85.0 / 256.0;
            }
        }
    }
}

pub fn apply_status_from_effect(mv: &mut Move) {
    mv.status_move_type = match mv.effect_index {
        NO_DAMAGE_SLEEP_EFFECT
        | NO_DAMAGE_CONFUSION_EFFECT
        | NO_DAMAGE_POISON_EFFECT
        | NO_DAMAGE_PARALYZE_EFFECT => StatusMoveType::NoDamage,
        DAMAGE_POISON_20_PERCENT_EFFECT
        | DAMAGE_BURN_10_PERCENT_EFFECT
        | DAMAGE_FREEZE_10_PERCENT_EFFECT
        | DAMAGE_PARALYZE_10_PERCENT_EFFECT
        | DAMAGE_POISON_40_PERCENT_EFFECT
        | DAMAGE_BURN_30_PERCENT_EFFECT
        | DAMAGE_FREEZE_30_PERCENT_EFFECT
        | DAMAGE_PARALYZE_30_PERCENT_EFFECT
        | DAMAGE_CONFUSION_EFFECT
        | TWINEEDLE_EFFECT => StatusMoveType::Damage,
        _ => return,
    };
    match mv.effect_index {
        NO_DAMAGE_SLEEP_EFFECT => mv.status_type = StatusType::Sleep,
        DAMAGE_POISON_20_PERCENT_EFFECT
        | DAMAGE_POISON_40_PERCENT_EFFECT
        | NO_DAMAGE_POISON_EFFECT
        | TWINEEDLE_EFFECT => {
            mv.status_type = if mv.number == m::TOXIC {
                StatusType::ToxicPoison
            } else {
                StatusType::Poison
            };
        }
        DAMAGE_BURN_10_PERCENT_EFFECT | DAMAGE_BURN_30_PERCENT_EFFECT => {
            mv.status_type = StatusType::Burn
        }
        DAMAGE_FREEZE_10_PERCENT_EFFECT | DAMAGE_FREEZE_30_PERCENT_EFFECT => {
            mv.status_type = StatusType::Freeze
        }
        DAMAGE_PARALYZE_10_PERCENT_EFFECT
        | DAMAGE_PARALYZE_30_PERCENT_EFFECT
        | NO_DAMAGE_PARALYZE_EFFECT => {
            mv.status_type = StatusType::Paralyze;
        }
        NO_DAMAGE_CONFUSION_EFFECT | DAMAGE_CONFUSION_EFFECT => {
            mv.status_type = StatusType::Confusion
        }
        _ => {}
    }
    if mv.status_move_type == StatusMoveType::Damage {
        match mv.effect_index {
            DAMAGE_BURN_10_PERCENT_EFFECT
            | DAMAGE_FREEZE_10_PERCENT_EFFECT
            | DAMAGE_PARALYZE_10_PERCENT_EFFECT
            | DAMAGE_CONFUSION_EFFECT => {
                mv.status_percent_chance = 10.0;
            }
            DAMAGE_POISON_20_PERCENT_EFFECT | TWINEEDLE_EFFECT => mv.status_percent_chance = 20.0,
            DAMAGE_BURN_30_PERCENT_EFFECT
            | DAMAGE_FREEZE_30_PERCENT_EFFECT
            | DAMAGE_PARALYZE_30_PERCENT_EFFECT => {
                mv.status_percent_chance = 30.0;
            }
            DAMAGE_POISON_40_PERCENT_EFFECT => mv.status_percent_chance = 40.0,
            _ => {}
        }
    }
}

pub fn apply_misc_info_from_effect(mv: &mut Move) {
    match mv.effect_index {
        FLINCH_10_PERCENT_EFFECT => mv.flinch_percent_chance = 10.0,
        FLINCH_30_PERCENT_EFFECT => mv.flinch_percent_chance = 30.0,
        DAMAGE_ABSORB_EFFECT | DREAM_EATER_EFFECT => mv.absorb_percent = 50,
        DAMAGE_RECOIL_EFFECT => mv.recoil_percent = 25,
        CHARGE_EFFECT | FLY_EFFECT => mv.is_charge_move = true,
        HYPER_BEAM_EFFECT => mv.is_recharge_move = true,
        _ => {}
    }
    if INCREASED_CRIT_MOVES.contains(&mv.number) {
        mv.critical_chance = CriticalChance::Increased;
    }
}

pub fn initial_category(mv: &Move) -> MoveCategory {
    let mut category = if gb::is_physical_type(mv.kind) {
        MoveCategory::Physical
    } else {
        MoveCategory::Special
    };
    if mv.power == 0 && !global::NO_POWER_NON_STATUS_MOVES.contains(&mv.number) {
        category = MoveCategory::Status;
    }
    category
}

pub fn hit_count_for_rom_id(rom_id: i32) -> f64 {
    if global::NORMAL_MULTIHIT_MOVES.contains(&rom_id) {
        3.0
    } else if global::DOUBLE_HIT_MOVES.contains(&rom_id) {
        2.0
    } else {
        1.0
    }
}
