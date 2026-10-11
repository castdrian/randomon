use std::collections::BTreeMap;

use crate::constants::{abilities, items, moves, species};
use crate::items::ItemList;
use crate::trainer::{MultiBattleStatus, Trainer};
use crate::types::{MoveCategory, Type};

use species::gen4_formes;

pub const CONSUMABLE_HELD_ITEMS: &[i32] = &[
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
    items::FIGY_BERRY,
    items::WIKI_BERRY,
    items::MAGO_BERRY,
    items::AGUAV_BERRY,
    items::IAPAPA_BERRY,
    items::OCCA_BERRY,
    items::PASSHO_BERRY,
    items::WACAN_BERRY,
    items::RINDO_BERRY,
    items::YACHE_BERRY,
    items::CHOPLE_BERRY,
    items::KEBIA_BERRY,
    items::SHUCA_BERRY,
    items::COBA_BERRY,
    items::PAYAPA_BERRY,
    items::TANGA_BERRY,
    items::CHARTI_BERRY,
    items::KASIB_BERRY,
    items::HABAN_BERRY,
    items::COLBUR_BERRY,
    items::BABIRI_BERRY,
    items::CHILAN_BERRY,
    items::LIECHI_BERRY,
    items::GANLON_BERRY,
    items::SALAC_BERRY,
    items::PETAYA_BERRY,
    items::APICOT_BERRY,
    items::LANSAT_BERRY,
    items::STARF_BERRY,
    items::ENIGMA_BERRY,
    items::MICLE_BERRY,
    items::CUSTAP_BERRY,
    items::JABOCA_BERRY,
    items::ROWAP_BERRY,
    items::BERRY_JUICE,
    items::WHITE_HERB,
    items::MENTAL_HERB,
    items::POWER_HERB,
    items::FOCUS_SASH,
];

pub const NON_CONSUMABLE_HELD_ITEMS: &[i32] = &[
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
    items::WIDE_LENS,
    items::MUSCLE_BAND,
    items::WISE_GLASSES,
    items::EXPERT_BELT,
    items::LIGHT_CLAY,
    items::LIFE_ORB,
    items::TOXIC_ORB,
    items::FLAME_ORB,
    items::ZOOM_LENS,
    items::METRONOME,
    items::IRON_BALL,
    items::LAGGING_TAIL,
    items::DESTINY_KNOT,
    items::BLACK_SLUDGE,
    items::ICY_ROCK,
    items::SMOOTH_ROCK,
    items::HEAT_ROCK,
    items::DAMP_ROCK,
    items::GRIP_CLAW,
    items::CHOICE_SCARF,
    items::STICKY_BARB,
    items::SHED_SHELL,
    items::BIG_ROOT,
    items::CHOICE_SPECS,
    items::FLAME_PLATE,
    items::SPLASH_PLATE,
    items::ZAP_PLATE,
    items::MEADOW_PLATE,
    items::ICICLE_PLATE,
    items::FIST_PLATE,
    items::TOXIC_PLATE,
    items::EARTH_PLATE,
    items::SKY_PLATE,
    items::MIND_PLATE,
    items::INSECT_PLATE,
    items::STONE_PLATE,
    items::SPOOKY_PLATE,
    items::DRACO_PLATE,
    items::DREAD_PLATE,
    items::IRON_PLATE,
    items::ODD_INCENSE,
    items::ROCK_INCENSE,
    items::FULL_INCENSE,
    items::WAVE_INCENSE,
    items::ROSE_INCENSE,
    items::RAZOR_CLAW,
    items::RAZOR_FANG,
];

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
    items::ENIGMA_BERRY,
    items::MICLE_BERRY,
    items::CUSTAP_BERRY,
    items::JABOCA_BERRY,
    items::ROWAP_BERRY,
    items::BERRY_JUICE,
    items::WHITE_HERB,
    items::MENTAL_HERB,
    items::FOCUS_SASH,
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
    items::WIDE_LENS,
    items::EXPERT_BELT,
    items::LIFE_ORB,
    items::ZOOM_LENS,
    items::DESTINY_KNOT,
    items::SHED_SHELL,
    items::RAZOR_CLAW,
    items::RAZOR_FANG,
];

pub fn consumable_held_items() -> Vec<i32> {
    CONSUMABLE_HELD_ITEMS.to_vec()
}

pub fn all_held_items() -> Vec<i32> {
    [NON_CONSUMABLE_HELD_ITEMS, CONSUMABLE_HELD_ITEMS].concat()
}

pub fn general_purpose_consumable_items() -> Vec<i32> {
    GENERAL_PURPOSE_CONSUMABLE_ITEMS.to_vec()
}

pub fn general_purpose_items() -> Vec<i32> {
    GENERAL_PURPOSE_ITEMS.to_vec()
}

pub fn type_boosting_items(kind: Type) -> Option<Vec<i32>> {
    let boosting: &[i32] = match kind {
        Type::Bug => &[items::SILVER_POWDER, items::INSECT_PLATE],
        Type::Dark => &[items::BLACK_GLASSES, items::DREAD_PLATE],
        Type::Dragon => &[items::DRAGON_FANG, items::DRACO_PLATE],
        Type::Electric => &[items::MAGNET, items::ZAP_PLATE],
        Type::Fighting => &[items::BLACK_BELT, items::FIST_PLATE],
        Type::Fire => &[items::CHARCOAL, items::FLAME_PLATE],
        Type::Flying => &[items::SHARP_BEAK, items::SKY_PLATE],
        Type::Ghost => &[items::SPELL_TAG, items::SPOOKY_PLATE],
        Type::Grass => &[
            items::MIRACLE_SEED,
            items::MEADOW_PLATE,
            items::ROSE_INCENSE,
        ],
        Type::Ground => &[items::SOFT_SAND, items::EARTH_PLATE],
        Type::Ice => &[items::NEVER_MELT_ICE, items::ICICLE_PLATE],
        Type::Normal => &[items::SILK_SCARF],
        Type::Poison => &[items::POISON_BARB, items::TOXIC_PLATE],
        Type::Psychic => &[items::TWISTED_SPOON, items::MIND_PLATE, items::ODD_INCENSE],
        Type::Rock => &[items::HARD_STONE, items::STONE_PLATE, items::ROCK_INCENSE],
        Type::Steel => &[items::METAL_COAT, items::IRON_PLATE],
        Type::Water => &[
            items::MYSTIC_WATER,
            items::SEA_INCENSE,
            items::SPLASH_PLATE,
            items::WAVE_INCENSE,
        ],
        _ => return None,
    };
    Some(boosting.to_vec())
}

pub fn move_boosting_items(move_number: i32) -> Option<Vec<i32>> {
    let boosting: &[i32] = match move_number {
        moves::BOUNCE
        | moves::DIG
        | moves::DIVE
        | moves::FLY
        | moves::RAZOR_WIND
        | moves::SKULL_BASH
        | moves::SKY_ATTACK
        | moves::SOLAR_BEAM => &[items::POWER_HERB],
        moves::FLING => &[items::TOXIC_ORB, items::FLAME_ORB, items::IRON_BALL],
        moves::TRICK | moves::SWITCHEROO => &[
            items::TOXIC_ORB,
            items::FLAME_ORB,
            items::FULL_INCENSE,
            items::LAGGING_TAIL,
        ],
        moves::TRICK_ROOM => &[items::IRON_BALL],
        moves::FACADE | moves::PSYCHO_SHIFT => &[items::TOXIC_ORB, items::FLAME_ORB],
        moves::LIGHT_SCREEN | moves::REFLECT => &[items::LIGHT_CLAY],
        moves::HAIL => &[items::ICY_ROCK],
        moves::SANDSTORM => &[items::SMOOTH_ROCK],
        moves::SUNNY_DAY => &[items::HEAT_ROCK],
        moves::RAIN_DANCE => &[items::DAMP_ROCK],
        moves::BIND
        | moves::CLAMP
        | moves::FIRE_SPIN
        | moves::MAGMA_STORM
        | moves::OUTRAGE
        | moves::SAND_TOMB
        | moves::UPROAR
        | moves::WHIRLPOOL
        | moves::WRAP => &[items::GRIP_CLAW],
        moves::ABSORB
        | moves::AQUA_RING
        | moves::DRAIN_PUNCH
        | moves::DREAM_EATER
        | moves::GIGA_DRAIN
        | moves::INGRAIN
        | moves::LEECH_LIFE
        | moves::LEECH_SEED
        | moves::MEGA_DRAIN => &[items::BIG_ROOT],
        _ => return None,
    };
    Some(boosting.to_vec())
}

pub fn weakness_reducing_berry(kind: Type) -> Option<i32> {
    match kind {
        Type::Fire => Some(items::OCCA_BERRY),
        Type::Water => Some(items::PASSHO_BERRY),
        Type::Electric => Some(items::WACAN_BERRY),
        Type::Grass => Some(items::RINDO_BERRY),
        Type::Ice => Some(items::YACHE_BERRY),
        Type::Fighting => Some(items::CHOPLE_BERRY),
        Type::Poison => Some(items::KEBIA_BERRY),
        Type::Ground => Some(items::SHUCA_BERRY),
        Type::Flying => Some(items::COBA_BERRY),
        Type::Psychic => Some(items::PAYAPA_BERRY),
        Type::Bug => Some(items::TANGA_BERRY),
        Type::Rock => Some(items::CHARTI_BERRY),
        Type::Ghost => Some(items::KASIB_BERRY),
        Type::Dragon => Some(items::HABAN_BERRY),
        Type::Dark => Some(items::COLBUR_BERRY),
        Type::Steel => Some(items::BABIRI_BERRY),
        _ => None,
    }
}

pub fn species_boosting_items(species_number: i32) -> Option<Vec<i32>> {
    let boosting: &[i32] = match species_number {
        species::DIALGA => &[items::ADAMANT_ORB],
        species::PALKIA => &[items::LUSTROUS_ORB],
        species::LATIAS | species::LATIOS => &[items::SOUL_DEW],
        species::CLAMPERL => &[items::DEEP_SEA_TOOTH, items::DEEP_SEA_SCALE],
        species::PIKACHU => &[items::LIGHT_BALL],
        species::CHANSEY => &[items::LUCKY_PUNCH],
        species::DITTO => &[items::METAL_POWDER, items::QUICK_POWDER],
        species::CUBONE | species::MAROWAK => &[items::THICK_CLUB],
        species::FARFETCHD => &[items::LEEK],
        _ => return None,
    };
    Some(boosting.to_vec())
}

pub fn ability_boosting_items(ability: i32) -> Option<Vec<i32>> {
    let boosting: &[i32] = match ability {
        abilities::GUTS => &[items::FLAME_ORB, items::TOXIC_ORB],
        abilities::MAGIC_GUARD => &[items::STICKY_BARB, items::LIFE_ORB],
        _ => return None,
    };
    Some(boosting.to_vec())
}

pub const TYPE_DP: i32 = 0;
pub const TYPE_PLAT: i32 = 1;
pub const TYPE_HGSS: i32 = 2;

pub const ARM9_OFFSET: i32 = 0x0200_0000;

pub const POKEMON_COUNT: i32 = 493;
pub const MOVE_COUNT: i32 = 467;
pub const DP_FORME_COUNT: i32 = 5;
pub const PLAT_HGSS_FORME_COUNT: i32 = 12;
pub const FORME_OFFSET: i32 = 2;

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
pub const BS_TMHM_COMPAT_OFFSET: i32 = 28;

pub const STARTER_CRIES_PREFIX: &str =
    "0004000C10BD0000000000000000000000E000000000000000E0000000000200";

pub const HGSS_STARTER_CODE_SUFFIX: &[u8] = &[0x03, 0x03, 0x1A, 0x12, 0x1, 0x23, 0x0, 0x0];

pub const HGSS_FILES_WITH_RIVAL_SCRIPT: &[i32] = &[7, 23, 96, 110, 819, 850, 866];

pub const HGSS_RIVAL_SCRIPT_MAGIC: &[u8] = &[
    0xCE, 0x00, 0x0C, 0x80, 0x11, 0x00, 0x0C, 0x80, 152, 0, 0x1C, 0x00, 0x05,
];

pub const PT_FILES_WITH_RIVAL_SCRIPT: &[i32] = &[31, 36, 112, 123, 186, 427, 429, 1096];

pub const DP_FILES_WITH_RIVAL_SCRIPT: &[i32] = &[34, 90, 118, 180, 195, 394];

pub const DPPT_RIVAL_SCRIPT_MAGIC: &[u8] = &[
    0xDE, 0x00, 0x0C, 0x80, 0x11, 0x00, 0x0C, 0x80, 0x83, 0x01, 0x1C, 0x00, 0x01,
];

pub const DPPT_TAG_BATTLE_SCRIPT_MAGIC1: &[u8] = &[0xDE, 0x00, 0x0C, 0x80, 0x28, 0x00, 0x04, 0x80];

pub const DPPT_TAG_BATTLE_SCRIPT_MAGIC2: &[u8] =
    &[0x11, 0x00, 0x0C, 0x80, 0x86, 0x01, 0x1C, 0x00, 0x01];

pub const PT_FILES_WITH_TAG_SCRIPT: &[i32] = &[2, 136, 201, 236];

pub const DP_FILES_WITH_TAG_SCRIPT: &[i32] = &[2, 131, 230];

pub const DP_STARTER_STRING_INDEX: i32 = 19;
pub const PT_STARTER_STRING_INDEX: i32 = 36;

pub const FOSSIL_COUNT: i32 = 7;

pub const DPPT_TM_DATA_PREFIX: &str = "D100D200D300D400";
pub const HGSS_TM_DATA_PREFIX: &str = "1E003200";

pub const TM_COUNT: i32 = 92;
pub const HM_COUNT: i32 = 8;

pub const TM_ITEM_OFFSET: i32 = items::TM01;

pub const DPPT_TEXT_CHARS_PER_LINE: i32 = 38;
pub const HGSS_TEXT_CHARS_PER_LINE: i32 = 36;

pub const DP_ITEM_PALETTES_PREFIX: &str = "8D018E01210132018D018F0122013301";
pub const PTHGSS_ITEM_PALETTES_PREFIX: &str = "8D018E01210133018D018F0122013401";

pub const PT_SPEAR_PILLAR_PORTAL_SCRIPT_FILE: i32 = 237;

pub const EVOLUTION_METHOD_COUNT: i32 = 26;

pub const HIGHEST_ABILITY_INDEX: i32 = abilities::BAD_DREAMS;

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
];

pub const DPPT_SET_VAR_SCRIPT: i32 = 0x28;
pub const HGSS_SET_VAR_SCRIPT: i32 = 0x29;

pub const SCRIPT_LIST_TERMINATOR: i32 = 0xFD13;

pub const ITEM_SCRIPT_VARIABLE: i32 = 0x8008;

pub const DP_SHOP_NAMES: &[&str] = &[
    "Sunyshore Secondary",
    "Jubilife Secondary",
    "Floaroma Secondary",
    "Oreburgh Secondary",
    "Eterna Secondary",
    "Eterna Herbs",
    "Snowpoint Secondary",
    "Solaceon Secondary",
    "Pastoria Secondary",
    "Celestic Secondary",
    "Hearthome Secondary",
    "Canalave Secondary",
    "Veilstone Department Store Secret Base Decorations 1",
    "Veilstone Department Store Secret Base Decorations 2",
    "Veilstone Department Store Vitamins",
    "Veilstone Department Store TMs 1",
    "Sunyshore Market Seals 1",
    "Sunyshore Market Seals 2",
    "Sunyshore Market Seals 3",
    "Sunyshore Market Seals 4",
    "Veilstone Department Store TMs 2",
    "Sunyshore Market Seals 5",
    "Sunyshore Market Seals 6",
    "Sunyshore Market Seals 7",
    "Pokemon League Secondary",
    "Veilstone Department Store X Items",
    "Veilstone Department Store Healing",
    "Veilstone Department Store Balls Etc.",
    "Progressive Shops",
];

pub const PT_SHOP_NAMES: &[&str] = &[
    "Jubilife Secondary",
    "Sunyshore Secondary",
    "Floaroma Secondary",
    "Oreburgh Secondary",
    "Eterna Herbs",
    "Canalave Secondary",
    "Pastoria Secondary",
    "Celestic Secondary",
    "Snowpoint Secondary",
    "Solaceon Secondary",
    "Eterna Secondary",
    "Hearthome Secondary",
    "Veilstone Department Store B1 Berries",
    "Veilstone Department Store Secret Base Decorations 1",
    "Veilstone Department Store Vitamins",
    "Veilstone Department Store Secret Base Decorations 2",
    "Veilstone Department Store TMs 1",
    "Sunyshore Market Seals 1",
    "Sunyshore Market Seals 2",
    "Sunyshore Market Seals 3",
    "Sunyshore Market Seals 4",
    "Veilstone Department Store TMs 2",
    "Sunyshore Market Seals 5",
    "Sunyshore Market Seals 6",
    "Sunyshore Market Seals 7",
    "Pokemon League Secondary",
    "Veilstone Department Store X Items",
    "Veilstone Department Store Healing",
    "Veilstone Department Store Balls Etc.",
    "Progressive Shops",
];

pub const HGSS_SHOP_NAMES: &[&str] = &[
    "Cherrygrove Secondary",
    "Cerulean Secondary",
    "Ecruteak Secondary",
    "Celadon Department Store Mail",
    "Saffron Secondary",
    "Violet Secondary",
    "Blackthorn Secondary",
    "Olivine Secondary",
    "Fuchsia Secondary",
    "Lavender Secondary",
    "Pewter Secondary",
    "Viridian Secondary",
    "Azalea Secondary",
    "Mahogany Before Hideout",
    "Safari Zone Gate Southwest",
    "Goldenrod Herb Shop",
    "Cianwood Pharmacy",
    "Veilstone Department Store Secret Base Decorations 1",
    "Veilstone Department Store Secret Base Decorations 2",
    "Goldenrod Department Store Vitamins",
    "Celadon Department Store Vitamins",
    "Mt. Moon Square",
    "Sunyshore Market Seals 1",
    "Sunyshore Market Seals 2",
    "Sunyshore Market Seals 3",
    "Sunyshore Market Seals 4",
    "Sunyshore Market Seals 5",
    "Sunyshore Market Seals 6",
    "Unused Secondary",
    "Sunyshore Market Seals 7",
    "Pokeathlon Dome Data Card Shop 25-27",
    "Goldenrod Department Store X Items",
    "Celadon Department Store X Items",
    "Mahogany After Hideout",
    "Goldenrod Department Store Healing",
    "Celadon Department Store Healing",
    "Goldenrod Department Store Balls Etc.",
    "Goldenrod TMs",
    "Celadon Department Store Balls Etc.",
    "Celadon TMs",
    "Pokeathlon Dome Athlete Shop Sunday (Pre-National Dex)",
    "Pokeathlon Dome Data Card Shop 19-24",
    "Pokeathlon Dome Data Card Shop 1-6",
    "Pokeathlon Dome Athlete Shop Monday (Pre-National Dex)",
    "Pokeathlon Dome Athlete Shop Tuesday (Pre-National Dex)",
    "Pokeathlon Dome Data Card Shop 7-12",
    "Pokeathlon Dome Athlete Shop Wednesday (Pre-National Dex)",
    "Pokeathlon Dome Athlete Shop Thursday (Pre-National Dex)",
    "Pokeathlon Dome Athlete Shop Friday (Pre-National Dex)",
    "Pokeathlon Dome Athlete Shop Saturday (Pre-National Dex)",
    "Pokeathlon Dome Data Card Shop 13-18",
    "Pokeathlon Dome Athlete Shop Sunday (Post-National Dex)",
    "Pokeathlon Dome Athlete Shop Monday (Post-National Dex)",
    "Pokeathlon Dome Athlete Shop Tuesday (Post-National Dex)",
    "Pokeathlon Dome Athlete Shop Wednesday (Post-National Dex)",
    "Pokeathlon Dome Athlete Shop Thursday (Post-National Dex)",
    "Pokeathlon Dome Athlete Shop Friday (Post-National Dex)",
    "Pokeathlon Dome Athlete Shop Saturday (Post-National Dex)",
    "Progressive Shops",
];

pub fn shop_names(rom_type: i32) -> &'static [&'static str] {
    match rom_type {
        TYPE_DP => DP_SHOP_NAMES,
        TYPE_PLAT => PT_SHOP_NAMES,
        TYPE_HGSS => HGSS_SHOP_NAMES,
        _ => &[],
    }
}

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
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormeMapping {
    pub internal_index: i32,
    pub base_forme: i32,
    pub forme_number: i32,
    pub suffix: &'static str,
}

const fn forme(
    gen4_forme: i32,
    base_forme: i32,
    forme_number: i32,
    suffix: &'static str,
) -> FormeMapping {
    FormeMapping {
        internal_index: gen4_forme + FORME_OFFSET,
        base_forme,
        forme_number,
        suffix,
    }
}

pub const FORME_MAPPINGS: &[FormeMapping] = &[
    forme(gen4_formes::DEOXYS_A, species::DEOXYS, 1, "-A"),
    forme(gen4_formes::DEOXYS_D, species::DEOXYS, 2, "-D"),
    forme(gen4_formes::DEOXYS_S, species::DEOXYS, 3, "-S"),
    forme(gen4_formes::WORMADAM_S, species::WORMADAM, 1, "-S"),
    forme(gen4_formes::WORMADAM_T, species::WORMADAM, 2, "-T"),
    forme(gen4_formes::GIRATINA_O, species::GIRATINA, 1, "-O"),
    forme(gen4_formes::SHAYMIN_S, species::SHAYMIN, 1, "-S"),
    forme(gen4_formes::ROTOM_H, species::ROTOM, 1, "-H"),
    forme(gen4_formes::ROTOM_W, species::ROTOM, 2, "-W"),
    forme(gen4_formes::ROTOM_FR, species::ROTOM, 3, "-Fr"),
    forme(gen4_formes::ROTOM_FA, species::ROTOM, 4, "-Fa"),
    forme(gen4_formes::ROTOM_M, species::ROTOM, 5, "-M"),
];

pub fn forme_count(rom_type: i32) -> i32 {
    match rom_type {
        TYPE_DP => DP_FORME_COUNT,
        TYPE_PLAT | TYPE_HGSS => PLAT_HGSS_FORME_COUNT,
        _ => 0,
    }
}

pub fn cosmetic_forms(species_number: i32) -> i32 {
    match species_number {
        species::UNOWN => 28,
        species::BURMY => 3,
        species::SHELLOS | species::GASTRODON => 2,
        _ => 0,
    }
}

const FORME_SUFFIXES_BY_BASE_FORME: &[(i32, &[(i32, &str)])] = &[
    (species::DEOXYS, &[(1, "-A"), (2, "-D"), (3, "-S")]),
    (species::WORMADAM, &[(1, "-S"), (2, "-T")]),
    (species::SHAYMIN, &[(1, "-S")]),
    (species::GIRATINA, &[(1, "-O")]),
    (
        species::ROTOM,
        &[(1, "-H"), (2, "-W"), (3, "-Fr"), (4, "-Fa"), (5, "-M")],
    ),
];

const DUMMY_FORME_SUFFIXES: &[(i32, &str)] = &[(0, "")];

const ABSOLUTE_POKE_NUMS_BY_BASE_FORME: &[(i32, &[(i32, i32)])] = &[
    (
        species::DEOXYS,
        &[
            (1, gen4_formes::DEOXYS_A),
            (2, gen4_formes::DEOXYS_D),
            (3, gen4_formes::DEOXYS_S),
        ],
    ),
    (
        species::WORMADAM,
        &[(1, gen4_formes::WORMADAM_S), (2, gen4_formes::WORMADAM_T)],
    ),
    (species::GIRATINA, &[(1, gen4_formes::GIRATINA_O)]),
    (species::SHAYMIN, &[(1, gen4_formes::SHAYMIN_S)]),
    (
        species::ROTOM,
        &[
            (1, gen4_formes::ROTOM_H),
            (2, gen4_formes::ROTOM_W),
            (3, gen4_formes::ROTOM_FR),
            (4, gen4_formes::ROTOM_FA),
            (5, gen4_formes::ROTOM_M),
        ],
    ),
];

const DUMMY_ABSOLUTE_POKE_NUMS: &[(i32, i32)] = &[(255, 0)];

fn lookup<K: PartialEq + Copy, V: Copy>(table: &[(K, V)], key: K) -> Option<V> {
    table
        .iter()
        .find(|(candidate, _)| *candidate == key)
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

pub const LYRA_ETHAN_MARILL_SPRITE_PREFIX: &str = "274E0604C301274E0704E101274E0804";

const OVERWORLD_SPRITE_SINGLE_GAPS: &[i32] = &[
    300, 323, 453, 472, 530, 537, 544, 753, 761, 763, 784, 786, 788, 793, 795, 838,
];

const OVERWORLD_SPRITE_FORME_RUNS: &[(i32, i32)] =
    &[(501, 27), (716, 3), (745, 2), (748, 2), (824, 5)];

const ARCEUS_FIRST_OVERWORLD_SPRITE: i32 = 845;
const FIRST_OVERWORLD_POKEMON_SPRITE: i32 = 296;

fn forme_run_adjustment(sprite: i32, run_start: i32, run_length: i32) -> i32 {
    if sprite > run_start + run_length {
        run_length
    } else if sprite > run_start {
        sprite - run_start
    } else {
        0
    }
}

pub fn convert_overworld_sprite_to_species(overworld_sprite_id: i32) -> i32 {
    let single_gaps = OVERWORLD_SPRITE_SINGLE_GAPS
        .iter()
        .filter(|&&gap| overworld_sprite_id >= gap)
        .count() as i32;
    let forme_gaps: i32 = OVERWORLD_SPRITE_FORME_RUNS
        .iter()
        .map(|&(start, length)| forme_run_adjustment(overworld_sprite_id, start, length))
        .sum();
    let arceus_gap = (overworld_sprite_id - ARCEUS_FIRST_OVERWORLD_SPRITE).max(0);
    overworld_sprite_id - FIRST_OVERWORLD_POKEMON_SPRITE - single_gaps - forme_gaps - arceus_gap
}

pub const DPPT_ALTERNATE_SLOTS: [usize; 20] =
    [0, 1, 2, 3, 2, 3, 4, 5, 10, 11, 8, 9, 8, 9, 8, 9, 8, 9, 8, 9];

pub const DPPT_WATER_SLOT_SET_NAMES: [&str; 5] =
    ["Surfing", "Filler", "Old Rod", "Good Rod", "Super Rod"];

pub const HGSS_TIME_OF_DAY_NAMES: [&str; 3] = ["Morning", "Day", "Night"];

pub const HGSS_NON_GRASS_SET_NAMES: [&str; 6] = [
    "",
    "Surfing",
    "Rock Smash",
    "Old Rod",
    "Good Rod",
    "Super Rod",
];

pub const HGSS_GOOD_ROD_REPLACEMENT_INDEX: usize = 3;
pub const HGSS_SUPER_ROD_REPLACEMENT_INDEX: usize = 1;

pub const MOVE_CATEGORY_INDICES: [MoveCategory; 3] = [
    MoveCategory::Physical,
    MoveCategory::Special,
    MoveCategory::Status,
];

pub fn move_category_from_index(index: i32) -> Option<MoveCategory> {
    usize::try_from(index)
        .ok()
        .and_then(|index| MOVE_CATEGORY_INDICES.get(index).copied())
}

pub fn move_category_to_byte(category: MoveCategory) -> u8 {
    match category {
        MoveCategory::Physical => 0,
        MoveCategory::Special => 1,
        MoveCategory::Status => 2,
    }
}

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
pub const STOMP_EFFECT: i32 = 150;
pub const SOLARBEAM_EFFECT: i32 = 151;
pub const THUNDER_EFFECT: i32 = 152;
pub const FLY_EFFECT: i32 = 155;
pub const DEFENSE_CURL_EFFECT: i32 = 156;
pub const FAKE_OUT_EFFECT: i32 = 158;
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
pub const DAMAGE_POISON_WITH_INCREASED_CRIT_EFFECT: i32 = 209;
pub const NO_DAMAGE_SP_ATK_AND_SP_DEF_PLUS_ONE_EFFECT: i32 = 211;
pub const NO_DAMAGE_ATK_AND_SPE_PLUS_ONE_EFFECT: i32 = 212;
pub const DAMAGE_USER_SPE_MINUS_ONE_EFFECT: i32 = 218;
pub const DAMAGE_USER_DEF_AND_SP_DEF_MINUS_ONE_EFFECT: i32 = 229;
pub const FLARE_BLITZ_EFFECT: i32 = 253;
pub const DIVE_EFFECT: i32 = 255;
pub const DIG_EFFECT: i32 = 256;
pub const BLIZZARD_EFFECT: i32 = 260;
pub const VOLT_TACKLE_EFFECT: i32 = 262;
pub const BOUNCE_EFFECT: i32 = 263;
pub const NO_DAMAGE_SP_ATK_MINUS_TWO_EFFECT: i32 = 265;
pub const CHATTER_EFFECT: i32 = 267;
pub const DAMAGE_RECOIL_50_PERCENT_EFFECT: i32 = 269;
pub const DAMAGE_SP_DEF_MINUS_TWO_EFFECT: i32 = 271;
pub const SHADOW_FORCE_EFFECT: i32 = 272;
pub const FIRE_FANG_EFFECT: i32 = 273;
pub const ICE_FANG_EFFECT: i32 = 274;
pub const THUNDER_FANG_EFFECT: i32 = 275;
pub const DAMAGE_USER_SP_ATK_PLUS_ONE_EFFECT: i32 = 276;

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
    moves::BUG_BUZZ,
    moves::CHATTER,
    moves::PERISH_SONG,
    moves::HEAL_BELL,
];

pub const PUNCH_MOVES: &[i32] = &[
    moves::ICE_PUNCH,
    moves::FIRE_PUNCH,
    moves::THUNDER_PUNCH,
    moves::MACH_PUNCH,
    moves::FOCUS_PUNCH,
    moves::DIZZY_PUNCH,
    moves::DYNAMIC_PUNCH,
    moves::HAMMER_ARM,
    moves::MEGA_PUNCH,
    moves::COMET_PUNCH,
    moves::METEOR_MASH,
    moves::SHADOW_PUNCH,
    moves::DRAIN_PUNCH,
    moves::BULLET_PUNCH,
    moves::SKY_UPPERCUT,
];

pub const DP_REQUIRED_FIELD_TMS: &[i32] = &[
    2, 3, 5, 9, 12, 19, 23, 28, 34, 39, 41, 43, 46, 47, 49, 50, 62, 69, 79, 80, 82, 84, 85, 87,
];

pub const PT_REQUIRED_FIELD_TMS: &[i32] = &[
    2, 3, 5, 7, 9, 11, 12, 18, 19, 23, 28, 34, 37, 39, 41, 43, 46, 47, 49, 50, 62, 69, 79, 80, 82,
    84, 85, 87,
];

pub const DPPT_FIELD_MOVES: &[i32] = &[
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
    moves::DEFOG,
    moves::ROCK_CLIMB,
];

pub const HGSS_FIELD_MOVES: &[i32] = &[
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
    moves::ROCK_CLIMB,
];

pub const DPPT_EARLY_REQUIRED_HM_MOVES: &[i32] = &[moves::ROCK_SMASH, moves::CUT];

pub const HGSS_EARLY_REQUIRED_HM_MOVES: &[i32] = &[moves::CUT];

pub const SHEDINJA_SPECIES_LOCATOR: &str = "492080000090281C0521";

pub const ILEX_FOREST_SCRIPT_FILE: i32 = 92;
pub const ILEX_FOREST_STRINGS_FILE: i32 = 115;
pub const HEADBUTT_TUTOR_SCRIPT_OFFSETS: &[i32] = &[0xF55, 0xFC5, 0x100A, 0x104C];

pub const DOUBLE_BATTLE_FIX_PREFIX_DP: &str = "022912D90221214201";
pub const DOUBLE_BATTLE_FIX_PREFIX_PT: &str = "022919D90221214205";
pub const DOUBLE_BATTLE_FIX_PREFIX_HGSS: &str = "2C2815D00221214201";

pub const FEEBAS_LEVEL_PREFIX_DPPT: &str = "019813B0F0BD";
pub const HONEY_TREE_LEVEL_PREFIX_DPPT: &str = "F0BDF0B589B0051C0C1C";

pub const RUNNING_SHOES_CHECK_PREFIX_DPPT: &str = "281C0C24";
pub const RUNNING_SHOES_CHECK_PREFIX_HGSS: &str = "301C0C24";

pub const DISTORTION_WORLD_GROUND_CHECK_PREFIX: &str = "23D849187944C988090409148F44";

pub const DPPT_INTRO_PREFIXES: &[&str] = &["381CF8BDC046", "08B0F8BD"];

pub const HP_BAR_SPEED_PREFIX: &str = "0CD106200090";
pub const EXP_BAR_SPEED_PREFIX: &str = "011C00D101212E6C";
pub const BOTH_BARS_SPEED_PREFIX: &str = "70BD90421DDA";

pub const DPPT_EGG_MOVE_TABLE_PREFIX: &str = "40016601";

pub const TYPE_EFFECTIVENESS_TABLE_LOCATOR: &str = "000505000805";

pub const TROPHY_GARDEN_GRASS_ENCOUNTER_INDEX_DP: usize = 304;
pub const TROPHY_GARDEN_GRASS_ENCOUNTER_INDEX_PT: usize = 308;
pub const MARSH_GRASS_ENCOUNTER_INDICES_DP: &[usize] = &[76, 82, 88, 94, 100, 102];
pub const MARSH_GRASS_ENCOUNTER_INDICES_PT: &[usize] = &[76, 82, 88, 94, 100, 106];

pub const PICKUP_TABLE_LOCATOR: &str = "110012001A000300";
pub const RARE_PICKUP_TABLE_LOCATOR: &str = "19005C00DD00";
pub const NUMBER_OF_COMMON_PICKUP_ITEMS: i32 = 18;
pub const NUMBER_OF_RARE_PICKUP_ITEMS: i32 = 11;

pub const FRIENDSHIP_VALUE_FOR_EVO_LOCATOR: &str = "DC286AD3";

pub const PERFECT_ODDS_BRANCH_LOCATOR: &str = "FF2901D30425";

pub const POKEDEX_AREA_DATA_SIZE: i32 = 495;
pub const DPPT_MT_CORONET_DEX_INDEX: i32 = 3;
pub const DPPT_GREAT_MARSH_DEX_INDEX: i32 = 4;
pub const DPPT_TROPHY_GARDEN_DEX_INDEX: i32 = 14;
pub const DPPT_FLOAROMA_MEADOW_DEX_INDEX: i32 = 21;
pub const DPPT_OVERWORLD_HONEY_TREE_DEX_INDICES: &[i32] = &[
    6, 7, 17, 18, 19, 20, 21, 22, 23, 24, 26, 27, 28, 29, 30, 31, 34, 36, 37, 50,
];
pub const PARTNER_TRAINER_INDICES: &[i32] = &[608, 609, 610, 611, 612];

pub fn allowed_items() -> ItemList {
    let mut allowed = ItemList::new(items::ENIGMA_STONE as usize);
    allowed.ban_range(items::EXPLORER_KIT, 109);
    allowed.ban_range(items::GRISEOUS_ORB, 23);
    allowed.ban_range(items::HM01, 8);
    allowed.tm_range(items::TM01, 92);
    allowed
}

pub fn non_bad_items() -> ItemList {
    let mut non_bad = allowed_items();
    non_bad.ban_singles(&[
        items::ODD_KEYSTONE,
        items::GRISEOUS_ORB,
        items::SOUL_DEW,
        items::LIGHT_BALL,
        items::ORAN_BERRY,
        items::QUICK_POWDER,
    ]);
    non_bad.ban_range(items::SHOAL_SALT, 2);
    non_bad.ban_range(items::GROWTH_MULCH, 4);
    non_bad.ban_range(items::ADAMANT_ORB, 2);
    non_bad.ban_range(items::MAIL1, 12);
    non_bad.ban_range(items::FIGY_BERRY, 25);
    non_bad.ban_range(items::LUCKY_PUNCH, 4);
    non_bad.ban_range(items::RED_SCARF, 5);
    non_bad
}

pub fn regular_shop_items() -> Vec<i32> {
    (items::ULTRA_BALL..=items::POKE_BALL)
        .chain(items::POTION..=items::REVIVE)
        .chain(items::SUPER_REPEL..=items::REPEL)
        .collect()
}

pub fn op_shop_items() -> Vec<i32> {
    [items::RARE_CANDY]
        .into_iter()
        .chain(items::TINY_MUSHROOM..=items::NUGGET)
        .chain([items::RARE_BONE, items::LUCKY_EGG])
        .collect()
}

pub fn double_battle_fix_prefix(rom_type: i32) -> &'static str {
    match rom_type {
        TYPE_DP => DOUBLE_BATTLE_FIX_PREFIX_DP,
        TYPE_PLAT => DOUBLE_BATTLE_FIX_PREFIX_PT,
        _ => DOUBLE_BATTLE_FIX_PREFIX_HGSS,
    }
}

pub fn run_without_running_shoes_prefix(rom_type: i32) -> &'static str {
    match rom_type {
        TYPE_DP | TYPE_PLAT => RUNNING_SHOES_CHECK_PREFIX_DPPT,
        _ => RUNNING_SHOES_CHECK_PREFIX_HGSS,
    }
}

pub fn trophy_garden_grass_encounter_index(rom_type: i32) -> usize {
    if rom_type == TYPE_DP {
        TROPHY_GARDEN_GRASS_ENCOUNTER_INDEX_DP
    } else {
        TROPHY_GARDEN_GRASS_ENCOUNTER_INDEX_PT
    }
}

pub fn marsh_grass_encounter_indices(rom_type: i32) -> &'static [usize] {
    if rom_type == TYPE_DP {
        MARSH_GRASS_ENCOUNTER_INDICES_DP
    } else {
        MARSH_GRASS_ENCOUNTER_INDICES_PT
    }
}

pub fn text_chars_per_line(rom_type: i32) -> i32 {
    if rom_type == TYPE_HGSS {
        HGSS_TEXT_CHARS_PER_LINE
    } else {
        DPPT_TEXT_CHARS_PER_LINE
    }
}

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
        0x0A => Some(Type::Fire),
        0x0B => Some(Type::Water),
        0x0C => Some(Type::Grass),
        0x0D => Some(Type::Electric),
        0x0E => Some(Type::Psychic),
        0x0F => Some(Type::Ice),
        0x10 => Some(Type::Dragon),
        0x11 => Some(Type::Dark),
        _ => None,
    }
}

pub const MYSTERY_TYPE_BYTE: u8 = 0x09;

pub fn type_to_byte(kind: Option<Type>) -> u8 {
    match kind {
        None => MYSTERY_TYPE_BYTE,
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
        Some(_) => 0,
    }
}

fn trainer_at(trainers: &mut [Trainer], number: i32) -> Option<&mut Trainer> {
    usize::try_from(number)
        .ok()
        .and_then(|index| trainers.get_mut(index))
}

fn tag_index(trainers: &mut [Trainer], index: i32, tag: String) {
    if let Some(trainer) = trainer_at(trainers, index) {
        trainer.tag = Some(tag);
    }
}

fn tag(trainers: &mut [Trainer], tag_name: &str, numbers: &[i32]) {
    for &number in numbers {
        tag_index(trainers, number - 1, tag_name.to_owned());
    }
}

fn tag_one(trainers: &mut [Trainer], number: i32, tag_name: &str) {
    tag_index(trainers, number - 1, tag_name.to_owned());
}

fn tag_rival_consecutive(trainers: &mut [Trainer], tag_name: &str, offset_fire: i32) {
    tag_index(trainers, offset_fire - 1, format!("{tag_name}-0"));
    tag_index(trainers, offset_fire, format!("{tag_name}-1"));
    tag_index(trainers, offset_fire - 2, format!("{tag_name}-2"));
}

fn tag_friend_consecutive(trainers: &mut [Trainer], tag_name: &str, offset_grass: i32) {
    tag_index(trainers, offset_grass - 1, format!("{tag_name}-1"));
    tag_index(trainers, offset_grass, format!("{tag_name}-2"));
    tag_index(trainers, offset_grass + 1, format!("{tag_name}-0"));
}

fn tag_friend_consecutive2(trainers: &mut [Trainer], tag_name: &str, offset_water: i32) {
    tag_index(trainers, offset_water - 1, format!("{tag_name}-0"));
    tag_index(trainers, offset_water, format!("{tag_name}-1"));
    tag_index(trainers, offset_water + 1, format!("{tag_name}-2"));
}

fn tag_dppt_elite_four(trainers: &mut [Trainer]) {
    tag_one(trainers, 0x105, "ELITE1");
    tag_one(trainers, 0x106, "ELITE2");
    tag_one(trainers, 0x107, "ELITE3");
    tag_one(trainers, 0x108, "ELITE4");
    tag_one(trainers, 0x10b, "CHAMPION");
}

fn tag_dppt_friends(trainers: &mut [Trainer]) {
    tag_friend_consecutive(trainers, "FRIEND1", 0x265);
    tag_friend_consecutive(trainers, "FRIEND1", 0x268);
    tag_friend_consecutive2(trainers, "FRIEND2", 0x26D);
    tag_friend_consecutive2(trainers, "FRIEND2", 0x270);
}

pub fn tag_trainers_dp(trainers: &mut [Trainer]) {
    tag(trainers, "GYM1", &[0xf4, 0xf5]);
    tag(trainers, "GYM2", &[0x144, 0x103, 0x104, 0x15C]);
    tag(trainers, "GYM3", &[0x135, 0x136, 0x137, 0x138]);
    tag(
        trainers,
        "GYM4",
        &[0x1f1, 0x1f2, 0x191, 0x153, 0x125, 0x1E3],
    );
    tag(
        trainers,
        "GYM5",
        &[0x165, 0x145, 0x10a, 0x14a, 0x154, 0x157, 0x118, 0x11c],
    );
    tag(
        trainers,
        "GYM6",
        &[0x13a, 0x100, 0x101, 0x117, 0x16f, 0xe8, 0x11b],
    );
    tag(
        trainers,
        "GYM7",
        &[0x10c, 0x10d, 0x10e, 0x10f, 0x33b, 0x33c],
    );
    tag(
        trainers,
        "GYM8",
        &[0x158, 0x155, 0x12d, 0x12e, 0x12f, 0x11d, 0x119],
    );
    tag_one(trainers, 0xf6, "GYM1-LEADER");
    tag_one(trainers, 0x13b, "GYM2-LEADER");
    tag_one(trainers, 0x13d, "GYM3-LEADER");
    tag_one(trainers, 0x13c, "GYM4-LEADER");
    tag_one(trainers, 0x13e, "GYM5-LEADER");
    tag_one(trainers, 0xfa, "GYM6-LEADER");
    tag_one(trainers, 0x13f, "GYM7-LEADER");
    tag_one(trainers, 0x140, "GYM8-LEADER");
    tag_dppt_elite_four(trainers);
    tag_rival_consecutive(trainers, "RIVAL1", 0xf8);
    tag_rival_consecutive(trainers, "RIVAL2", 0x1d7);
    tag_rival_consecutive(trainers, "RIVAL3", 0x1da);
    tag_rival_consecutive(trainers, "RIVAL4", 0x1dd);
    tag_one(trainers, 0x26b, "RIVAL5-0");
    tag_one(trainers, 0x26c, "RIVAL5-1");
    tag_one(trainers, 0x25f, "RIVAL5-2");
    tag_rival_consecutive(trainers, "RIVAL6", 0x1e0);
    tag_rival_consecutive(trainers, "RIVAL7", 0x346);
    tag_rival_consecutive(trainers, "RIVAL8", 0x349);
    tag(trainers, "THEMED:CYRUS-LEADER", &[0x193, 0x194]);
    tag(trainers, "THEMED:MARS-STRONG", &[0x127, 0x195, 0x210]);
    tag(trainers, "THEMED:JUPITER-STRONG", &[0x196, 0x197]);
    tag(trainers, "THEMED:SATURN-STRONG", &[0x198, 0x199]);
    tag_dppt_friends(trainers);
}

pub fn tag_trainers_pt(trainers: &mut [Trainer]) {
    tag(trainers, "GYM1", &[0xf4, 0xf5]);
    tag(trainers, "GYM2", &[0x144, 0x103, 0x104, 0x15C]);
    tag(
        trainers,
        "GYM3",
        &[0x165, 0x145, 0x154, 0x157, 0x118, 0x11c],
    );
    tag(trainers, "GYM4", &[0x135, 0x136, 0x137, 0x138]);
    tag(
        trainers,
        "GYM5",
        &[0x1f1, 0x1f2, 0x191, 0x153, 0x125, 0x1E3],
    );
    tag(
        trainers,
        "GYM6",
        &[0x13a, 0x100, 0x101, 0x117, 0x16f, 0xe8, 0x11b],
    );
    tag(
        trainers,
        "GYM7",
        &[0x10c, 0x10d, 0x10e, 0x10f, 0x33b, 0x33c],
    );
    tag(
        trainers,
        "GYM8",
        &[0x158, 0x155, 0x12d, 0x12e, 0x12f, 0x11d, 0x119, 0x14b],
    );
    tag_one(trainers, 0xf6, "GYM1-LEADER");
    tag_one(trainers, 0x13b, "GYM2-LEADER");
    tag_one(trainers, 0x13e, "GYM3-LEADER");
    tag_one(trainers, 0x13d, "GYM4-LEADER");
    tag_one(trainers, 0x13c, "GYM5-LEADER");
    tag_one(trainers, 0xfa, "GYM6-LEADER");
    tag_one(trainers, 0x13f, "GYM7-LEADER");
    tag_one(trainers, 0x140, "GYM8-LEADER");
    tag_dppt_elite_four(trainers);
    tag_rival_consecutive(trainers, "RIVAL1", 0x353);
    tag_rival_consecutive(trainers, "RIVAL2", 0xf8);
    tag_rival_consecutive(trainers, "RIVAL3", 0x1d7);
    tag_rival_consecutive(trainers, "RIVAL4", 0x1da);
    tag_rival_consecutive(trainers, "RIVAL5", 0x1dd);
    tag_one(trainers, 0x26b, "RIVAL6-0");
    tag_one(trainers, 0x26c, "RIVAL6-1");
    tag_one(trainers, 0x25f, "RIVAL6-2");
    tag_rival_consecutive(trainers, "RIVAL7", 0x1e0);
    tag_rival_consecutive(trainers, "RIVAL8", 0x346);
    tag_rival_consecutive(trainers, "RIVAL9", 0x349);
    tag_rival_consecutive(trainers, "RIVAL10", 0x368);
    tag_one(trainers, 0x35A, "GYM1");
    tag_one(trainers, 0x359, "GYM2");
    tag_one(trainers, 0x35C, "GYM3");
    tag_one(trainers, 0x356, "GYM4");
    tag_one(trainers, 0x35B, "GYM5");
    tag_one(trainers, 0x358, "GYM6");
    tag_one(trainers, 0x355, "GYM7");
    tag_one(trainers, 0x357, "GYM8");
    tag_one(trainers, 0x399, "GYM8");
    tag_one(trainers, 0x39A, "ELITE3");
    tag_one(trainers, 0x362, "ELITE1");
    tag_one(trainers, 0x363, "ELITE2");
    tag_one(trainers, 0x364, "ELITE3");
    tag_one(trainers, 0x365, "ELITE4");
    tag_one(trainers, 0x366, "CHAMPION");
    tag(trainers, "THEMED:CYRUS-LEADER", &[0x391, 0x193, 0x194]);
    tag(
        trainers,
        "THEMED:MARS-STRONG",
        &[0x127, 0x195, 0x210, 0x39e],
    );
    tag(trainers, "THEMED:JUPITER-STRONG", &[0x196, 0x197, 0x39f]);
    tag(trainers, "THEMED:SATURN-STRONG", &[0x198, 0x199]);
    tag_dppt_friends(trainers);
}

fn tag_hgss_gyms(trainers: &mut [Trainer]) {
    tag(trainers, "GYM1", &[0x32, 0x1D]);
    tag(trainers, "GYM2", &[0x43, 0x44, 0x45, 0x0a]);
    tag(trainers, "GYM3", &[0x05, 0x46, 0x47, 0x16]);
    tag(trainers, "GYM4", &[0x1ed, 0x1ee, 0x59, 0x2e]);
    tag(trainers, "GYM5", &[0x9c, 0x9d, 0x9f, 0xfb]);
    tag(trainers, "GYM7", &[0x1e0, 0x1e1, 0x1e2, 0x1e3, 0x1e4]);
    tag(trainers, "GYM8", &[0x6e, 0x6f, 0x70, 0x75, 0x77]);
    tag(trainers, "GYM9", &[0x134, 0x2ad]);
    tag(trainers, "GYM10", &[0x2a4, 0x2a5, 0x2a6, 0x129, 0x12a]);
    tag(trainers, "GYM11", &[0x18c, 0xe8, 0x151]);
    tag(trainers, "GYM12", &[0x150, 0x146, 0x164, 0x15a]);
    tag(trainers, "GYM13", &[0x53, 0x54, 0xb7, 0x88]);
    tag(trainers, "GYM14", &[0x170, 0x171, 0xe6, 0x19f]);
    tag(
        trainers,
        "GYM15",
        &[0x2b1, 0x2b2, 0x2b3, 0x2b4, 0x2b5, 0x2b6],
    );
    tag(trainers, "GYM16", &[0x2a9, 0x2aa, 0x2ab, 0x2ac]);
}

const HGSS_LEADERS: &[(i32, &str)] = &[
    (0x14, "GYM1-LEADER"),
    (0x15, "GYM2-LEADER"),
    (0x1e, "GYM3-LEADER"),
    (0x1f, "GYM4-LEADER"),
    (0x22, "GYM5-LEADER"),
    (0x21, "GYM6-LEADER"),
    (0x20, "GYM7-LEADER"),
    (0x23, "GYM8-LEADER"),
    (0xFD, "GYM9-LEADER"),
    (0xFE, "GYM10-LEADER"),
    (0xFF, "GYM11-LEADER"),
    (0x100, "GYM12-LEADER"),
    (0x101, "GYM13-LEADER"),
    (0x102, "GYM14-LEADER"),
    (0x103, "GYM15-LEADER"),
    (0x105, "GYM16-LEADER"),
    (0xf5, "ELITE1"),
    (0xf7, "ELITE2"),
    (0x1a2, "ELITE3"),
    (0xf6, "ELITE4"),
    (0xf4, "CHAMPION"),
    (0x104, "UBER"),
    (0x2c8, "GYM1-LEADER"),
    (0x2c9, "GYM2-LEADER"),
    (0x2ca, "GYM3-LEADER"),
    (0x2cb, "GYM4-LEADER"),
    (0x2ce, "GYM5-LEADER"),
    (0x2cd, "GYM6-LEADER"),
    (0x2cc, "GYM7-LEADER"),
    (0x2cf, "GYM8-LEADER"),
    (0x2d0, "GYM9-LEADER"),
    (0x2d1, "GYM10-LEADER"),
    (0x2d2, "GYM11-LEADER"),
    (0x2d3, "GYM12-LEADER"),
    (0x2d4, "GYM13-LEADER"),
    (0x2d5, "GYM14-LEADER"),
    (0x2d6, "GYM15-LEADER"),
    (0x2d7, "GYM16-LEADER"),
    (0x2be, "ELITE1"),
    (0x2bf, "ELITE2"),
    (0x2c0, "ELITE3"),
    (0x2c1, "ELITE4"),
    (0x2bd, "CHAMPION"),
];

const HGSS_RIVAL_SINGLES: &[(i32, &str)] = &[
    (0x10a, "RIVAL2-0"),
    (0x10d, "RIVAL2-1"),
    (0x1, "RIVAL2-2"),
    (0x10B, "RIVAL3-0"),
    (0x10E, "RIVAL3-1"),
    (0x107, "RIVAL3-2"),
    (0x121, "RIVAL4-0"),
    (0x10f, "RIVAL4-1"),
    (0x120, "RIVAL4-2"),
    (0x10C, "RIVAL5-0"),
    (0x110, "RIVAL5-1"),
    (0x108, "RIVAL5-2"),
];

const HGSS_STRONG_TRAINERS: &[(i32, &str)] = &[
    (0x2DE, "GYM8"),
    (0x2DD, "CHAMPION"),
    (0xa0, "KIMONO1-STRONG"),
    (0xa1, "KIMONO2-STRONG"),
    (0xa2, "KIMONO3-STRONG"),
    (0xa3, "KIMONO4-STRONG"),
    (0xa4, "KIMONO5-STRONG"),
];

fn tag_each(trainers: &mut [Trainer], tags: &[(i32, &str)]) {
    for &(number, tag_name) in tags {
        tag_one(trainers, number, tag_name);
    }
}

pub fn tag_trainers_hgss(trainers: &mut [Trainer]) {
    tag_hgss_gyms(trainers);
    tag_each(trainers, HGSS_LEADERS);
    tag_rival_consecutive(trainers, "RIVAL1", 0x1F0);
    tag_each(trainers, HGSS_RIVAL_SINGLES);
    tag_rival_consecutive(trainers, "RIVAL6", 0x11e);
    tag_rival_consecutive(trainers, "RIVAL7", 0x2e0);
    tag_rival_consecutive(trainers, "RIVAL8", 0x1EA);
    tag_each(trainers, HGSS_STRONG_TRAINERS);
    tag(trainers, "THEMED:ARIANA-STRONG", &[0x1df, 0x1de]);
    tag(trainers, "THEMED:PETREL-STRONG", &[0x1e8, 0x1e7]);
    tag(trainers, "THEMED:PROTON-STRONG", &[0x1e6, 0x2c2]);
    tag(
        trainers,
        "THEMED:SPROUTTOWER",
        &[0x2b, 0x33, 0x34, 0x35, 0x36, 0x37, 0x122],
    );
    tag_one(trainers, 485, "LEADER");
}

fn set_multi_battle_status(trainers: &mut [Trainer], status: MultiBattleStatus, numbers: &[i32]) {
    for &number in numbers {
        if let Some(trainer) = trainer_at(trainers, number - 1) {
            trainer.multi_battle_status = status;
        }
    }
}

const DPPT_ALWAYS_MULTI_BATTLES: &[i32] = &[
    414, 415, 419, 420, 426, 427, 521, 527, 528, 561, 590, 835, 836, 848, 849,
];

const PT_EXTRA_ALWAYS_MULTI_BATTLES: &[i32] = &[921, 922];

const DPPT_POTENTIAL_MULTI_BATTLES: &[i32] = &[
    34, 35, 82, 83, 84, 85, 95, 96, 104, 106, 160, 186, 191, 201, 202, 203, 204, 205, 206, 278,
    287, 337, 358, 359, 360, 372, 373, 379, 383, 386, 388, 389, 390, 392, 393, 394, 395, 396, 397,
    398, 399, 400, 443, 445, 446, 447, 450, 452, 453, 454, 455, 459, 494, 496, 499, 500, 505, 506,
    554, 555, 556, 560, 562, 566, 569, 570, 571, 572, 575, 579, 580, 581, 582, 584, 585, 586, 589,
    591, 594, 596, 599, 602, 606,
];

pub fn set_multi_battle_status_dp(trainers: &mut [Trainer]) {
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Always,
        DPPT_ALWAYS_MULTI_BATTLES,
    );
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Potential,
        DPPT_POTENTIAL_MULTI_BATTLES,
    );
}

pub fn set_multi_battle_status_pt(trainers: &mut [Trainer]) {
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Always,
        DPPT_ALWAYS_MULTI_BATTLES,
    );
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Always,
        PT_EXTRA_ALWAYS_MULTI_BATTLES,
    );
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Potential,
        DPPT_POTENTIAL_MULTI_BATTLES,
    );
}

pub fn set_multi_battle_status_hgss(trainers: &mut [Trainer]) {
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Always,
        &[120, 354, 355, 417, 479, 499, 679, 680, 733, 734],
    );
    set_multi_battle_status(
        trainers,
        MultiBattleStatus::Potential,
        &[147, 151, 423, 564, 567, 575, 576, 577, 579],
    );
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
        items::DOUSE_DRIVE => 0,
        items::SHOCK_DRIVE => 0,
        items::BURN_DRIVE => 0,
        items::CHILL_DRIVE => 0,
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
        items::SWEET_HEART => 0,
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
        items::TM01 => 300,
        items::TM02 => 300,
        items::TM03 => 300,
        items::TM04 => 150,
        items::TM05 => 100,
        items::TM06 => 300,
        items::TM07 => 200,
        items::TM08 => 150,
        items::TM09 => 200,
        items::TM10 => 200,
        items::TM11 => 200,
        items::TM12 => 150,
        items::TM13 => 300,
        items::TM14 => 550,
        items::TM15 => 750,
        items::TM16 => 200,
        items::TM17 => 200,
        items::TM18 => 200,
        items::TM19 => 300,
        items::TM20 => 200,
        items::TM21 => 100,
        items::TM22 => 300,
        items::TM23 => 300,
        items::TM24 => 300,
        items::TM25 => 550,
        items::TM26 => 300,
        items::TM27 => 100,
        items::TM28 => 200,
        items::TM29 => 300,
        items::TM30 => 300,
        items::TM31 => 300,
        items::TM32 => 100,
        items::TM33 => 200,
        items::TM34 => 300,
        items::TM35 => 300,
        items::TM36 => 300,
        items::TM37 => 200,
        items::TM38 => 550,
        items::TM39 => 200,
        items::TM40 => 300,
        items::TM41 => 150,
        items::TM42 => 300,
        items::TM43 => 200,
        items::TM44 => 300,
        items::TM45 => 300,
        items::TM46 => 200,
        items::TM47 => 300,
        items::TM48 => 300,
        items::TM49 => 150,
        items::TM50 => 550,
        items::TM51 => 200,
        items::TM52 => 550,
        items::TM53 => 300,
        items::TM54 => 200,
        items::TM55 => 300,
        items::TM56 => 200,
        items::TM57 => 300,
        items::TM58 => 200,
        items::TM59 => 300,
        items::TM60 => 300,
        items::TM61 => 200,
        items::TM62 => 300,
        items::TM63 => 200,
        items::TM64 => 750,
        items::TM65 => 300,
        items::TM66 => 300,
        items::TM67 => 100,
        items::TM68 => 750,
        items::TM69 => 150,
        items::TM70 => 100,
        items::TM71 => 300,
        items::TM72 => 300,
        items::TM73 => 200,
        items::TM74 => 300,
        items::TM75 => 150,
        items::TM76 => 200,
        items::TM77 => 150,
        items::TM78 => 150,
        items::TM79 => 300,
        items::TM80 => 300,
        items::TM81 => 300,
        items::TM82 => 100,
        items::TM83 => 200,
        items::TM84 => 300,
        items::TM85 => 300,
        items::TM86 => 300,
        items::TM87 => 150,
        items::TM88 => 300,
        items::TM89 => 300,
        items::TM90 => 200,
        items::TM91 => 300,
        items::TM92 => 550,
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
        items::RAGE_CANDY_BAR => 0,
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
        _ => return None,
    };
    Some(price)
}

pub const DPPT_OVERWORLD_DEX_MAPS: &[i32] = &[
    1, 2, 3, 4, 5, -1, -1, 6, -1, 7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, 8, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 9, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, 16,
    17, 18, 19, 20, 21, 22, -1, -1, -1, -1, -1, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35,
    36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
];

pub const DPPT_DUNGEON_DEX_MAPS: &[i32] = &[
    -1, -1, -1, -1, -1, 1, 1, -1, 2, -1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 6, 7, 8, 8, -1, 9, 9, 10,
    -1, -1, -1, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 11, 11,
    11, 11, 11, 11, 12, 12, 13, 13, 13, 14, -1, 15, 15, 15, 15, 15, 15, 16, 16, 16, 16, 16, 16, 16,
    16, 16, 17, 17, 18, 19, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 20, 20, 20, 20, 20,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1,
];

pub const HGSS_OVERWORLD_DEX_MAPS: &[i32] = &[
    1, 2, 3, 4, 5, 6, -1, -1, 7, -1, -1, -1, -1, -1, -1, -1, -1, 8, -1, -1, -1, 9, 10, -1, -1, 11,
    12, 13, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 14, 15, 16, 17, 18, -1, -1, -1, -1, -1, -1, -1,
    -1, 19, 20, -1, -1, -1, -1, 21, 22, 23, -1, -1, -1, -1, -1, 24, -1, 25, 26, -1, -1, 27, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 28, -1, -1, -1, -1, -1, -1, 29, 30, 31, 32, 33, 34,
    35, 36, 37, 38, 39, 40, 41, 42, -1, -1, -1, -1, -1, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53,
    54, 55, 56, 57, 58, 59, 60, 61, 62, 63, -1, -1, -1, -1, 64, -1, -1, -1, -1, -1,
];

pub const HGSS_DUNGEON_DEX_MAPS: &[i32] = &[
    -1, -1, -1, -1, -1, -1, 1, 1, -1, 2, 2, 2, 2, 2, 3, 3, 3, -1, 4, 4, 5, -1, -1, 6, -1, -1, -1,
    -1, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, -1, -1, -1, -1, -1, 9, 9, -1, 9, -1, 9, -1, -1, -1, -1, 10,
    10, 10, 10, -1, -1, -1, 11, 11, 11, 11, -1, -1, 12, -1, -1, 13, 13, -1, 14, 14, 15, 15, 15, 15,
    15, 16, 16, 16, 17, 18, 8, -1, 16, 16, 16, 16, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, 14, 14, 20, 20, 21, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, 22, 23, 21, 21, -1, 24, -1, 25, 25, 25,
];

pub const HGSS_HEADBUTT_OVERWORLD_DEX_MAPS: &[i32] = &[
    43, 44, 45, 46, 47, 48, 49, 50, 53, 29, 54, 55, 56, 59, 61, 63, 40, 41, 42, 2, 4, 5, 7, 8, 9,
    10, 11, 12, 14, 15, 20, 21, 23, 25, 26, 32, 33, 65, 34, 35, 36, 37, 1, 3, 6, 66, 13, 22, 28,
    60, -1, -1, -1, 27, 39, 67, 64, 57, -1,
];

pub const HGSS_HEADBUTT_DUNGEON_DEX_MAPS: &[i32] = &[
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, 6, 5, 24, -1, -1, -1, -1, -1, 16,
];

pub const HGSS_BIG_OVERWORLD_POKEMON: &[i32] = &[
    536, 537, 579, 580, 651, 712, 713, 714, 833, 834, 836, 837, 838, 845, 846, 847, 848, 849, 850,
    851, 852, 853, 854, 855, 856, 857, 858, 859, 860, 861, 862,
];

pub const HGSS_BANNED_OVERWORLD_POKEMON: &[i32] = &[
    502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513, 514, 515, 516, 517, 518, 519, 520,
    521, 522, 523, 524, 525, 526, 527, 528, 846, 847, 848, 849, 850, 851, 852, 853, 854, 855, 856,
    857, 858, 859, 860, 861, 862,
];
