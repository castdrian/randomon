use randomon_data::constants::moves;
use randomon_data::{ItemList, Trainer, Type};

use crate::items;

pub const VIET_CRYSTAL_CHECK_OFFSET: usize = 0x63;
pub const VIET_CRYSTAL_CHECK_VALUE: u8 = 0xF5;
pub const VIET_CRYSTAL_ROM_NAME: &str = "Pokemon VietCrystal";

pub const POKEMON_COUNT: i32 = 251;
pub const MOVE_COUNT: i32 = 251;
pub const BASE_STATS_ENTRY_SIZE: i32 = 0x20;

pub const BS_HP_OFFSET: i32 = 1;
pub const BS_ATTACK_OFFSET: i32 = 2;
pub const BS_DEFENSE_OFFSET: i32 = 3;
pub const BS_SPEED_OFFSET: i32 = 4;
pub const BS_SP_ATK_OFFSET: i32 = 5;
pub const BS_SP_DEF_OFFSET: i32 = 6;
pub const BS_PRIMARY_TYPE_OFFSET: i32 = 7;
pub const BS_SECONDARY_TYPE_OFFSET: i32 = 8;
pub const BS_CATCH_RATE_OFFSET: i32 = 9;
pub const BS_COMMON_HELD_ITEM_OFFSET: i32 = 11;
pub const BS_RARE_HELD_ITEM_OFFSET: i32 = 12;
pub const BS_PIC_DIMENSIONS_OFFSET: i32 = 17;
pub const BS_GROWTH_CURVE_OFFSET: i32 = 22;
pub const BS_TMHM_COMPAT_OFFSET: i32 = 24;
pub const BS_MT_COMPAT_OFFSET: i32 = 31;

pub const FISHING_GROUP_COUNT: i32 = 12;
pub const POKES_PER_FISHING_GROUP: i32 = 11;
pub const FISHING_GROUP_ENTRY_LENGTH: i32 = 3;
pub const TIME_SPECIFIC_FISHING_GROUP_COUNT: i32 = 11;
pub const POKES_PER_TS_FISHING_GROUP: i32 = 4;
pub const LAND_ENCOUNTER_SLOTS: i32 = 7;
pub const SEA_ENCOUNTER_SLOTS: i32 = 3;
pub const ODD_EGG_POKEMON_COUNT: i32 = 14;

pub const TM_COUNT: i32 = 50;
pub const HM_COUNT: i32 = 7;
pub const MT_MENU_CANCEL_STRING: &str = "CANCEL";
pub const MT_MENU_INIT_BYTE: u8 = 0x80;
pub const MAX_TRAINER_NAME_LENGTH: i32 = 17;
pub const FLEEING_SET_TWO_OFFSET: i32 = 0xE;
pub const FLEEING_SET_THREE_OFFSET: i32 = 0x17;
pub const MAP_GROUP_COUNT: i32 = 26;
pub const MAPS_IN_LAST_GROUP: i32 = 11;

pub const NO_DAMAGE_SLEEP_EFFECT: i32 = 1;
pub const DAMAGE_POISON_EFFECT: i32 = 2;
pub const DAMAGE_ABSORB_EFFECT: i32 = 3;
pub const DAMAGE_BURN_EFFECT: i32 = 4;
pub const DAMAGE_FREEZE_EFFECT: i32 = 5;
pub const DAMAGE_PARALYZE_EFFECT: i32 = 6;
pub const DREAM_EATER_EFFECT: i32 = 8;
pub const NO_DAMAGE_ATK_PLUS_ONE_EFFECT: i32 = 10;
pub const NO_DAMAGE_DEF_PLUS_ONE_EFFECT: i32 = 11;
pub const NO_DAMAGE_SP_ATK_PLUS_ONE_EFFECT: i32 = 13;
pub const NO_DAMAGE_EVASION_PLUS_ONE_EFFECT: i32 = 16;
pub const NO_DAMAGE_ATK_MINUS_ONE_EFFECT: i32 = 18;
pub const NO_DAMAGE_DEF_MINUS_ONE_EFFECT: i32 = 19;
pub const NO_DAMAGE_SPE_MINUS_ONE_EFFECT: i32 = 20;
pub const NO_DAMAGE_ACCURACY_MINUS_ONE_EFFECT: i32 = 23;
pub const NO_DAMAGE_EVASION_MINUS_ONE_EFFECT: i32 = 24;
pub const FLINCH_EFFECT: i32 = 31;
pub const TOXIC_EFFECT: i32 = 33;
pub const RAZOR_WIND_EFFECT: i32 = 39;
pub const BINDING_EFFECT: i32 = 42;
pub const DAMAGE_RECOIL_EFFECT: i32 = 48;
pub const NO_DAMAGE_CONFUSION_EFFECT: i32 = 49;
pub const NO_DAMAGE_ATK_PLUS_TWO_EFFECT: i32 = 50;
pub const NO_DAMAGE_DEF_PLUS_TWO_EFFECT: i32 = 51;
pub const NO_DAMAGE_SPE_PLUS_TWO_EFFECT: i32 = 52;
pub const NO_DAMAGE_SP_DEF_PLUS_TWO_EFFECT: i32 = 54;
pub const NO_DAMAGE_ATK_MINUS_TWO_EFFECT: i32 = 58;
pub const NO_DAMAGE_DEF_MINUS_TWO_EFFECT: i32 = 59;
pub const NO_DAMAGE_SPE_MINUS_TWO_EFFECT: i32 = 60;
pub const NO_DAMAGE_SP_DEF_MINUS_TWO_EFFECT: i32 = 62;
pub const NO_DAMAGE_POISON_EFFECT: i32 = 66;
pub const NO_DAMAGE_PARALYZE_EFFECT: i32 = 67;
pub const DAMAGE_ATK_MINUS_ONE_EFFECT: i32 = 68;
pub const DAMAGE_DEF_MINUS_ONE_EFFECT: i32 = 69;
pub const DAMAGE_SPE_MINUS_ONE_EFFECT: i32 = 70;
pub const DAMAGE_SP_DEF_MINUS_ONE_EFFECT: i32 = 72;
pub const DAMAGE_ACCURACY_MINUS_ONE_EFFECT: i32 = 73;
pub const SKY_ATTACK_EFFECT: i32 = 75;
pub const DAMAGE_CONFUSION_EFFECT: i32 = 76;
pub const TWINEEDLE_EFFECT: i32 = 77;
pub const HYPER_BEAM_EFFECT: i32 = 80;
pub const SNORE_EFFECT: i32 = 92;
pub const FLAIL_AND_REVERSAL_EFFECT: i32 = 102;
pub const TRAPPING_EFFECT: i32 = 106;
pub const SWAGGER_EFFECT: i32 = 118;
pub const DAMAGE_BURN_AND_THAW_USER_EFFECT: i32 = 125;
pub const DAMAGE_USER_DEF_PLUS_ONE_EFFECT: i32 = 138;
pub const DAMAGE_USER_ATK_PLUS_ONE_EFFECT: i32 = 139;
pub const DAMAGE_USER_ALL_PLUS_ONE_EFFECT: i32 = 140;
pub const SKULL_BASH_EFFECT: i32 = 145;
pub const TWISTER_EFFECT: i32 = 146;
pub const FUTURE_SIGHT_EFFECT: i32 = 148;
pub const STOMP_EFFECT: i32 = 150;
pub const SOLARBEAM_EFFECT: i32 = 151;
pub const THUNDER_EFFECT: i32 = 152;
pub const SEMI_INVULNERABLE_EFFECT: i32 = 155;
pub const DEFENSE_CURL_EFFECT: i32 = 156;

pub const PRIORITY_HIT_EFFECT_INDEX: i32 = 0x67;
pub const PROTECT_EFFECT_INDEX: i32 = 0x6F;
pub const ENDURE_EFFECT_INDEX: i32 = 0x74;
pub const FORCE_SWITCH_EFFECT_INDEX: i32 = 0x1C;
pub const COUNTER_EFFECT_INDEX: i32 = 0x59;
pub const MIRROR_COAT_EFFECT_INDEX: i32 = 0x90;

pub const INCREASED_CRIT_MOVES: &[i32] = &[
    moves::KARATE_CHOP,
    moves::RAZOR_WIND,
    moves::RAZOR_LEAF,
    moves::CRABHAMMER,
    moves::SLASH,
    moves::AEROBLAST,
    moves::CROSS_CHOP,
];

pub const REQUIRED_FIELD_TMS: &[i32] = &[4, 20, 22, 26, 28, 34, 35, 39, 40, 43, 44, 46];

pub const FIELD_MOVES: &[i32] = &[
    moves::CUT,
    moves::FLY,
    moves::SURF,
    moves::STRENGTH,
    moves::FLASH,
    moves::DIG,
    moves::TELEPORT,
    moves::WHIRLPOOL,
    moves::WATERFALL,
    moves::ROCK_SMASH,
    moves::HEADBUTT,
    moves::SWEET_SCENT,
];

pub const EARLY_REQUIRED_HM_MOVES: &[i32] = &[moves::CUT];
pub const BANNED_LEVELUP_MOVES: &[i32] = &[moves::TRANSFORM, moves::THIEF];
pub const BROKEN_MOVES: &[i32] = &[
    moves::SONIC_BOOM,
    moves::DRAGON_RAGE,
    moves::HORN_DRILL,
    moves::FISSURE,
    moves::GUILLOTINE,
];
pub const ILLEGAL_VIET_CRYSTAL_MOVES: &[i32] = &[
    moves::PROTECT,
    moves::REST,
    moves::SPIKE_CANNON,
    moves::DETECT,
];

pub const TM_BLOCK_ONE_INDEX: i32 = items::TM01;
pub const TM_BLOCK_ONE_SIZE: i32 = 4;
pub const TM_BLOCK_TWO_INDEX: i32 = items::TM05;
pub const TM_BLOCK_TWO_SIZE: i32 = 24;
pub const TM_BLOCK_THREE_INDEX: i32 = items::TM29;
pub const TM_BLOCK_THREE_SIZE: i32 = 22;

pub const FRIENDSHIP_VALUE_FOR_EVO_LOCATOR: &str = "FEDCDA";

pub fn type_table(value: i32) -> Option<Type> {
    match value {
        0x00 => Some(Type::Normal),
        0x01 => Some(Type::Fighting),
        0x02 => Some(Type::Flying),
        0x03 => Some(Type::Poison),
        0x04 => Some(Type::Ground),
        0x05 => Some(Type::Rock),
        0x07 => Some(Type::Bug),
        0x08 => Some(Type::Ghost),
        0x09 => Some(Type::Steel),
        0x14 => Some(Type::Fire),
        0x15 => Some(Type::Water),
        0x16 => Some(Type::Grass),
        0x17 => Some(Type::Electric),
        0x18 => Some(Type::Psychic),
        0x19 => Some(Type::Ice),
        0x1A => Some(Type::Dragon),
        0x1B => Some(Type::Dark),
        _ => None,
    }
}

pub fn type_to_byte(kind: Option<Type>) -> u8 {
    let Some(kind) = kind else {
        return 0x13;
    };
    (0..0x20)
        .find(|&value| type_table(value) == Some(kind))
        .map_or(0, |value| value as u8)
}

pub fn allowed_items() -> ItemList {
    let mut allowed = ItemList::new(items::HM07 as usize);
    allowed.ban_singles(&[
        items::BICYCLE,
        items::COIN_CASE,
        items::ITEMFINDER,
        items::OLD_ROD,
        items::GOOD_ROD,
        items::SUPER_ROD,
        items::GS_BALL,
        items::BLUE_CARD,
        items::BASEMENT_KEY,
        items::PASS,
        items::SQUIRT_BOTTLE,
        items::RAINBOW_WING,
    ]);
    allowed.ban_range(items::RED_SCALE, 6);
    allowed.ban_range(items::CARD_KEY, 4);
    allowed.ban_range(items::HM01, 7);
    allowed.ban_singles(&[
        items::TERUSAMA6,
        items::TERUSAMA25,
        items::TERUSAMA45,
        items::TERUSAMA50,
        items::TERUSAMA56,
        items::TERUSAMA90,
        items::TERUSAMA100,
        items::TERUSAMA120,
        items::TERUSAMA135,
        items::TERUSAMA136,
        items::TERUSAMA137,
        items::TERUSAMA141,
        items::TERUSAMA142,
        items::TERUSAMA145,
        items::TERUSAMA147,
        items::TERUSAMA148,
        items::TERUSAMA149,
        items::TERUSAMA153,
        items::TERUSAMA154,
        items::TERUSAMA155,
        items::TERUSAMA162,
        items::TERUSAMA171,
        items::TERUSAMA176,
        items::TERUSAMA179,
        items::TERUSAMA190,
        items::TM04_UNUSED,
        items::TM28_UNUSED,
    ]);
    allowed.tm_range(TM_BLOCK_ONE_INDEX, TM_BLOCK_ONE_SIZE);
    allowed.tm_range(TM_BLOCK_TWO_INDEX, TM_BLOCK_TWO_SIZE);
    allowed.tm_range(TM_BLOCK_THREE_INDEX, TM_BLOCK_THREE_SIZE);
    allowed
}

pub fn non_bad_items() -> ItemList {
    let mut non_bad = allowed_items();
    non_bad.ban_singles(&[
        items::LUCKY_PUNCH,
        items::METAL_POWDER,
        items::SILVER_LEAF,
        items::GOLD_LEAF,
        items::RED_APRICORN,
        items::BLU_APRICORN,
        items::WHT_APRICORN,
        items::BLK_APRICORN,
        items::PNK_APRICORN,
        items::STICK,
        items::THICK_CLUB,
        items::FLOWER_MAIL,
        items::LIGHT_BALL,
        items::BERRY,
        items::BRICK_PIECE,
    ]);
    non_bad.ban_range(items::YLW_APRICORN, 2);
    non_bad.ban_range(items::NORMAL_BOX, 2);
    non_bad.ban_range(items::SURF_MAIL, 9);
    non_bad
}

pub fn tm_number_for_item(item: i32) -> i32 {
    if (TM_BLOCK_ONE_INDEX..TM_BLOCK_ONE_INDEX + TM_BLOCK_ONE_SIZE).contains(&item) {
        item - TM_BLOCK_ONE_INDEX + 1
    } else if (TM_BLOCK_TWO_INDEX..TM_BLOCK_TWO_INDEX + TM_BLOCK_TWO_SIZE).contains(&item) {
        item - TM_BLOCK_TWO_INDEX + 1 + TM_BLOCK_ONE_SIZE
    } else {
        item - TM_BLOCK_THREE_INDEX + 1 + TM_BLOCK_ONE_SIZE + TM_BLOCK_TWO_SIZE
    }
}

pub fn item_for_tm_number(tm: i32) -> i32 {
    if (1..=TM_BLOCK_ONE_SIZE).contains(&tm) {
        tm + TM_BLOCK_ONE_INDEX - 1
    } else if (TM_BLOCK_ONE_SIZE + 1..=TM_BLOCK_ONE_SIZE + TM_BLOCK_TWO_SIZE).contains(&tm) {
        tm + TM_BLOCK_TWO_INDEX - 1 - TM_BLOCK_ONE_SIZE
    } else {
        tm + TM_BLOCK_THREE_INDEX - 1 - TM_BLOCK_ONE_SIZE - TM_BLOCK_TWO_SIZE
    }
}

fn tag_by_class(trainers: &mut [Trainer], class_number: i32, number: i32, tag: &str) {
    let mut current = -1;
    for trainer in trainers.iter_mut() {
        if trainer.trainerclass == class_number - 1 {
            current += 1;
            if current == number {
                trainer.tag = Some(tag.to_owned());
                return;
            }
        }
    }
}

fn apply_tags(trainers: &mut [Trainer], tags: &[(i32, i32, &str)]) {
    for &(class_number, number, tag) in tags {
        tag_by_class(trainers, class_number, number, tag);
    }
}

const UNIVERSAL_TAGS: &[(i32, i32, &str)] = &[
    (1, 0, "GYM1"),
    (3, 0, "GYM2"),
    (2, 0, "GYM3"),
    (4, 0, "GYM4"),
    (7, 0, "GYM5"),
    (6, 0, "GYM6"),
    (5, 0, "GYM7"),
    (8, 0, "GYM8"),
    (17, 0, "GYM9"),
    (18, 0, "GYM10"),
    (19, 0, "GYM11"),
    (21, 0, "GYM12"),
    (26, 0, "GYM13"),
    (35, 0, "GYM14"),
    (46, 0, "GYM15"),
    (64, 0, "GYM16"),
    (11, 0, "ELITE1"),
    (15, 0, "ELITE2"),
    (13, 0, "ELITE3"),
    (14, 0, "ELITE4"),
    (16, 0, "CHAMPION"),
    (63, 0, "UBER"),
    (9, 0, "RIVAL1-1"),
    (9, 1, "RIVAL1-2"),
    (9, 2, "RIVAL1-0"),
    (9, 3, "RIVAL2-1"),
    (9, 4, "RIVAL2-2"),
    (9, 5, "RIVAL2-0"),
    (9, 6, "RIVAL3-1"),
    (9, 7, "RIVAL3-2"),
    (9, 8, "RIVAL3-0"),
    (9, 9, "RIVAL4-1"),
    (9, 10, "RIVAL4-2"),
    (9, 11, "RIVAL4-0"),
    (9, 12, "RIVAL5-1"),
    (9, 13, "RIVAL5-2"),
    (9, 14, "RIVAL5-0"),
    (42, 0, "RIVAL6-1"),
    (42, 1, "RIVAL6-2"),
    (42, 2, "RIVAL6-0"),
    (42, 3, "RIVAL7-1"),
    (42, 4, "RIVAL7-2"),
    (42, 5, "RIVAL7-0"),
    (55, 0, "THEMED:ARIANA"),
    (55, 1, "THEMED:ARIANA"),
    (51, 2, "THEMED:PETREL"),
    (51, 3, "THEMED:PETREL"),
    (51, 1, "THEMED:PROTON"),
    (31, 0, "THEMED:PROTON"),
    (56, 0, "THEMED:SPROUTTOWER"),
    (56, 1, "THEMED:SPROUTTOWER"),
    (56, 2, "THEMED:SPROUTTOWER"),
    (56, 3, "THEMED:SPROUTTOWER"),
    (56, 6, "THEMED:SPROUTTOWER"),
    (56, 7, "THEMED:SPROUTTOWER"),
    (56, 8, "THEMED:SPROUTTOWER"),
];

const GYM_TRAINER_TAGS: &[(i32, i32, &str)] = &[
    (24, 0, "GYM1"),
    (24, 1, "GYM1"),
    (36, 4, "GYM2"),
    (36, 5, "GYM2"),
    (36, 6, "GYM2"),
    (61, 0, "GYM2"),
    (61, 3, "GYM2"),
    (25, 0, "GYM3"),
    (25, 1, "GYM3"),
    (29, 0, "GYM3"),
    (29, 1, "GYM3"),
    (56, 4, "GYM4"),
    (56, 5, "GYM4"),
    (57, 0, "GYM4"),
    (57, 1, "GYM4"),
    (50, 1, "GYM5"),
    (50, 3, "GYM5"),
    (50, 4, "GYM5"),
    (50, 6, "GYM5"),
    (58, 0, "GYM7"),
    (58, 1, "GYM7"),
    (58, 2, "GYM7"),
    (33, 0, "GYM7"),
    (33, 1, "GYM7"),
    (27, 2, "GYM8"),
    (27, 4, "GYM8"),
    (27, 3, "GYM8"),
    (28, 2, "GYM8"),
    (28, 3, "GYM8"),
    (54, 17, "GYM9"),
    (38, 20, "GYM10"),
    (39, 17, "GYM10"),
    (39, 18, "GYM10"),
    (49, 2, "GYM11"),
    (43, 1, "GYM11"),
    (32, 2, "GYM11"),
    (61, 4, "GYM12"),
    (61, 5, "GYM12"),
    (25, 8, "GYM12"),
    (53, 18, "GYM12"),
    (29, 13, "GYM12"),
    (25, 2, "GYM13"),
    (25, 5, "GYM13"),
    (53, 4, "GYM13"),
    (54, 4, "GYM13"),
    (57, 5, "GYM14"),
    (57, 6, "GYM14"),
    (52, 1, "GYM14"),
    (52, 10, "GYM14"),
];

pub fn tag_trainers(trainers: &mut [Trainer]) {
    apply_tags(trainers, UNIVERSAL_TAGS);
    apply_tags(trainers, GYM_TRAINER_TAGS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_bytes_round_trip() {
        for value in 0..0x20 {
            if let Some(kind) = type_table(value) {
                assert_eq!(i32::from(type_to_byte(Some(kind))), value);
            }
        }
        assert_eq!(type_to_byte(None), 0x13);
    }

    #[test]
    fn tm_items_map_to_tm_numbers_and_back() {
        for tm in 1..=TM_COUNT {
            assert_eq!(tm_number_for_item(item_for_tm_number(tm)), tm);
        }
        assert_eq!(item_for_tm_number(5), items::TM05);
        assert_eq!(item_for_tm_number(29), items::TM29);
    }

    #[test]
    fn allowed_items_ban_key_items_and_mark_tms() {
        let allowed = allowed_items();
        assert!(!allowed.is_allowed(items::BICYCLE));
        assert!(!allowed.is_allowed(items::HM01));
        assert!(!allowed.is_allowed(items::TM04_UNUSED));
        assert!(allowed.is_tm(items::TM01));
        assert!(!allowed.is_tm(items::TM04_UNUSED));
        assert!(allowed.is_allowed(items::LUCKY_PUNCH));
        assert!(!non_bad_items().is_allowed(items::LUCKY_PUNCH));
    }
}
