use std::collections::BTreeMap;

use randomon_data::Type;
use randomon_data::constants::{abilities, items, moves, species};

const GEN4_CONSUMABLE_HELD_ITEMS: &[i32] = &[
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

const GEN4_NON_CONSUMABLE_HELD_ITEMS: &[i32] = &[
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

const GEN4_GENERAL_PURPOSE_CONSUMABLE_ITEMS: &[i32] = &[
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

const GEN4_GENERAL_PURPOSE_ITEMS: &[i32] = &[
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

const GEN5_GEMS_AND_CONSUMABLES: &[i32] = &[
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

const GEN5_NEW_HELD_ITEMS: &[i32] = &[
    items::EVIOLITE,
    items::FLOAT_STONE,
    items::ROCKY_HELMET,
    items::RING_TARGET,
    items::BINDING_BAND,
];

fn concat(parts: &[&[i32]]) -> Vec<i32> {
    parts.iter().flat_map(|part| part.iter().copied()).collect()
}

pub fn gen6_consumable_held_items() -> Vec<i32> {
    concat(&[
        GEN4_CONSUMABLE_HELD_ITEMS,
        GEN5_GEMS_AND_CONSUMABLES,
        &[
            items::WEAKNESS_POLICY,
            items::LUMINOUS_MOSS,
            items::SNOWBALL,
            items::ROSELI_BERRY,
            items::KEE_BERRY,
            items::MARANGA_BERRY,
            items::FAIRY_GEM,
        ],
    ])
}

pub fn gen6_all_held_items() -> Vec<i32> {
    concat(&[
        GEN4_NON_CONSUMABLE_HELD_ITEMS,
        GEN4_CONSUMABLE_HELD_ITEMS,
        GEN5_GEMS_AND_CONSUMABLES,
        GEN5_NEW_HELD_ITEMS,
        &[
            items::WEAKNESS_POLICY,
            items::SNOWBALL,
            items::ROSELI_BERRY,
            items::KEE_BERRY,
            items::MARANGA_BERRY,
            items::FAIRY_GEM,
        ],
        &[
            items::ASSAULT_VEST,
            items::PIXIE_PLATE,
            items::SAFETY_GOGGLES,
        ],
    ])
}

pub fn gen6_general_purpose_consumable_items() -> Vec<i32> {
    concat(&[
        GEN4_GENERAL_PURPOSE_CONSUMABLE_ITEMS,
        &[
            items::RED_CARD,
            items::ABSORB_BULB,
            items::CELL_BATTERY,
            items::EJECT_BUTTON,
        ],
        &[
            items::WEAKNESS_POLICY,
            items::LUMINOUS_MOSS,
            items::SNOWBALL,
            items::KEE_BERRY,
            items::MARANGA_BERRY,
        ],
    ])
}

pub fn gen6_general_purpose_items() -> Vec<i32> {
    concat(&[
        GEN4_GENERAL_PURPOSE_ITEMS,
        &[items::FLOAT_STONE, items::ROCKY_HELMET],
        &[items::SAFETY_GOGGLES],
    ])
}

pub fn gen6_weakness_reducing_berry(kind: Type) -> Option<i32> {
    Some(match kind {
        Type::Fire => items::OCCA_BERRY,
        Type::Water => items::PASSHO_BERRY,
        Type::Electric => items::WACAN_BERRY,
        Type::Grass => items::RINDO_BERRY,
        Type::Ice => items::YACHE_BERRY,
        Type::Fighting => items::CHOPLE_BERRY,
        Type::Poison => items::KEBIA_BERRY,
        Type::Ground => items::SHUCA_BERRY,
        Type::Flying => items::COBA_BERRY,
        Type::Psychic => items::PAYAPA_BERRY,
        Type::Bug => items::TANGA_BERRY,
        Type::Rock => items::CHARTI_BERRY,
        Type::Ghost => items::KASIB_BERRY,
        Type::Dragon => items::HABAN_BERRY,
        Type::Dark => items::COLBUR_BERRY,
        Type::Steel => items::BABIRI_BERRY,
        Type::Fairy => items::ROSELI_BERRY,
        _ => return None,
    })
}

pub fn gen6_consumable_type_boosting_item(kind: Type) -> Option<i32> {
    Some(match kind {
        Type::Fire => items::FIRE_GEM,
        Type::Water => items::WATER_GEM,
        Type::Electric => items::ELECTRIC_GEM,
        Type::Grass => items::GRASS_GEM,
        Type::Ice => items::ICE_GEM,
        Type::Fighting => items::FIGHTING_GEM,
        Type::Poison => items::POISON_GEM,
        Type::Ground => items::GROUND_GEM,
        Type::Flying => items::FLYING_GEM,
        Type::Psychic => items::PSYCHIC_GEM,
        Type::Bug => items::BUG_GEM,
        Type::Rock => items::ROCK_GEM,
        Type::Ghost => items::GHOST_GEM,
        Type::Dragon => items::DRAGON_GEM,
        Type::Dark => items::DARK_GEM,
        Type::Steel => items::STEEL_GEM,
        Type::Normal => items::NORMAL_GEM,
        Type::Fairy => items::FAIRY_GEM,
        _ => return None,
    })
}

pub fn gen6_type_boosting_items(kind: Option<Type>) -> Option<Vec<i32>> {
    let Some(kind) = kind else {
        return Some(Vec::new());
    };
    let list: &[i32] = match kind {
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
        Type::Fairy => &[items::PIXIE_PLATE],
        _ => return None,
    };
    Some(list.to_vec())
}

pub fn gen6_move_boosting_items() -> BTreeMap<i32, Vec<i32>> {
    let power_herb = vec![items::POWER_HERB];
    let mut map: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for move_number in [
        moves::BOUNCE,
        moves::DIG,
        moves::DIVE,
        moves::FLY,
        moves::RAZOR_WIND,
        moves::SKULL_BASH,
        moves::SKY_ATTACK,
        moves::SOLAR_BEAM,
    ] {
        map.insert(move_number, power_herb.clone());
    }
    map.insert(
        moves::FLING,
        vec![items::TOXIC_ORB, items::FLAME_ORB, items::IRON_BALL],
    );
    map.insert(moves::TRICK_ROOM, vec![items::IRON_BALL]);
    map.insert(moves::FACADE, vec![items::TOXIC_ORB, items::FLAME_ORB]);
    map.insert(
        moves::PSYCHO_SHIFT,
        vec![items::TOXIC_ORB, items::FLAME_ORB],
    );
    map.insert(moves::LIGHT_SCREEN, vec![items::LIGHT_CLAY]);
    map.insert(moves::REFLECT, vec![items::LIGHT_CLAY]);
    map.insert(moves::HAIL, vec![items::ICY_ROCK]);
    map.insert(moves::SANDSTORM, vec![items::SMOOTH_ROCK]);
    map.insert(moves::SUNNY_DAY, vec![items::HEAT_ROCK]);
    map.insert(moves::RAIN_DANCE, vec![items::DAMP_ROCK]);
    for move_number in [moves::OUTRAGE, moves::UPROAR] {
        map.insert(move_number, vec![items::GRIP_CLAW]);
    }
    for move_number in [
        moves::ABSORB,
        moves::AQUA_RING,
        moves::DRAIN_PUNCH,
        moves::DREAM_EATER,
        moves::GIGA_DRAIN,
        moves::INGRAIN,
        moves::LEECH_LIFE,
        moves::LEECH_SEED,
        moves::MEGA_DRAIN,
        moves::HORN_LEECH,
        moves::DRAINING_KISS,
        moves::OBLIVION_WING,
        moves::PARABOLIC_CHARGE,
    ] {
        map.insert(move_number, vec![items::BIG_ROOT]);
    }
    for move_number in [moves::TRICK, moves::SWITCHEROO] {
        map.insert(
            move_number,
            vec![items::TOXIC_ORB, items::FLAME_ORB, items::RING_TARGET],
        );
    }
    for move_number in [
        moves::BIND,
        moves::CLAMP,
        moves::FIRE_SPIN,
        moves::MAGMA_STORM,
        moves::SAND_TOMB,
        moves::WHIRLPOOL,
        moves::WRAP,
        moves::INFESTATION,
    ] {
        map.insert(move_number, vec![items::GRIP_CLAW, items::BINDING_BAND]);
    }
    map
}

pub fn gen6_ability_boosting_items() -> BTreeMap<i32, Vec<i32>> {
    BTreeMap::from([
        (abilities::GUTS, vec![items::FLAME_ORB, items::TOXIC_ORB]),
        (
            abilities::MAGIC_GUARD,
            vec![items::STICKY_BARB, items::LIFE_ORB],
        ),
        (abilities::DRIZZLE, vec![items::DAMP_ROCK]),
        (abilities::DROUGHT, vec![items::HEAT_ROCK]),
        (abilities::SAND_STREAM, vec![items::SMOOTH_ROCK]),
        (abilities::SNOW_WARNING, vec![items::ICY_ROCK]),
    ])
}

pub fn species_boosting_items(number: i32) -> Option<&'static [i32]> {
    Some(match number {
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
    })
}

pub fn gen5_ability_variations() -> BTreeMap<i32, Vec<i32>> {
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
            vec![abilities::CLOUD_NINE, abilities::AIR_LOCK],
        ),
        (
            abilities::FILTER,
            vec![abilities::FILTER, abilities::SOLID_ROCK],
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
    ])
}
