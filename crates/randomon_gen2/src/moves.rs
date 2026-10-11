use randomon_data::constants::{gb, global, moves as m};
use randomon_data::{
    CriticalChance, Move, MoveCategory, StatChangeMoveType, StatChangeType, StatusMoveType,
    StatusType,
};

use crate::constants::*;

fn stat_change_for_effect(effect: i32) -> Option<(StatChangeType, i32)> {
    let change = match effect {
        NO_DAMAGE_ATK_PLUS_ONE_EFFECT | DAMAGE_USER_ATK_PLUS_ONE_EFFECT => {
            (StatChangeType::Attack, 1)
        }
        NO_DAMAGE_DEF_PLUS_ONE_EFFECT | DAMAGE_USER_DEF_PLUS_ONE_EFFECT | DEFENSE_CURL_EFFECT => {
            (StatChangeType::Defense, 1)
        }
        NO_DAMAGE_SP_ATK_PLUS_ONE_EFFECT => (StatChangeType::SpecialAttack, 1),
        NO_DAMAGE_EVASION_PLUS_ONE_EFFECT => (StatChangeType::Evasion, 1),
        NO_DAMAGE_ATK_MINUS_ONE_EFFECT | DAMAGE_ATK_MINUS_ONE_EFFECT => {
            (StatChangeType::Attack, -1)
        }
        NO_DAMAGE_DEF_MINUS_ONE_EFFECT | DAMAGE_DEF_MINUS_ONE_EFFECT => {
            (StatChangeType::Defense, -1)
        }
        NO_DAMAGE_SPE_MINUS_ONE_EFFECT | DAMAGE_SPE_MINUS_ONE_EFFECT => (StatChangeType::Speed, -1),
        NO_DAMAGE_ACCURACY_MINUS_ONE_EFFECT | DAMAGE_ACCURACY_MINUS_ONE_EFFECT => {
            (StatChangeType::Accuracy, -1)
        }
        NO_DAMAGE_EVASION_MINUS_ONE_EFFECT => (StatChangeType::Evasion, -1),
        NO_DAMAGE_ATK_PLUS_TWO_EFFECT | SWAGGER_EFFECT => (StatChangeType::Attack, 2),
        NO_DAMAGE_DEF_PLUS_TWO_EFFECT => (StatChangeType::Defense, 2),
        NO_DAMAGE_SPE_PLUS_TWO_EFFECT => (StatChangeType::Speed, 2),
        NO_DAMAGE_SP_DEF_PLUS_TWO_EFFECT => (StatChangeType::SpecialDefense, 2),
        NO_DAMAGE_ATK_MINUS_TWO_EFFECT => (StatChangeType::Attack, -2),
        NO_DAMAGE_DEF_MINUS_TWO_EFFECT => (StatChangeType::Defense, -2),
        NO_DAMAGE_SPE_MINUS_TWO_EFFECT => (StatChangeType::Speed, -2),
        NO_DAMAGE_SP_DEF_MINUS_TWO_EFFECT => (StatChangeType::SpecialDefense, -2),
        DAMAGE_SP_DEF_MINUS_ONE_EFFECT => (StatChangeType::SpecialDefense, -1),
        DAMAGE_USER_ALL_PLUS_ONE_EFFECT => (StatChangeType::All, 1),
        _ => return None,
    };
    Some(change)
}

fn stat_change_move_type(mv: &Move) -> Option<StatChangeMoveType> {
    let kind = match mv.effect_index {
        NO_DAMAGE_ATK_PLUS_ONE_EFFECT
        | NO_DAMAGE_DEF_PLUS_ONE_EFFECT
        | NO_DAMAGE_SP_ATK_PLUS_ONE_EFFECT
        | NO_DAMAGE_EVASION_PLUS_ONE_EFFECT
        | NO_DAMAGE_ATK_MINUS_ONE_EFFECT
        | NO_DAMAGE_DEF_MINUS_ONE_EFFECT
        | NO_DAMAGE_SPE_MINUS_ONE_EFFECT
        | NO_DAMAGE_ACCURACY_MINUS_ONE_EFFECT
        | NO_DAMAGE_EVASION_MINUS_ONE_EFFECT
        | NO_DAMAGE_ATK_PLUS_TWO_EFFECT
        | NO_DAMAGE_DEF_PLUS_TWO_EFFECT
        | NO_DAMAGE_SPE_PLUS_TWO_EFFECT
        | NO_DAMAGE_SP_DEF_PLUS_TWO_EFFECT
        | NO_DAMAGE_ATK_MINUS_TWO_EFFECT
        | NO_DAMAGE_DEF_MINUS_TWO_EFFECT
        | NO_DAMAGE_SPE_MINUS_TWO_EFFECT
        | NO_DAMAGE_SP_DEF_MINUS_TWO_EFFECT
        | SWAGGER_EFFECT
        | DEFENSE_CURL_EFFECT => {
            if mv.stat_changes[0].stages < 0 || mv.effect_index == SWAGGER_EFFECT {
                StatChangeMoveType::NoDamageTarget
            } else {
                StatChangeMoveType::NoDamageUser
            }
        }
        DAMAGE_ATK_MINUS_ONE_EFFECT
        | DAMAGE_DEF_MINUS_ONE_EFFECT
        | DAMAGE_SPE_MINUS_ONE_EFFECT
        | DAMAGE_SP_DEF_MINUS_ONE_EFFECT
        | DAMAGE_ACCURACY_MINUS_ONE_EFFECT => StatChangeMoveType::DamageTarget,
        DAMAGE_USER_DEF_PLUS_ONE_EFFECT
        | DAMAGE_USER_ATK_PLUS_ONE_EFFECT
        | DAMAGE_USER_ALL_PLUS_ONE_EFFECT => StatChangeMoveType::DamageUser,
        _ => return None,
    };
    Some(kind)
}

fn chance_or_certain(chance: f64) -> f64 {
    if chance == 0.0 { 100.0 } else { chance }
}

pub fn apply_stat_changes_from_effect(mv: &mut Move, secondary_chance: f64) {
    let Some((kind, stages)) = stat_change_for_effect(mv.effect_index) else {
        return;
    };
    mv.stat_changes[0].kind = kind;
    mv.stat_changes[0].stages = stages;
    if let Some(move_type) = stat_change_move_type(mv) {
        mv.stat_change_move_type = move_type;
    }
    if matches!(
        mv.stat_change_move_type,
        StatChangeMoveType::DamageTarget | StatChangeMoveType::DamageUser
    ) {
        for change in &mut mv.stat_changes {
            if change.kind != StatChangeType::None {
                change.percent_chance = chance_or_certain(secondary_chance);
            }
        }
    }
}

fn status_move_type(effect: i32) -> Option<StatusMoveType> {
    match effect {
        NO_DAMAGE_SLEEP_EFFECT
        | TOXIC_EFFECT
        | NO_DAMAGE_CONFUSION_EFFECT
        | NO_DAMAGE_POISON_EFFECT
        | NO_DAMAGE_PARALYZE_EFFECT
        | SWAGGER_EFFECT => Some(StatusMoveType::NoDamage),
        DAMAGE_POISON_EFFECT
        | DAMAGE_BURN_EFFECT
        | DAMAGE_FREEZE_EFFECT
        | DAMAGE_PARALYZE_EFFECT
        | DAMAGE_CONFUSION_EFFECT
        | TWINEEDLE_EFFECT
        | DAMAGE_BURN_AND_THAW_USER_EFFECT
        | THUNDER_EFFECT => Some(StatusMoveType::Damage),
        _ => None,
    }
}

fn status_type(effect: i32) -> Option<StatusType> {
    match effect {
        NO_DAMAGE_SLEEP_EFFECT => Some(StatusType::Sleep),
        DAMAGE_POISON_EFFECT | NO_DAMAGE_POISON_EFFECT | TWINEEDLE_EFFECT => {
            Some(StatusType::Poison)
        }
        DAMAGE_BURN_EFFECT | DAMAGE_BURN_AND_THAW_USER_EFFECT => Some(StatusType::Burn),
        DAMAGE_FREEZE_EFFECT => Some(StatusType::Freeze),
        DAMAGE_PARALYZE_EFFECT | NO_DAMAGE_PARALYZE_EFFECT | THUNDER_EFFECT => {
            Some(StatusType::Paralyze)
        }
        TOXIC_EFFECT => Some(StatusType::ToxicPoison),
        NO_DAMAGE_CONFUSION_EFFECT | DAMAGE_CONFUSION_EFFECT | SWAGGER_EFFECT => {
            Some(StatusType::Confusion)
        }
        _ => None,
    }
}

pub fn apply_status_from_effect(mv: &mut Move, secondary_chance: f64) {
    let Some(move_type) = status_move_type(mv.effect_index) else {
        return;
    };
    mv.status_move_type = move_type;
    if let Some(kind) = status_type(mv.effect_index) {
        mv.status_type = kind;
    }
    if mv.status_move_type == StatusMoveType::Damage {
        mv.status_percent_chance = chance_or_certain(secondary_chance);
    }
}

pub fn apply_misc_info_from_effect(mv: &mut Move, secondary_chance: f64) {
    match mv.effect_index {
        FLINCH_EFFECT | SNORE_EFFECT | TWISTER_EFFECT | STOMP_EFFECT => {
            mv.flinch_percent_chance = secondary_chance
        }
        DAMAGE_ABSORB_EFFECT | DREAM_EATER_EFFECT => mv.absorb_percent = 50,
        DAMAGE_RECOIL_EFFECT => mv.recoil_percent = 25,
        FLAIL_AND_REVERSAL_EFFECT | FUTURE_SIGHT_EFFECT => {
            mv.critical_chance = CriticalChance::None
        }
        BINDING_EFFECT | TRAPPING_EFFECT => mv.is_trap_move = true,
        RAZOR_WIND_EFFECT
        | SKY_ATTACK_EFFECT
        | SKULL_BASH_EFFECT
        | SOLARBEAM_EFFECT
        | SEMI_INVULNERABLE_EFFECT => mv.is_charge_move = true,
        HYPER_BEAM_EFFECT => mv.is_recharge_move = true,
        _ => {}
    }
    if INCREASED_CRIT_MOVES.contains(&mv.number) {
        mv.critical_chance = CriticalChance::Increased;
    }
}

pub fn initial_category(mv: &Move) -> MoveCategory {
    if mv.power == 0 && !global::NO_POWER_NON_STATUS_MOVES.contains(&mv.number) {
        MoveCategory::Status
    } else if gb::is_physical_type(mv.kind) {
        MoveCategory::Physical
    } else {
        MoveCategory::Special
    }
}

pub fn hit_count(number: i32) -> f64 {
    if global::NORMAL_MULTIHIT_MOVES.contains(&number) {
        3.0
    } else if global::DOUBLE_HIT_MOVES.contains(&number) {
        2.0
    } else if number == m::TRIPLE_KICK {
        2.71
    } else {
        1.0
    }
}

pub fn priority(effect: i32) -> i32 {
    match effect {
        PRIORITY_HIT_EFFECT_INDEX => 2,
        PROTECT_EFFECT_INDEX | ENDURE_EFFECT_INDEX => 3,
        FORCE_SWITCH_EFFECT_INDEX | COUNTER_EFFECT_INDEX | MIRROR_COAT_EFFECT_INDEX => 0,
        _ => 1,
    }
}
