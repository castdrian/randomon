use randomon_data::constants::{gb, global, moves as m};
use randomon_data::{
    CriticalChance, Move, MoveCategory, StatChangeMoveType, StatChangeType, StatusMoveType,
    StatusType,
};

use crate::constants::*;
use crate::handler::Gen3RomHandler;

const USER_TARGET: i32 = 16;
const MAKES_CONTACT_FLAG: i32 = 1;
const TRIPLE_KICK_HIT_COUNT: f64 = 2.71;

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
        | NO_DAMAGE_SP_ATK_PLUS_TWO_EFFECT
        | NO_DAMAGE_SP_DEF_PLUS_TWO_EFFECT
        | NO_DAMAGE_ATK_MINUS_TWO_EFFECT
        | NO_DAMAGE_DEF_MINUS_TWO_EFFECT
        | NO_DAMAGE_SPE_MINUS_TWO_EFFECT
        | NO_DAMAGE_SP_DEF_MINUS_TWO_EFFECT
        | MINIMIZE_EFFECT
        | SWAGGER_EFFECT
        | DEFENSE_CURL_EFFECT
        | FLATTER_EFFECT
        | CHARGE_EFFECT
        | NO_DAMAGE_ATK_AND_DEF_MINUS_ONE_EFFECT
        | NO_DAMAGE_DEF_AND_SP_DEF_PLUS_ONE_EFFECT
        | NO_DAMAGE_ATK_AND_DEF_PLUS_ONE_EFFECT
        | NO_DAMAGE_SP_ATK_AND_SP_DEF_PLUS_ONE_EFFECT
        | NO_DAMAGE_ATK_AND_SPE_PLUS_ONE_EFFECT => {
            if mv.target == USER_TARGET {
                StatChangeMoveType::NoDamageUser
            } else {
                StatChangeMoveType::NoDamageTarget
            }
        }
        DAMAGE_ATK_MINUS_ONE_EFFECT
        | DAMAGE_DEF_MINUS_ONE_EFFECT
        | DAMAGE_SPE_MINUS_ONE_EFFECT
        | DAMAGE_SP_ATK_MINUS_ONE_EFFECT
        | DAMAGE_SP_DEF_MINUS_ONE_EFFECT
        | DAMAGE_ACCURACY_MINUS_ONE_EFFECT => StatChangeMoveType::DamageTarget,
        DAMAGE_USER_DEF_PLUS_ONE_EFFECT
        | DAMAGE_USER_ATK_PLUS_ONE_EFFECT
        | DAMAGE_USER_ALL_PLUS_ONE_EFFECT
        | DAMAGE_USER_ATK_AND_DEF_MINUS_ONE_EFFECT
        | DAMAGE_USER_SP_ATK_MINUS_TWO_EFFECT => StatChangeMoveType::DamageUser,
        _ => return None,
    };
    Some(kind)
}

fn stat_changes_for_effect(effect: i32) -> &'static [(StatChangeType, i32)] {
    use StatChangeType::*;
    match effect {
        NO_DAMAGE_ATK_PLUS_ONE_EFFECT | DAMAGE_USER_ATK_PLUS_ONE_EFFECT => &[(Attack, 1)],
        NO_DAMAGE_DEF_PLUS_ONE_EFFECT | DAMAGE_USER_DEF_PLUS_ONE_EFFECT | DEFENSE_CURL_EFFECT => {
            &[(Defense, 1)]
        }
        NO_DAMAGE_SP_ATK_PLUS_ONE_EFFECT | FLATTER_EFFECT => &[(SpecialAttack, 1)],
        NO_DAMAGE_EVASION_PLUS_ONE_EFFECT | MINIMIZE_EFFECT => &[(Evasion, 1)],
        NO_DAMAGE_ATK_MINUS_ONE_EFFECT | DAMAGE_ATK_MINUS_ONE_EFFECT => &[(Attack, -1)],
        NO_DAMAGE_DEF_MINUS_ONE_EFFECT | DAMAGE_DEF_MINUS_ONE_EFFECT => &[(Defense, -1)],
        NO_DAMAGE_SPE_MINUS_ONE_EFFECT | DAMAGE_SPE_MINUS_ONE_EFFECT => &[(Speed, -1)],
        NO_DAMAGE_ACCURACY_MINUS_ONE_EFFECT | DAMAGE_ACCURACY_MINUS_ONE_EFFECT => &[(Accuracy, -1)],
        NO_DAMAGE_EVASION_MINUS_ONE_EFFECT => &[(Evasion, -1)],
        NO_DAMAGE_ATK_PLUS_TWO_EFFECT | SWAGGER_EFFECT => &[(Attack, 2)],
        NO_DAMAGE_DEF_PLUS_TWO_EFFECT => &[(Defense, 2)],
        NO_DAMAGE_SPE_PLUS_TWO_EFFECT => &[(Speed, 2)],
        NO_DAMAGE_SP_ATK_PLUS_TWO_EFFECT => &[(SpecialAttack, 2)],
        NO_DAMAGE_SP_DEF_PLUS_TWO_EFFECT => &[(SpecialDefense, 2)],
        NO_DAMAGE_ATK_MINUS_TWO_EFFECT => &[(Attack, -2)],
        NO_DAMAGE_DEF_MINUS_TWO_EFFECT => &[(Defense, -2)],
        NO_DAMAGE_SPE_MINUS_TWO_EFFECT => &[(Speed, -2)],
        NO_DAMAGE_SP_DEF_MINUS_TWO_EFFECT => &[(SpecialDefense, -2)],
        DAMAGE_SP_ATK_MINUS_ONE_EFFECT => &[(SpecialAttack, -1)],
        DAMAGE_SP_DEF_MINUS_ONE_EFFECT => &[(SpecialDefense, -1)],
        DAMAGE_USER_ALL_PLUS_ONE_EFFECT => &[(All, 1)],
        CHARGE_EFFECT => &[(SpecialDefense, 1)],
        DAMAGE_USER_ATK_AND_DEF_MINUS_ONE_EFFECT | NO_DAMAGE_ATK_AND_DEF_MINUS_ONE_EFFECT => {
            &[(Attack, -1), (Defense, -1)]
        }
        DAMAGE_USER_SP_ATK_MINUS_TWO_EFFECT => &[(SpecialAttack, -2)],
        NO_DAMAGE_DEF_AND_SP_DEF_PLUS_ONE_EFFECT => &[(Defense, 1), (SpecialDefense, 1)],
        NO_DAMAGE_ATK_AND_DEF_PLUS_ONE_EFFECT => &[(Attack, 1), (Defense, 1)],
        NO_DAMAGE_SP_ATK_AND_SP_DEF_PLUS_ONE_EFFECT => &[(SpecialAttack, 1), (SpecialDefense, 1)],
        NO_DAMAGE_ATK_AND_SPE_PLUS_ONE_EFFECT => &[(Attack, 1), (Speed, 1)],
        _ => &[],
    }
}

pub(crate) fn load_stat_changes_from_effect(mv: &mut Move, secondary_chance: i32) {
    let Some(kind) = stat_change_move_type(mv) else {
        return;
    };
    mv.stat_change_move_type = kind;
    for (slot, &(stat, stages)) in stat_changes_for_effect(mv.effect_index).iter().enumerate() {
        mv.stat_changes[slot].kind = stat;
        mv.stat_changes[slot].stages = stages;
    }
    if matches!(
        kind,
        StatChangeMoveType::DamageTarget | StatChangeMoveType::DamageUser
    ) {
        for change in &mut mv.stat_changes {
            if change.kind != StatChangeType::None {
                change.percent_chance = f64::from(secondary_chance);
                if change.percent_chance == 0.0 {
                    change.percent_chance = 100.0;
                }
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
        | NO_DAMAGE_BURN_EFFECT
        | SWAGGER_EFFECT
        | FLATTER_EFFECT
        | TEETER_DANCE_EFFECT => Some(StatusMoveType::NoDamage),
        DAMAGE_POISON_EFFECT
        | DAMAGE_BURN_EFFECT
        | DAMAGE_FREEZE_EFFECT
        | DAMAGE_PARALYZE_EFFECT
        | DAMAGE_CONFUSION_EFFECT
        | TWINEEDLE_EFFECT
        | DAMAGE_BURN_AND_THAW_USER_EFFECT
        | THUNDER_EFFECT
        | BLAZE_KICK_EFFECT
        | POISON_FANG_EFFECT
        | POISON_TAIL_EFFECT => Some(StatusMoveType::Damage),
        _ => None,
    }
}

fn status_type(effect: i32) -> Option<StatusType> {
    match effect {
        NO_DAMAGE_SLEEP_EFFECT => Some(StatusType::Sleep),
        DAMAGE_POISON_EFFECT | NO_DAMAGE_POISON_EFFECT | TWINEEDLE_EFFECT | POISON_TAIL_EFFECT => {
            Some(StatusType::Poison)
        }
        DAMAGE_BURN_EFFECT
        | DAMAGE_BURN_AND_THAW_USER_EFFECT
        | NO_DAMAGE_BURN_EFFECT
        | BLAZE_KICK_EFFECT => Some(StatusType::Burn),
        DAMAGE_FREEZE_EFFECT => Some(StatusType::Freeze),
        DAMAGE_PARALYZE_EFFECT | NO_DAMAGE_PARALYZE_EFFECT | THUNDER_EFFECT => {
            Some(StatusType::Paralyze)
        }
        TOXIC_EFFECT | POISON_FANG_EFFECT => Some(StatusType::ToxicPoison),
        NO_DAMAGE_CONFUSION_EFFECT
        | DAMAGE_CONFUSION_EFFECT
        | SWAGGER_EFFECT
        | FLATTER_EFFECT
        | TEETER_DANCE_EFFECT => Some(StatusType::Confusion),
        _ => None,
    }
}

pub(crate) fn load_status_from_effect(mv: &mut Move, secondary_chance: i32) {
    if mv.number == m::BOUNCE {
        mv.status_move_type = StatusMoveType::Damage;
        mv.status_type = StatusType::Paralyze;
        mv.status_percent_chance = f64::from(secondary_chance);
        return;
    }
    let Some(kind) = status_move_type(mv.effect_index) else {
        return;
    };
    mv.status_move_type = kind;
    if let Some(status) = status_type(mv.effect_index) {
        mv.status_type = status;
    }
    if kind == StatusMoveType::Damage {
        mv.status_percent_chance = f64::from(secondary_chance);
        if mv.status_percent_chance == 0.0 {
            mv.status_percent_chance = 100.0;
        }
    }
}

pub(crate) fn load_misc_info_from_effect(mv: &mut Move, secondary_chance: i32) {
    match mv.effect_index {
        INCREASED_CRIT_EFFECT | BLAZE_KICK_EFFECT | POISON_TAIL_EFFECT => {
            mv.critical_chance = CriticalChance::Increased;
        }
        FUTURE_SIGHT_AND_DOOM_DESIRE_EFFECT | SPIT_UP_EFFECT => {
            mv.critical_chance = CriticalChance::None;
            mv.flinch_percent_chance = f64::from(secondary_chance);
        }
        FLINCH_EFFECT
        | SNORE_EFFECT
        | TWISTER_EFFECT
        | FLINCH_WITH_MINIMIZE_BONUS_EFFECT
        | FAKE_OUT_EFFECT => mv.flinch_percent_chance = f64::from(secondary_chance),
        DAMAGE_ABSORB_EFFECT | DREAM_EATER_EFFECT => mv.absorb_percent = 50,
        DAMAGE_RECOIL_25_PERCENT_EFFECT => mv.recoil_percent = 25,
        DAMAGE_RECOIL_33_PERCENT_EFFECT => mv.recoil_percent = 33,
        BINDING_EFFECT | TRAPPING_EFFECT => mv.is_trap_move = true,
        RAZOR_WIND_EFFECT | SKULL_BASH_EFFECT | SOLARBEAM_EFFECT | SEMI_INVULNERABLE_EFFECT => {
            mv.is_charge_move = true;
        }
        RECHARGE_EFFECT => mv.is_recharge_move = true,
        SKY_ATTACK_EFFECT => {
            mv.critical_chance = CriticalChance::Increased;
            mv.flinch_percent_chance = f64::from(secondary_chance);
            mv.is_charge_move = true;
        }
        _ => {}
    }
}

fn hit_count(number: i32) -> f64 {
    if global::NORMAL_MULTIHIT_MOVES.contains(&number) {
        3.0
    } else if global::DOUBLE_HIT_MOVES.contains(&number) {
        2.0
    } else if number == m::TRIPLE_KICK {
        TRIPLE_KICK_HIT_COUNT
    } else {
        1.0
    }
}

fn initial_category(mv: &Move) -> MoveCategory {
    if mv.power == 0 && !global::NO_POWER_NON_STATUS_MOVES.contains(&mv.number) {
        MoveCategory::Status
    } else if gb::is_physical_type(mv.kind) {
        MoveCategory::Physical
    } else {
        MoveCategory::Special
    }
}

impl Gen3RomHandler {
    pub(crate) fn move_count(&self) -> i32 {
        self.value("MoveCount")
    }

    fn move_data_offset(&self, number: i32) -> i32 {
        self.value("MoveData") + number * MOVE_DATA_ENTRY_SIZE
    }

    pub(crate) fn load_moves(&mut self) {
        let names = self.value("MoveNames");
        let name_length = self.value("MoveNameLength");
        for number in 1..=self.move_count() {
            let base = self.move_data_offset(number);
            let rom = &self.rom;
            let mut mv = Move {
                name: self.read_string(names + number * name_length, name_length),
                number,
                internal_id: number,
                effect_index: rom.int(base),
                hitratio: f64::from(rom.int(base + 3)),
                power: rom.int(base + 1),
                pp: rom.int(base + 4),
                kind: type_table(rom.int(base + 2)),
                target: rom.int(base + 6),
                priority: rom.signed(base + 7),
                makes_contact: rom.int(base + 8) & MAKES_CONTACT_FLAG != 0,
                is_sound_move: SOUND_MOVES.contains(&number),
                hit_count: hit_count(number),
                ..Move::default()
            };
            mv.category = initial_category(&mv);
            if number == m::SWIFT {
                self.core.engine.perfect_accuracy = mv.hitratio as i32;
            }
            let secondary_chance = rom.int(base + 5);
            load_stat_changes_from_effect(&mut mv, secondary_chance);
            load_status_from_effect(&mut mv, secondary_chance);
            load_misc_info_from_effect(&mut mv, secondary_chance);
            self.core.data.create_move(mv);
        }
    }

    pub(crate) fn save_moves(&mut self) {
        for number in 1..=self.move_count() {
            let Some(mv) = self.core.data.move_by_number(number).cloned() else {
                continue;
            };
            let base = self.move_data_offset(number);
            let hitratio = (randomon_java::math::round_double(mv.hitratio) as i32).clamp(0, 100);
            self.rom.set_byte(base, mv.effect_index as u8);
            self.rom.set_byte(base + 1, mv.power as u8);
            self.rom.set_byte(base + 2, type_to_byte(mv.kind));
            self.rom.set_byte(base + 3, hitratio as u8);
            self.rom.set_byte(base + 4, mv.pp as u8);
        }
    }

    pub(crate) fn move_name(&self, number: i32) -> Option<String> {
        self.core.data.move_name(number).map(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn move_with(effect_index: i32, target: i32) -> Move {
        Move {
            effect_index,
            target,
            number: 1,
            ..Move::default()
        }
    }

    #[test]
    fn user_targeted_stat_moves_raise_the_user() {
        let mut swords_dance = move_with(NO_DAMAGE_ATK_PLUS_TWO_EFFECT, USER_TARGET);
        load_stat_changes_from_effect(&mut swords_dance, 0);
        assert_eq!(
            swords_dance.stat_change_move_type,
            StatChangeMoveType::NoDamageUser
        );
        assert_eq!(swords_dance.stat_changes[0].kind, StatChangeType::Attack);
        assert_eq!(swords_dance.stat_changes[0].stages, 2);
        assert_eq!(swords_dance.stat_changes[0].percent_chance, 0.0);
    }

    #[test]
    fn damaging_stat_moves_default_to_a_certain_chance() {
        let mut overheat = move_with(DAMAGE_USER_SP_ATK_MINUS_TWO_EFFECT, 0);
        load_stat_changes_from_effect(&mut overheat, 0);
        assert_eq!(
            overheat.stat_change_move_type,
            StatChangeMoveType::DamageUser
        );
        assert_eq!(overheat.stat_changes[0].percent_chance, 100.0);
        let mut bulk_up = move_with(NO_DAMAGE_ATK_AND_DEF_PLUS_ONE_EFFECT, USER_TARGET);
        load_stat_changes_from_effect(&mut bulk_up, 0);
        assert_eq!(bulk_up.stat_changes[1].kind, StatChangeType::Defense);
    }

    #[test]
    fn status_and_misc_effects_follow_the_effect_table() {
        let mut bounce = move_with(0, 0);
        bounce.number = m::BOUNCE;
        load_status_from_effect(&mut bounce, 30);
        assert_eq!(bounce.status_type, StatusType::Paralyze);
        assert_eq!(bounce.status_percent_chance, 30.0);
        let mut flamethrower = move_with(DAMAGE_BURN_EFFECT, 0);
        load_status_from_effect(&mut flamethrower, 0);
        assert_eq!(flamethrower.status_percent_chance, 100.0);
        let mut doom_desire = move_with(FUTURE_SIGHT_AND_DOOM_DESIRE_EFFECT, 0);
        load_misc_info_from_effect(&mut doom_desire, 10);
        assert_eq!(doom_desire.critical_chance, CriticalChance::None);
        assert_eq!(doom_desire.flinch_percent_chance, 10.0);
        let mut sky_attack = move_with(SKY_ATTACK_EFFECT, 0);
        load_misc_info_from_effect(&mut sky_attack, 30);
        assert!(sky_attack.is_charge_move);
        assert_eq!(sky_attack.critical_chance, CriticalChance::Increased);
    }

    #[test]
    fn category_uses_the_type_split_and_zero_power_rule() {
        let mut mv = Move {
            power: 40,
            kind: type_table(0x0A),
            ..Move::default()
        };
        assert_eq!(initial_category(&mv), MoveCategory::Special);
        mv.kind = type_table(0x00);
        assert_eq!(initial_category(&mv), MoveCategory::Physical);
        mv.power = 0;
        mv.number = m::GROWL;
        assert_eq!(initial_category(&mv), MoveCategory::Status);
        assert_eq!(hit_count(m::TRIPLE_KICK), TRIPLE_KICK_HIT_COUNT);
    }
}
