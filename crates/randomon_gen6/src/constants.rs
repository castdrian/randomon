use randomon_ctr::{TYPE_ORAS, TYPE_XY};
use randomon_data::constants::{abilities, items, moves, species};
use randomon_data::{ItemList, MoveCategory, MultiBattleStatus, Trainer, Type};

use crate::tables;

pub const POKEMON_COUNT: i32 = 721;
const XY_FORME_COUNT: i32 = 77;
const ORAS_FORME_COUNT: i32 = 104;

pub const CRIES_TABLE_PREFIX_XY: &str = "60000A006B000A0082000A003D010A00";
pub const INTRO_POKEMON_MODEL_OFFSET_XY: &str = "01000400020002000200000003000000";
pub const INTRO_INITIAL_CRY_OFFSET1_XY: &str = "3AFEFFEB000055E31400D40507005001";
pub const INTRO_INITIAL_CRY_OFFSET2_XY: &str = "0800A0E110FEFFEB000057E31550C405";
pub const INTRO_INITIAL_CRY_OFFSET3_XY: &str = "0020E0E30310A0E1E4FDFFEB0000A0E3";
pub const INTRO_REPEATED_CRY_OFFSET_XY: &str = "1080BDE800002041000000008D001000";

pub const USELESS_ABILITIES: &[i32] = &[
    abilities::FORECAST,
    abilities::MULTITYPE,
    abilities::FLOWER_GIFT,
    abilities::ZEN_MODE,
    abilities::STANCE_CHANGE,
];

pub const NO_DAMAGE_TARGET_TRAPPING_EFFECT: i32 = 106;
pub const NO_DAMAGE_FIELD_TRAPPING_EFFECT: i32 = 354;
pub const DAMAGE_ADJACENT_FOES_TRAPPING_EFFECT: i32 = 373;

pub const NO_DAMAGE_STATUS_QUALITY: i32 = 1;
pub const NO_DAMAGE_STAT_CHANGE_QUALITY: i32 = 2;
pub const DAMAGE_STATUS_QUALITY: i32 = 4;
pub const NO_DAMAGE_STATUS_AND_STAT_CHANGE_QUALITY: i32 = 5;
pub const DAMAGE_TARGET_DEBUFF_QUALITY: i32 = 6;
pub const DAMAGE_USER_BUFF_QUALITY: i32 = 7;
pub const DAMAGE_ABSORB_QUALITY: i32 = 8;

pub const BANNED_MOVES: &[i32] = &[moves::HYPERSPACE_FURY];

pub const BS_HP_OFFSET: usize = 0;
pub const BS_ATTACK_OFFSET: usize = 1;
pub const BS_DEFENSE_OFFSET: usize = 2;
pub const BS_SPEED_OFFSET: usize = 3;
pub const BS_SP_ATK_OFFSET: usize = 4;
pub const BS_SP_DEF_OFFSET: usize = 5;
pub const BS_PRIMARY_TYPE_OFFSET: usize = 6;
pub const BS_SECONDARY_TYPE_OFFSET: usize = 7;
pub const BS_CATCH_RATE_OFFSET: usize = 8;
pub const BS_COMMON_HELD_ITEM_OFFSET: usize = 12;
pub const BS_RARE_HELD_ITEM_OFFSET: usize = 14;
pub const BS_DARK_GRASS_HELD_ITEM_OFFSET: usize = 16;
pub const BS_GENDER_OFFSET: usize = 18;
pub const BS_GROWTH_CURVE_OFFSET: usize = 21;
pub const BS_ABILITY1_OFFSET: usize = 24;
pub const BS_ABILITY2_OFFSET: usize = 25;
pub const BS_ABILITY3_OFFSET: usize = 26;
pub const BS_FORME_OFFSET: usize = 28;
pub const BS_FORME_COUNT_OFFSET: usize = 32;
pub const BS_TMHM_COMPAT_OFFSET: usize = 40;
pub const BS_MT_COMPAT_OFFSET: usize = 64;

pub const EVOLUTION_METHOD_COUNT: i32 = 34;

pub const STATIC_POKEMON_SIZE: usize = 0xC;

pub const TM_DATA_PREFIX: &str = "D400AE02AF02B002";
pub const TM_COUNT: i32 = 100;
pub const TM_BLOCK_ONE_COUNT: i32 = 92;
pub const TM_BLOCK_TWO_COUNT: i32 = 3;
pub const TM_BLOCK_THREE_COUNT: i32 = 5;
pub const TM_BLOCK_ONE_OFFSET: i32 = items::TM01;
pub const TM_BLOCK_TWO_OFFSET: i32 = items::TM93;
pub const TM_BLOCK_THREE_OFFSET: i32 = items::TM96;
pub const HM_BLOCK_ONE_COUNT: i32 = 5;
pub const ROCK_SMASH_OFFSET_ORAS: i32 = 10;
pub const DIVE_OFFSET_ORAS: i32 = 28;
pub const HIDDEN_ITEM_COUNT_ORAS: i32 = 170;
pub const HIDDEN_ITEMS_PREFIX_ORAS: &str = "A100A200A300A400A5001400010053004A0084000900";
pub const ITEM_PALETTES_PREFIX: &str = "6F7461746500FF920A063F";

pub const TUTOR_MOVE_COUNT: i32 = 60;
pub const TUTORS_LOCATOR: &str = "C2015701A20012024401BA01";
pub const TUTORS_SHOP_PREFIX: &str = "8A02000030000000";
pub const TUTOR_SIZE: [usize; 4] = [15, 17, 16, 15];

pub const INGAME_TRADE_SIZE: i32 = 0x24;
pub const FRIENDSHIP_VALUE_FOR_EVO_LOCATOR: &str = "DC0050E3BC00002A";
pub const PERFECT_ODDS_BRANCH_LOCATOR: &str = "050000BA000050E3";
pub const FASTEST_TEXT_PREFIXES: [&str; 2] = [
    "1080BDE80000A0E31080BDE8F0412DE9",
    "485080E59C4040E24C50C0E5EC009FE5",
];

pub const FALLING_ENCOUNTER_OFFSET: usize = 0xF4270;
pub const FALLING_ENCOUNTER_COUNT: usize = 55;
pub const FIELD_ENCOUNTER_SIZE: usize = 0x3C;
pub const RUSTLING_BUSH_ENCOUNTER_OFFSET: usize = 0xF40CC;
pub const RUSTLING_BUSH_ENCOUNTER_COUNT: usize = 7;
pub const PER_POKEMON_AREA_DATA_LENGTH_XY: usize = 0xE8;
pub const PER_POKEMON_AREA_DATA_LENGTH_ORAS: usize = 0x2A0;

pub const AFTER_BATTLE_FORME_REVERSION_PREFIX: &str = "E4FFFFEA0000000000000000";
pub const NINJASK_SPECIES_PREFIX: &str = "241094E5B810D1E1";
pub const SHEDINJA_SPECIES_PREFIX: &str = "C2FFFFEB0040A0E10020A0E3";
pub const BOX_LEGENDARY_FUNCTION_PREFIX_XY: &str = "14D08DE20900A0E1";
pub const BOX_LEGENDARY_ENCOUNTER_FILE_XY: usize = 341;
pub const BOX_LEGENDARY_LOCAL_SCRIPT_OFFSET_XY: usize = 0x6E0;
pub const BOX_LEGENDARY_CODE_OFFSETS_XY: [usize; 3] = [144, 300, 584];
pub const SEA_SPIRITS_DEN_ENCOUNTER_FILE_XY: usize = 351;
pub const SEA_SPIRITS_DEN_LOCAL_SCRIPT_OFFSET_XY: usize = 0x1C0;
pub const SEA_SPIRITS_DEN_SCRIPT_OFFSETS_XY: [usize; 3] = [0x500, 0x508, 0x510];
pub const RAYQUAZA_FUNCTION_PREFIX_ORAS: &str = "0900A0E1F08FBDE8";
pub const RAYQUAZA_SCRIPT_OFFSETS_ORAS: [usize; 2] = [3334, 14734];
pub const RAYQUAZA_CODE_OFFSETS_ORAS: [usize; 3] = [136, 292, 576];
pub const NATIONAL_DEX_FUNCTION_LOCATOR: &str = "080094E5010000E21080BDE8170F122F";
pub const XY_GET_DEX_FLAG_FUNCTION_LOCATOR: &str = "000055E30100A0030A00000A";
pub const ORAS_GET_HOENN_DEX_CAUGHT_FUNCTION_PREFIX: &str = "170F122F1CC15800";
pub const MEGASTONE_TABLE_STARTING_OFFSET_ORAS: usize = 0xABA;
pub const MEGASTONE_TABLE_ENTRY_SIZE_ORAS: usize = 0x20;
pub const MEGASTONE_TABLE_LENGTH_ORAS: usize = 27;

pub const PICKUP_TABLE_LOCATOR: &str = "110012001A00";
pub const NUMBER_OF_PICKUP_ITEMS: usize = 29;

pub const XY_ROAMER_FREE_SPACE_POSTFIX: &str = "540095E50220A0E30810A0E1";
pub const XY_ROAMER_SPECIES_LOCATOR: &str = "9040A0030400000A";
pub const XY_ROAMER_LEVEL_PREFIX: &str = "B020DDE13F3BC1E3";

pub const XY_TRASH_ENCOUNTERS_TABLE_PREFIX: &str = "4028100000";
pub const XY_TRASH_ENCOUNTER_DATA_LENGTH: usize = 16;
pub const XY_TRASH_CAN_ENCOUNTER_COUNT: usize = 24;
pub const XY_TRASH_CAN_SETS: [(usize, usize, bool); 5] = [
    (0, 6, true),
    (6, 6, true),
    (12, 3, true),
    (15, 3, true),
    (18, 6, false),
];

pub const XY_HARDCODED_TRADE_OFFSETS: [usize; 2] = [1, 8];
pub const XY_HARDCODED_TRADE_TEXTS: [usize; 2] = [129, 349];

const MAIN_GAME_SHOPS_XY: &[i32] = &[10, 11, 12, 13, 16, 17, 20, 21, 24, 25];
const MAIN_GAME_SHOPS_ORAS: &[i32] = &[10, 11, 13, 14, 16, 17, 18, 19, 20, 21];

const SHOP_NAMES_XY: &[&str] = &[
    "Primary 0 Badges",
    "Primary 1 Badges",
    "Primary 2 Badges",
    "Primary 3 Badges",
    "Primary 4 Badges",
    "Primary 5 Badges",
    "Primary 6 Badges",
    "Primary 7 Badges",
    "Primary 8 Badges",
    "Unused",
    "Lumiose Herboriste",
    "Lumiose Poké Ball Boutique",
    "Lumiose Stone Emporium",
    "Coumarine Incenses",
    "Aquacorde Poké Ball",
    "Aquacorde Potion",
    "Lumiose North Secondary",
    "Cyllage Secondary",
    "Shalour Secondary (TMs)",
    "Lumiose South Secondary (TMs)",
    "Laverre Secondary",
    "Snowbelle Secondary",
    "Kiloude Secondary (TMs)",
    "Anistar Secondary (TMs)",
    "Santalune Secondary",
    "Coumarine Secondary",
];

const SHOP_NAMES_ORAS: &[&str] = &[
    "Primary 0 Badges (After Pokédex)",
    "Primary 1 Badges",
    "Primary 2 Badges",
    "Primary 3 Badges",
    "Primary 4 Badges",
    "Primary 5 Badges",
    "Primary 6 Badges",
    "Primary 7 Badges",
    "Primary 8 Badges",
    "Primary 0 Badges (Before Pokédex)",
    "Slateport Incenses",
    "Slateport Vitamins",
    "Slateport TMs",
    "Rustboro Secondary",
    "Slateport Secondary",
    "Mauville Secondary (TMs)",
    "Verdanturf Secondary",
    "Fallarbor Secondary",
    "Lavaridge Herbs",
    "Lilycove Dept. Store 2F Left",
    "Lilycove Dept. Store 3F Left",
    "Lilycove Dept. Store 3F Right",
    "Lilycove Dept. Store 4F Left (TMs)",
    "Lilycove Dept. Store 4F Right (TMs)",
];

const REQUIRED_FIELD_TMS_XY: &[i32] = &[
    1, 9, 40, 19, 65, 73, 69, 74, 81, 57, 61, 97, 95, 71, 79, 30, 31, 36, 53, 29, 22, 3, 2, 80, 26,
];

const REQUIRED_FIELD_TMS_ORAS: &[i32] = &[
    37, 32, 62, 11, 86, 29, 59, 43, 53, 69, 6, 2, 13, 18, 22, 61, 30, 97, 7, 90, 26, 55, 34, 35,
    64, 65, 66, 74, 79, 80, 81, 84, 89, 91, 93, 95,
];

pub const FIELD_MOVES_XY: &[i32] = &[
    moves::CUT,
    moves::FLY,
    moves::SURF,
    moves::STRENGTH,
    moves::FLASH,
    moves::DIG,
    moves::TELEPORT,
    moves::WATERFALL,
    moves::SWEET_SCENT,
    moves::ROCK_SMASH,
];

pub const FIELD_MOVES_ORAS: &[i32] = &[
    moves::CUT,
    moves::FLY,
    moves::SURF,
    moves::STRENGTH,
    moves::FLASH,
    moves::DIG,
    moves::TELEPORT,
    moves::WATERFALL,
    moves::SWEET_SCENT,
    moves::ROCK_SMASH,
    moves::SECRET_POWER,
    moves::DIVE,
];

pub use tables::{ACTUALLY_COSMETIC_FORMS, EVOLUTION_ITEMS, UNIQUE_NO_SELL_ITEMS};

pub fn is_actually_cosmetic(number: i32) -> bool {
    ACTUALLY_COSMETIC_FORMS.contains(&number)
}

pub fn forme_suffix(number: i32) -> &'static str {
    tables::FORME_SUFFIXES
        .iter()
        .find(|(candidate, _)| *candidate == number)
        .map_or("", |(_, suffix)| suffix)
}

fn mega_species_oras() -> Vec<i32> {
    vec![
        species::VENUSAUR,
        species::CHARIZARD,
        species::BLASTOISE,
        species::ALAKAZAM,
        species::GENGAR,
        species::KANGASKHAN,
        species::PINSIR,
        species::GYARADOS,
        species::AERODACTYL,
        species::MEWTWO,
        species::AMPHAROS,
        species::SCIZOR,
        species::HERACROSS,
        species::HOUNDOOM,
        species::TYRANITAR,
        species::BLAZIKEN,
        species::GARDEVOIR,
        species::MAWILE,
        species::AGGRON,
        species::MEDICHAM,
        species::MANECTRIC,
        species::BANETTE,
        species::ABSOL,
        species::LATIAS,
        species::LATIOS,
        species::GARCHOMP,
        species::LUCARIO,
        species::ABOMASNOW,
        species::BEEDRILL,
        species::PIDGEOT,
        species::SLOWBRO,
        species::STEELIX,
        species::SCEPTILE,
        species::SWAMPERT,
        species::SABLEYE,
        species::SHARPEDO,
        species::CAMERUPT,
        species::ALTARIA,
        species::GLALIE,
        species::SALAMENCE,
        species::METAGROSS,
        species::LOPUNNY,
        species::GALLADE,
        species::AUDINO,
        species::DIANCIE,
    ]
}

fn named_forme_suffixes(base_forme: i32) -> Option<&'static [(i32, &'static str)]> {
    Some(match base_forme {
        species::DEOXYS => &[(1, "-A"), (2, "-D"), (3, "-S")],
        species::WORMADAM => &[(1, "-S"), (2, "-T")],
        species::SHAYMIN => &[(1, "-S")],
        species::GIRATINA => &[(1, "-O")],
        species::ROTOM => &[(1, "-H"), (2, "-W"), (3, "-Fr"), (4, "-Fa"), (5, "-M")],
        species::CASTFORM => &[(1, "-F"), (2, "-W"), (3, "-I")],
        species::BASCULIN => &[(1, "-B")],
        species::DARMANITAN => &[(1, "-Z")],
        species::MELOETTA => &[(1, "-P")],
        species::KYUREM => &[(1, "-W"), (2, "-B")],
        species::TORNADUS | species::THUNDURUS | species::LANDORUS => &[(1, "-T")],
        species::MEOWSTIC => &[(1, "-F")],
        species::AEGISLASH => &[(1, "-B")],
        species::PUMPKABOO | species::GOURGEIST => &[(1, "-M"), (2, "-L"), (3, "-XL")],
        species::FLOETTE => &[(5, "-E")],
        species::KYOGRE | species::GROUDON => &[(1, "-P")],
        species::RAYQUAZA => &[(1, "-Mega")],
        species::HOOPA => &[(1, "-U")],
        _ => return None,
    })
}

pub fn forme_suffix_by_base_forme(base_forme: i32, form_number: i32) -> &'static str {
    if mega_species_oras().contains(&base_forme) {
        let mega: &[(i32, &str)] =
            if base_forme == species::CHARIZARD || base_forme == species::MEWTWO {
                &[(1, "-Mega-X"), (2, "-Mega-Y")]
            } else {
                &[(1, "-Mega")]
            };
        return lookup_suffix(mega, form_number);
    }
    match named_forme_suffixes(base_forme) {
        Some(suffixes) => lookup_suffix(suffixes, form_number),
        None => "",
    }
}

fn lookup_suffix(suffixes: &[(i32, &'static str)], form_number: i32) -> &'static str {
    suffixes
        .iter()
        .find(|(number, _)| *number == form_number)
        .map_or("", |(_, suffix)| suffix)
}

pub fn irregular_formes(rom_type: i32) -> &'static [i32] {
    match rom_type {
        TYPE_XY => tables::XY_IRREGULAR_FORMES,
        TYPE_ORAS => tables::ORAS_IRREGULAR_FORMES,
        _ => &[],
    }
}

pub fn forme_count(rom_type: i32) -> i32 {
    match rom_type {
        TYPE_XY => XY_FORME_COUNT,
        TYPE_ORAS => ORAS_FORME_COUNT,
        _ => 0,
    }
}

pub fn move_count(rom_type: i32) -> i32 {
    if rom_type == TYPE_ORAS { 621 } else { 617 }
}

pub fn highest_ability_index(rom_type: i32) -> i32 {
    if rom_type == TYPE_ORAS {
        abilities::DELTA_STREAM
    } else {
        abilities::AURA_BREAK
    }
}

pub fn bs_size(rom_type: i32) -> usize {
    if rom_type == TYPE_XY { 0x40 } else { 0x50 }
}

pub fn static_pokemon_count(rom_type: i32) -> usize {
    if rom_type == TYPE_ORAS { 0x3B } else { 0xD }
}

pub fn gift_pokemon_count(rom_type: i32) -> usize {
    if rom_type == TYPE_ORAS { 0x25 } else { 0x13 }
}

pub fn gift_pokemon_size(rom_type: i32) -> usize {
    if rom_type == TYPE_ORAS { 0x24 } else { 0x18 }
}

pub fn tm_block_two_starting_offset(rom_type: i32) -> i32 {
    if rom_type == TYPE_ORAS { 98 } else { 97 }
}

pub fn hm_count(rom_type: i32) -> i32 {
    if rom_type == TYPE_ORAS { 7 } else { 5 }
}

pub fn shop_items_locator(rom_type: i32) -> &'static str {
    if rom_type == TYPE_ORAS {
        "04001100120004000300"
    } else {
        "0400110004000300"
    }
}

pub fn ingame_trades_prefix(rom_type: i32) -> &'static str {
    if rom_type == TYPE_XY {
        "BA0A02015E000100BC0A150069000100"
    } else {
        "810B7A0097000A00000047006B000A00"
    }
}

pub fn save_load_forme_reversion_prefix(rom_type: i32) -> &'static str {
    if rom_type == TYPE_XY {
        "09EB000094E5141094E54A0B80E2"
    } else {
        "09EB000094E5141094E5120A80E2"
    }
}

pub fn required_field_tms(rom_type: i32) -> &'static [i32] {
    if rom_type == TYPE_XY {
        REQUIRED_FIELD_TMS_XY
    } else {
        REQUIRED_FIELD_TMS_ORAS
    }
}

pub fn main_game_shops(rom_type: i32) -> &'static [i32] {
    if rom_type == TYPE_XY {
        MAIN_GAME_SHOPS_XY
    } else {
        MAIN_GAME_SHOPS_ORAS
    }
}

pub fn shop_names(rom_type: i32) -> &'static [&'static str] {
    if rom_type == TYPE_XY {
        SHOP_NAMES_XY
    } else {
        SHOP_NAMES_ORAS
    }
}

pub fn map_num_to_pokedex_index(rom_type: i32) -> &'static [i32] {
    if rom_type == TYPE_XY {
        tables::XY_MAP_NUM_TO_POKEDEX_INDEX
    } else {
        tables::ORAS_MAP_NUM_TO_POKEDEX_INDEX
    }
}

pub fn balanced_item_price(item: i32) -> Option<i32> {
    tables::BALANCED_ITEM_PRICES
        .iter()
        .find(|(candidate, _)| *candidate == item)
        .map(|(_, price)| *price)
}

pub fn is_mega_stone(item: i32) -> bool {
    (items::GENGARITE..=items::LATIOSITE).contains(&item)
        || (items::SWAMPERTITE..=items::DIANCITE).contains(&item)
        || (items::CAMERUPTITE..=items::BEEDRILLITE).contains(&item)
}

pub const MOVE_CATEGORY_INDICES: [MoveCategory; 3] = [
    MoveCategory::Status,
    MoveCategory::Physical,
    MoveCategory::Special,
];

pub fn move_category_to_byte(category: MoveCategory) -> u8 {
    match category {
        MoveCategory::Physical => 1,
        MoveCategory::Special => 2,
        MoveCategory::Status => 0,
    }
}

const TYPE_TABLE: [Type; 18] = [
    Type::Normal,
    Type::Fighting,
    Type::Flying,
    Type::Poison,
    Type::Ground,
    Type::Rock,
    Type::Bug,
    Type::Ghost,
    Type::Steel,
    Type::Fire,
    Type::Water,
    Type::Grass,
    Type::Electric,
    Type::Psychic,
    Type::Ice,
    Type::Dragon,
    Type::Dark,
    Type::Fairy,
];

pub fn type_from_byte(value: u8) -> Option<Type> {
    TYPE_TABLE.get(value as usize).copied()
}

pub fn type_to_byte(kind: Option<Type>) -> u8 {
    kind.and_then(|kind| TYPE_TABLE.iter().position(|&candidate| candidate == kind))
        .map_or(0, |index| index as u8)
}

pub fn allowed_items_xy() -> ItemList {
    let mut list = ItemList::new(items::MEGA_GLOVE as usize);
    list.ban_range(items::EXPLORER_KIT, 76);
    list.ban_range(items::DATA_CARD01, 32);
    list.ban_range(items::XTRANSCEIVER_MALE, 18);
    list.ban_singles(&[
        items::EXP_SHARE,
        items::LIBERTY_PASS,
        items::PROP_CASE,
        items::DRAGON_SKULL,
        items::LIGHT_STONE,
        items::DARK_STONE,
    ]);
    list.ban_range(items::TEA, 3);
    list.ban_range(items::UNUSED120, 14);
    list.tm_range(items::TM01, 92);
    list.tm_range(items::TM93, 3);
    list.ban_range(items::TM01, 100);
    list.ban_range(items::TM93, 3);
    list.ban_range(items::DIRE_HIT2, 24);
    list.ban_range(items::HOLO_CASTER_MALE, 3);
    list.ban_singles(&[items::POKE_FLUTE, items::SPRINKLOTAD]);
    list.ban_range(items::POWER_PLANT_PASS, 4);
    list.ban_range(items::ELEVATOR_KEY, 4);
    list.ban_range(items::LENS_CASE, 3);
    list.ban_range(items::LOOKER_TICKET, 3);
    list.ban_range(items::MEGA_CHARM, 2);
    list.tm_range(items::TM96, 5);
    list.ban_range(items::TM96, 5);
    list
}

pub fn allowed_items_oras() -> ItemList {
    let mut list = allowed_items_xy().copy_with_max(items::EON_FLUTE as usize);
    list.ban_range(items::MACH_BIKE, 34);
    list.ban_range(items::PRISON_BOTTLE, 2);
    list.ban_range(items::METEORITE_THIRD_FORM, 5);
    list
}

pub fn allowed_items(rom_type: i32) -> ItemList {
    if rom_type == TYPE_XY {
        allowed_items_xy()
    } else {
        allowed_items_oras()
    }
}

fn ban_non_bad_common(list: &mut ItemList) {
    list.ban_singles(&[
        items::ODD_KEYSTONE,
        items::GRISEOUS_ORB,
        items::SOUL_DEW,
        items::LIGHT_BALL,
        items::ORAN_BERRY,
        items::QUICK_POWDER,
        items::PASS_ORB,
        items::DISCOUNT_COUPON,
        items::STRANGE_SOUVENIR,
    ]);
    list.ban_range(items::GROWTH_MULCH, 4);
    list.ban_range(items::ADAMANT_ORB, 2);
    list.ban_range(items::MAIL1, 12);
    list.ban_range(items::FIGY_BERRY, 25);
    list.ban_range(items::LUCKY_PUNCH, 4);
    list.ban_range(items::RED_SCARF, 5);
    list.ban_range(items::RELIC_COPPER, 7);
    list.ban_range(items::RICH_MULCH, 4);
}

pub fn non_bad_items(rom_type: i32) -> ItemList {
    let mut list = allowed_items(rom_type);
    ban_non_bad_common(&mut list);
    if rom_type == TYPE_XY {
        list.ban_range(items::SHOAL_SALT, 6);
    }
    list
}

fn inclusive(start: i32, end: i32) -> impl Iterator<Item = i32> {
    start..=end
}

pub fn regular_shop_items() -> Vec<i32> {
    inclusive(items::ULTRA_BALL, items::POKE_BALL)
        .chain(inclusive(items::POTION, items::REVIVE))
        .chain(inclusive(items::SUPER_REPEL, items::REPEL))
        .collect()
}

pub fn op_shop_items() -> Vec<i32> {
    let mut list = vec![
        items::LAVA_COOKIE,
        items::BERRY_JUICE,
        items::RARE_CANDY,
        items::OLD_GATEAU,
    ];
    list.extend(inclusive(items::BLUE_FLUTE, items::SHOAL_SHELL));
    list.extend(inclusive(items::TINY_MUSHROOM, items::NUGGET));
    list.push(items::RARE_BONE);
    list.extend(inclusive(items::LANSAT_BERRY, items::ROWAP_BERRY));
    list.push(items::LUCKY_EGG);
    list.push(items::PRETTY_FEATHER);
    list.extend(inclusive(items::BALM_MUSHROOM, items::CASTELIACONE));
    list
}

pub fn falling_encounter_name(index: usize) -> Option<&'static str> {
    Some(match index {
        0 => "Glittering Cave Ceiling Encounter",
        4 => "Reflection Cave Ceiling Encounter",
        20 => "Victory Road Outside 2 Sky Encounter",
        24 => "Victory Road Inside 2 Encounter",
        28 => "Victory Road Outside 3 Sky Encounter",
        32 => "Victory Road Inside 3 Ceiling Encounter",
        36 => "Victory Road Outside 4 Sky Encounter",
        46 => "Terminus Cave Ceiling Encounter",
        _ => return None,
    })
}

pub fn rustling_bush_encounter_name(index: usize) -> Option<&'static str> {
    Some(match index {
        0 => "Route 6 Rustling Bush Encounter",
        3 => "Route 18 Rustling Bush Encounter",
        _ => return None,
    })
}

fn tag(trainers: &mut [Trainer], tag: &str, numbers: &[usize]) {
    for &number in numbers {
        if trainers.len() > number - 1 {
            trainers[number - 1].tag = Some(tag.to_owned());
        }
    }
}

fn tag_rival(trainers: &mut [Trainer], tag: &str, offset: usize) {
    trainers[offset - 1].tag = Some(format!("{tag}-0"));
    trainers[offset].tag = Some(format!("{tag}-1"));
    trainers[offset + 1].tag = Some(format!("{tag}-2"));
}

pub fn tag_trainers_xy(trainers: &mut [Trainer]) {
    tag(trainers, "GYM1", &[39, 40, 48]);
    tag(trainers, "GYM2", &[64, 63, 106, 105]);
    tag(trainers, "GYM3", &[83, 84, 146, 147]);
    tag(trainers, "GYM4", &[121, 122, 123, 124]);
    tag(
        trainers,
        "GYM5",
        &[461, 462, 463, 464, 465, 466, 467, 468, 469, 28, 29, 30],
    );
    tag(trainers, "GYM6", &[245, 250, 248, 243]);
    tag(trainers, "GYM7", &[170, 171, 172, 365, 366]);
    tag(trainers, "GYM8", &[168, 169, 31, 32]);
    tag(trainers, "GYM1-LEADER", &[6]);
    tag(trainers, "GYM2-LEADER", &[76]);
    tag(trainers, "GYM3-LEADER", &[21]);
    tag(trainers, "GYM4-LEADER", &[22]);
    tag(trainers, "GYM5-LEADER", &[23]);
    tag(trainers, "GYM6-LEADER", &[24]);
    tag(trainers, "GYM7-LEADER", &[25]);
    tag(trainers, "GYM8-LEADER", &[26]);
    tag(trainers, "NOTSTRONG", &[188]);
    tag(trainers, "ELITE1", &[269]);
    tag(trainers, "ELITE2", &[271]);
    tag(trainers, "ELITE3", &[187]);
    tag(trainers, "ELITE4", &[270]);
    tag(trainers, "CHAMPION", &[276]);
    tag(trainers, "THEMED:LYSANDRE-LEADER", &[303, 525, 526]);
    tag(
        trainers,
        "STRONG",
        &[
            174, 175, 304, 344, 345, 346, 347, 348, 349, 350, 351, 470, 471, 472, 473, 474, 475,
            476, 477, 478, 479,
        ],
    );
    tag(trainers, "STRONG", &[324, 325, 438, 439, 573]);
    tag(trainers, "STRONG", &[327, 328]);
    for (rival, offset) in [
        ("RIVAL1", 596),
        ("RIVAL2", 575),
        ("RIVAL3", 581),
        ("RIVAL4", 578),
        ("RIVAL5", 584),
        ("RIVAL6", 607),
        ("RIVAL7", 587),
        ("RIVAL8", 590),
        ("RIVAL9", 593),
        ("RIVAL10", 599),
        ("RIVAL1", 435),
        ("RIVAL2", 130),
        ("RIVAL3", 329),
        ("RIVAL4", 184),
        ("RIVAL5", 332),
        ("RIVAL6", 604),
        ("RIVAL7", 335),
        ("RIVAL8", 338),
        ("RIVAL9", 341),
        ("RIVAL10", 519),
        ("FRIEND1", 137),
        ("FRIEND2", 321),
    ] {
        tag_rival(trainers, rival, offset);
    }
}

pub fn tag_trainers_oras(trainers: &mut [Trainer]) {
    tag(trainers, "GYM1", &[562, 22, 667]);
    tag(trainers, "GYM2", &[60, 56, 59]);
    tag(trainers, "GYM3", &[34, 568, 614, 35]);
    tag(trainers, "GYM4", &[81, 824, 83, 615, 823, 613, 85]);
    tag(trainers, "GYM5", &[63, 64, 65, 66, 67, 68, 69]);
    tag(trainers, "GYM6", &[115, 517, 516, 118, 730]);
    tag(trainers, "GYM7", &[157, 158, 159, 226, 320, 225]);
    tag(trainers, "GYM8", &[647, 342, 594, 646, 338, 339, 340, 341]);
    tag(trainers, "GYM1-LEADER", &[561]);
    tag(trainers, "GYM2-LEADER", &[563]);
    tag(trainers, "GYM3-LEADER", &[567]);
    tag(trainers, "GYM4-LEADER", &[569]);
    tag(trainers, "GYM5-LEADER", &[570]);
    tag(trainers, "GYM6-LEADER", &[571]);
    tag(trainers, "GYM7-LEADER", &[552]);
    tag(trainers, "GYM8-LEADER", &[572, 943]);
    tag(trainers, "ELITE1", &[553, 909]);
    tag(trainers, "ELITE2", &[554, 910]);
    tag(trainers, "ELITE3", &[555, 911]);
    tag(trainers, "ELITE4", &[556, 912]);
    tag(trainers, "CHAMPION", &[557, 913, 680, 942]);
    tag(trainers, "THEMED:MAXIE-LEADER", &[235, 236, 271]);
    tag(trainers, "THEMED:ARCHIE-LEADER", &[178, 231, 266]);
    tag(trainers, "THEMED:MATT-STRONG", &[683, 684, 685, 686, 687]);
    tag(trainers, "THEMED:SHELLY-STRONG", &[688, 689, 690]);
    tag(trainers, "THEMED:TABITHA-STRONG", &[691, 692, 693]);
    tag(
        trainers,
        "THEMED:COURTNEY-STRONG",
        &[694, 695, 696, 697, 698],
    );
    tag(trainers, "THEMED:WALLY-STRONG", &[518, 583, 944, 946]);
    for (rival, offset) in [
        ("RIVAL1", 1),
        ("RIVAL2", 289),
        ("RIVAL3", 674),
        ("RIVAL4", 292),
        ("RIVAL5", 527),
        ("RIVAL6", 699),
        ("RIVAL1", 4),
        ("RIVAL2", 295),
        ("RIVAL3", 677),
        ("RIVAL4", 298),
        ("RIVAL5", 530),
        ("RIVAL6", 906),
    ] {
        tag_rival(trainers, rival, offset);
    }
}

fn set_multi_battle_status(trainers: &mut [Trainer], numbers: &[usize]) {
    for &number in numbers {
        if trainers.len() > number - 1 {
            trainers[number - 1].multi_battle_status = MultiBattleStatus::Always;
        }
    }
}

pub fn set_multi_battle_status_xy(trainers: &mut [Trainer]) {
    set_multi_battle_status(
        trainers,
        &[108, 111, 348, 350, 438, 439, 470, 472, 476, 610, 611, 612],
    );
}

pub fn set_multi_battle_status_oras(trainers: &mut [Trainer]) {
    set_multi_battle_status(
        trainers,
        &[683, 687, 688, 691, 694, 698, 900, 901, 902, 903, 904, 905],
    );
}
