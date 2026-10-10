use std::collections::BTreeMap;

use crate::constants::{abilities, gen4, items, moves, species};
use crate::items::ItemList;
use crate::trainer::{MultiBattleStatus, Trainer};
use crate::types::{MoveCategory, Type};

use species::gen5_formes;

pub const TYPE_BW: i32 = 0;
pub const TYPE_BW2: i32 = 1;

pub const ARM9_OFFSET: i32 = 0x02004000;

pub const POKEMON_COUNT: i32 = 649;
pub const MOVE_COUNT: i32 = 559;
pub const BW1_FORME_COUNT: i32 = 18;
pub const BW2_FORME_COUNT: i32 = 24;
pub const BW1_FORME_OFFSET: i32 = 0;
pub const BW2_FORME_OFFSET: i32 = 35;

pub const BW1_NON_POKEMON_BATTLE_SPRITE_COUNT: i32 = 3;
pub const BW2_NON_POKEMON_BATTLE_SPRITE_COUNT: i32 = 36;

pub const BS_HP_OFFSET: i32 = 0;
pub const BS_ATTACK_OFFSET: i32 = 1;
pub const BS_DEFENSE_OFFSET: i32 = 2;
pub const BS_SPEED_OFFSET: i32 = 3;
pub const BS_SP_ATK_OFFSET: i32 = 4;
pub const BS_SP_DEF_OFFSET: i32 = 5;
pub const BS_PRIMARY_TYPE_OFFSET: i32 = 6;
pub const BS_SECONDARY_TYPE_OFFSET: i32 = 7;
pub const BS_CATCH_RATE_OFFSET: i32 = 8;
pub const BS_COMMON_HELD_ITEM_OFFSET: i32 = 12;
pub const BS_RARE_HELD_ITEM_OFFSET: i32 = 14;
pub const BS_DARK_GRASS_HELD_ITEM_OFFSET: i32 = 16;
pub const BS_GROWTH_CURVE_OFFSET: i32 = 21;
pub const BS_ABILITY1_OFFSET: i32 = 24;
pub const BS_ABILITY2_OFFSET: i32 = 25;
pub const BS_ABILITY3_OFFSET: i32 = 26;
pub const BS_FORME_OFFSET: i32 = 28;
pub const BS_FORME_SPRITE_OFFSET: i32 = 30;
pub const BS_FORME_COUNT_OFFSET: i32 = 32;
pub const BS_TMHM_COMPAT_OFFSET: i32 = 40;
pub const BS_MT_COMPAT_OFFSET: i32 = 60;

pub const BW1_NEW_STARTER_SCRIPT: &[u8] = &[
    0x24, 0x00, 0xA7, 0x02, 0xE7, 0x00, 0x00, 0x00, 0xDE, 0x00, 0x00, 0x00, 0xF8, 0x01, 0x05, 0x00,
];

pub const BW1_STARTER_SCRIPT_MAGIC: &str = "2400A702";

pub const BW1_STARTER_TEXT_OFFSET: i32 = 18;
pub const BW1_CHEREN_TEXT1_OFFSET: i32 = 26;
pub const BW1_CHEREN_TEXT2_OFFSET: i32 = 53;

pub const BW2_NEW_STARTER_SCRIPT: &[u8] = &[
    0x28, 0x00, 0xA1, 0x40, 0x04, 0x00, 0xDE, 0x00, 0x00, 0x00, 0xFD, 0x01, 0x05, 0x00,
];

pub const BW2_STARTER_SCRIPT_MAGIC: &str = "2800A1400400";

pub const BW2_STARTER_TEXT_OFFSET: i32 = 37;
pub const BW2_RIVAL_TEXT_OFFSET: i32 = 60;

pub const PER_SEASON_ENCOUNTER_DATA_LENGTH: i32 = 232;
pub const BW1_AREA_DATA_ENTRY_LENGTH: i32 = 249;
pub const BW2_AREA_DATA_ENTRY_LENGTH: i32 = 345;
pub const BW1_ENCOUNTER_AREA_COUNT: i32 = 61;
pub const BW2_ENCOUNTER_AREA_COUNT: i32 = 85;

pub const ENCOUNTERS_OF_EACH_TYPE: [i32; 7] = [12, 12, 12, 5, 5, 5, 5];

pub const ENCOUNTER_TYPE_NAMES: [&str; 7] = [
    "Grass/Cave",
    "Doubles Grass",
    "Shaking Spots",
    "Surfing",
    "Surfing Spots",
    "Fishing",
    "Fishing Spots",
];

pub const HABITAT_CLASSIFICATION_OF_EACH_TYPE: [i32; 7] = [0, 0, 0, 1, 1, 2, 2];

pub const BW2_ROUTE4_AREA_INDEX: i32 = 40;
pub const BW2_VICTORY_ROAD_AREA_INDEX: i32 = 76;
pub const BW2_REVERSAL_MOUNTAIN_AREA_INDEX: i32 = 73;

pub const B2_ROUTE4_ENCOUNTER_FILE: i32 = 104;
pub const B2_VR_EXCLUSIVE_ROOM1: i32 = 71;
pub const B2_VR_EXCLUSIVE_ROOM2: i32 = 73;
pub const B2_REVERSAL_MOUNTAIN_START: i32 = 49;
pub const B2_REVERSAL_MOUNTAIN_END: i32 = 54;

pub const W2_ROUTE4_ENCOUNTER_FILE: i32 = 105;
pub const W2_VR_EXCLUSIVE_ROOM1: i32 = 78;
pub const W2_VR_EXCLUSIVE_ROOM2: i32 = 79;
pub const W2_REVERSAL_MOUNTAIN_START: i32 = 55;
pub const W2_REVERSAL_MOUNTAIN_END: i32 = 60;

pub const BW2_HIDDEN_HOLLOW_UNOVA_POKEMON: &[i32] = &[
    species::WATCHOG,
    species::HERDIER,
    species::LIEPARD,
    species::PANSAGE,
    species::PANSEAR,
    species::PANPOUR,
    species::PIDOVE,
    species::ZEBSTRIKA,
    species::BOLDORE,
    species::WOOBAT,
    species::DRILBUR,
    species::AUDINO,
    species::GURDURR,
    species::TYMPOLE,
    species::THROH,
    species::SAWK,
    species::LEAVANNY,
    species::SCOLIPEDE,
    species::COTTONEE,
    species::PETILIL,
    species::BASCULIN,
    species::KROOKODILE,
    species::MARACTUS,
    species::CRUSTLE,
    species::SCRAGGY,
    species::SIGILYPH,
    species::TIRTOUGA,
    species::GARBODOR,
    species::MINCCINO,
    species::GOTHORITA,
    species::DUOSION,
    species::DUCKLETT,
    species::VANILLISH,
    species::EMOLGA,
    species::KARRABLAST,
    species::ALOMOMOLA,
    species::GALVANTULA,
    species::KLINKLANG,
    species::ELGYEM,
    species::LITWICK,
    species::AXEW,
    species::CUBCHOO,
    species::SHELMET,
    species::STUNFISK,
    species::MIENFOO,
    species::DRUDDIGON,
    species::GOLETT,
    species::PAWNIARD,
    species::BOUFFALANT,
    species::BRAVIARY,
    species::MANDIBUZZ,
    species::HEATMOR,
    species::DURANT,
];

pub const TM_DATA_PREFIX: &str = "87038803";

pub const TM_COUNT: i32 = 95;
pub const HM_COUNT: i32 = 6;
pub const TM_BLOCK_ONE_COUNT: i32 = 92;
pub const TM_BLOCK_ONE_OFFSET: i32 = items::TM01;
pub const TM_BLOCK_TWO_OFFSET: i32 = items::TM93;

pub const BW1_ITEM_PALETTES_PREFIX: &str = "E903EA03020003000400050006000700";
pub const BW2_ITEM_PALETTES_PREFIX: &str = "FD03FE03020003000400050006000700";

pub const BW2_MOVE_TUTOR_COUNT: i32 = 60;
pub const BW2_MOVE_TUTOR_BYTES_PER_ENTRY: i32 = 12;

pub const EVOLUTION_METHOD_COUNT: i32 = 27;

pub const HIGHEST_ABILITY_INDEX: i32 = abilities::TERAVOLT;

pub const FOSSIL_POKEMON_FILE: i32 = 877;
pub const FOSSIL_POKEMON_LEVEL_OFFSET: i32 = 0x3F7;

const ABILITY_VARIATIONS: &[(i32, &[i32])] = &[
    (
        abilities::INSOMNIA,
        &[abilities::INSOMNIA, abilities::VITAL_SPIRIT],
    ),
    (
        abilities::CLEAR_BODY,
        &[abilities::CLEAR_BODY, abilities::WHITE_SMOKE],
    ),
    (
        abilities::HUGE_POWER,
        &[abilities::HUGE_POWER, abilities::PURE_POWER],
    ),
    (
        abilities::BATTLE_ARMOR,
        &[abilities::BATTLE_ARMOR, abilities::SHELL_ARMOR],
    ),
    (
        abilities::CLOUD_NINE,
        &[abilities::CLOUD_NINE, abilities::AIR_LOCK],
    ),
    (
        abilities::FILTER,
        &[abilities::FILTER, abilities::SOLID_ROCK],
    ),
    (
        abilities::ROUGH_SKIN,
        &[abilities::ROUGH_SKIN, abilities::IRON_BARBS],
    ),
    (
        abilities::MOLD_BREAKER,
        &[
            abilities::MOLD_BREAKER,
            abilities::TURBOBLAZE,
            abilities::TERAVOLT,
        ],
    ),
];

pub fn ability_variations() -> BTreeMap<i32, Vec<i32>> {
    ABILITY_VARIATIONS
        .iter()
        .map(|&(ability, variations)| (ability, variations.to_vec()))
        .collect()
}

pub const USELESS_ABILITIES: &[i32] = &[
    abilities::FORECAST,
    abilities::MULTITYPE,
    abilities::FLOWER_GIFT,
    abilities::ZEN_MODE,
];

pub const NORMAL_ITEM_SET_VAR_COMMAND: i32 = 0x28;
pub const HIDDEN_ITEM_SET_VAR_COMMAND: i32 = 0x2A;
pub const NORMAL_ITEM_VAR_SET: i32 = 0x800C;
pub const HIDDEN_ITEM_VAR_SET: i32 = 0x8000;

pub const SCRIPT_LIST_TERMINATOR: i32 = 0xFD13;

pub const MULCH_INDICES: [i32; 4] = [
    items::GROWTH_MULCH,
    items::DAMP_MULCH,
    items::STABLE_MULCH,
    items::GOOEY_MULCH,
];

pub const MOVE_CATEGORY_INDICES: [MoveCategory; 3] = [
    MoveCategory::Status,
    MoveCategory::Physical,
    MoveCategory::Special,
];

pub fn move_category_from_index(index: i32) -> Option<MoveCategory> {
    usize::try_from(index)
        .ok()
        .and_then(|index| MOVE_CATEGORY_INDICES.get(index).copied())
}

pub fn move_category_to_byte(category: MoveCategory) -> u8 {
    match category {
        MoveCategory::Physical => 1,
        MoveCategory::Special => 2,
        MoveCategory::Status => 0,
    }
}

pub const TRAPPING_EFFECT: i32 = 106;

pub const NO_DAMAGE_STATUS_QUALITY: i32 = 1;
pub const NO_DAMAGE_STAT_CHANGE_QUALITY: i32 = 2;
pub const DAMAGE_STATUS_QUALITY: i32 = 4;
pub const NO_DAMAGE_STATUS_AND_STAT_CHANGE_QUALITY: i32 = 5;
pub const DAMAGE_TARGET_DEBUFF_QUALITY: i32 = 6;
pub const DAMAGE_USER_BUFF_QUALITY: i32 = 7;
pub const DAMAGE_ABSORB_QUALITY: i32 = 8;

pub fn type_table(value: i32) -> Option<Type> {
    match value {
        0x00 => Some(Type::Normal),
        0x01 => Some(Type::Fighting),
        0x02 => Some(Type::Flying),
        0x03 => Some(Type::Poison),
        0x04 => Some(Type::Ground),
        0x05 => Some(Type::Rock),
        0x06 => Some(Type::Bug),
        0x07 => Some(Type::Ghost),
        0x08 => Some(Type::Steel),
        0x09 => Some(Type::Fire),
        0x0A => Some(Type::Water),
        0x0B => Some(Type::Grass),
        0x0C => Some(Type::Electric),
        0x0D => Some(Type::Psychic),
        0x0E => Some(Type::Ice),
        0x0F => Some(Type::Dragon),
        0x10 => Some(Type::Dark),
        _ => None,
    }
}

pub fn type_to_byte(kind: Option<Type>) -> u8 {
    match kind {
        Some(Type::Normal) => 0x00,
        Some(Type::Fighting) => 0x01,
        Some(Type::Flying) => 0x02,
        Some(Type::Poison) => 0x03,
        Some(Type::Ground) => 0x04,
        Some(Type::Rock) => 0x05,
        Some(Type::Bug) => 0x06,
        Some(Type::Ghost) => 0x07,
        Some(Type::Fire) => 0x09,
        Some(Type::Water) => 0x0A,
        Some(Type::Grass) => 0x0B,
        Some(Type::Electric) => 0x0C,
        Some(Type::Psychic) => 0x0D,
        Some(Type::Ice) => 0x0E,
        Some(Type::Dragon) => 0x0F,
        Some(Type::Steel) => 0x08,
        Some(Type::Dark) => 0x10,
        _ => 0,
    }
}

const BW1_FORME_SUFFIXES: &[(i32, &str)] = &[
    (gen5_formes::DEOXYS_A, "-A"),
    (gen5_formes::DEOXYS_D, "-D"),
    (gen5_formes::DEOXYS_S, "-S"),
    (gen5_formes::WORMADAM_S, "-S"),
    (gen5_formes::WORMADAM_T, "-T"),
    (gen5_formes::SHAYMIN_S, "-S"),
    (gen5_formes::GIRATINA_O, "-O"),
    (gen5_formes::ROTOM_H, "-H"),
    (gen5_formes::ROTOM_W, "-W"),
    (gen5_formes::ROTOM_FR, "-Fr"),
    (gen5_formes::ROTOM_FA, "-Fa"),
    (gen5_formes::ROTOM_M, "-M"),
    (gen5_formes::CASTFORM_F, "-F"),
    (gen5_formes::CASTFORM_W, "-W"),
    (gen5_formes::CASTFORM_I, "-I"),
    (gen5_formes::BASCULIN_B, "-B"),
    (gen5_formes::DARMANITAN_Z, "-Z"),
    (gen5_formes::MELOETTA_P, "-P"),
];

const BW2_FORME_SUFFIXES: &[(i32, &str)] = &[
    (gen5_formes::DEOXYS_A + BW2_FORME_OFFSET, "-A"),
    (gen5_formes::DEOXYS_D + BW2_FORME_OFFSET, "-D"),
    (gen5_formes::DEOXYS_S + BW2_FORME_OFFSET, "-S"),
    (gen5_formes::WORMADAM_S + BW2_FORME_OFFSET, "-S"),
    (gen5_formes::WORMADAM_T + BW2_FORME_OFFSET, "-T"),
    (gen5_formes::SHAYMIN_S + BW2_FORME_OFFSET, "-S"),
    (gen5_formes::GIRATINA_O + BW2_FORME_OFFSET, "-O"),
    (gen5_formes::ROTOM_H + BW2_FORME_OFFSET, "-H"),
    (gen5_formes::ROTOM_W + BW2_FORME_OFFSET, "-W"),
    (gen5_formes::ROTOM_FR + BW2_FORME_OFFSET, "-Fr"),
    (gen5_formes::ROTOM_FA + BW2_FORME_OFFSET, "-Fa"),
    (gen5_formes::ROTOM_M + BW2_FORME_OFFSET, "-M"),
    (gen5_formes::CASTFORM_F + BW2_FORME_OFFSET, "-F"),
    (gen5_formes::CASTFORM_W + BW2_FORME_OFFSET, "-W"),
    (gen5_formes::CASTFORM_I + BW2_FORME_OFFSET, "-I"),
    (gen5_formes::BASCULIN_B + BW2_FORME_OFFSET, "-B"),
    (gen5_formes::DARMANITAN_Z + BW2_FORME_OFFSET, "-Z"),
    (gen5_formes::MELOETTA_P + BW2_FORME_OFFSET, "-P"),
    (gen5_formes::KYUREM_W + BW2_FORME_OFFSET, "-W"),
    (gen5_formes::KYUREM_B + BW2_FORME_OFFSET, "-B"),
    (gen5_formes::TORNADUS_T + BW2_FORME_OFFSET, "-T"),
    (gen5_formes::THUNDURUS_T + BW2_FORME_OFFSET, "-T"),
    (gen5_formes::LANDORUS_T + BW2_FORME_OFFSET, "-T"),
];

const FORME_SUFFIXES_BY_BASE_FORME: &[(i32, &[(i32, &str)])] = &[
    (species::DEOXYS, &[(1, "-A"), (2, "-D"), (3, "-S")]),
    (species::WORMADAM, &[(1, "-S"), (2, "-T")]),
    (species::SHAYMIN, &[(1, "-S")]),
    (species::GIRATINA, &[(1, "-O")]),
    (
        species::ROTOM,
        &[(1, "-H"), (2, "-W"), (3, "-Fr"), (4, "-Fa"), (5, "-M")],
    ),
    (species::CASTFORM, &[(1, "-F"), (2, "-W"), (3, "-I")]),
    (species::BASCULIN, &[(1, "-B")]),
    (species::DARMANITAN, &[(1, "-Z")]),
    (species::MELOETTA, &[(1, "-P")]),
    (species::KYUREM, &[(1, "-W"), (2, "-B")]),
    (species::KELDEO, &[(1, "-R")]),
    (species::TORNADUS, &[(1, "-T")]),
    (species::THUNDURUS, &[(1, "-T")]),
    (species::LANDORUS, &[(1, "-T")]),
];

const DUMMY_FORME_SUFFIXES: &[(i32, &str)] = &[(0, "")];

const ABSOLUTE_POKE_NUMS_BY_BASE_FORME: &[(i32, &[(i32, i32)])] = &[
    (
        species::DEOXYS,
        &[
            (1, gen5_formes::DEOXYS_A),
            (2, gen5_formes::DEOXYS_D),
            (3, gen5_formes::DEOXYS_S),
        ],
    ),
    (
        species::WORMADAM,
        &[(1, gen5_formes::WORMADAM_S), (2, gen5_formes::WORMADAM_T)],
    ),
    (species::SHAYMIN, &[(1, gen5_formes::SHAYMIN_S)]),
    (species::GIRATINA, &[(1, gen5_formes::GIRATINA_O)]),
    (
        species::ROTOM,
        &[
            (1, gen5_formes::ROTOM_H),
            (2, gen5_formes::ROTOM_W),
            (3, gen5_formes::ROTOM_FR),
            (4, gen5_formes::ROTOM_FA),
            (5, gen5_formes::ROTOM_M),
        ],
    ),
    (
        species::CASTFORM,
        &[
            (1, gen5_formes::CASTFORM_F),
            (2, gen5_formes::CASTFORM_W),
            (3, gen5_formes::CASTFORM_I),
        ],
    ),
    (species::BASCULIN, &[(1, gen5_formes::BASCULIN_B)]),
    (species::DARMANITAN, &[(1, gen5_formes::DARMANITAN_Z)]),
    (species::MELOETTA, &[(1, gen5_formes::MELOETTA_P)]),
    (
        species::KYUREM,
        &[(1, gen5_formes::KYUREM_W), (2, gen5_formes::KYUREM_B)],
    ),
    (species::KELDEO, &[(1, gen5_formes::KELDEO_COSMETIC1)]),
    (species::TORNADUS, &[(1, gen5_formes::TORNADUS_T)]),
    (species::THUNDURUS, &[(1, gen5_formes::THUNDURUS_T)]),
    (species::LANDORUS, &[(1, gen5_formes::LANDORUS_T)]),
];

const DUMMY_ABSOLUTE_POKE_NUMS: &[(i32, i32)] = &[(255, 0)];

fn lookup<K: PartialEq + Copy, V: Copy>(table: &[(K, V)], key: K) -> Option<V> {
    table
        .iter()
        .find(|&&(candidate, _)| candidate == key)
        .map(|&(_, value)| value)
}

pub fn forme_suffix_by_base_forme(base_forme: i32, form_num: i32) -> &'static str {
    let suffixes = lookup(FORME_SUFFIXES_BY_BASE_FORME, base_forme).unwrap_or(DUMMY_FORME_SUFFIXES);
    lookup(suffixes, form_num).unwrap_or("")
}

pub fn absolute_poke_num_by_base_forme(base_forme: i32, form_num: i32) -> i32 {
    let numbers =
        lookup(ABSOLUTE_POKE_NUMS_BY_BASE_FORME, base_forme).unwrap_or(DUMMY_ABSOLUTE_POKE_NUMS);
    lookup(numbers, form_num).unwrap_or(base_forme)
}

pub fn forme_suffix(internal_index: i32, rom_type: i32) -> &'static str {
    let suffixes = match rom_type {
        TYPE_BW => BW1_FORME_SUFFIXES,
        TYPE_BW2 => BW2_FORME_SUFFIXES,
        _ => return "",
    };
    lookup(suffixes, internal_index).unwrap_or("")
}

pub const BW1_IRREGULAR_FORMES: &[i32] = &[
    gen5_formes::CASTFORM_F,
    gen5_formes::CASTFORM_W,
    gen5_formes::CASTFORM_I,
    gen5_formes::DARMANITAN_Z,
    gen5_formes::MELOETTA_P,
];

pub const BW2_IRREGULAR_FORMES: &[i32] = &[
    gen5_formes::CASTFORM_F,
    gen5_formes::CASTFORM_W,
    gen5_formes::CASTFORM_I,
    gen5_formes::DARMANITAN_Z,
    gen5_formes::MELOETTA_P,
    gen5_formes::KYUREM_W,
    gen5_formes::KYUREM_B,
];

pub const EMPTY_PLAYTHROUGH_TRAINERS: &[i32] = &[];

pub const BW1_MAIN_PLAYTHROUGH_TRAINERS: &[i32] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 79, 80, 81, 82, 83, 84,
    85, 86, 87, 88, 89, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
    109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127,
    128, 129, 130, 131, 132, 133, 134, 135, 137, 138, 139, 140, 141, 142, 143, 144, 145, 150, 151,
    152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170,
    171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 186, 187, 188, 190, 191,
    192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 212, 213, 214, 215, 216, 217,
    218, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237,
    238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 256,
    257, 258, 263, 264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279,
    280, 281, 282, 283, 284, 285, 286, 290, 291, 292, 293, 294, 295, 296, 297, 298, 299, 300, 301,
    302, 303, 304, 305, 306, 307, 308, 309, 310, 311, 312, 313, 315, 316, 401, 402, 408, 409, 412,
    413, 438, 439, 441, 442, 443, 445, 447, 450, 460, 461, 462, 465, 466, 468, 469, 470, 471, 472,
    473, 474, 475, 476, 477, 478, 479, 480, 481, 484, 485, 488, 489, 490, 501, 502, 503, 504, 505,
    506, 513, 514, 515, 516, 517, 518, 519, 520, 526, 531, 532, 533, 534, 535, 536, 537, 538, 544,
    545, 546, 549, 550, 552, 553, 554, 555, 556, 557, 582, 583, 584, 585, 586, 587, 600, 601, 602,
    603, 604, 605, 606, 607, 610, 611, 612, 613,
];

pub const BW2_MAIN_PLAYTHROUGH_TRAINERS: &[i32] = &[
    4, 5, 6, 133, 134, 135, 136, 137, 138, 139, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156,
    157, 158, 159, 160, 164, 165, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181,
    182, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220,
    221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 237, 238, 239, 240,
    242, 243, 244, 245, 247, 248, 249, 250, 252, 253, 254, 255, 256, 257, 258, 259, 260, 261, 262,
    263, 264, 265, 266, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 280, 281,
    282, 283, 284, 285, 286, 287, 288, 289, 290, 291, 292, 293, 294, 295, 296, 297, 298, 299, 300,
    301, 302, 303, 304, 305, 306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319,
    320, 321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333, 334, 335, 336, 337, 338,
    339, 340, 341, 342, 343, 344, 345, 346, 347, 348, 349, 350, 351, 352, 353, 354, 355, 356, 357,
    358, 359, 360, 361, 362, 363, 364, 365, 366, 367, 372, 373, 374, 375, 376, 377, 381, 382, 383,
    384, 385, 386, 387, 388, 389, 390, 391, 392, 426, 427, 428, 429, 430, 431, 432, 433, 434, 435,
    436, 437, 438, 439, 440, 441, 442, 443, 444, 445, 446, 447, 448, 449, 450, 451, 452, 453, 454,
    455, 461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 471, 472, 473, 474, 475, 476, 477, 478,
    479, 480, 481, 482, 483, 484, 485, 486, 497, 498, 499, 500, 501, 502, 503, 510, 511, 512, 513,
    514, 515, 516, 517, 518, 519, 520, 521, 522, 523, 524, 537, 538, 539, 540, 541, 542, 543, 544,
    545, 546, 547, 548, 549, 550, 551, 552, 553, 554, 555, 556, 557, 558, 559, 560, 561, 562, 563,
    564, 565, 566, 567, 568, 569, 570, 580, 581, 583, 584, 585, 586, 587, 592, 593, 594, 595, 596,
    597, 598, 599, 600, 601, 602, 603, 604, 605, 606, 607, 608, 609, 610, 611, 612, 613, 614, 615,
    621, 622, 623, 624, 625, 626, 627, 628, 629, 630, 631, 657, 658, 659, 660, 661, 662, 663, 664,
    665, 666, 667, 668, 669, 670, 671, 672, 673, 679, 680, 681, 682, 683, 690, 691, 692, 703, 704,
    705, 712, 713, 714, 715, 716, 717, 718, 719, 720, 721, 722, 723, 724, 725, 726, 727, 728, 729,
    730, 731, 732, 733, 734, 735, 736, 737, 738, 745, 746, 747, 748, 749, 750, 751, 752, 754, 755,
    756, 763, 764, 765, 766, 767, 768, 769, 770, 771, 772, 773, 774, 775, 776, 786, 787, 788, 789,
    797, 798, 799, 800, 801, 802, 803, 804, 805, 806, 807, 808, 809, 810, 811, 812,
];

pub const BW2_DRIFTVEIL_TRAINER_OFFSETS: &[i32] = &[
    56, 57, 0, 1, 2, 3, 4, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77,
];

pub const NORMAL_TRAINER_NAME_LENGTH: i32 = 813;
pub const NORMAL_TRAINER_CLASS_LENGTH: i32 = 236;

pub const BW1_MAIN_GAME_SHOPS: &[i32] = &[3, 5, 6, 8, 9, 12, 14, 17, 18, 19, 21, 22];

pub const BW1_SHOP_NAMES: &[&str] = &[
    "Primary 0 Badges",
    "Shopping Mall 9 TMs",
    "Icirrus Secondary (TMs)",
    "Driftveil Herb Salesman",
    "Mistralton Secondary (TMs)",
    "Shopping Mall 9 F3 Left",
    "Accumula Secondary",
    "Nimbasa Secondary (TMs)",
    "Striaton Secondary",
    "League Secondary",
    "Lacunosa Secondary",
    "Black City/White Forest Secondary",
    "Nacrene/Shopping Mall 9 X Items",
    "Driftveil Incense Salesman",
    "Nacrene Secondary",
    "Undella Secondary",
    "Primary 2 Badges",
    "Castelia Secondary",
    "Driftveil Secondary",
    "Opelucid Secondary",
    "Primary 3 Badges",
    "Shopping Mall 9 F1",
    "Shopping Mall 9 F2",
    "Primary 5 Badges",
    "Primary 7 Badges",
    "Primary 8 Badges",
];

pub const BW2_MAIN_GAME_SHOPS: &[i32] = &[
    9, 11, 14, 15, 16, 18, 20, 21, 22, 23, 25, 26, 27, 28, 29, 30, 31,
];

pub const BW2_SHOP_NAMES: &[&str] = &[
    "Primary 0 Badges",
    "Primary 1 Badges",
    "Primary 3 Badges",
    "Primary 5 Badges",
    "Primary 7 Badges",
    "Primary 8 Badges",
    "Accumula Secondary",
    "Striaton Secondary (TMs)",
    "Nacrene Secondary",
    "Castelia Secondary",
    "Nimbasa Secondary (TMs)",
    "Driftveil Secondary",
    "Mistralton Secondary (TMs)",
    "Icirrus Secondary",
    "Opelucid Secondary",
    "Victory Road Secondary",
    "Pokemon League Secondary",
    "Lacunosa Secondary (TMs)",
    "Undella Secondary",
    "Black City/White Forest Secondary",
    "Nacrene/Shopping Mall 9 X Items",
    "Driftveil Herb Salesman",
    "Driftveil Incense Salesman",
    "Shopping Mall 9 F1",
    "Shopping Mall 9 TMs",
    "Shopping Mall 9 F2",
    "Shopping Mall 9 F3 Left",
    "Aspertia Secondary",
    "Virbank Secondary",
    "Humilau Secondary",
    "Floccesy Secondary",
    "Lentimas Secondary",
];

pub const EVOLUTION_ITEMS: &[i32] = &[
    items::SUN_STONE,
    items::MOON_STONE,
    items::FIRE_STONE,
    items::THUNDER_STONE,
    items::WATER_STONE,
    items::LEAF_STONE,
    items::SHINY_STONE,
    items::DUSK_STONE,
    items::DAWN_STONE,
    items::OVAL_STONE,
    items::KINGS_ROCK,
    items::DEEP_SEA_TOOTH,
    items::DEEP_SEA_SCALE,
    items::METAL_COAT,
    items::DRAGON_SCALE,
    items::UPGRADE,
    items::PROTECTOR,
    items::ELECTIRIZER,
    items::MAGMARIZER,
    items::DUBIOUS_DISC,
    items::REAPER_CLOTH,
    items::RAZOR_CLAW,
    items::RAZOR_FANG,
    items::PRISM_SCALE,
];

pub const BW1_REQUIRED_FIELD_TMS: &[i32] = &[
    2, 3, 5, 6, 9, 12, 13, 19, 22, 24, 26, 29, 30, 35, 36, 39, 41, 46, 47, 50, 52, 53, 55, 58, 61,
    63, 65, 66, 71, 80, 81, 84, 85, 86, 90, 91, 92, 93,
];

pub const BW2_REQUIRED_FIELD_TMS: &[i32] = &[
    1, 2, 3, 5, 6, 12, 13, 19, 22, 26, 28, 29, 30, 36, 39, 41, 46, 47, 50, 52, 53, 56, 58, 61, 63,
    65, 66, 67, 69, 71, 80, 81, 84, 85, 86, 90, 91, 92, 93,
];

pub const BW1_EARLY_REQUIRED_HM_MOVES: &[i32] = &[moves::CUT];

pub const BW2_EARLY_REQUIRED_HM_MOVES: &[i32] = &[];

pub const FIELD_MOVES: &[i32] = &[
    moves::CUT,
    moves::FLY,
    moves::SURF,
    moves::STRENGTH,
    moves::FLASH,
    moves::DIG,
    moves::TELEPORT,
    moves::WATERFALL,
    moves::SWEET_SCENT,
    moves::DIVE,
];

pub const SHEDINJA_SPECIES_LOCATOR: &str = "24010000";

pub const RUNNING_SHOES_PREFIX: &str = "01D0012008BD002008BD63";

pub const INTRO_GRAPHIC_PREFIX: &str = "5A0000010000001700000001000000";
pub const BW1_INTRO_CRY_PREFIX: &str = "0021009101910291";
pub const BW2_INTRO_CRY_LOCATOR: &str = "3D020000F8B51C1C";

pub const TYPE_EFFECTIVENESS_TABLE_LOCATOR: &str = "0404040404020400";

pub const FORCE_CHALLENGE_MODE_LOCATOR: &str = "816A406B0B1C07490022434090000858834201D1";

pub const PICKUP_TABLE_LOCATOR: &str = "19005C00DD00";
pub const NUMBER_OF_PICKUP_ITEMS: i32 = 29;

pub const FRIENDSHIP_VALUE_FOR_EVO_LOCATOR: &str = "DC282FD3";

pub const PERFECT_ODDS_BRANCH_LOCATOR: &str = "08DB002801D0012000E0";

pub const LOW_HEALTH_MUSIC_LOCATOR: &str = "00D10127";

pub const NEW_CONSUMABLE_HELD_ITEMS: &[i32] = &[
    items::AIR_BALLOON,
    items::RED_CARD,
    items::ABSORB_BULB,
    items::CELL_BATTERY,
    items::EJECT_BUTTON,
    items::FIRE_GEM,
    items::WATER_GEM,
    items::ELECTRIC_GEM,
    items::GRASS_GEM,
    items::ICE_GEM,
    items::FIGHTING_GEM,
    items::POISON_GEM,
    items::GROUND_GEM,
    items::FLYING_GEM,
    items::PSYCHIC_GEM,
    items::BUG_GEM,
    items::ROCK_GEM,
    items::GHOST_GEM,
    items::DRAGON_GEM,
    items::DARK_GEM,
    items::STEEL_GEM,
    items::NORMAL_GEM,
];

pub const NEW_NON_CONSUMABLE_HELD_ITEMS: &[i32] = &[
    items::EVIOLITE,
    items::FLOAT_STONE,
    items::ROCKY_HELMET,
    items::RING_TARGET,
    items::BINDING_BAND,
];

pub const NEW_GENERAL_PURPOSE_CONSUMABLE_ITEMS: &[i32] = &[
    items::RED_CARD,
    items::ABSORB_BULB,
    items::CELL_BATTERY,
    items::EJECT_BUTTON,
];

pub const NEW_GENERAL_PURPOSE_ITEMS: &[i32] = &[items::FLOAT_STONE, items::ROCKY_HELMET];

pub fn consumable_held_items() -> Vec<i32> {
    [gen4::CONSUMABLE_HELD_ITEMS, NEW_CONSUMABLE_HELD_ITEMS].concat()
}

pub fn all_held_items() -> Vec<i32> {
    [
        gen4::all_held_items().as_slice(),
        NEW_CONSUMABLE_HELD_ITEMS,
        NEW_NON_CONSUMABLE_HELD_ITEMS,
    ]
    .concat()
}

pub fn general_purpose_consumable_items() -> Vec<i32> {
    [
        gen4::GENERAL_PURPOSE_CONSUMABLE_ITEMS,
        NEW_GENERAL_PURPOSE_CONSUMABLE_ITEMS,
    ]
    .concat()
}

pub fn general_purpose_items() -> Vec<i32> {
    [gen4::GENERAL_PURPOSE_ITEMS, NEW_GENERAL_PURPOSE_ITEMS].concat()
}

pub fn consumable_type_boosting_item(kind: Type) -> Option<i32> {
    match kind {
        Type::Fire => Some(items::FIRE_GEM),
        Type::Water => Some(items::WATER_GEM),
        Type::Electric => Some(items::ELECTRIC_GEM),
        Type::Grass => Some(items::GRASS_GEM),
        Type::Ice => Some(items::ICE_GEM),
        Type::Fighting => Some(items::FIGHTING_GEM),
        Type::Poison => Some(items::POISON_GEM),
        Type::Ground => Some(items::GROUND_GEM),
        Type::Flying => Some(items::FLYING_GEM),
        Type::Psychic => Some(items::PSYCHIC_GEM),
        Type::Bug => Some(items::BUG_GEM),
        Type::Rock => Some(items::ROCK_GEM),
        Type::Ghost => Some(items::GHOST_GEM),
        Type::Dragon => Some(items::DRAGON_GEM),
        Type::Dark => Some(items::DARK_GEM),
        Type::Steel => Some(items::STEEL_GEM),
        Type::Normal => Some(items::NORMAL_GEM),
        _ => None,
    }
}

fn gen5_move_boosting_items(move_number: i32) -> Option<&'static [i32]> {
    match move_number {
        moves::TRICK | moves::SWITCHEROO => {
            Some(&[items::TOXIC_ORB, items::FLAME_ORB, items::RING_TARGET])
        }
        moves::BIND
        | moves::CLAMP
        | moves::FIRE_SPIN
        | moves::MAGMA_STORM
        | moves::SAND_TOMB
        | moves::WHIRLPOOL
        | moves::WRAP => Some(&[items::GRIP_CLAW, items::BINDING_BAND]),
        moves::HORN_LEECH => Some(&[items::BIG_ROOT]),
        _ => None,
    }
}

pub fn move_boosting_items(move_number: i32) -> Option<Vec<i32>> {
    gen5_move_boosting_items(move_number)
        .map(<[i32]>::to_vec)
        .or_else(|| gen4::move_boosting_items(move_number))
}

pub fn ability_boosting_items(ability: i32) -> Option<Vec<i32>> {
    gen4::ability_boosting_items(ability)
}

pub fn species_boosting_items(species_number: i32) -> Option<Vec<i32>> {
    gen4::species_boosting_items(species_number)
}

pub fn type_boosting_items(kind: Type) -> Option<Vec<i32>> {
    gen4::type_boosting_items(kind)
}

pub fn weakness_reducing_berry(kind: Type) -> Option<i32> {
    gen4::weakness_reducing_berry(kind)
}

pub fn area_data_entry_length(rom_type: i32) -> i32 {
    match rom_type {
        TYPE_BW => BW1_AREA_DATA_ENTRY_LENGTH,
        TYPE_BW2 => BW2_AREA_DATA_ENTRY_LENGTH,
        _ => 0,
    }
}

pub fn encounter_area_count(rom_type: i32) -> i32 {
    match rom_type {
        TYPE_BW => BW1_ENCOUNTER_AREA_COUNT,
        TYPE_BW2 => BW2_ENCOUNTER_AREA_COUNT,
        _ => 0,
    }
}

pub fn wild_file_to_area_map(rom_type: i32) -> &'static [i32] {
    match rom_type {
        TYPE_BW => BW1_WILD_FILE_TO_AREA_MAP,
        TYPE_BW2 => BW2_WILD_FILE_TO_AREA_MAP,
        _ => &[],
    }
}

pub fn main_game_shops(rom_type: i32) -> &'static [i32] {
    match rom_type {
        TYPE_BW => BW1_MAIN_GAME_SHOPS,
        TYPE_BW2 => BW2_MAIN_GAME_SHOPS,
        _ => &[],
    }
}

pub fn irregular_formes(rom_type: i32) -> &'static [i32] {
    match rom_type {
        TYPE_BW => BW1_IRREGULAR_FORMES,
        TYPE_BW2 => BW2_IRREGULAR_FORMES,
        _ => &[],
    }
}

pub fn forme_count(rom_type: i32) -> i32 {
    match rom_type {
        TYPE_BW => BW1_FORME_COUNT,
        TYPE_BW2 => BW2_FORME_COUNT,
        _ => 0,
    }
}

pub fn forme_offset(rom_type: i32) -> i32 {
    match rom_type {
        TYPE_BW => BW1_FORME_OFFSET,
        TYPE_BW2 => BW2_FORME_OFFSET,
        _ => 0,
    }
}

pub fn non_pokemon_battle_sprite_count(rom_type: i32) -> i32 {
    match rom_type {
        TYPE_BW => BW1_NON_POKEMON_BATTLE_SPRITE_COUNT,
        TYPE_BW2 => BW2_NON_POKEMON_BATTLE_SPRITE_COUNT,
        _ => 0,
    }
}

pub const BLACK_BOX_LEGENDARY_CHECK_PREFIX1: &str = "79F6BAEF07B0F0BDC046";
pub const BLACK_BOX_LEGENDARY_CHECK_PREFIX2: &str = "DEDB0020C04302B0F8BDC046";
pub const WHITE_BOX_LEGENDARY_CHECK_PREFIX1: &str = "00F0FEF8002070BD";
pub const WHITE_BOX_LEGENDARY_CHECK_PREFIX2: &str = "64F62EF970BD0000";

pub fn allowed_items() -> ItemList {
    let mut allowed = ItemList::new(items::REVEAL_GLASS as usize);
    allowed.ban_range(items::EXPLORER_KIT, 76);
    allowed.ban_range(items::DATA_CARD01, 32);
    allowed.ban_range(items::XTRANSCEIVER_MALE, 18);
    allowed.ban_singles(&[
        items::LIBERTY_PASS,
        items::PROP_CASE,
        items::DRAGON_SKULL,
        items::LIGHT_STONE,
        items::DARK_STONE,
    ]);
    allowed.ban_range(items::TEA, 3);
    allowed.ban_range(items::UNUSED120, 14);
    allowed.tm_range(items::TM01, 92);
    allowed.tm_range(items::TM93, 3);
    allowed.ban_range(items::TM01, 100);
    allowed.ban_range(items::TM93, 3);
    allowed.ban_range(items::DIRE_HIT2, 24);
    allowed
}

fn non_bad_items_bw2() -> ItemList {
    let mut non_bad = allowed_items();
    non_bad.ban_singles(&[
        items::ODD_KEYSTONE,
        items::GRISEOUS_ORB,
        items::SOUL_DEW,
        items::LIGHT_BALL,
        items::ORAN_BERRY,
        items::QUICK_POWDER,
        items::PASS_ORB,
    ]);
    non_bad.ban_range(items::GROWTH_MULCH, 4);
    non_bad.ban_range(items::ADAMANT_ORB, 2);
    non_bad.ban_range(items::MAIL1, 12);
    non_bad.ban_range(items::FIGY_BERRY, 25);
    non_bad.ban_range(items::LUCKY_PUNCH, 4);
    non_bad.ban_range(items::RED_SCARF, 5);
    non_bad
}

fn non_bad_items_bw1() -> ItemList {
    let mut non_bad = non_bad_items_bw2();
    non_bad.ban_range(items::RED_SHARD, 4);
    non_bad
}

pub fn non_bad_items(rom_type: i32) -> ItemList {
    if rom_type == TYPE_BW2 {
        non_bad_items_bw2()
    } else {
        non_bad_items_bw1()
    }
}

pub fn regular_shop_items() -> Vec<i32> {
    (items::ULTRA_BALL..=items::POKE_BALL)
        .chain(items::POTION..=items::REVIVE)
        .chain(items::SUPER_REPEL..=items::REPEL)
        .collect()
}

pub fn op_shop_items() -> Vec<i32> {
    [
        items::LAVA_COOKIE,
        items::BERRY_JUICE,
        items::RARE_CANDY,
        items::OLD_GATEAU,
    ]
    .into_iter()
    .chain(items::BLUE_FLUTE..=items::SHOAL_SHELL)
    .chain(items::TINY_MUSHROOM..=items::NUGGET)
    .chain([items::RARE_BONE])
    .chain(items::LANSAT_BERRY..=items::ROWAP_BERRY)
    .chain([items::LUCKY_EGG, items::PRETTY_FEATHER])
    .chain(items::BALM_MUSHROOM..=items::CASTELIACONE)
    .collect()
}

pub fn balanced_item_price(item: i32) -> Option<i32> {
    let price = match item {
        items::MASTER_BALL => 300,
        items::ULTRA_BALL => 120,
        items::GREAT_BALL => 60,
        items::POKE_BALL => 20,
        items::SAFARI_BALL => 50,
        items::NET_BALL => 100,
        items::DIVE_BALL => 100,
        items::NEST_BALL => 100,
        items::REPEAT_BALL => 100,
        items::TIMER_BALL => 100,
        items::LUXURY_BALL => 100,
        items::PREMIER_BALL => 20,
        items::DUSK_BALL => 100,
        items::HEAL_BALL => 30,
        items::QUICK_BALL => 100,
        items::CHERISH_BALL => 20,
        items::POTION => 30,
        items::ANTIDOTE => 10,
        items::BURN_HEAL => 25,
        items::ICE_HEAL => 25,
        items::AWAKENING => 25,
        items::PARALYZE_HEAL => 20,
        items::FULL_RESTORE => 300,
        items::MAX_POTION => 250,
        items::HYPER_POTION => 120,
        items::SUPER_POTION => 70,
        items::FULL_HEAL => 60,
        items::REVIVE => 150,
        items::MAX_REVIVE => 400,
        items::FRESH_WATER => 40,
        items::SODA_POP => 60,
        items::LEMONADE => 70,
        items::MOOMOO_MILK => 80,
        items::ENERGY_POWDER => 40,
        items::ENERGY_ROOT => 110,
        items::HEAL_POWDER => 45,
        items::REVIVAL_HERB => 280,
        items::ETHER => 300,
        items::MAX_ETHER => 450,
        items::ELIXIR => 1500,
        items::MAX_ELIXIR => 1800,
        items::LAVA_COOKIE => 45,
        items::BERRY_JUICE => 10,
        items::SACRED_ASH => 1000,
        items::HP_UP => 980,
        items::PROTEIN => 980,
        items::IRON => 980,
        items::CARBOS => 980,
        items::CALCIUM => 980,
        items::RARE_CANDY => 1000,
        items::PP_UP => 980,
        items::ZINC => 980,
        items::PP_MAX => 2490,
        items::OLD_GATEAU => 45,
        items::GUARD_SPEC => 70,
        items::DIRE_HIT => 65,
        items::X_ATTACK => 50,
        items::X_DEFENSE => 55,
        items::X_SPEED => 35,
        items::X_ACCURACY => 95,
        items::X_SP_ATK => 35,
        items::X_SP_DEF => 35,
        items::POKE_DOLL => 100,
        items::FLUFFY_TAIL => 100,
        items::BLUE_FLUTE => 2,
        items::YELLOW_FLUTE => 2,
        items::RED_FLUTE => 2,
        items::BLACK_FLUTE => 2,
        items::WHITE_FLUTE => 2,
        items::SHOAL_SALT => 2,
        items::SHOAL_SHELL => 2,
        items::RED_SHARD => 40,
        items::BLUE_SHARD => 40,
        items::YELLOW_SHARD => 40,
        items::GREEN_SHARD => 40,
        items::SUPER_REPEL => 50,
        items::MAX_REPEL => 70,
        items::ESCAPE_ROPE => 55,
        items::REPEL => 35,
        items::SUN_STONE => 300,
        items::MOON_STONE => 300,
        items::FIRE_STONE => 300,
        items::THUNDER_STONE => 300,
        items::WATER_STONE => 300,
        items::LEAF_STONE => 300,
        items::TINY_MUSHROOM => 50,
        items::BIG_MUSHROOM => 500,
        items::PEARL => 140,
        items::BIG_PEARL => 750,
        items::STARDUST => 200,
        items::STAR_PIECE => 980,
        items::NUGGET => 1000,
        items::HEART_SCALE => 500,
        items::HONEY => 50,
        items::GROWTH_MULCH => 20,
        items::DAMP_MULCH => 20,
        items::STABLE_MULCH => 20,
        items::GOOEY_MULCH => 20,
        items::ROOT_FOSSIL => 500,
        items::CLAW_FOSSIL => 500,
        items::HELIX_FOSSIL => 500,
        items::DOME_FOSSIL => 500,
        items::OLD_AMBER => 800,
        items::ARMOR_FOSSIL => 500,
        items::SKULL_FOSSIL => 500,
        items::RARE_BONE => 1000,
        items::SHINY_STONE => 300,
        items::DUSK_STONE => 300,
        items::DAWN_STONE => 300,
        items::OVAL_STONE => 300,
        items::ODD_KEYSTONE => 210,
        items::GRISEOUS_ORB => 1000,
        items::TEA => 0,
        items::UNUSED114 => 0,
        items::AUTOGRAPH => 0,
        items::DOUSE_DRIVE => 100,
        items::SHOCK_DRIVE => 100,
        items::BURN_DRIVE => 100,
        items::CHILL_DRIVE => 100,
        items::UNUSED120 => 0,
        items::POKEMON_BOX => 0,
        items::MEDICINE_POCKET => 0,
        items::TM_CASE => 0,
        items::CANDY_JAR => 0,
        items::POWER_UP_POCKET => 0,
        items::CLOTHING_TRUNK => 0,
        items::CATCHING_POCKET => 0,
        items::BATTLE_POCKET => 0,
        items::UNUSED129 => 0,
        items::UNUSED130 => 0,
        items::UNUSED131 => 0,
        items::UNUSED132 => 0,
        items::UNUSED133 => 0,
        items::SWEET_HEART => 15,
        items::ADAMANT_ORB => 1000,
        items::LUSTROUS_ORB => 1000,
        items::MAIL1 => 5,
        items::MAIL2 => 5,
        items::MAIL3 => 5,
        items::MAIL4 => 5,
        items::MAIL5 => 5,
        items::MAIL6 => 5,
        items::MAIL7 => 5,
        items::MAIL8 => 5,
        items::MAIL9 => 5,
        items::MAIL10 => 5,
        items::MAIL11 => 5,
        items::MAIL12 => 5,
        items::CHERI_BERRY => 20,
        items::CHESTO_BERRY => 25,
        items::PECHA_BERRY => 10,
        items::RAWST_BERRY => 25,
        items::ASPEAR_BERRY => 25,
        items::LEPPA_BERRY => 300,
        items::ORAN_BERRY => 5,
        items::PERSIM_BERRY => 20,
        items::LUM_BERRY => 50,
        items::SITRUS_BERRY => 50,
        items::FIGY_BERRY => 10,
        items::WIKI_BERRY => 10,
        items::MAGO_BERRY => 10,
        items::AGUAV_BERRY => 10,
        items::IAPAPA_BERRY => 10,
        items::RAZZ_BERRY => 50,
        items::BLUK_BERRY => 50,
        items::NANAB_BERRY => 50,
        items::WEPEAR_BERRY => 50,
        items::PINAP_BERRY => 50,
        items::POMEG_BERRY => 50,
        items::KELPSY_BERRY => 50,
        items::QUALOT_BERRY => 50,
        items::HONDEW_BERRY => 50,
        items::GREPA_BERRY => 50,
        items::TAMATO_BERRY => 50,
        items::CORNN_BERRY => 50,
        items::MAGOST_BERRY => 50,
        items::RABUTA_BERRY => 50,
        items::NOMEL_BERRY => 50,
        items::SPELON_BERRY => 50,
        items::PAMTRE_BERRY => 50,
        items::WATMEL_BERRY => 50,
        items::DURIN_BERRY => 50,
        items::BELUE_BERRY => 50,
        items::OCCA_BERRY => 100,
        items::PASSHO_BERRY => 100,
        items::WACAN_BERRY => 100,
        items::RINDO_BERRY => 100,
        items::YACHE_BERRY => 100,
        items::CHOPLE_BERRY => 100,
        items::KEBIA_BERRY => 100,
        items::SHUCA_BERRY => 100,
        items::COBA_BERRY => 100,
        items::PAYAPA_BERRY => 100,
        items::TANGA_BERRY => 100,
        items::CHARTI_BERRY => 100,
        items::KASIB_BERRY => 100,
        items::HABAN_BERRY => 100,
        items::COLBUR_BERRY => 100,
        items::BABIRI_BERRY => 100,
        items::CHILAN_BERRY => 100,
        items::LIECHI_BERRY => 100,
        items::GANLON_BERRY => 100,
        items::SALAC_BERRY => 100,
        items::PETAYA_BERRY => 100,
        items::APICOT_BERRY => 100,
        items::LANSAT_BERRY => 100,
        items::STARF_BERRY => 100,
        items::ENIGMA_BERRY => 100,
        items::MICLE_BERRY => 100,
        items::CUSTAP_BERRY => 100,
        items::JABOCA_BERRY => 100,
        items::ROWAP_BERRY => 100,
        items::BRIGHT_POWDER => 300,
        items::WHITE_HERB => 100,
        items::MACHO_BRACE => 300,
        items::EXP_SHARE => 600,
        items::QUICK_CLAW => 450,
        items::SOOTHE_BELL => 100,
        items::MENTAL_HERB => 100,
        items::CHOICE_BAND => 1000,
        items::KINGS_ROCK => 500,
        items::SILVER_POWDER => 200,
        items::AMULET_COIN => 1500,
        items::CLEANSE_TAG => 100,
        items::SOUL_DEW => 20,
        items::DEEP_SEA_TOOTH => 300,
        items::DEEP_SEA_SCALE => 300,
        items::SMOKE_BALL => 20,
        items::EVERSTONE => 20,
        items::FOCUS_BAND => 300,
        items::LUCKY_EGG => 1000,
        items::SCOPE_LENS => 500,
        items::METAL_COAT => 300,
        items::LEFTOVERS => 1000,
        items::DRAGON_SCALE => 300,
        items::LIGHT_BALL => 10,
        items::SOFT_SAND => 200,
        items::HARD_STONE => 200,
        items::MIRACLE_SEED => 200,
        items::BLACK_GLASSES => 200,
        items::BLACK_BELT => 200,
        items::MAGNET => 200,
        items::MYSTIC_WATER => 200,
        items::SHARP_BEAK => 200,
        items::POISON_BARB => 200,
        items::NEVER_MELT_ICE => 200,
        items::SPELL_TAG => 200,
        items::TWISTED_SPOON => 200,
        items::CHARCOAL => 200,
        items::DRAGON_FANG => 200,
        items::SILK_SCARF => 200,
        items::UPGRADE => 300,
        items::SHELL_BELL => 600,
        items::SEA_INCENSE => 200,
        items::LAX_INCENSE => 300,
        items::LUCKY_PUNCH => 1,
        items::METAL_POWDER => 1,
        items::THICK_CLUB => 50,
        items::LEEK => 20,
        items::RED_SCARF => 10,
        items::BLUE_SCARF => 10,
        items::PINK_SCARF => 10,
        items::GREEN_SCARF => 10,
        items::YELLOW_SCARF => 10,
        items::WIDE_LENS => 150,
        items::MUSCLE_BAND => 200,
        items::WISE_GLASSES => 200,
        items::EXPERT_BELT => 600,
        items::LIGHT_CLAY => 150,
        items::LIFE_ORB => 1000,
        items::POWER_HERB => 100,
        items::TOXIC_ORB => 150,
        items::FLAME_ORB => 150,
        items::QUICK_POWDER => 1,
        items::FOCUS_SASH => 200,
        items::ZOOM_LENS => 150,
        items::METRONOME => 300,
        items::IRON_BALL => 100,
        items::LAGGING_TAIL => 100,
        items::DESTINY_KNOT => 150,
        items::BLACK_SLUDGE => 500,
        items::ICY_ROCK => 20,
        items::SMOOTH_ROCK => 20,
        items::HEAT_ROCK => 20,
        items::DAMP_ROCK => 20,
        items::GRIP_CLAW => 150,
        items::CHOICE_SCARF => 1000,
        items::STICKY_BARB => 150,
        items::POWER_BRACER => 300,
        items::POWER_BELT => 300,
        items::POWER_LENS => 300,
        items::POWER_BAND => 300,
        items::POWER_ANKLET => 300,
        items::POWER_WEIGHT => 300,
        items::SHED_SHELL => 50,
        items::BIG_ROOT => 150,
        items::CHOICE_SPECS => 1000,
        items::FLAME_PLATE => 200,
        items::SPLASH_PLATE => 200,
        items::ZAP_PLATE => 200,
        items::MEADOW_PLATE => 200,
        items::ICICLE_PLATE => 200,
        items::FIST_PLATE => 200,
        items::TOXIC_PLATE => 200,
        items::EARTH_PLATE => 200,
        items::SKY_PLATE => 200,
        items::MIND_PLATE => 200,
        items::INSECT_PLATE => 200,
        items::STONE_PLATE => 200,
        items::SPOOKY_PLATE => 200,
        items::DRACO_PLATE => 200,
        items::DREAD_PLATE => 200,
        items::IRON_PLATE => 200,
        items::ODD_INCENSE => 200,
        items::ROCK_INCENSE => 200,
        items::FULL_INCENSE => 100,
        items::WAVE_INCENSE => 200,
        items::ROSE_INCENSE => 200,
        items::LUCK_INCENSE => 1500,
        items::PURE_INCENSE => 100,
        items::PROTECTOR => 300,
        items::ELECTIRIZER => 300,
        items::MAGMARIZER => 300,
        items::DUBIOUS_DISC => 300,
        items::REAPER_CLOTH => 300,
        items::RAZOR_CLAW => 500,
        items::RAZOR_FANG => 500,
        items::TM01 => 1000,
        items::TM02 => 1000,
        items::TM03 => 1000,
        items::TM04 => 1000,
        items::TM05 => 1000,
        items::TM06 => 1000,
        items::TM07 => 2000,
        items::TM08 => 1000,
        items::TM09 => 1000,
        items::TM10 => 1000,
        items::TM11 => 2000,
        items::TM12 => 1000,
        items::TM13 => 1000,
        items::TM14 => 2000,
        items::TM15 => 2000,
        items::TM16 => 2000,
        items::TM17 => 1000,
        items::TM18 => 2000,
        items::TM19 => 1000,
        items::TM20 => 2000,
        items::TM21 => 1000,
        items::TM22 => 1000,
        items::TM23 => 1000,
        items::TM24 => 1000,
        items::TM25 => 2000,
        items::TM26 => 1000,
        items::TM27 => 1000,
        items::TM28 => 1000,
        items::TM29 => 1000,
        items::TM30 => 1000,
        items::TM31 => 1000,
        items::TM32 => 1000,
        items::TM33 => 2000,
        items::TM34 => 1000,
        items::TM35 => 1000,
        items::TM36 => 1000,
        items::TM37 => 2000,
        items::TM38 => 2000,
        items::TM39 => 1000,
        items::TM40 => 1000,
        items::TM41 => 1000,
        items::TM42 => 1000,
        items::TM43 => 1000,
        items::TM44 => 1000,
        items::TM45 => 1000,
        items::TM46 => 1000,
        items::TM47 => 1000,
        items::TM48 => 1000,
        items::TM49 => 1000,
        items::TM50 => 1000,
        items::TM51 => 1000,
        items::TM52 => 1000,
        items::TM53 => 1000,
        items::TM54 => 1000,
        items::TM55 => 1000,
        items::TM56 => 1000,
        items::TM57 => 1000,
        items::TM58 => 1000,
        items::TM59 => 1000,
        items::TM60 => 1000,
        items::TM61 => 1000,
        items::TM62 => 1000,
        items::TM63 => 1000,
        items::TM64 => 1000,
        items::TM65 => 1000,
        items::TM66 => 1000,
        items::TM67 => 1000,
        items::TM68 => 2000,
        items::TM69 => 1000,
        items::TM70 => 1000,
        items::TM71 => 1000,
        items::TM72 => 1000,
        items::TM73 => 1000,
        items::TM74 => 1000,
        items::TM75 => 1000,
        items::TM76 => 1000,
        items::TM77 => 1000,
        items::TM78 => 1000,
        items::TM79 => 1000,
        items::TM80 => 1000,
        items::TM81 => 1000,
        items::TM82 => 1000,
        items::TM83 => 1000,
        items::TM84 => 1000,
        items::TM85 => 1000,
        items::TM86 => 1000,
        items::TM87 => 1000,
        items::TM88 => 1000,
        items::TM89 => 1000,
        items::TM90 => 1000,
        items::TM91 => 1000,
        items::TM92 => 1000,
        items::HM01 => 0,
        items::HM02 => 0,
        items::HM03 => 0,
        items::HM04 => 0,
        items::HM05 => 0,
        items::HM06 => 0,
        items::HM07 => 0,
        items::HM08 => 0,
        items::EXPLORER_KIT => 0,
        items::LOOT_SACK => 0,
        items::RULE_BOOK => 0,
        items::POKE_RADAR => 0,
        items::POINT_CARD => 0,
        items::JOURNAL => 0,
        items::SEAL_CASE => 0,
        items::FASHION_CASE => 0,
        items::SEAL_BAG => 0,
        items::PAL_PAD => 0,
        items::WORKS_KEY => 0,
        items::OLD_CHARM => 0,
        items::GALACTIC_KEY => 0,
        items::RED_CHAIN => 0,
        items::TOWN_MAP => 0,
        items::VS_SEEKER => 0,
        items::COIN_CASE => 0,
        items::OLD_ROD => 0,
        items::GOOD_ROD => 0,
        items::SUPER_ROD => 0,
        items::SPRAYDUCK => 0,
        items::POFFIN_CASE => 0,
        items::BIKE => 0,
        items::SUITE_KEY => 0,
        items::OAKS_LETTER => 0,
        items::LUNAR_WING => 0,
        items::MEMBER_CARD => 0,
        items::AZURE_FLUTE => 0,
        items::SS_TICKET_JOHTO => 0,
        items::CONTEST_PASS => 0,
        items::MAGMA_STONE => 0,
        items::PARCEL_SINNOH => 0,
        items::COUPON1 => 0,
        items::COUPON2 => 0,
        items::COUPON3 => 0,
        items::STORAGE_KEY_SINNOH => 0,
        items::SECRET_POTION => 0,
        items::VS_RECORDER => 0,
        items::GRACIDEA => 0,
        items::SECRET_KEY_SINNOH => 0,
        items::APRICORN_BOX => 0,
        items::UNOWN_REPORT => 0,
        items::BERRY_POTS => 0,
        items::DOWSING_MACHINE => 0,
        items::BLUE_CARD => 0,
        items::SLOWPOKE_TAIL => 0,
        items::CLEAR_BELL => 0,
        items::CARD_KEY_JOHTO => 0,
        items::BASEMENT_KEY_JOHTO => 0,
        items::SQUIRT_BOTTLE => 0,
        items::RED_SCALE => 0,
        items::LOST_ITEM => 0,
        items::PASS => 0,
        items::MACHINE_PART => 0,
        items::SILVER_WING => 0,
        items::RAINBOW_WING => 0,
        items::MYSTERY_EGG => 0,
        items::RED_APRICORN => 2,
        items::BLUE_APRICORN => 2,
        items::YELLOW_APRICORN => 2,
        items::GREEN_APRICORN => 2,
        items::PINK_APRICORN => 2,
        items::WHITE_APRICORN => 2,
        items::BLACK_APRICORN => 2,
        items::FAST_BALL => 30,
        items::LEVEL_BALL => 30,
        items::LURE_BALL => 30,
        items::HEAVY_BALL => 30,
        items::LOVE_BALL => 30,
        items::FRIEND_BALL => 30,
        items::MOON_BALL => 30,
        items::SPORT_BALL => 30,
        items::PARK_BALL => 0,
        items::PHOTO_ALBUM => 0,
        items::GB_SOUNDS => 0,
        items::TIDAL_BELL => 0,
        items::RAGE_CANDY_BAR => 1500,
        items::DATA_CARD01 => 0,
        items::DATA_CARD02 => 0,
        items::DATA_CARD03 => 0,
        items::DATA_CARD04 => 0,
        items::DATA_CARD05 => 0,
        items::DATA_CARD06 => 0,
        items::DATA_CARD07 => 0,
        items::DATA_CARD08 => 0,
        items::DATA_CARD09 => 0,
        items::DATA_CARD10 => 0,
        items::DATA_CARD11 => 0,
        items::DATA_CARD12 => 0,
        items::DATA_CARD13 => 0,
        items::DATA_CARD14 => 0,
        items::DATA_CARD15 => 0,
        items::DATA_CARD16 => 0,
        items::DATA_CARD17 => 0,
        items::DATA_CARD18 => 0,
        items::DATA_CARD19 => 0,
        items::DATA_CARD20 => 0,
        items::DATA_CARD21 => 0,
        items::DATA_CARD22 => 0,
        items::DATA_CARD23 => 0,
        items::DATA_CARD24 => 0,
        items::DATA_CARD25 => 0,
        items::DATA_CARD26 => 0,
        items::DATA_CARD27 => 0,
        items::JADE_ORB => 0,
        items::LOCK_CAPSULE => 0,
        items::RED_ORB => 0,
        items::BLUE_ORB => 0,
        items::ENIGMA_STONE => 0,
        items::PRISM_SCALE => 300,
        items::EVIOLITE => 1000,
        items::FLOAT_STONE => 100,
        items::ROCKY_HELMET => 600,
        items::AIR_BALLOON => 100,
        items::RED_CARD => 100,
        items::RING_TARGET => 100,
        items::BINDING_BAND => 200,
        items::ABSORB_BULB => 100,
        items::CELL_BATTERY => 100,
        items::EJECT_BUTTON => 100,
        items::FIRE_GEM => 100,
        items::WATER_GEM => 100,
        items::ELECTRIC_GEM => 100,
        items::GRASS_GEM => 100,
        items::ICE_GEM => 100,
        items::FIGHTING_GEM => 100,
        items::POISON_GEM => 100,
        items::GROUND_GEM => 100,
        items::FLYING_GEM => 100,
        items::PSYCHIC_GEM => 100,
        items::BUG_GEM => 100,
        items::ROCK_GEM => 100,
        items::GHOST_GEM => 100,
        items::DRAGON_GEM => 100,
        items::DARK_GEM => 100,
        items::STEEL_GEM => 100,
        items::NORMAL_GEM => 100,
        items::HEALTH_FEATHER => 300,
        items::MUSCLE_FEATHER => 300,
        items::RESIST_FEATHER => 300,
        items::GENIUS_FEATHER => 300,
        items::CLEVER_FEATHER => 300,
        items::SWIFT_FEATHER => 300,
        items::PRETTY_FEATHER => 20,
        items::COVER_FOSSIL => 500,
        items::PLUME_FOSSIL => 500,
        items::LIBERTY_PASS => 0,
        items::PASS_ORB => 20,
        items::DREAM_BALL => 100,
        items::POKE_TOY => 100,
        items::PROP_CASE => 0,
        items::DRAGON_SKULL => 0,
        items::BALM_MUSHROOM => 0,
        items::BIG_NUGGET => 0,
        items::PEARL_STRING => 0,
        items::COMET_SHARD => 0,
        items::RELIC_COPPER => 0,
        items::RELIC_SILVER => 0,
        items::RELIC_GOLD => 0,
        items::RELIC_VASE => 0,
        items::RELIC_BAND => 0,
        items::RELIC_STATUE => 0,
        items::RELIC_CROWN => 0,
        items::CASTELIACONE => 45,
        items::DIRE_HIT2 => 0,
        items::X_SPEED2 => 0,
        items::X_SP_ATK2 => 0,
        items::X_SP_DEF2 => 0,
        items::X_DEFENSE2 => 0,
        items::X_ATTACK2 => 0,
        items::X_ACCURACY2 => 0,
        items::X_SPEED3 => 0,
        items::X_SP_ATK3 => 0,
        items::X_SP_DEF3 => 0,
        items::X_DEFENSE3 => 0,
        items::X_ATTACK3 => 0,
        items::X_ACCURACY3 => 0,
        items::X_SPEED6 => 0,
        items::X_SP_ATK6 => 0,
        items::X_SP_DEF6 => 0,
        items::X_DEFENSE6 => 0,
        items::X_ATTACK6 => 0,
        items::X_ACCURACY6 => 0,
        items::ABILITY_URGE => 0,
        items::ITEM_DROP => 0,
        items::ITEM_URGE => 0,
        items::RESET_URGE => 0,
        items::DIRE_HIT3 => 0,
        items::LIGHT_STONE => 0,
        items::DARK_STONE => 0,
        items::TM93 => 1000,
        items::TM94 => 1000,
        items::TM95 => 1000,
        items::XTRANSCEIVER_MALE => 0,
        items::UNUSED622 => 0,
        items::GRAM1 => 0,
        items::GRAM2 => 0,
        items::GRAM3 => 0,
        items::XTRANSCEIVER_FEMALE => 0,
        items::MEDAL_BOX => 0,
        items::D_NASPLICERS_FUSE => 0,
        items::D_NASPLICERS_SEPARATE => 0,
        items::PERMIT => 0,
        items::OVAL_CHARM => 0,
        items::SHINY_CHARM => 0,
        items::PLASMA_CARD => 0,
        items::GRUBBY_HANKY => 0,
        items::COLRESS_MACHINE => 0,
        items::DROPPED_ITEM_CURTIS => 0,
        items::DROPPED_ITEM_YANCY => 0,
        items::REVEAL_GLASS => 0,
        _ => return None,
    };
    Some(price)
}

pub const BW1_WILD_FILE_TO_AREA_MAP: &[i32] = &[
    2, 6, 8, 18, 18, 19, 19, 20, 20, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21,
    21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 22, 23, 23, 23, 24, 24, 24, 24,
    25, 25, 25, 25, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 26, 27, 27, 27, 27,
    29, 36, 57, 59, 60, 38, 39, 40, 30, 30, 41, 42, 43, 31, 31, 31, 44, 33, 33, 33, 33, 45, 34, 46,
    32, 32, 32, 47, 47, 48, 49, 50, 51, 35, 52, 53, 37, 55, 12, 54,
];

pub const BW2_WILD_FILE_TO_AREA_MAP: &[i32] = &[
    2, 4, 8, 59, 61, 63, 19, 19, 20, 20, 21, 21, 22, 22, 22, 22, 22, 22, 22, 22, 24, 24, 24, 25,
    25, 25, 25, 26, 26, 26, 26, 76, 27, 27, 27, 27, 27, 70, 70, 70, 70, 70, 29, 35, 71, 71, 72, 72,
    73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 73, 74, 74, 74, 74, 74, 74, 74, 74, 74, 74, 76,
    76, 76, 76, 76, 76, 76, 76, 76, 76, 77, 77, 77, 79, 79, 79, 79, 79, 79, 79, 79, 79, 78, 78, -1,
    55, 57, 58, 37, 38, 39, 30, 30, 40, 40, 41, 42, 31, 31, 31, 43, 32, 32, 32, 32, 44, 33, 45, 46,
    47, 48, 49, 34, 50, 51, 36, 53, 66, 67, 69, 75, 12, 52, 68,
];

fn is_in_bounds(trainers: &[Trainer], number: i32) -> bool {
    trainers.len() as i32 > number - 1
}

fn tag_numbers(trainers: &mut [Trainer], tag: &str, numbers: &[i32]) {
    for &number in numbers {
        if is_in_bounds(trainers, number) {
            trainers[(number - 1) as usize].tag = Some(tag.to_owned());
        }
    }
}

fn tag_rival(trainers: &mut [Trainer], tag: &str, offset: i32) {
    trainers[(offset - 1) as usize].tag = Some(format!("{tag}-0"));
    trainers[offset as usize].tag = Some(format!("{tag}-1"));
    trainers[(offset + 1) as usize].tag = Some(format!("{tag}-2"));
}

pub fn tag_trainers_bw(trainers: &mut [Trainer]) {
    tag_numbers(trainers, "GYM11", &[0x09, 0x0A]);
    tag_numbers(trainers, "GYM2", &[0x56, 0x57, 0x58]);
    tag_numbers(trainers, "GYM3", &[0xC4, 0xC6, 0xC7, 0xC8]);
    tag_numbers(trainers, "GYM4", &[0x42, 0x43, 0x44, 0x45]);
    tag_numbers(trainers, "GYM5", &[0xC9, 0xCA, 0xCB, 0x5F, 0xA8]);
    tag_numbers(trainers, "GYM6", &[0x7D, 0x7F, 0x80, 0x46, 0x47]);
    tag_numbers(trainers, "GYM7", &[0xD7, 0xD8, 0xD9, 0xD4, 0xD5, 0xD6]);
    tag_numbers(
        trainers,
        "GYM8",
        &[0x109, 0x10A, 0x10F, 0x10E, 0x110, 0x10B, 0x113, 0x112],
    );
    tag_numbers(trainers, "GYM1-LEADER", &[0x0C]);
    tag_numbers(trainers, "GYM9-LEADER", &[0x0B]);
    tag_numbers(trainers, "GYM10-LEADER", &[0x0D]);
    tag_numbers(trainers, "GYM2-LEADER", &[0x15]);
    tag_numbers(trainers, "GYM3-LEADER", &[0x16]);
    tag_numbers(trainers, "GYM4-LEADER", &[0x17]);
    tag_numbers(trainers, "GYM5-LEADER", &[0x18]);
    tag_numbers(trainers, "GYM6-LEADER", &[0x19]);
    tag_numbers(trainers, "GYM7-LEADER", &[0x83]);
    tag_numbers(trainers, "GYM8-LEADER", &[0x84]);
    tag_numbers(trainers, "GYM8-LEADER", &[0x85]);
    tag_numbers(trainers, "ELITE1", &[0xE4]);
    tag_numbers(trainers, "ELITE2", &[0xE6]);
    tag_numbers(trainers, "ELITE3", &[0xE7]);
    tag_numbers(trainers, "ELITE4", &[0xE5]);
    tag_numbers(trainers, "ELITE1", &[0x233]);
    tag_numbers(trainers, "ELITE2", &[0x235]);
    tag_numbers(trainers, "ELITE3", &[0x236]);
    tag_numbers(trainers, "ELITE4", &[0x234]);
    tag_numbers(trainers, "CHAMPION", &[0x197]);
    tag_numbers(trainers, "UBER", &[0x21E]);
    tag_numbers(trainers, "UBER", &[0x237]);
    tag_numbers(trainers, "UBER", &[0xE8]);
    tag_numbers(trainers, "UBER", &[0x24A]);
    tag_numbers(trainers, "UBER", &[0x24B]);
    tag_rival(trainers, "RIVAL1", 0x35);
    tag_rival(trainers, "RIVAL2", 0x11F);
    tag_rival(trainers, "RIVAL3", 0x38);
    tag_rival(trainers, "RIVAL4", 0x193);
    tag_rival(trainers, "RIVAL5", 0x5A);
    tag_rival(trainers, "RIVAL6", 0x21B);
    tag_rival(trainers, "RIVAL7", 0x24C);
    tag_rival(trainers, "RIVAL8", 0x24F);
    tag_rival(trainers, "FRIEND1", 0x3B);
    tag_rival(trainers, "FRIEND2", 0x1F2);
    tag_rival(trainers, "FRIEND3", 0x1FB);
    tag_rival(trainers, "FRIEND4", 0x1EB);
    tag_rival(trainers, "FRIEND5", 0x1EE);
    tag_rival(trainers, "FRIEND6", 0x252);
    tag_numbers(trainers, "NOTSTRONG", &[64]);
    tag_numbers(trainers, "STRONG", &[65, 89, 218]);
}

pub fn tag_trainers_bw2(trainers: &mut [Trainer]) {
    tag_numbers(trainers, "GYM1", &[0xab, 0xac]);
    tag_numbers(trainers, "GYM2", &[0xb2, 0xb3]);
    tag_numbers(trainers, "GYM3", &[0x2de, 0x2df, 0x2e0, 0x2e1]);
    tag_numbers(trainers, "GYM4", &[0x26d, 0x94, 0xcf, 0xd0, 0xd1]);
    tag_numbers(
        trainers,
        "GYM5",
        &[0x13f, 0x140, 0x141, 0x142, 0x143, 0x144, 0x145],
    );
    tag_numbers(trainers, "GYM6", &[0x95, 0x96, 0x97, 0x98, 0x14c]);
    tag_numbers(trainers, "GYM7", &[0x17d, 0x17e, 0x17f, 0x180, 0x181]);
    tag_numbers(
        trainers,
        "GYM8",
        &[0x15e, 0x15f, 0x160, 0x161, 0x162, 0x163],
    );
    tag_numbers(trainers, "GYM1-LEADER", &[0x9c, 0x2fc]);
    tag_numbers(trainers, "GYM2-LEADER", &[0x9d, 0x2fd]);
    tag_numbers(trainers, "GYM3-LEADER", &[0x9a, 0x2fe]);
    tag_numbers(trainers, "GYM4-LEADER", &[0x99, 0x2ff]);
    tag_numbers(trainers, "GYM5-LEADER", &[0x9e, 0x300]);
    tag_numbers(trainers, "GYM6-LEADER", &[0x9b, 0x301]);
    tag_numbers(trainers, "GYM7-LEADER", &[0x9f, 0x302]);
    tag_numbers(trainers, "GYM8-LEADER", &[0xa0, 0x303]);
    tag_numbers(trainers, "ELITE1", &[0x26, 0x304, 0x8f, 0x309]);
    tag_numbers(trainers, "ELITE2", &[0x28, 0x305, 0x91, 0x30a]);
    tag_numbers(trainers, "ELITE3", &[0x29, 0x307, 0x92, 0x30c]);
    tag_numbers(trainers, "ELITE4", &[0x27, 0x306, 0x90, 0x30b]);
    tag_numbers(trainers, "CHAMPION", &[0x155, 0x308, 0x218, 0x30d]);
    tag_rival(trainers, "RIVAL1", 0xa1);
    tag_rival(trainers, "RIVAL2", 0xa6);
    tag_rival(trainers, "RIVAL3", 0x24c);
    tag_rival(trainers, "RIVAL4", 0x170);
    tag_rival(trainers, "RIVAL5", 0x17a);
    tag_rival(trainers, "RIVAL6", 0x2bd);
    tag_rival(trainers, "RIVAL7", 0x31a);
    tag_rival(trainers, "RIVAL8", 0x2ac);
    tag_rival(trainers, "RIVAL9", 0x2b5);
    tag_rival(trainers, "RIVAL10", 0x2b8);
    tag_rival(trainers, "FRIEND1", 0x168);
    tag_rival(trainers, "FRIEND1", 0x16b);
    tag_numbers(trainers, "GYM1", &[0x173, 0x278, 0x32E]);
    tag_numbers(trainers, "GYM9-LEADER", &[0x1f0]);
    tag_numbers(trainers, "GYM10-LEADER", &[0x1ee]);
    tag_numbers(trainers, "GYM11-LEADER", &[0x1ef]);
    tag_numbers(
        trainers,
        "THEMED:ZINZOLIN-STRONG",
        &[0x2c0, 0x248, 0x15b, 0x1f1],
    );
    tag_numbers(
        trainers,
        "THEMED:COLRESS-STRONG",
        &[0x166, 0x158, 0x32d, 0x32f],
    );
    tag_numbers(trainers, "THEMED:SHADOW1", &[0x247, 0x15c, 0x2af]);
    tag_numbers(trainers, "THEMED:SHADOW2", &[0x1f2, 0x2b0]);
    tag_numbers(trainers, "THEMED:SHADOW3", &[0x1f3, 0x2b1]);
    tag_numbers(trainers, "UBER", &[0x246]);
    tag_numbers(trainers, "UBER", &[0x1c8]);
    tag_numbers(trainers, "UBER", &[0xca]);
    tag_numbers(trainers, "UBER", &[0xc9]);
    tag_numbers(trainers, "UBER", &[0x5]);
    tag_numbers(trainers, "UBER", &[0x6]);
    tag_numbers(trainers, "UBER", &[0x30e]);
    tag_numbers(trainers, "UBER", &[0x30f]);
    tag_numbers(trainers, "UBER", &[0x310]);
    tag_numbers(trainers, "UBER", &[0x311]);
    tag_numbers(trainers, "UBER", &[0x159]);
    tag_numbers(trainers, "UBER", &[0x8c]);
    tag_numbers(trainers, "UBER", &[0x24f]);
}

fn set_multi_battle_status(trainers: &mut [Trainer], status: MultiBattleStatus, numbers: &[i32]) {
    for &number in numbers {
        if is_in_bounds(trainers, number) {
            trainers[(number - 1) as usize].multi_battle_status = status;
        }
    }
}

pub fn set_multi_battle_status_bw(trainers: &mut [Trainer]) {
    set_multi_battle_status(trainers, MultiBattleStatus::Always, &[62, 63, 401, 402]);
}

pub fn set_multi_battle_status_bw2(trainers: &mut [Trainer], is_black2: bool) {
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Always,
        &[
            342, 347, 356, 374, 375, 376, 377, 494, 495, 496, 614, 615, 643, 644, 704, 705, 797,
            798, 799, 807, 809,
        ],
    );
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Potential,
        &[513, 522, 602, 603, 788, 790, 791, 792, 793],
    );
    if is_black2 {
        set_multi_battle_status(trainers, MultiBattleStatus::Always, &[521, 789]);
        set_multi_battle_status(trainers, MultiBattleStatus::Potential, &[520, 786]);
    } else {
        set_multi_battle_status(trainers, MultiBattleStatus::Potential, &[514, 521]);
        set_multi_battle_status(trainers, MultiBattleStatus::Always, &[519, 787]);
    }
}
