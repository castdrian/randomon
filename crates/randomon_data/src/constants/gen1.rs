use crate::constants::{gen1_items, moves};
use crate::items::ItemList;
use crate::trainer::Trainer;
use crate::types::Type;

pub const BASE_STATS_ENTRY_SIZE: i32 = 0x1C;
pub const BS_HP_OFFSET: i32 = 1;
pub const BS_ATTACK_OFFSET: i32 = 2;
pub const BS_DEFENSE_OFFSET: i32 = 3;
pub const BS_SPEED_OFFSET: i32 = 4;
pub const BS_SPECIAL_OFFSET: i32 = 5;
pub const BS_PRIMARY_TYPE_OFFSET: i32 = 6;
pub const BS_SECONDARY_TYPE_OFFSET: i32 = 7;
pub const BS_CATCH_RATE_OFFSET: i32 = 8;
pub const BS_EXP_YIELD_OFFSET: i32 = 9;
pub const BS_FRONT_SPRITE_OFFSET: i32 = 11;
pub const BS_LEVEL1_MOVES_OFFSET: i32 = 15;
pub const BS_GROWTH_CURVE_OFFSET: i32 = 19;
pub const BS_TMHM_COMPAT_OFFSET: i32 = 20;

pub const ENCOUNTER_TABLE_END: i32 = 0xFFFF;
pub const ENCOUNTER_TABLE_SIZE: i32 = 10;
pub const YELLOW_SUPER_ROD_TABLE_SIZE: i32 = 4;
pub const TRAINER_CLASS_COUNT: i32 = 47;
pub const CHAMP_RIVAL_OFFSET_FROM_GYM_LEADER_MOVES: i32 = 0x44;
pub const TM_COUNT: i32 = 50;
pub const HM_COUNT: i32 = 5;
pub const GYM_LEADER_TMS: [i32; 8] = [34, 11, 24, 21, 6, 46, 38, 27];
pub const TCLASSES_COUNTS: [i32; 2] = [21, 47];
pub const SINGULAR_TRAINERS: &[i32] = &[28, 32, 33, 34, 35, 36, 37, 38, 39, 43, 45, 46];

pub const BANNED_MOVES_WITH_X_ACC_BANNED: &[i32] =
    &[moves::SONIC_BOOM, moves::DRAGON_RAGE, moves::SPORE];
pub const BANNED_MOVES_WITHOUT_X_ACC_BANNED: &[i32] = &[
    moves::SONIC_BOOM,
    moves::DRAGON_RAGE,
    moves::SPORE,
    moves::HORN_DRILL,
    moves::FISSURE,
    moves::GUILLOTINE,
];
pub const BANNED_LEVELUP_MOVES: &[i32] = &[moves::TRANSFORM];
pub const FIELD_MOVES: &[i32] = &[
    moves::CUT,
    moves::FLY,
    moves::SURF,
    moves::STRENGTH,
    moves::FLASH,
    moves::DIG,
    moves::TELEPORT,
];

pub const DAMAGE_POISON_20_PERCENT_EFFECT: i32 = 2;
pub const DAMAGE_ABSORB_EFFECT: i32 = 3;
pub const DAMAGE_BURN_10_PERCENT_EFFECT: i32 = 4;
pub const DAMAGE_FREEZE_10_PERCENT_EFFECT: i32 = 5;
pub const DAMAGE_PARALYZE_10_PERCENT_EFFECT: i32 = 6;
pub const DREAM_EATER_EFFECT: i32 = 8;
pub const NO_DAMAGE_ATK_PLUS_ONE_EFFECT: i32 = 10;
pub const NO_DAMAGE_DEF_PLUS_ONE_EFFECT: i32 = 11;
pub const NO_DAMAGE_SPECIAL_PLUS_ONE_EFFECT: i32 = 13;
pub const NO_DAMAGE_EVASION_PLUS_ONE_EFFECT: i32 = 15;
pub const NO_DAMAGE_ATK_MINUS_ONE_EFFECT: i32 = 18;
pub const NO_DAMAGE_DEF_MINUS_ONE_EFFECT: i32 = 19;
pub const NO_DAMAGE_SPE_MINUS_ONE_EFFECT: i32 = 20;
pub const NO_DAMAGE_ACCURACY_MINUS_ONE_EFFECT: i32 = 22;
pub const FLINCH_10_PERCENT_EFFECT: i32 = 31;
pub const NO_DAMAGE_SLEEP_EFFECT: i32 = 32;
pub const DAMAGE_POISON_40_PERCENT_EFFECT: i32 = 33;
pub const DAMAGE_BURN_30_PERCENT_EFFECT: i32 = 34;
pub const DAMAGE_FREEZE_30_PERCENT_EFFECT: i32 = 35;
pub const DAMAGE_PARALYZE_30_PERCENT_EFFECT: i32 = 36;
pub const FLINCH_30_PERCENT_EFFECT: i32 = 37;
pub const CHARGE_EFFECT: i32 = 39;
pub const FLY_EFFECT: i32 = 43;
pub const DAMAGE_RECOIL_EFFECT: i32 = 48;
pub const NO_DAMAGE_CONFUSION_EFFECT: i32 = 49;
pub const NO_DAMAGE_ATK_PLUS_TWO_EFFECT: i32 = 50;
pub const NO_DAMAGE_DEF_PLUS_TWO_EFFECT: i32 = 51;
pub const NO_DAMAGE_SPE_PLUS_TWO_EFFECT: i32 = 52;
pub const NO_DAMAGE_SPECIAL_PLUS_TWO_EFFECT: i32 = 53;
pub const NO_DAMAGE_DEF_MINUS_TWO_EFFECT: i32 = 59;
pub const NO_DAMAGE_POISON_EFFECT: i32 = 66;
pub const NO_DAMAGE_PARALYZE_EFFECT: i32 = 67;
pub const DAMAGE_ATK_MINUS_ONE_EFFECT: i32 = 68;
pub const DAMAGE_DEF_MINUS_ONE_EFFECT: i32 = 69;
pub const DAMAGE_SPE_MINUS_ONE_EFFECT: i32 = 70;
pub const DAMAGE_SPECIAL_MINUS_ONE_EFFECT: i32 = 71;
pub const DAMAGE_CONFUSION_EFFECT: i32 = 76;
pub const TWINEEDLE_EFFECT: i32 = 77;
pub const HYPER_BEAM_EFFECT: i32 = 80;

pub const INCREASED_CRIT_MOVES: &[i32] = &[
    moves::KARATE_CHOP,
    moves::RAZOR_LEAF,
    moves::CRABHAMMER,
    moves::SLASH,
];
pub const EARLY_REQUIRED_HMS: &[i32] = &[moves::CUT];
pub const HMS_START_INDEX: i32 = gen1_items::HM01;
pub const TMS_START_INDEX: i32 = gen1_items::TM01;
pub const REQUIRED_FIELD_TMS: &[i32] = &[
    3, 4, 8, 10, 12, 14, 16, 19, 20, 22, 25, 26, 30, 40, 43, 44, 45, 47,
];
pub const TOWER_MAPS_START_INDEX: i32 = 0x90;
pub const TOWER_MAPS_END_INDEX: i32 = 0x94;
pub const GUARANTEED_CATCH_PREFIX: &str = "CF7EFE01";

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
        0x14 => Some(Type::Fire),
        0x15 => Some(Type::Water),
        0x16 => Some(Type::Grass),
        0x17 => Some(Type::Electric),
        0x18 => Some(Type::Psychic),
        0x19 => Some(Type::Ice),
        0x1A => Some(Type::Dragon),
        _ => None,
    }
}

pub fn type_to_byte(kind: Option<Type>) -> u8 {
    (0..0x20)
        .find(|&value| type_table(value) == kind)
        .map_or(0, |value| value as u8)
}

pub fn allowed_items() -> ItemList {
    let mut allowed = ItemList::new(gen1_items::TM50 as usize);
    allowed.ban_singles(&[
        gen1_items::TOWN_MAP,
        gen1_items::BICYCLE,
        gen1_items::QUESTION_MARK7,
        gen1_items::SAFARI_BALL,
        gen1_items::POKEDEX,
        gen1_items::OLD_AMBER,
        gen1_items::CARD_KEY,
        gen1_items::PP_UP_GLITCH,
        gen1_items::COIN,
        gen1_items::SS_TICKET,
        gen1_items::GOLD_TEETH,
    ]);
    allowed.ban_range(gen1_items::BOULDER_BADGE, 8);
    allowed.ban_range(gen1_items::DOME_FOSSIL, 5);
    allowed.ban_range(gen1_items::COIN_CASE, 10);
    allowed.ban_range(gen1_items::UNUSED84, 112);
    allowed.ban_range(HMS_START_INDEX, HM_COUNT);
    allowed.tm_range(TMS_START_INDEX, TM_COUNT);
    allowed
}

fn tag_by_class(trainers: &mut [Trainer], class_number: i32, number: i32, tag: &str) {
    let mut current = -1;
    for trainer in trainers.iter_mut() {
        if trainer.trainerclass == class_number {
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

pub fn tag_trainers_universal(trainers: &mut [Trainer]) {
    apply_tags(
        trainers,
        &[
            (34, 0, "GYM1"),
            (35, 0, "GYM2"),
            (36, 0, "GYM3"),
            (37, 0, "GYM4"),
            (38, 0, "GYM5"),
            (40, 0, "GYM6"),
            (39, 0, "GYM7"),
            (29, 2, "GYM8"),
            (29, 0, "GIO1"),
            (29, 1, "GIO2"),
            (44, 0, "ELITE1"),
            (33, 0, "ELITE2"),
            (46, 0, "ELITE3"),
            (47, 0, "ELITE4"),
        ],
    );
}

const GYM_TRAINER_TAGS_RB: &[(i32, i32, &str)] = &[
    (5, 0, "GYM1"),
    (15, 0, "GYM2"),
    (6, 0, "GYM2"),
    (4, 7, "GYM3"),
    (20, 0, "GYM3"),
    (41, 2, "GYM3"),
    (3, 16, "GYM4"),
    (3, 17, "GYM4"),
    (6, 10, "GYM4"),
    (18, 0, "GYM4"),
    (18, 1, "GYM4"),
    (18, 2, "GYM4"),
    (32, 0, "GYM4"),
    (21, 2, "GYM5"),
    (21, 3, "GYM5"),
    (21, 6, "GYM5"),
    (21, 7, "GYM5"),
    (22, 0, "GYM5"),
    (22, 1, "GYM5"),
    (19, 0, "GYM6"),
    (19, 1, "GYM6"),
    (19, 2, "GYM6"),
    (19, 3, "GYM6"),
    (45, 21, "GYM6"),
    (45, 22, "GYM6"),
    (45, 23, "GYM6"),
    (8, 8, "GYM7"),
    (8, 9, "GYM7"),
    (8, 10, "GYM7"),
    (8, 11, "GYM7"),
    (11, 3, "GYM7"),
    (11, 4, "GYM7"),
    (11, 5, "GYM7"),
    (22, 2, "GYM8"),
    (22, 3, "GYM8"),
    (24, 5, "GYM8"),
    (24, 6, "GYM8"),
    (24, 7, "GYM8"),
    (31, 0, "GYM8"),
    (31, 8, "GYM8"),
    (31, 9, "GYM8"),
];

pub fn tag_trainers_red_blue(trainers: &mut [Trainer]) {
    apply_tags(
        trainers,
        &[
            (25, 0, "RIVAL1-0"),
            (25, 1, "RIVAL1-1"),
            (25, 2, "RIVAL1-2"),
            (25, 3, "RIVAL2-0"),
            (25, 4, "RIVAL2-1"),
            (25, 5, "RIVAL2-2"),
            (25, 6, "RIVAL3-0"),
            (25, 7, "RIVAL3-1"),
            (25, 8, "RIVAL3-2"),
            (42, 0, "RIVAL4-0"),
            (42, 1, "RIVAL4-1"),
            (42, 2, "RIVAL4-2"),
            (42, 3, "RIVAL5-0"),
            (42, 4, "RIVAL5-1"),
            (42, 5, "RIVAL5-2"),
            (42, 6, "RIVAL6-0"),
            (42, 7, "RIVAL6-1"),
            (42, 8, "RIVAL6-2"),
            (42, 9, "RIVAL7-0"),
            (42, 10, "RIVAL7-1"),
            (42, 11, "RIVAL7-2"),
            (43, 0, "RIVAL8-0"),
            (43, 1, "RIVAL8-1"),
            (43, 2, "RIVAL8-2"),
        ],
    );
    apply_tags(trainers, GYM_TRAINER_TAGS_RB);
}

pub fn tag_trainers_yellow(trainers: &mut [Trainer]) {
    apply_tags(
        trainers,
        &[
            (25, 0, "IRIVAL"),
            (25, 1, "RIVAL1-0"),
            (25, 2, "RIVAL2-0"),
            (42, 0, "RIVAL3-0"),
            (42, 1, "RIVAL4-0"),
            (42, 2, "RIVAL4-1"),
            (42, 3, "RIVAL4-2"),
            (42, 4, "RIVAL5-0"),
            (42, 5, "RIVAL5-1"),
            (42, 6, "RIVAL5-2"),
            (42, 7, "RIVAL6-0"),
            (42, 8, "RIVAL6-1"),
            (42, 9, "RIVAL6-2"),
            (43, 0, "RIVAL7-0"),
            (43, 1, "RIVAL7-1"),
            (43, 2, "RIVAL7-2"),
            (30, 41, "THEMED:JESSIE&JAMES"),
            (30, 42, "THEMED:JESSIE&JAMES"),
            (30, 43, "THEMED:JESSIE&JAMES"),
            (30, 44, "THEMED:JESSIE&JAMES"),
            (5, 0, "GYM1"),
            (6, 0, "GYM2"),
            (15, 0, "GYM2"),
        ],
    );
    apply_tags(trainers, &GYM_TRAINER_TAGS_RB[3..]);
}
