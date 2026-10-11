use std::collections::BTreeMap;

use randomon_ctr::{TYPE_SM, TYPE_USUM};
use randomon_data::constants::species::{sm_formes, usum_formes};
use randomon_data::constants::{abilities, items, moves, species};
use randomon_data::{ItemList, MoveCategory, MultiBattleStatus, Trainer, Type};

use crate::tables;

pub use tables::{EVOLUTION_ITEMS, HELD_Z_CRYSTALS};

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
pub const BS_CALL_RATE_OFFSET: usize = 27;
pub const BS_FORME_OFFSET: usize = 28;
pub const BS_FORME_COUNT_OFFSET: usize = 32;
pub const BS_TMHM_COMPAT_OFFSET: usize = 40;
pub const BS_MT_COMPAT_OFFSET: usize = 60;
pub const BS_SIZE: usize = 0x54;

pub const EVOLUTION_METHOD_COUNT: i32 = 42;

pub const NO_DAMAGE_TARGET_TRAPPING_EFFECT: i32 = 106;
pub const NO_DAMAGE_FIELD_TRAPPING_EFFECT: i32 = 354;
pub const DAMAGE_ADJACENT_FOES_TRAPPING_EFFECT: i32 = 373;
pub const DAMAGE_TARGET_TRAPPING_EFFECT: i32 = 384;

pub const NO_DAMAGE_STATUS_QUALITY: i32 = 1;
pub const NO_DAMAGE_STAT_CHANGE_QUALITY: i32 = 2;
pub const DAMAGE_STATUS_QUALITY: i32 = 4;
pub const NO_DAMAGE_STATUS_AND_STAT_CHANGE_QUALITY: i32 = 5;
pub const DAMAGE_TARGET_DEBUFF_QUALITY: i32 = 6;
pub const DAMAGE_USER_BUFF_QUALITY: i32 = 7;
pub const DAMAGE_ABSORB_QUALITY: i32 = 8;

pub const BANNED_MOVES: &[i32] = &[moves::DARK_VOID, moves::HYPERSPACE_FURY];

pub const TM_COUNT: i32 = 100;
pub const TM_BLOCK_ONE_COUNT: i32 = 92;
pub const TM_BLOCK_TWO_COUNT: i32 = 3;
pub const TM_BLOCK_THREE_COUNT: i32 = 5;
pub const TM_BLOCK_ONE_OFFSET: i32 = items::TM01;
pub const TM_BLOCK_TWO_OFFSET: i32 = items::TM93;
pub const TM_BLOCK_THREE_OFFSET: i32 = items::TM96;
pub const ITEM_PALETTES_PREFIX: &str = "070000000000000000010100";

pub const TUTORS_OFFSET: usize = 0x54DE;
pub const TUTORS_PREFIX: &str = "5F6F6E5F6F6666FF";
pub const TUTOR_MOVE_COUNT: usize = 67;

pub const FASTEST_TEXT_PREFIXES: [&str; 2] = ["1080BDE80E000500F0412DE9", "34019FE50060A0E3"];

pub const AFTER_BATTLE_FORME_REVERSION_PREFIX: &str = "0055E10B00001A0010A0E30700A0E1";
pub const NINJASK_SPECIES_PREFIX: &str = "11FF2FE11CD08DE2F080BDE8";
pub const SHEDINJA_PREFIX: &str = "A0E194FDFFEB0040A0E1";
pub const BEAST_LUSAMINE_POKEMON_BOOSTS_PREFIX: &str = "1D14FFFF";
pub const BEAST_LUSAMINE_TRAINER_INDEX: usize = 157;
pub const MINIOR_WILD_ENCOUNTER_PATCH_PREFIX: &str = "032C42E2062052E2";
pub const ZYGARDE_ASSEMBLY_SCRIPT_FILE: usize = 45;
pub const ZYGARDE_ASSEMBLY_FORME_PREFIX: &str = "BC21CDE1B801CDE1";
pub const ZYGARDE_ASSEMBLY_SPECIES_PREFIX: &str = "FBEB4CD08DE20400A0E1F08FBDE8";
pub const FRIENDSHIP_VALUE_FOR_EVO_LOCATOR: &str = "DC0050E3F700002A";
pub const PERFECT_ODDS_BRANCH_LOCATOR: &str = "050000BA000050E3";

pub const USELESS_ABILITIES: &[i32] = &[
    abilities::FORECAST,
    abilities::MULTITYPE,
    abilities::FLOWER_GIFT,
    abilities::ZEN_MODE,
    abilities::STANCE_CHANGE,
    abilities::SHIELDS_DOWN,
    abilities::SCHOOLING,
    abilities::DISGUISE,
    abilities::BATTLE_BOND,
    abilities::POWER_CONSTRUCT,
    abilities::RKS_SYSTEM,
];

pub fn is_sm(rom_type: i32) -> bool {
    rom_type == TYPE_SM
}

pub fn pokemon_count(rom_type: i32) -> i32 {
    if rom_type == TYPE_USUM { 807 } else { 802 }
}

pub fn forme_count(rom_type: i32) -> i32 {
    if is_sm(rom_type) { 158 } else { 168 }
}

pub fn move_count(rom_type: i32) -> i32 {
    if rom_type == TYPE_USUM { 728 } else { 719 }
}

pub fn highest_ability_index(rom_type: i32) -> i32 {
    if rom_type == TYPE_USUM {
        abilities::NEUROFORCE
    } else {
        abilities::PRISM_ARMOR
    }
}

pub fn tm_data_prefix(rom_type: i32) -> &'static str {
    if rom_type == TYPE_USUM {
        "03BC03BD03BE03BF03"
    } else {
        "034003410342034303"
    }
}

pub fn shop_items_offset(rom_type: i32) -> usize {
    if is_sm(rom_type) { 0x50A8 } else { 0x50BC }
}

pub fn save_load_forme_reversion_prefix(rom_type: i32) -> &'static str {
    if is_sm(rom_type) {
        "00EB040094E50C1094E5F70E80E2"
    } else {
        "00EB040094E50C1094E5030B80E2EE0F80E2"
    }
}

pub fn irregular_formes(rom_type: i32) -> &'static [i32] {
    if rom_type == TYPE_USUM {
        tables::IRREGULAR_FORMES_USUM
    } else {
        tables::IRREGULAR_FORMES_SM
    }
}

pub fn actually_cosmetic_forms(rom_type: i32) -> &'static [i32] {
    if is_sm(rom_type) {
        tables::ACTUALLY_COSMETIC_FORMS_SM
    } else {
        tables::ACTUALLY_COSMETIC_FORMS_USUM
    }
}

pub fn ignore_forms(rom_type: i32) -> &'static [i32] {
    if is_sm(rom_type) {
        tables::IGNORE_FORMS_SM
    } else {
        tables::IGNORE_FORMS_USUM
    }
}

pub fn alt_formes_with_cosmetic_forms(rom_type: i32) -> &'static [(i32, i32)] {
    if is_sm(rom_type) {
        &[
            (sm_formes::RATICATE_A, 1),
            (sm_formes::ZYGARDE10, 1),
            (sm_formes::MINIOR_C, 6),
        ]
    } else {
        &[
            (usum_formes::RATICATE_A, 1),
            (usum_formes::MAROWAK_A, 1),
            (usum_formes::ZYGARDE10, 1),
            (usum_formes::MINIOR_C, 6),
        ]
    }
}

pub fn cosmetic_forms_of_alt_forme(rom_type: i32, number: i32) -> Option<i32> {
    alt_formes_with_cosmetic_forms(rom_type)
        .iter()
        .find(|(forme, _)| *forme == number)
        .map(|(_, count)| *count)
}

pub fn required_field_tms(rom_type: i32) -> Vec<i32> {
    let source = if is_sm(rom_type) {
        tables::REQUIRED_FIELD_TMS_SM
    } else {
        tables::REQUIRED_FIELD_TMS_USUM
    };
    let mut distinct = Vec::new();
    for &tm in source {
        if !distinct.contains(&tm) {
            distinct.push(tm);
        }
    }
    distinct
}

pub fn main_game_shops(rom_type: i32) -> &'static [i32] {
    if is_sm(rom_type) {
        tables::MAIN_GAME_SHOPS_SM
    } else {
        tables::MAIN_GAME_SHOPS_USUM
    }
}

pub fn shop_names(rom_type: i32) -> Vec<String> {
    let mut names: Vec<&str> = vec![
        "Primary 0 Trials",
        "Primary 1 Trials",
        "Primary 2 Trials",
        "Primary 3 Trials",
        "Primary 4 Trials",
        "Primary 5 Trials",
        "Primary 6 Trials",
        "Primary 7 Trials",
        "Konikoni City Incenses",
        "Konikoni City Herbs",
        "Hau'oli City Secondary",
        "Route 2 Secondary",
        "Heahea City Secondary (TMs)",
        "Royal Avenue Secondary (TMs)",
        "Route 8 Secondary",
        "Paniola Town Secondary",
        "Malie City Secondary (TMs)",
        "Mount Hokulani Secondary",
        "Seafolk Village Secondary (TMs)",
        "Konikoni City TMs",
        "Konikoni City Stones",
        "Thrifty Megamart, Center-Left",
        "Thrifty Megamart, Center-Right",
        "Thrifty Megamart, Right",
    ];
    if rom_type == TYPE_USUM {
        names.extend([
            "Route 5 Secondary",
            "Konikoni City Secondary",
            "Tapu Village Secondary",
            "Mount Lanakila Secondary",
        ]);
    }
    names.into_iter().map(str::to_owned).collect()
}

pub fn relevant_encounter_files(rom_type: i32) -> Vec<bool> {
    let count = if is_sm(rom_type) { 2761 } else { 3696 };
    (0..count)
        .map(|index: i32| (index - 9) % 11 == 0 || index % 11 == 0)
        .collect()
}

pub fn hardcoded_trade_text_offsets(rom_type: i32, trade: usize) -> Option<&'static [usize]> {
    match trade {
        0 if rom_type == TYPE_USUM => Some(&[20, 21, 22]),
        1 => Some(&[26, 28, 30]),
        2 => Some(&[32, 33, 34, 36]),
        3 => Some(&[38, 39, 40, 42]),
        4 => Some(&[44, 45, 46, 48]),
        5 => Some(&[50, 51, 52, 54]),
        6 => Some(&[56, 57, 58, 60]),
        _ => None,
    }
}

pub fn is_z_crystal(item: i32) -> bool {
    (items::NORMALIUM_ZHELD..=items::PIKANIUM_ZHELD).contains(&item)
        || (items::DECIDIUM_ZHELD..=items::PIKASHUNIUM_ZBAG).contains(&item)
        || (items::SOLGANIUM_ZBAG..=items::KOMMONIUM_ZBAG).contains(&item)
}

pub fn is_mega_stone(item: i32) -> bool {
    (items::GENGARITE..=items::LATIOSITE).contains(&item)
        || (items::SWAMPERTITE..=items::DIANCITE).contains(&item)
        || (items::CAMERUPTITE..=items::BEEDRILLITE).contains(&item)
}

pub fn balanced_item_price(item: i32) -> Option<i32> {
    tables::BALANCED_ITEM_PRICES
        .iter()
        .find(|(candidate, _)| *candidate == item)
        .map(|(_, price)| *price)
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

fn mega_species() -> &'static [i32] {
    &[
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
        species::WISHIWASHI => &[(1, "-S")],
        species::ORICORIO => &[(1, "-E"), (2, "-P"), (3, "-G")],
        species::LYCANROC => &[(1, "-M"), (2, "-D")],
        species::GRENINJA => &[(2, "-A")],
        species::ZYGARDE => &[(1, "-10"), (4, "-C")],
        species::MINIOR => &[(7, "-C")],
        species::NECROZMA => &[(1, "-DM"), (2, "-DW"), (3, "-U")],
        _ => return None,
    })
}

fn lookup_suffix(suffixes: &[(i32, &'static str)], form_number: i32) -> &'static str {
    suffixes
        .iter()
        .find(|(number, _)| *number == form_number)
        .map_or("", |(_, suffix)| suffix)
}

pub fn forme_suffix_by_base_forme(base_forme: i32, form_number: i32) -> &'static str {
    if mega_species().contains(&base_forme) {
        let mega: &[(i32, &str)] =
            if base_forme == species::CHARIZARD || base_forme == species::MEWTWO {
                &[(1, "-Mega-X"), (2, "-Mega-Y")]
            } else {
                &[(1, "-Mega")]
            };
        return lookup_suffix(mega, form_number);
    }
    if tables::SPECIES_WITH_ALOLAN_FORMS.contains(&base_forme) {
        return lookup_suffix(&[(1, "-A")], form_number);
    }
    named_forme_suffixes(base_forme).map_or("", |suffixes| lookup_suffix(suffixes, form_number))
}

fn allowed_items_sm() -> ItemList {
    let mut list = ItemList::new(items::FAIRY_MEMORY as usize);
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
    list.ban_range(items::MACH_BIKE, 34);
    list.ban_range(items::PRISON_BOTTLE, 2);
    list.ban_range(items::METEORITE_THIRD_FORM, 5);
    list.ban_range(items::NORMALIUM_ZHELD, 19);
    list.ban_range(items::DECIDIUM_ZHELD, 39);
    list.ban_singles(&[
        items::Z_RING,
        items::SPARKLING_STONE,
        items::ZYGARDE_CUBE,
        items::RIDE_PAGER,
        items::SUN_FLUTE,
        items::MOON_FLUTE,
        items::ENIGMATIC_CARD,
    ]);
    list.ban_range(items::FORAGE_BAG, 3);
    list.ban_singles(&[items::UNUSED848, items::UNUSED859]);
    list.ban_range(items::UNUSED837, 4);
    list.ban_range(items::SILVER_RAZZ_BERRY, 18);
    list.ban_range(items::STRETCHY_SPRING, 19);
    list
}

fn allowed_items_usum() -> ItemList {
    let mut list = allowed_items_sm().copy_with_max(items::ROTO_CATCH as usize);
    list.ban_range(items::SOLGANIUM_ZBAG, 12);
    list.ban_range(items::Z_POWER_RING, 16);
    list.ban_range(items::ROTO_HATCH, 11);
    list
}

pub fn allowed_items(rom_type: i32) -> ItemList {
    if is_sm(rom_type) {
        allowed_items_sm()
    } else {
        allowed_items_usum()
    }
}

pub fn non_bad_items() -> ItemList {
    let mut list = allowed_items_sm();
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
        items::FESTIVAL_TICKET,
    ]);
    list.ban_range(items::GROWTH_MULCH, 4);
    list.ban_range(items::ADAMANT_ORB, 2);
    list.ban_range(items::MAIL1, 12);
    list.ban_range(items::FIGY_BERRY, 25);
    list.ban_range(items::LUCKY_PUNCH, 4);
    list.ban_range(items::RED_SCARF, 5);
    list.ban_range(items::RICH_MULCH, 4);
    list.ban_range(items::GENGARITE, 30);
    list.ban_range(items::SWAMPERTITE, 13);
    list.ban_range(items::CAMERUPTITE, 4);
    list.ban_range(items::FIGHTING_MEMORY, 17);
    list.ban_range(items::RELIC_COPPER, 7);
    list.ban_singles(&[items::SHOAL_SALT, items::SHOAL_SHELL]);
    list.ban_range(items::BLUE_FLUTE, 5);
    list
}

pub fn regular_shop_items(rom_type: i32) -> Vec<i32> {
    let mut list: Vec<i32> = (items::ULTRA_BALL..=items::POKE_BALL)
        .chain(items::POTION..=items::REVIVE)
        .chain(items::SUPER_REPEL..=items::REPEL)
        .collect();
    list.push(items::HONEY);
    list.push(items::ADRENALINE_ORB);
    if !is_sm(rom_type) {
        list.push(items::POKE_TOY);
    }
    list
}

pub fn op_shop_items() -> Vec<i32> {
    let mut list = vec![
        items::LAVA_COOKIE,
        items::BERRY_JUICE,
        items::RARE_CANDY,
        items::OLD_GATEAU,
    ];
    list.extend(items::TINY_MUSHROOM..=items::NUGGET);
    list.push(items::RARE_BONE);
    list.extend(items::LANSAT_BERRY..=items::ROWAP_BERRY);
    list.push(items::LUCKY_EGG);
    list.push(items::PRETTY_FEATHER);
    list.extend(items::BALM_MUSHROOM..=items::CASTELIACONE);
    list
}

pub fn ability_variations() -> BTreeMap<i32, Vec<i32>> {
    BTreeMap::from([
        (
            abilities::INSOMNIA,
            vec![abilities::INSOMNIA, abilities::VITAL_SPIRIT],
        ),
        (
            abilities::CLEAR_BODY,
            vec![
                abilities::CLEAR_BODY,
                abilities::WHITE_SMOKE,
                abilities::FULL_METAL_BODY,
            ],
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
            vec![abilities::CLOUD_NINE, abilities::AIR_LOCK],
        ),
        (
            abilities::FILTER,
            vec![
                abilities::FILTER,
                abilities::SOLID_ROCK,
                abilities::PRISM_ARMOR,
            ],
        ),
        (
            abilities::ROUGH_SKIN,
            vec![abilities::ROUGH_SKIN, abilities::IRON_BARBS],
        ),
        (
            abilities::MOLD_BREAKER,
            vec![
                abilities::MOLD_BREAKER,
                abilities::TURBOBLAZE,
                abilities::TERAVOLT,
            ],
        ),
        (
            abilities::WIMP_OUT,
            vec![abilities::WIMP_OUT, abilities::EMERGENCY_EXIT],
        ),
        (
            abilities::QUEENLY_MAJESTY,
            vec![abilities::QUEENLY_MAJESTY, abilities::DAZZLING],
        ),
        (
            abilities::GOOEY,
            vec![abilities::GOOEY, abilities::TANGLING_HAIR],
        ),
        (
            abilities::RECEIVER,
            vec![abilities::RECEIVER, abilities::POWER_OF_ALCHEMY],
        ),
        (
            abilities::MULTISCALE,
            vec![abilities::MULTISCALE, abilities::SHADOW_SHIELD],
        ),
    ])
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

const FRIENDS_SM: [usize; 12] = [6, 9, 12, 76, 82, 438, 217, 220, 447, 450, 482, 356];
const FRIENDS_USUM: [usize; 12] = [491, 9, 12, 76, 82, 438, 217, 220, 447, 450, 494, 356];

fn tag_friends(trainers: &mut [Trainer], offsets: &[usize]) {
    for (index, &offset) in offsets.iter().enumerate() {
        tag_rival(trainers, &format!("FRIEND{}", index + 1), offset);
    }
}

pub fn tag_trainers_sm(trainers: &mut [Trainer]) {
    tag(trainers, "ELITE1", &[23, 152, 349]);
    tag(trainers, "ELITE2", &[90, 153, 351]);
    tag(trainers, "ELITE3", &[154, 403]);
    tag(trainers, "ELITE4", &[155, 359]);
    tag(trainers, "ELITE5", &[149, 350]);
    tag(trainers, "ELITE6", &[156, 352]);
    tag(trainers, "RIVAL2-0", &[129]);
    tag(trainers, "RIVAL2-1", &[413]);
    tag(trainers, "RIVAL2-2", &[414]);
    tag_rival(trainers, "RIVAL3", 477);
    tag_friends(trainers, &FRIENDS_SM);
    tag(
        trainers,
        "THEMED:GLADION-STRONG",
        &[79, 185, 239, 240, 415, 416, 417, 418, 419, 441],
    );
    tag(trainers, "THEMED:ILIMA-STRONG", &[52, 215, 216, 396]);
    tag(trainers, "THEMED:LANA-STRONG", &[144]);
    tag(trainers, "THEMED:KIAWE-STRONG", &[398]);
    tag(trainers, "THEMED:MALLOW-STRONG", &[146]);
    tag(trainers, "THEMED:SOPHOCLES-STRONG", &[405]);
    tag(trainers, "THEMED:MOLAYNE-STRONG", &[167, 481]);
    tag(trainers, "THEMED:MINA-STRONG", &[435, 467]);
    tag(trainers, "THEMED:PLUMERIA-STRONG", &[89, 238, 401]);
    tag(trainers, "THEMED:SINA-STRONG", &[75]);
    tag(trainers, "THEMED:DEXIO-STRONG", &[74, 412]);
    tag(trainers, "THEMED:FABA-STRONG", &[132, 241, 360, 410]);
    tag(trainers, "THEMED:GUZMA-LEADER", &[138, 235, 236, 400]);
    tag(trainers, "THEMED:LUSAMINE-LEADER", &[131, 158]);
}

pub fn tag_trainers_usum(trainers: &mut [Trainer]) {
    tag(trainers, "ELITE1", &[23, 650]);
    tag(trainers, "ELITE2", &[90, 153, 351]);
    tag(trainers, "ELITE3", &[154, 508]);
    tag(trainers, "ELITE4", &[359, 497]);
    tag(trainers, "ELITE5", &[489, 490]);
    tag(trainers, "ELITE6", &[149, 350]);
    tag(trainers, "ELITE7", &[156, 352]);
    tag_rival(trainers, "RIVAL2", 477);
    tag_friends(trainers, &FRIENDS_USUM);
    tag(
        trainers,
        "THEMED:GLADION-STRONG",
        &[79, 185, 239, 240, 415, 416, 417, 418, 419, 441],
    );
    tag(trainers, "THEMED:ILIMA-STRONG", &[52, 215, 216, 396, 502]);
    tag(trainers, "THEMED:LANA-STRONG", &[144, 503]);
    tag(trainers, "THEMED:KIAWE-STRONG", &[398, 504]);
    tag(trainers, "THEMED:MALLOW-STRONG", &[146, 505]);
    tag(trainers, "THEMED:SOPHOCLES-STRONG", &[405, 506]);
    tag(trainers, "THEMED:MINA-STRONG", &[507]);
    tag(trainers, "THEMED:PLUMERIA-STRONG", &[89, 238, 401]);
    tag(trainers, "THEMED:SINA-STRONG", &[75]);
    tag(trainers, "THEMED:DEXIO-STRONG", &[74, 412, 623]);
    tag(trainers, "THEMED:FABA-STRONG", &[132, 241, 410, 561]);
    tag(trainers, "THEMED:SOLIERA-STRONG", &[498, 499, 648, 651]);
    tag(trainers, "THEMED:DULSE-STRONG", &[500, 501, 649, 652]);
    tag(trainers, "THEMED:GUZMA-LEADER", &[138, 235, 236, 558, 647]);
    tag(trainers, "THEMED:LUSAMINE-LEADER", &[131, 644]);
    tag(
        trainers,
        "UBER",
        &[541, 542, 543, 580, 572, 573, 559, 560, 562, 645],
    );
}

fn set_multi_battle_status(trainers: &mut [Trainer], numbers: &[usize]) {
    for &number in numbers {
        if trainers.len() > number - 1 {
            trainers[number - 1].multi_battle_status = MultiBattleStatus::Always;
        }
    }
}

pub fn set_multi_battle_status_sm(trainers: &mut [Trainer]) {
    set_multi_battle_status(
        trainers,
        &[
            92, 93, 97, 98, 134, 136, 141, 227, 241, 262, 265, 270, 278, 280, 299, 303, 307, 315,
            316, 331, 332, 371, 372, 373, 374, 375, 376, 421, 422, 425, 426, 429, 430, 442, 443,
            444, 453, 454, 455, 456,
        ],
    );
}

pub fn set_multi_battle_status_usum(trainers: &mut [Trainer]) {
    set_multi_battle_status(
        trainers,
        &[
            92, 93, 97, 98, 134, 136, 141, 178, 227, 241, 262, 265, 270, 278, 280, 299, 303, 307,
            315, 316, 331, 332, 371, 372, 373, 374, 375, 376, 421, 422, 425, 426, 429, 430, 442,
            443, 444, 453, 454, 455, 456, 511, 514, 515, 521, 529, 530, 534, 544, 557, 561, 578,
            586, 595, 613, 617, 618, 619, 620, 626,
        ],
    );
}

pub fn set_forced_rival_starter_positions_usum(trainers: &mut [Trainer]) {
    for number in [12, 13, 14, 217, 218, 219] {
        trainers[number - 1].force_starter_position = 0;
    }
}
