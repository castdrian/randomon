use crate::constants::{abilities, items, moves, species};
use crate::types::Type;

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
