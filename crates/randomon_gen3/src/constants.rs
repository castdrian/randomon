use std::collections::BTreeMap;

use randomon_data::constants::{abilities, moves, species};
use randomon_data::{ItemList, MultiBattleStatus, Trainer, Type};

use crate::items;
use crate::rom_entry::RomType;

pub const SIZE_8M: usize = 0x80_0000;
pub const SIZE_16M: usize = 0x100_0000;
pub const SIZE_32M: usize = 0x200_0000;

pub const UNOFFICIAL_EMERALD_ROM_NAME: &str = "YJencrypted";
pub const UNOFFICIAL_EMERALD_ROM_CODE: &str = "BPET";
pub const UNOFFICIAL_EMERALD_HEADER_CHECKSUM: u8 = 0x66;

pub const ROM_NAME_OFFSET: usize = 0xA0;
pub const ROM_CODE_OFFSET: usize = 0xAC;
pub const ROM_VERSION_OFFSET: usize = 0xBC;
pub const HEADER_CHECKSUM_OFFSET: usize = 0xBD;

pub const DETECTION_PREFIX_BYTES: usize = 0x10_0000;

pub const WILD_POKEMON_POINTER_PREFIX: &str = "0348048009E00000FFFF0000";
pub const MAP_BANKS_POINTER_PREFIX: &str = "80180068890B091808687047";
pub const RS_POKEMON_NAMES_POINTER_SUFFIX: &str = "30B50025084CC8F7";
pub const FRLG_MAP_LABELS_POINTER_PREFIX: &str = "AC470000AE470000B0470000";
pub const RSE_MAP_LABELS_POINTER_PREFIX: &str = "C078288030BC01BC00470000";
pub const POKEDEX_ORDER_POINTER_PREFIX: &str = "0448814208D0481C0004000C05E00000";
pub const RS_FRONT_SPRITES_POINTER_PREFIX: &str = "05E0";
pub const RS_FRONT_SPRITES_POINTER_SUFFIX: &str = "1068191C";
pub const RS_POKEMON_PALETTES_POINTER_PREFIX: &str = "04D90148006817E0";
pub const RS_POKEMON_PALETTES_POINTER_SUFFIX: &str = "080C064A11404840";

const RUNNING_SHOES_CHECK_PREFIX_RS: &str = "0440002C1DD08620";
const RUNNING_SHOES_CHECK_PREFIX_FRLG: &str = "02200540002D29D0";
const RUNNING_SHOES_CHECK_PREFIX_E: &str = "0640002E1BD08C20";

pub const EFRLG_POKEMON_NAMES_POINTER: i32 = 0x144;
pub const EFRLG_MOVE_NAMES_POINTER: i32 = 0x148;
pub const EFRLG_ABILITY_NAMES_POINTER: i32 = 0x1C0;
pub const EFRLG_ITEM_DATA_POINTER: i32 = 0x1C8;
pub const EFRLG_MOVE_DATA_POINTER: i32 = 0x1CC;
pub const EFRLG_POKEMON_STATS_POINTER: i32 = 0x1BC;
pub const EFRLG_FRONT_SPRITES_POINTER: i32 = 0x128;
pub const EFRLG_POKEMON_PALETTES_POINTER: i32 = 0x130;

pub const EMPTY_POKEMON_SIG: [u8; 28] = [
    0x32, 0x96, 0x32, 0x96, 0x96, 0x32, 0x00, 0x00, 0x03, 0x01, 0xAA, 0x0A, 0x00, 0x00, 0x00, 0x00,
    0xFF, 0x78, 0x00, 0x00, 0x0F, 0x0F, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00,
];

pub const BASE_STATS_ENTRY_SIZE: i32 = 0x1C;
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
pub const BS_GENDER_RATIO_OFFSET: i32 = 16;
pub const BS_GROWTH_CURVE_OFFSET: i32 = 19;
pub const BS_ABILITY1_OFFSET: i32 = 22;
pub const BS_ABILITY2_OFFSET: i32 = 23;

pub const MOVE_DATA_ENTRY_SIZE: i32 = 0xC;
pub const EVOLUTION_ENTRY_SIZE: i32 = 0x28;
pub const EVOLUTIONS_PER_POKEMON: i32 = 5;
pub const EVOLUTION_SLOT_SIZE: i32 = 8;
pub const TMHM_COMPAT_ENTRY_SIZE: i32 = 8;
pub const TRADE_ENTRY_LENGTH: i32 = 60;
pub const MAP_HEADER_LABEL_OFFSET: i32 = 0x14;
pub const PERSON_EVENT_SIZE: i32 = 24;
pub const SIGNPOST_EVENT_SIZE: i32 = 12;
pub const WILD_HEADER_SIZE: i32 = 20;
pub const MOSSDEEP_STEVEN_POKEMON_SIZE: i32 = 20;
pub const EGG_MOVES_SPECIES_MARKER: i32 = 20000;
pub const EGG_MOVES_TERMINATOR: i32 = 0xFFFF;

pub const FREE_SPACE_BYTE: u8 = 0xFF;

pub const RSE_STARTER2_OFFSET: i32 = 2;
pub const RSE_STARTER3_OFFSET: i32 = 4;
pub const FRLG_STARTER2_OFFSET: i32 = 515;
pub const FRLG_STARTER3_OFFSET: i32 = 461;
pub const FRLG_STARTER_REPEAT_OFFSET: i32 = 5;
pub const FRLG_BASE_STARTER1: i32 = 1;
pub const FRLG_BASE_STARTER2: i32 = 4;
pub const FRLG_BASE_STARTER3: i32 = 7;
pub const FRLG_STARTER_ITEMS_OFFSET: i32 = 218;

pub const GBA_ADD_RX_OPCODE: u8 = 0x30;
pub const GBA_UNCONDITIONAL_JUMP_OPCODE: i32 = 0xE0;
pub const GBA_SET_RX_OPCODE: u8 = 0x20;
pub const GBA_CMP_RX_OPCODE: i32 = 0x28;
pub const GBA_NOP_OPCODE: i32 = 0x46C0;
pub const GBA_R0: u8 = 0;
pub const GBA_R1: u8 = 1;
pub const GBA_R2: u8 = 2;
pub const GBA_R4: u8 = 4;

pub const GRASS_SLOTS: i32 = 12;
pub const SURFING_SLOTS: i32 = 5;
pub const ROCK_SMASH_SLOTS: i32 = 5;
pub const FISHING_SLOTS: i32 = 10;

pub const TM_COUNT: i32 = 50;
pub const HM_COUNT: i32 = 8;
pub const HM_MOVES: &[i32] = &[
    moves::CUT,
    moves::FLY,
    moves::SURF,
    moves::STRENGTH,
    moves::FLASH,
    moves::ROCK_SMASH,
    moves::WATERFALL,
    moves::DIVE,
];
pub const TM_ITEM_OFFSET: i32 = items::TM01;

pub const RSE_ITEM_DESC_CHARS_PER_LINE: i32 = 18;
pub const FRLG_ITEM_DESC_CHARS_PER_LINE: i32 = 24;
pub const REGULAR_TEXTBOX_CHARS_PER_LINE: i32 = 36;
pub const POINTER_SEARCH_RADIUS: i32 = 500;
pub const ITEM_DATA_DESCRIPTION_OFFSET: i32 = 0x14;
pub const ITEM_DATA_PRICE_OFFSET: i32 = 16;

pub const DEOXYS_OBEY_CODE: &str = "CD21490088420FD0";
pub const MEW_OBEY_OFFSET_FROM_DEOXYS_OBEY: i32 = 0x16;
pub const LEVEL_EVO_KANTO_DEX_CHECK_CODE: &str = "972814DD";
pub const STONE_EVO_KANTO_DEX_CHECK_CODE: &str = "972808D9";
pub const LEVEL_EVO_KANTO_DEX_JUMP_AMOUNT: i32 = 0x14;
pub const STONE_EVO_KANTO_DEX_JUMP_AMOUNT: i32 = 0x08;

pub const RS_POKEDEX_SCRIPT_IDENTIFIER: &str = "326629010803";
pub const RS_NAT_DEX_SCRIPT_PART1: &str = "31720167";
pub const RS_NAT_DEX_SCRIPT_PART2: &str =
    "32662901082B00801102006B02021103016B020211DABE4E020211675A6A02022A008003";
pub const FRLG_POKEDEX_SCRIPT_IDENTIFIER: &str = "292908258101";
pub const FRLG_NAT_DEX_SCRIPT: &str = "292908258101256F0103";
pub const FRLG_NAT_DEX_FLAG_CHECKER: &str = "260D809301210D800100";
pub const FRLG_E4_FLAG_CHECKER: &str = "2B2C0800000000000000";
pub const FRLG_OAKS_LAB_KANTO_DEX_CHECKER: &str = "257D011604800000260D80D400";
pub const FRLG_OAKS_LAB_FIX: &str = "257D011604800100";
pub const FRLG_OAK_OUTSIDE_HOUSE_CHECK: &str =
    "1604800000260D80D4001908800580190980068083000880830109802109803C";
pub const FRLG_OAK_OUTSIDE_HOUSE_FIX: &str = "1604800100";
pub const FRLG_OAK_AIDE_CHECK_PREFIX: &str = "00B5064800880028";
pub const E_POKEDEX_SCRIPT_IDENTIFIER: &str = "3229610825F00129E40816CD40010003";
pub const E_NAT_DEX_SCRIPT_PART1: &str = "31720167";
pub const E_NAT_DEX_SCRIPT_PART2: &str = "3229610825F00129E40825F30116CD40010003";

pub const FRIENDSHIP_VALUE_FOR_EVO_LOCATOR: &str = "DB2900D8";
pub const PERFECT_ODDS_BRANCH_LOCATOR: &str = "FE2E2FD90020";

pub const UNHACKED_MAX_POKEDEX: i32 = 411;
pub const UNHACKED_REAL_POKEDEX: i32 = 386;
pub const HOENN_POKES_START: i32 = 252;
pub const EVOLUTION_METHOD_COUNT: i32 = 15;
pub const CACOPHONY_INDEX: i32 = 76;
pub const AIR_LOCK_INDEX: i32 = 77;
pub const HIGHEST_ABILITY_INDEX: i32 = 77;
pub const EM_METEOR_FALLS_STEVEN_INDEX: i32 = 804;
pub const FRLG_MAP_LABELS_START: i32 = 0x58;
pub const MOSSDEEP_STEVEN_TEAM_SIZE: usize = 3;
pub const HAPPINESS_FOR_EVOLUTION: u8 = 219;
pub const EASIER_HAPPINESS_FOR_EVOLUTION: u8 = 159;
pub const HAPPINESS_DAY_LOCATOR_DISTANCE: i32 = 38;
pub const HAPPINESS_NIGHT_LOCATOR_DISTANCE: i32 = 66;
pub const RUNNING_SHOES_BRANCH_DISTANCE: i32 = 0x12;

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
pub const INCREASED_CRIT_EFFECT: i32 = 43;
pub const DAMAGE_RECOIL_25_PERCENT_EFFECT: i32 = 48;
pub const NO_DAMAGE_CONFUSION_EFFECT: i32 = 49;
pub const NO_DAMAGE_ATK_PLUS_TWO_EFFECT: i32 = 50;
pub const NO_DAMAGE_DEF_PLUS_TWO_EFFECT: i32 = 51;
pub const NO_DAMAGE_SPE_PLUS_TWO_EFFECT: i32 = 52;
pub const NO_DAMAGE_SP_ATK_PLUS_TWO_EFFECT: i32 = 53;
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
pub const DAMAGE_SP_ATK_MINUS_ONE_EFFECT: i32 = 71;
pub const DAMAGE_SP_DEF_MINUS_ONE_EFFECT: i32 = 72;
pub const DAMAGE_ACCURACY_MINUS_ONE_EFFECT: i32 = 73;
pub const SKY_ATTACK_EFFECT: i32 = 75;
pub const DAMAGE_CONFUSION_EFFECT: i32 = 76;
pub const TWINEEDLE_EFFECT: i32 = 77;
pub const RECHARGE_EFFECT: i32 = 80;
pub const SNORE_EFFECT: i32 = 92;
pub const TRAPPING_EFFECT: i32 = 106;
pub const MINIMIZE_EFFECT: i32 = 108;
pub const SWAGGER_EFFECT: i32 = 118;
pub const DAMAGE_BURN_AND_THAW_USER_EFFECT: i32 = 125;
pub const DAMAGE_USER_DEF_PLUS_ONE_EFFECT: i32 = 138;
pub const DAMAGE_USER_ATK_PLUS_ONE_EFFECT: i32 = 139;
pub const DAMAGE_USER_ALL_PLUS_ONE_EFFECT: i32 = 140;
pub const SKULL_BASH_EFFECT: i32 = 145;
pub const TWISTER_EFFECT: i32 = 146;
pub const FUTURE_SIGHT_AND_DOOM_DESIRE_EFFECT: i32 = 148;
pub const FLINCH_WITH_MINIMIZE_BONUS_EFFECT: i32 = 150;
pub const SOLARBEAM_EFFECT: i32 = 151;
pub const THUNDER_EFFECT: i32 = 152;
pub const SEMI_INVULNERABLE_EFFECT: i32 = 155;
pub const DEFENSE_CURL_EFFECT: i32 = 156;
pub const FAKE_OUT_EFFECT: i32 = 158;
pub const SPIT_UP_EFFECT: i32 = 161;
pub const FLATTER_EFFECT: i32 = 166;
pub const NO_DAMAGE_BURN_EFFECT: i32 = 167;
pub const CHARGE_EFFECT: i32 = 174;
pub const DAMAGE_USER_ATK_AND_DEF_MINUS_ONE_EFFECT: i32 = 182;
pub const DAMAGE_RECOIL_33_PERCENT_EFFECT: i32 = 198;
pub const TEETER_DANCE_EFFECT: i32 = 199;
pub const BLAZE_KICK_EFFECT: i32 = 200;
pub const POISON_FANG_EFFECT: i32 = 202;
pub const DAMAGE_USER_SP_ATK_MINUS_TWO_EFFECT: i32 = 204;
pub const NO_DAMAGE_ATK_AND_DEF_MINUS_ONE_EFFECT: i32 = 205;
pub const NO_DAMAGE_DEF_AND_SP_DEF_PLUS_ONE_EFFECT: i32 = 206;
pub const NO_DAMAGE_ATK_AND_DEF_PLUS_ONE_EFFECT: i32 = 208;
pub const POISON_TAIL_EFFECT: i32 = 209;
pub const NO_DAMAGE_SP_ATK_AND_SP_DEF_PLUS_ONE_EFFECT: i32 = 211;
pub const NO_DAMAGE_ATK_AND_SPE_PLUS_ONE_EFFECT: i32 = 212;

pub const SOUND_MOVES: &[i32] = &[
    moves::GROWL,
    moves::ROAR,
    moves::SING,
    moves::SUPERSONIC,
    moves::SCREECH,
    moves::SNORE,
    moves::UPROAR,
    moves::METAL_SOUND,
    moves::GRASS_WHISTLE,
    moves::HYPER_VOICE,
    moves::PERISH_SONG,
    moves::HEAL_BELL,
];

pub const RS_REQUIRED_FIELD_TMS: &[i32] = &[1, 2, 6, 7, 11, 18, 22, 23, 26, 30, 37, 48];
pub const E_REQUIRED_FIELD_TMS: &[i32] = &[2, 6, 7, 11, 18, 22, 23, 30, 37, 48];
pub const FRLG_REQUIRED_FIELD_TMS: &[i32] = &[
    1, 2, 7, 8, 9, 11, 12, 14, 17, 18, 21, 22, 25, 32, 36, 37, 40, 41, 44, 46, 47, 48, 49, 50,
];

pub const RSE_FIELD_MOVES: &[i32] = &[
    moves::CUT,
    moves::FLY,
    moves::SURF,
    moves::STRENGTH,
    moves::FLASH,
    moves::DIG,
    moves::TELEPORT,
    moves::WATERFALL,
    moves::ROCK_SMASH,
    moves::SWEET_SCENT,
    moves::DIVE,
    moves::SECRET_POWER,
];

pub const FRLG_FIELD_MOVES: &[i32] = &[
    moves::CUT,
    moves::FLY,
    moves::SURF,
    moves::STRENGTH,
    moves::FLASH,
    moves::DIG,
    moves::TELEPORT,
    moves::WATERFALL,
    moves::ROCK_SMASH,
    moves::SWEET_SCENT,
];

pub const RSE_EARLY_REQUIRED_HM_MOVES: &[i32] = &[moves::ROCK_SMASH];
pub const FRLG_EARLY_REQUIRED_HM_MOVES: &[i32] = &[moves::CUT];

const RS_SHOP_NAMES: &[&str] = &[
    "Slateport Vitamins",
    "Slateport TMs",
    "Oldale Poké Mart (Before Pokédex)",
    "Oldale Poké Mart (After Pokédex)",
    "Lavaridge Herbs",
    "Lavaridge Poké Mart",
    "Fallarbor Poké Mart",
    "Verdanturf Poké Mart",
    "Petalburg Poké Mart (Before 4 Badges)",
    "Petalburg Poké Mart (After 4 Badges)",
    "Slateport Poké Mart",
    "Mauville Poké Mart",
    "Rustboro Poké Mart (Before Delivering Devon Goods)",
    "Rustboro Poké Mart (After Delivering Devon Goods)",
    "Fortree Poké Mart",
    "Lilycove Department Store 2F Left",
    "Lilycove Department Store 2F Right",
    "Lilycove Department Store 3F Left",
    "Lilycove Department Store 3F Right",
    "Lilycove Department Store 4F Left (TMs)",
    "Lilycove Department Store 4F Right (TMs)",
    "Mossdeep Poké Mart",
    "Sootopolis Poké Mart",
    "Pokémon League Poké Mart",
];

const FRLG_SHOP_NAMES: &[&str] = &[
    "Trainer Tower Poké Mart",
    "Two Island Market Stall (Initial)",
    "Two Island Market Stall (After Saving Lostelle)",
    "Two Island Market Stall (After Hall of Fame)",
    "Two Island Market Stall (After Ruby/Sapphire Quest)",
    "Viridian Poké Mart",
    "Pewter Poké Mart",
    "Cerulean Poké Mart",
    "Lavender Poké Mart",
    "Vermillion Poké Mart",
    "Celadon Department 2F South",
    "Celadon Department 2F North (TMs)",
    "Celadon Department 4F",
    "Celadon Department 5F South",
    "Celadon Department 5F North",
    "Fuchsia Poké Mart",
    "Cinnabar Poké Mart",
    "Indigo Plateau Poké Mart",
    "Saffron Poké Mart",
    "Seven Island Poké Mart",
    "Three Island Poké Mart",
    "Four Island Poké Mart",
    "Six Island Poké Mart",
];

const EM_EXTRA_SHOP_NAMES: &[&str] = &[
    "Battle Frontier Poké Mart",
    "Trainer Hill Poké Mart (Before Hall of Fame)",
    "Trainer Hill Poké Mart (After Hall of Fame)",
];

pub fn shop_names(rom_type: RomType) -> Vec<&'static str> {
    match rom_type {
        RomType::Ruby | RomType::Sapphire => RS_SHOP_NAMES.to_vec(),
        RomType::FireRedLeafGreen => FRLG_SHOP_NAMES.to_vec(),
        RomType::Emerald => RS_SHOP_NAMES
            .iter()
            .chain(EM_EXTRA_SHOP_NAMES)
            .copied()
            .collect(),
    }
}

pub const EVOLUTION_ITEMS: &[i32] = &[
    items::SUN_STONE,
    items::MOON_STONE,
    items::FIRE_STONE,
    items::THUNDERSTONE,
    items::WATER_STONE,
    items::LEAF_STONE,
];

pub const X_ITEMS: &[i32] = &[
    items::GUARD_SPEC,
    items::DIRE_HIT,
    items::X_ATTACK,
    items::X_DEFEND,
    items::X_SPEED,
    items::X_ACCURACY,
    items::X_SPECIAL,
];

pub const CONSUMABLE_HELD_ITEMS: &[i32] = &[
    items::CHERI_BERRY,
    items::CHESTO_BERRY,
    items::PECHA_BERRY,
    items::RAWST_BERRY,
    items::RAWST_BERRY,
    items::LEPPA_BERRY,
    items::ORAN_BERRY,
    items::PERSIM_BERRY,
    items::LUM_BERRY,
    items::SITRUS_BERRY,
    items::FIGY_BERRY,
    items::WIKI_BERRY,
    items::MAGO_BERRY,
    items::AGUAV_BERRY,
    items::IAPAPA_BERRY,
    items::LIECHI_BERRY,
    items::GANLON_BERRY,
    items::SALAC_BERRY,
    items::PETAYA_BERRY,
    items::APICOT_BERRY,
    items::LANSAT_BERRY,
    items::STARF_BERRY,
    items::BERRY_JUICE,
    items::WHITE_HERB,
    items::MENTAL_HERB,
];

const NON_CONSUMABLE_HELD_ITEMS: &[i32] = &[
    items::BRIGHT_POWDER,
    items::QUICK_CLAW,
    items::CHOICE_BAND,
    items::KINGS_ROCK,
    items::SILVER_POWDER,
    items::FOCUS_BAND,
    items::SCOPE_LENS,
    items::METAL_COAT,
    items::LEFTOVERS,
    items::SOFT_SAND,
    items::HARD_STONE,
    items::MIRACLE_SEED,
    items::BLACK_GLASSES,
    items::BLACK_BELT,
    items::MAGNET,
    items::MYSTIC_WATER,
    items::SHARP_BEAK,
    items::POISON_BARB,
    items::NEVER_MELT_ICE,
    items::SPELL_TAG,
    items::TWISTED_SPOON,
    items::CHARCOAL,
    items::DRAGON_FANG,
    items::SILK_SCARF,
    items::SHELL_BELL,
    items::SEA_INCENSE,
    items::LAX_INCENSE,
];

pub fn all_held_items() -> Vec<i32> {
    NON_CONSUMABLE_HELD_ITEMS
        .iter()
        .chain(CONSUMABLE_HELD_ITEMS)
        .copied()
        .collect()
}

pub const GENERAL_PURPOSE_CONSUMABLE_ITEMS: &[i32] = &[
    items::CHERI_BERRY,
    items::CHESTO_BERRY,
    items::PECHA_BERRY,
    items::RAWST_BERRY,
    items::ASPEAR_BERRY,
    items::LEPPA_BERRY,
    items::ORAN_BERRY,
    items::PERSIM_BERRY,
    items::LUM_BERRY,
    items::SITRUS_BERRY,
    items::GANLON_BERRY,
    items::SALAC_BERRY,
    items::APICOT_BERRY,
    items::LANSAT_BERRY,
    items::STARF_BERRY,
    items::BERRY_JUICE,
    items::WHITE_HERB,
    items::MENTAL_HERB,
];

pub const GENERAL_PURPOSE_ITEMS: &[i32] = &[
    items::BRIGHT_POWDER,
    items::QUICK_CLAW,
    items::KINGS_ROCK,
    items::FOCUS_BAND,
    items::SCOPE_LENS,
    items::LEFTOVERS,
    items::SHELL_BELL,
    items::LAX_INCENSE,
];

pub fn type_boosting_items(kind: Option<Type>) -> &'static [i32] {
    match kind {
        Some(Type::Bug) => &[items::SILVER_POWDER],
        Some(Type::Dark) => &[items::BLACK_GLASSES],
        Some(Type::Dragon) => &[items::DRAGON_FANG],
        Some(Type::Electric) => &[items::MAGNET],
        Some(Type::Fighting) => &[items::BLACK_BELT],
        Some(Type::Fire) => &[items::CHARCOAL],
        Some(Type::Flying) => &[items::SHARP_BEAK],
        Some(Type::Ghost) => &[items::SPELL_TAG],
        Some(Type::Grass) => &[items::MIRACLE_SEED],
        Some(Type::Ground) => &[items::SOFT_SAND],
        Some(Type::Ice) => &[items::NEVER_MELT_ICE],
        Some(Type::Normal) => &[items::SILK_SCARF],
        Some(Type::Poison) => &[items::POISON_BARB],
        Some(Type::Psychic) => &[items::TWISTED_SPOON],
        Some(Type::Rock) => &[items::HARD_STONE],
        Some(Type::Steel) => &[items::METAL_COAT],
        Some(Type::Water) => &[items::MYSTIC_WATER, items::SEA_INCENSE],
        _ => &[],
    }
}

pub fn species_boosting_items(number: i32) -> Option<&'static [i32]> {
    let found: &'static [i32] = match number {
        species::LATIAS | species::LATIOS => &[items::SOUL_DEW],
        species::CLAMPERL => &[items::DEEP_SEA_TOOTH, items::DEEP_SEA_SCALE],
        species::PIKACHU => &[items::LIGHT_BALL],
        species::CHANSEY => &[items::LUCKY_PUNCH],
        species::DITTO => &[items::METAL_POWDER],
        species::CUBONE | species::MAROWAK => &[items::THICK_CLUB],
        species::FARFETCHD => &[items::STICK],
        _ => return None,
    };
    Some(found)
}

pub fn ability_variations() -> BTreeMap<i32, Vec<i32>> {
    BTreeMap::from([
        (
            abilities::INSOMNIA,
            vec![abilities::INSOMNIA, abilities::VITAL_SPIRIT],
        ),
        (
            abilities::CLEAR_BODY,
            vec![abilities::CLEAR_BODY, abilities::WHITE_SMOKE],
        ),
        (
            abilities::HUGE_POWER,
            vec![abilities::HUGE_POWER, abilities::PURE_POWER],
        ),
        (
            abilities::BATTLE_ARMOR,
            vec![abilities::BATTLE_ARMOR, abilities::SHELL_ARMOR],
        ),
        (
            abilities::CLOUD_NINE,
            vec![abilities::CLOUD_NINE, AIR_LOCK_INDEX],
        ),
    ])
}

pub const USELESS_ABILITIES: &[i32] = &[abilities::FORECAST, CACOPHONY_INDEX];

pub fn type_table(value: i32) -> Option<Type> {
    let kind = match value {
        0x00 => Type::Normal,
        0x01 => Type::Fighting,
        0x02 => Type::Flying,
        0x03 => Type::Poison,
        0x04 => Type::Ground,
        0x05 => Type::Rock,
        0x06 => Type::Bug,
        0x07 => Type::Ghost,
        0x08 => Type::Steel,
        0x0A => Type::Fire,
        0x0B => Type::Water,
        0x0C => Type::Grass,
        0x0D => Type::Electric,
        0x0E => Type::Psychic,
        0x0F => Type::Ice,
        0x10 => Type::Dragon,
        0x11 => Type::Dark,
        _ => return None,
    };
    Some(kind)
}

pub fn type_to_byte(kind: Option<Type>) -> u8 {
    match kind {
        None => 0x09,
        Some(Type::Normal) => 0x00,
        Some(Type::Fighting) => 0x01,
        Some(Type::Flying) => 0x02,
        Some(Type::Poison) => 0x03,
        Some(Type::Ground) => 0x04,
        Some(Type::Rock) => 0x05,
        Some(Type::Bug) => 0x06,
        Some(Type::Ghost) => 0x07,
        Some(Type::Fire) => 0x0A,
        Some(Type::Water) => 0x0B,
        Some(Type::Grass) => 0x0C,
        Some(Type::Electric) => 0x0D,
        Some(Type::Psychic) => 0x0E,
        Some(Type::Ice) => 0x0F,
        Some(Type::Dragon) => 0x10,
        Some(Type::Steel) => 0x08,
        Some(Type::Dark) => 0x11,
        Some(_) => 0x00,
    }
}

pub fn running_shoes_check_prefix(rom_type: RomType) -> &'static str {
    match rom_type {
        RomType::Ruby | RomType::Sapphire => RUNNING_SHOES_CHECK_PREFIX_RS,
        RomType::FireRedLeafGreen => RUNNING_SHOES_CHECK_PREFIX_FRLG,
        RomType::Emerald => RUNNING_SHOES_CHECK_PREFIX_E,
    }
}

pub fn allowed_items() -> ItemList {
    let mut allowed = ItemList::new(items::OLD_SEA_MAP as usize);
    allowed.ban_range(items::MACH_BIKE, 30);
    allowed.ban_range(items::OAKS_PARCEL, 28);
    allowed.ban_range(items::UNKNOWN52, 11);
    allowed.ban_range(items::UNKNOWN87, 6);
    allowed.ban_range(items::UNKNOWN99, 4);
    allowed.ban_range(items::UNKNOWN112, 9);
    allowed.ban_range(items::UNKNOWN176, 3);
    allowed.ban_range(items::UNKNOWN226, 28);
    allowed.ban_range(items::UNKNOWN347, 2);
    allowed.ban_singles(&[
        items::UNKNOWN72,
        items::UNKNOWN82,
        items::UNKNOWN105,
        items::UNKNOWN267,
    ]);
    allowed.ban_range(items::HM01, 8);
    allowed.tm_range(items::TM01, 50);
    allowed
}

fn non_bad_items_rse() -> ItemList {
    let mut non_bad = allowed_items();
    non_bad.ban_singles(&[items::LIGHT_BALL, items::ORAN_BERRY, items::SOUL_DEW]);
    non_bad.ban_range(items::ORANGE_MAIL, 12);
    non_bad.ban_range(items::FIGY_BERRY, 33);
    non_bad.ban_range(items::LUCKY_PUNCH, 4);
    non_bad.ban_range(items::RED_SCARF, 5);
    non_bad
}

pub fn non_bad_items(rom_type: RomType) -> ItemList {
    let mut non_bad = non_bad_items_rse();
    if rom_type == RomType::FireRedLeafGreen {
        non_bad.ban_range(items::SHOAL_SALT, 6);
    }
    non_bad
}

pub fn regular_shop_items() -> Vec<i32> {
    (items::ULTRA_BALL..=items::POKE_BALL)
        .chain(items::POTION..=items::REVIVE)
        .chain(items::SUPER_REPEL..=items::REPEL)
        .collect()
}

pub fn op_shop_items() -> Vec<i32> {
    std::iter::once(items::RARE_CANDY)
        .chain(items::TINY_MUSHROOM..=items::BIG_MUSHROOM)
        .chain(items::PEARL..=items::NUGGET)
        .chain(std::iter::once(items::LUCKY_EGG))
        .collect()
}

pub fn balanced_item_price(item: i32) -> Option<i32> {
    BALANCED_ITEM_PRICES
        .iter()
        .find(|(index, _)| *index == item)
        .map(|(_, price)| *price)
}

fn tag_one(trainers: &mut [Trainer], number: i32, tag: &str) {
    if let Some(trainer) = usize::try_from(number - 1)
        .ok()
        .and_then(|index| trainers.get_mut(index))
    {
        trainer.tag = Some(tag.to_owned());
    }
}

fn tag_all(trainers: &mut [Trainer], tag: &str, numbers: &[i32]) {
    for &number in numbers {
        tag_one(trainers, number, tag);
    }
}

fn tag_singles(trainers: &mut [Trainer], tags: &[(i32, &str)]) {
    for &(number, tag) in tags {
        tag_one(trainers, number, tag);
    }
}

fn tag_groups(trainers: &mut [Trainer], groups: &[(&str, &[i32])]) {
    for &(tag, numbers) in groups {
        tag_all(trainers, tag, numbers);
    }
}

const RS_GYM_TRAINERS: &[(&str, &[i32])] = &[
    ("GYM1", &[0x140, 0x141]),
    ("GYM2", &[0x1AA, 0x1A9, 0xB3]),
    ("GYM3", &[0xBF, 0x143, 0xC2, 0x289]),
    ("GYM4", &[0xC9, 0x288, 0xCB, 0x28A, 0xCD]),
    ("GYM5", &[0x47, 0x59, 0x49, 0x5A, 0x48, 0x5B, 0x4A]),
    ("GYM6", &[0x191, 0x28F, 0x28E, 0x194]),
    ("GYM7", &[0xE9, 0xEA, 0xEB, 0xF4, 0xF5, 0xF6]),
    ("GYM8", &[0x82, 0x266, 0x83, 0x12D, 0x81, 0x74, 0x80, 0x265]),
];

const RS_SINGLE_TAGS: &[(i32, &str)] = &[
    (0x109, "GYM1-LEADER"),
    (0x10A, "GYM2-LEADER"),
    (0x10B, "GYM3-LEADER"),
    (0x10C, "GYM4-LEADER"),
    (0x10D, "GYM5-LEADER"),
    (0x10E, "GYM6-LEADER"),
    (0x10F, "GYM7-LEADER"),
    (0x110, "GYM8-LEADER"),
    (0x105, "ELITE1"),
    (0x106, "ELITE2"),
    (0x107, "ELITE3"),
    (0x108, "ELITE4"),
    (0x14F, "CHAMPION"),
    (0x208, "RIVAL1-2"),
    (0x20B, "RIVAL1-0"),
    (0x20E, "RIVAL1-1"),
    (0x209, "RIVAL2-2"),
    (0x20C, "RIVAL2-0"),
    (0x20F, "RIVAL2-1"),
    (0x20A, "RIVAL3-2"),
    (0x20D, "RIVAL3-0"),
    (0x210, "RIVAL3-1"),
    (0x295, "RIVAL4-2"),
    (0x296, "RIVAL4-0"),
    (0x297, "RIVAL4-1"),
    (0x211, "RIVAL1-2"),
    (0x214, "RIVAL1-0"),
    (0x217, "RIVAL1-1"),
    (0x212, "RIVAL2-2"),
    (0x215, "RIVAL2-0"),
    (0x218, "RIVAL2-1"),
    (0x213, "RIVAL3-2"),
    (0x216, "RIVAL3-0"),
    (0x219, "RIVAL3-1"),
    (0x298, "RIVAL4-2"),
    (0x299, "RIVAL4-0"),
    (0x29A, "RIVAL4-1"),
];

const RS_WALLY: (&str, &[i32]) = (
    "THEMED:WALLY-STRONG",
    &[0x207, 0x290, 0x291, 0x292, 0x293, 0x294],
);

const RUBY_ADMINS: &[(&str, &[i32])] = &[
    ("THEMED:MAXIE-LEADER", &[0x259, 0x25A]),
    ("THEMED:COURTNEY-STRONG", &[0x257, 0x258]),
    ("THEMED:TABITHA-STRONG", &[0x254, 0x255]),
];

const SAPPHIRE_ADMINS: &[(&str, &[i32])] = &[
    ("THEMED:ARCHIE-LEADER", &[0x23, 0x22]),
    ("THEMED:MATT-STRONG", &[0x1E, 0x1F]),
    ("THEMED:SHELLY-STRONG", &[0x20, 0x21]),
];

pub fn trainer_tags_rs(trainers: &mut [Trainer], rom_type: RomType) {
    tag_groups(trainers, RS_GYM_TRAINERS);
    tag_singles(trainers, RS_SINGLE_TAGS);
    tag_groups(trainers, &[RS_WALLY]);
    if rom_type == RomType::Ruby {
        tag_groups(trainers, RUBY_ADMINS);
    } else {
        tag_groups(trainers, SAPPHIRE_ADMINS);
    }
}

const E_GROUP_TAGS_BEFORE_SINGLES: &[(&str, &[i32])] = &[
    ("GYM1", &[0x140, 0x141, 0x23B]),
    ("GYM2", &[0x1AA, 0x1A9, 0xB3, 0x23C, 0x23D, 0x23E]),
    ("GYM3", &[0xBF, 0x143, 0xC2, 0x289, 0x322]),
    ("GYM4", &[0x288, 0xC9, 0xCB, 0x28A, 0xCA, 0xCC, 0x1F5, 0xCD]),
    ("GYM5", &[0x47, 0x59, 0x49, 0x5A, 0x48, 0x5B, 0x4A]),
    ("GYM6", &[0x192, 0x28F, 0x191, 0x28E, 0x194, 0x323]),
    (
        "GYM7",
        &[
            0xE9, 0xEA, 0xEB, 0xF4, 0xF5, 0xF6, 0x24F, 0x248, 0x247, 0x249, 0x246, 0x23F,
        ],
    ),
    (
        "GYM8",
        &[
            0x265, 0x80, 0x1F6, 0x73, 0x81, 0x76, 0x82, 0x12D, 0x83, 0x266,
        ],
    ),
    ("GYM1-LEADER", &[0x109, 0x302, 0x303, 0x304, 0x305]),
    ("GYM2-LEADER", &[0x10A, 0x306, 0x307, 0x308, 0x309]),
    ("GYM3-LEADER", &[0x10B, 0x30A, 0x30B, 0x30C, 0x30D]),
    ("GYM4-LEADER", &[0x10C, 0x30E, 0x30F, 0x310, 0x311]),
    ("GYM5-LEADER", &[0x10D, 0x312, 0x313, 0x314, 0x315]),
    ("GYM6-LEADER", &[0x10E, 0x316, 0x317, 0x318, 0x319]),
    ("GYM7-LEADER", &[0x10F, 0x31A, 0x31B, 0x31C, 0x31D]),
    ("GYM8-LEADER", &[0x110, 0x31E, 0x31F, 0x320, 0x321]),
];

const E_SINGLE_TAGS: &[(i32, &str)] = &[
    (0x105, "ELITE1"),
    (0x106, "ELITE2"),
    (0x107, "ELITE3"),
    (0x108, "ELITE4"),
    (0x14F, "CHAMPION"),
    (0x208, "RIVAL1-2"),
    (0x20B, "RIVAL1-0"),
    (0x20E, "RIVAL1-1"),
    (0x251, "RIVAL2-2"),
    (0x250, "RIVAL2-0"),
    (0x257, "RIVAL2-1"),
    (0x209, "RIVAL3-2"),
    (0x20C, "RIVAL3-0"),
    (0x20F, "RIVAL3-1"),
    (0x20A, "RIVAL4-2"),
    (0x20D, "RIVAL4-0"),
    (0x210, "RIVAL4-1"),
    (0x295, "RIVAL5-2"),
    (0x296, "RIVAL5-0"),
    (0x297, "RIVAL5-1"),
    (0x211, "RIVAL1-2"),
    (0x214, "RIVAL1-0"),
    (0x217, "RIVAL1-1"),
    (0x258, "RIVAL2-2"),
    (0x300, "RIVAL2-0"),
    (0x301, "RIVAL2-1"),
    (0x212, "RIVAL3-2"),
    (0x215, "RIVAL3-0"),
    (0x218, "RIVAL3-1"),
    (0x213, "RIVAL4-2"),
    (0x216, "RIVAL4-0"),
    (0x219, "RIVAL4-1"),
    (0x298, "RIVAL5-2"),
    (0x299, "RIVAL5-0"),
    (0x29A, "RIVAL5-1"),
];

const E_THEMED_TAGS: &[(&str, &[i32])] = &[
    ("THEMED:MAXIE-LEADER", &[0x259, 0x25A, 0x2DE]),
    ("THEMED:TABITHA-STRONG", &[0x202, 0x255, 0x2DC]),
    ("THEMED:ARCHIE-LEADER", &[0x22]),
    ("THEMED:MATT-STRONG", &[0x1E]),
    ("THEMED:SHELLY-STRONG", &[0x20, 0x21]),
    (
        "THEMED:WALLY-STRONG",
        &[0x207, 0x290, 0x291, 0x292, 0x293, 0x294],
    ),
];

pub fn trainer_tags_e(trainers: &mut [Trainer]) {
    tag_groups(trainers, E_GROUP_TAGS_BEFORE_SINGLES);
    tag_singles(trainers, E_SINGLE_TAGS);
    tag_groups(trainers, E_THEMED_TAGS);
    tag_one(trainers, EM_METEOR_FALLS_STEVEN_INDEX, "UBER");
}

const FRLG_GYM_TRAINERS: &[(&str, &[i32])] = &[
    ("GYM1", &[0x8E]),
    ("GYM2", &[0xEA, 0x96]),
    ("GYM3", &[0xDC, 0x8D, 0x1A7]),
    ("GYM4", &[0x10A, 0x84, 0x109, 0xA0, 0x192, 0x10B, 0x85]),
    ("GYM5", &[0x125, 0x124, 0x120, 0x127, 0x126, 0x121]),
    ("GYM6", &[0x11A, 0x119, 0x1CF, 0x11B, 0x1CE, 0x1D0, 0x118]),
    ("GYM7", &[0xD5, 0xB1, 0xB2, 0xD6, 0xB3, 0xD7, 0xB4]),
    (
        "GYM8",
        &[0x129, 0x143, 0x188, 0x190, 0x142, 0x128, 0x191, 0x144],
    ),
];

const FRLG_SINGLE_TAGS: &[(i32, &str)] = &[
    (0x19E, "GYM1-LEADER"),
    (0x19F, "GYM2-LEADER"),
    (0x1A0, "GYM3-LEADER"),
    (0x1A1, "GYM4-LEADER"),
    (0x1A2, "GYM5-LEADER"),
    (0x1A4, "GYM6-LEADER"),
    (0x1A3, "GYM7-LEADER"),
    (0x15E, "GYM8-LEADER"),
    (0x15C, "GIO1-LEADER"),
    (0x15D, "GIO2-LEADER"),
    (0x19A, "ELITE1-1"),
    (0x19B, "ELITE2-1"),
    (0x19C, "ELITE3-1"),
    (0x19D, "ELITE4-1"),
    (0x2DF, "ELITE1-2"),
    (0x2E0, "ELITE2-2"),
    (0x2E1, "ELITE3-2"),
    (0x2E2, "ELITE4-2"),
    (0x148, "RIVAL1-0"),
    (0x146, "RIVAL1-1"),
    (0x147, "RIVAL1-2"),
    (0x14B, "RIVAL2-0"),
    (0x149, "RIVAL2-1"),
    (0x14A, "RIVAL2-2"),
    (0x14E, "RIVAL3-0"),
    (0x14C, "RIVAL3-1"),
    (0x14D, "RIVAL3-2"),
    (0x1AC, "RIVAL4-0"),
    (0x1AA, "RIVAL4-1"),
    (0x1AB, "RIVAL4-2"),
    (0x1AF, "RIVAL5-0"),
    (0x1AD, "RIVAL5-1"),
    (0x1AE, "RIVAL5-2"),
    (0x1B2, "RIVAL6-0"),
    (0x1B0, "RIVAL6-1"),
    (0x1B1, "RIVAL6-2"),
    (0x1B5, "RIVAL7-0"),
    (0x1B3, "RIVAL7-1"),
    (0x1B4, "RIVAL7-2"),
    (0x1B8, "RIVAL8-0"),
    (0x1B6, "RIVAL8-1"),
    (0x1B7, "RIVAL8-2"),
    (0x2E5, "RIVAL9-0"),
    (0x2E3, "RIVAL9-1"),
    (0x2E4, "RIVAL9-2"),
];

pub fn trainer_tags_frlg(trainers: &mut [Trainer]) {
    tag_groups(trainers, FRLG_GYM_TRAINERS);
    tag_singles(trainers, FRLG_SINGLE_TAGS);
}

const EM_ALWAYS_MULTI_BATTLES: &[i32] = &[
    25, 105, 237, 397, 404, 504, 505, 508, 514, 569, 572, 573, 654, 721, 730, 734, 848, 850,
];

const EM_POTENTIAL_MULTI_BATTLES: &[i32] = &[
    1, 3, 8, 9, 11, 12, 13, 14, 15, 18, 28, 29, 31, 33, 35, 37, 38, 57, 64, 106, 107, 108, 115,
    118, 124, 129, 130, 131, 137, 144, 145, 146, 160, 168, 170, 171, 180, 182, 191, 192, 193, 194,
    195, 201, 204, 217, 232, 233, 234, 235, 236, 244, 245, 246, 247, 248, 249, 273, 301, 302, 307,
    321, 324, 325, 345, 375, 377, 383, 385, 400, 401, 403, 413, 415, 416, 417, 418, 420, 436, 445,
    450, 455, 459, 460, 464, 475, 491, 494, 495, 501, 502, 503, 506, 507, 509, 511, 512, 513, 539,
    540, 546, 547, 566, 567, 571, 575, 576, 577, 578, 579, 580, 582, 583, 584, 585, 591, 594, 595,
    596, 598, 605, 614, 629, 631, 648, 649, 653, 655, 674, 676, 694, 695, 697, 698, 699, 700, 701,
    702, 703, 704, 705, 706, 707, 708, 709, 710, 711, 712, 713, 714, 715, 719, 720, 727, 728, 733,
    735, 736, 737, 738, 739, 740, 741, 742, 743, 744, 745, 746, 747, 748, 749, 750, 751, 752, 753,
    754, 755, 756, 757, 758, 759, 760, 761, 762, 763, 764, 765, 766, 767, 802, 849,
];

fn set_multi_battle_status(trainers: &mut [Trainer], status: MultiBattleStatus, numbers: &[i32]) {
    for &number in numbers {
        if let Some(trainer) = usize::try_from(number - 1)
            .ok()
            .and_then(|index| trainers.get_mut(index))
        {
            trainer.multi_battle_status = status;
        }
    }
}

pub fn set_multi_battle_status_em(trainers: &mut [Trainer]) {
    set_multi_battle_status(trainers, MultiBattleStatus::Always, EM_ALWAYS_MULTI_BATTLES);
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Potential,
        EM_POTENTIAL_MULTI_BATTLES,
    );
}

pub const BALANCED_ITEM_PRICES: &[(i32, i32)] = &[
    (items::MASTER_BALL, 300),
    (items::ULTRA_BALL, 120),
    (items::GREAT_BALL, 60),
    (items::POKE_BALL, 20),
    (items::SAFARI_BALL, 50),
    (items::NET_BALL, 100),
    (items::DIVE_BALL, 100),
    (items::NEST_BALL, 100),
    (items::REPEAT_BALL, 100),
    (items::TIMER_BALL, 100),
    (items::LUXURY_BALL, 100),
    (items::PREMIER_BALL, 20),
    (items::POTION, 30),
    (items::ANTIDOTE, 10),
    (items::BURN_HEAL, 25),
    (items::ICE_HEAL, 25),
    (items::AWAKENING, 25),
    (items::PARLYZ_HEAL, 20),
    (items::FULL_RESTORE, 300),
    (items::MAX_POTION, 250),
    (items::HYPER_POTION, 120),
    (items::SUPER_POTION, 70),
    (items::FULL_HEAL, 60),
    (items::REVIVE, 150),
    (items::MAX_REVIVE, 400),
    (items::FRESH_WATER, 40),
    (items::SODA_POP, 60),
    (items::LEMONADE, 70),
    (items::MOOMOO_MILK, 80),
    (items::ENERGY_POWDER, 40),
    (items::ENERGY_ROOT, 110),
    (items::HEAL_POWDER, 45),
    (items::REVIVAL_HERB, 280),
    (items::ETHER, 300),
    (items::MAX_ETHER, 450),
    (items::ELIXIR, 1500),
    (items::MAX_ELIXIR, 1800),
    (items::LAVA_COOKIE, 45),
    (items::BLUE_FLUTE, 2),
    (items::YELLOW_FLUTE, 2),
    (items::RED_FLUTE, 2),
    (items::BLACK_FLUTE, 2),
    (items::WHITE_FLUTE, 2),
    (items::BERRY_JUICE, 10),
    (items::SACRED_ASH, 1000),
    (items::SHOAL_SALT, 2),
    (items::SHOAL_SHELL, 2),
    (items::RED_SHARD, 40),
    (items::BLUE_SHARD, 40),
    (items::YELLOW_SHARD, 40),
    (items::GREEN_SHARD, 40),
    (items::UNKNOWN52, 0),
    (items::UNKNOWN53, 0),
    (items::UNKNOWN54, 0),
    (items::UNKNOWN55, 0),
    (items::UNKNOWN56, 0),
    (items::UNKNOWN57, 0),
    (items::UNKNOWN58, 0),
    (items::UNKNOWN59, 0),
    (items::UNKNOWN60, 0),
    (items::UNKNOWN61, 0),
    (items::UNKNOWN62, 0),
    (items::HP_UP, 980),
    (items::PROTEIN, 980),
    (items::IRON, 980),
    (items::CARBOS, 980),
    (items::CALCIUM, 980),
    (items::RARE_CANDY, 1000),
    (items::PP_UP, 980),
    (items::ZINC, 980),
    (items::PP_MAX, 2490),
    (items::UNKNOWN72, 0),
    (items::GUARD_SPEC, 70),
    (items::DIRE_HIT, 65),
    (items::X_ATTACK, 50),
    (items::X_DEFEND, 55),
    (items::X_SPEED, 35),
    (items::X_ACCURACY, 95),
    (items::X_SPECIAL, 35),
    (items::POKE_DOLL, 100),
    (items::FLUFFY_TAIL, 100),
    (items::UNKNOWN82, 0),
    (items::SUPER_REPEL, 50),
    (items::MAX_REPEL, 70),
    (items::ESCAPE_ROPE, 55),
    (items::REPEL, 35),
    (items::UNKNOWN87, 0),
    (items::UNKNOWN88, 0),
    (items::UNKNOWN89, 0),
    (items::UNKNOWN90, 0),
    (items::UNKNOWN91, 0),
    (items::UNKNOWN92, 0),
    (items::SUN_STONE, 300),
    (items::MOON_STONE, 300),
    (items::FIRE_STONE, 300),
    (items::THUNDERSTONE, 300),
    (items::WATER_STONE, 300),
    (items::LEAF_STONE, 300),
    (items::UNKNOWN99, 0),
    (items::UNKNOWN100, 0),
    (items::UNKNOWN101, 0),
    (items::UNKNOWN102, 0),
    (items::TINY_MUSHROOM, 50),
    (items::BIG_MUSHROOM, 500),
    (items::UNKNOWN105, 0),
    (items::PEARL, 140),
    (items::BIG_PEARL, 750),
    (items::STARDUST, 200),
    (items::STAR_PIECE, 980),
    (items::NUGGET, 1000),
    (items::HEART_SCALE, 500),
    (items::UNKNOWN112, 0),
    (items::UNKNOWN113, 0),
    (items::UNKNOWN114, 0),
    (items::UNKNOWN115, 0),
    (items::UNKNOWN116, 0),
    (items::UNKNOWN117, 0),
    (items::UNKNOWN118, 0),
    (items::UNKNOWN119, 0),
    (items::UNKNOWN120, 0),
    (items::ORANGE_MAIL, 5),
    (items::HARBOR_MAIL, 5),
    (items::GLITTER_MAIL, 5),
    (items::MECH_MAIL, 5),
    (items::WOOD_MAIL, 5),
    (items::WAVE_MAIL, 5),
    (items::BEAD_MAIL, 5),
    (items::SHADOW_MAIL, 5),
    (items::TROPIC_MAIL, 5),
    (items::DREAM_MAIL, 5),
    (items::FAB_MAIL, 5),
    (items::RETRO_MAIL, 5),
    (items::CHERI_BERRY, 20),
    (items::CHESTO_BERRY, 25),
    (items::PECHA_BERRY, 10),
    (items::RAWST_BERRY, 25),
    (items::ASPEAR_BERRY, 25),
    (items::LEPPA_BERRY, 300),
    (items::ORAN_BERRY, 5),
    (items::PERSIM_BERRY, 20),
    (items::LUM_BERRY, 50),
    (items::SITRUS_BERRY, 50),
    (items::FIGY_BERRY, 10),
    (items::WIKI_BERRY, 10),
    (items::MAGO_BERRY, 10),
    (items::AGUAV_BERRY, 10),
    (items::IAPAPA_BERRY, 10),
    (items::RAZZ_BERRY, 50),
    (items::BLUK_BERRY, 50),
    (items::NANAB_BERRY, 50),
    (items::WEPEAR_BERRY, 50),
    (items::PINAP_BERRY, 50),
    (items::POMEG_BERRY, 50),
    (items::KELPSY_BERRY, 50),
    (items::QUALOT_BERRY, 50),
    (items::HONDEW_BERRY, 50),
    (items::GREPA_BERRY, 50),
    (items::TAMATO_BERRY, 50),
    (items::CORNN_BERRY, 50),
    (items::MAGOST_BERRY, 50),
    (items::RABUTA_BERRY, 50),
    (items::NOMEL_BERRY, 50),
    (items::SPELON_BERRY, 50),
    (items::PAMTRE_BERRY, 50),
    (items::WATMEL_BERRY, 50),
    (items::DURIN_BERRY, 50),
    (items::BELUE_BERRY, 50),
    (items::LIECHI_BERRY, 100),
    (items::GANLON_BERRY, 100),
    (items::SALAC_BERRY, 100),
    (items::PETAYA_BERRY, 100),
    (items::APICOT_BERRY, 100),
    (items::LANSAT_BERRY, 100),
    (items::STARF_BERRY, 100),
    (items::ENIGMA_BERRY, 100),
    (items::UNKNOWN176, 0),
    (items::UNKNOWN177, 0),
    (items::UNKNOWN178, 0),
    (items::BRIGHT_POWDER, 300),
    (items::WHITE_HERB, 100),
    (items::MACHO_BRACE, 300),
    (items::EXP_SHARE, 600),
    (items::QUICK_CLAW, 450),
    (items::SOOTHE_BELL, 100),
    (items::MENTAL_HERB, 100),
    (items::CHOICE_BAND, 1000),
    (items::KINGS_ROCK, 500),
    (items::SILVER_POWDER, 200),
    (items::AMULET_COIN, 1500),
    (items::CLEANSE_TAG, 100),
    (items::SOUL_DEW, 20),
    (items::DEEP_SEA_TOOTH, 300),
    (items::DEEP_SEA_SCALE, 300),
    (items::SMOKE_BALL, 20),
    (items::EVERSTONE, 20),
    (items::FOCUS_BAND, 300),
    (items::LUCKY_EGG, 1000),
    (items::SCOPE_LENS, 500),
    (items::METAL_COAT, 300),
    (items::LEFTOVERS, 1000),
    (items::DRAGON_SCALE, 300),
    (items::LIGHT_BALL, 10),
    (items::SOFT_SAND, 200),
    (items::HARD_STONE, 200),
    (items::MIRACLE_SEED, 200),
    (items::BLACK_GLASSES, 200),
    (items::BLACK_BELT, 200),
    (items::MAGNET, 200),
    (items::MYSTIC_WATER, 200),
    (items::SHARP_BEAK, 200),
    (items::POISON_BARB, 200),
    (items::NEVER_MELT_ICE, 200),
    (items::SPELL_TAG, 200),
    (items::TWISTED_SPOON, 200),
    (items::CHARCOAL, 200),
    (items::DRAGON_FANG, 200),
    (items::SILK_SCARF, 200),
    (items::UP_GRADE, 300),
    (items::SHELL_BELL, 600),
    (items::SEA_INCENSE, 200),
    (items::LAX_INCENSE, 300),
    (items::LUCKY_PUNCH, 1),
    (items::METAL_POWDER, 1),
    (items::THICK_CLUB, 50),
    (items::STICK, 20),
    (items::UNKNOWN226, 0),
    (items::UNKNOWN227, 0),
    (items::UNKNOWN228, 0),
    (items::UNKNOWN229, 0),
    (items::UNKNOWN230, 0),
    (items::UNKNOWN231, 0),
    (items::UNKNOWN232, 0),
    (items::UNKNOWN233, 0),
    (items::UNKNOWN234, 0),
    (items::UNKNOWN235, 0),
    (items::UNKNOWN236, 0),
    (items::UNKNOWN237, 0),
    (items::UNKNOWN238, 0),
    (items::UNKNOWN239, 0),
    (items::UNKNOWN240, 0),
    (items::UNKNOWN241, 0),
    (items::UNKNOWN242, 0),
    (items::UNKNOWN243, 0),
    (items::UNKNOWN244, 0),
    (items::UNKNOWN245, 0),
    (items::UNKNOWN246, 0),
    (items::UNKNOWN247, 0),
    (items::UNKNOWN248, 0),
    (items::UNKNOWN249, 0),
    (items::UNKNOWN250, 0),
    (items::UNKNOWN251, 0),
    (items::UNKNOWN252, 0),
    (items::UNKNOWN253, 0),
    (items::RED_SCARF, 10),
    (items::BLUE_SCARF, 10),
    (items::PINK_SCARF, 10),
    (items::GREEN_SCARF, 10),
    (items::YELLOW_SCARF, 10),
    (items::MACH_BIKE, 0),
    (items::COIN_CASE, 0),
    (items::ITEMFINDER, 0),
    (items::OLD_ROD, 0),
    (items::GOOD_ROD, 0),
    (items::SUPER_ROD, 0),
    (items::SS_TICKET, 0),
    (items::CONTEST_PASS, 0),
    (items::UNKNOWN267, 0),
    (items::WAILMER_PAIL, 0),
    (items::DEVON_GOODS, 0),
    (items::SOOT_SACK, 0),
    (items::BASEMENT_KEY, 0),
    (items::ACRO_BIKE, 0),
    (items::POKEBLOCK_CASE, 0),
    (items::LETTER, 0),
    (items::EON_TICKET, 0),
    (items::RED_ORB, 0),
    (items::BLUE_ORB, 0),
    (items::SCANNER, 0),
    (items::GO_GOGGLES, 0),
    (items::METEORITE, 0),
    (items::RM1_KEY, 0),
    (items::RM2_KEY, 0),
    (items::RM4_KEY, 0),
    (items::RM6_KEY, 0),
    (items::STORAGE_KEY, 0),
    (items::ROOT_FOSSIL, 0),
    (items::CLAW_FOSSIL, 0),
    (items::DEVON_SCOPE, 0),
    (items::TM01, 300),
    (items::TM02, 300),
    (items::TM03, 300),
    (items::TM04, 150),
    (items::TM05, 100),
    (items::TM06, 300),
    (items::TM07, 200),
    (items::TM08, 150),
    (items::TM09, 200),
    (items::TM10, 200),
    (items::TM11, 200),
    (items::TM12, 150),
    (items::TM13, 300),
    (items::TM14, 550),
    (items::TM15, 750),
    (items::TM16, 200),
    (items::TM17, 200),
    (items::TM18, 200),
    (items::TM19, 300),
    (items::TM20, 200),
    (items::TM21, 100),
    (items::TM22, 300),
    (items::TM23, 300),
    (items::TM24, 300),
    (items::TM25, 550),
    (items::TM26, 300),
    (items::TM27, 100),
    (items::TM28, 200),
    (items::TM29, 300),
    (items::TM30, 300),
    (items::TM31, 300),
    (items::TM32, 100),
    (items::TM33, 200),
    (items::TM34, 300),
    (items::TM35, 300),
    (items::TM36, 300),
    (items::TM37, 200),
    (items::TM38, 550),
    (items::TM39, 200),
    (items::TM40, 300),
    (items::TM41, 150),
    (items::TM42, 300),
    (items::TM43, 200),
    (items::TM44, 300),
    (items::TM45, 300),
    (items::TM46, 200),
    (items::TM47, 300),
    (items::TM48, 300),
    (items::TM49, 150),
    (items::TM50, 550),
    (items::HM01, 0),
    (items::HM02, 0),
    (items::HM03, 0),
    (items::HM04, 0),
    (items::HM05, 0),
    (items::HM06, 0),
    (items::HM07, 0),
    (items::HM08, 0),
    (items::UNKNOWN347, 0),
    (items::UNKNOWN348, 0),
    (items::OAKS_PARCEL, 0),
    (items::POKE_FLUTE, 0),
    (items::SECRET_KEY, 0),
    (items::BIKE_VOUCHER, 0),
    (items::GOLD_TEETH, 0),
    (items::OLD_AMBER, 0),
    (items::CARD_KEY, 0),
    (items::LIFT_KEY, 0),
    (items::HELIX_FOSSIL, 0),
    (items::DOME_FOSSIL, 0),
    (items::SILPH_SCOPE, 0),
    (items::BICYCLE, 0),
    (items::TOWN_MAP, 0),
    (items::VS_SEEKER, 0),
    (items::FAME_CHECKER, 0),
    (items::TM_CASE, 0),
    (items::BERRY_POUCH, 0),
    (items::TEACHY_TV, 0),
    (items::TRI_PASS, 0),
    (items::RAINBOW_PASS, 0),
    (items::TEA, 0),
    (items::MYSTIC_TICKET, 0),
    (items::AURORA_TICKET, 0),
    (items::POWDER_JAR, 0),
    (items::RUBY, 0),
    (items::SAPPHIRE, 0),
    (items::MAGMA_EMBLEM, 0),
    (items::OLD_SEA_MAP, 0),
];
