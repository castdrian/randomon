use std::collections::BTreeMap;

use randomon_ctr::held_items;
use randomon_data::constants::{abilities, items, moves};

const GEN7_CONSUMABLES: [i32; 5] = [
    items::ADRENALINE_ORB,
    items::ELECTRIC_SEED,
    items::PSYCHIC_SEED,
    items::MISTY_SEED,
    items::GRASSY_SEED,
];

pub fn consumable_held_items() -> Vec<i32> {
    let mut list = held_items::gen6_consumable_held_items();
    list.extend(GEN7_CONSUMABLES);
    list
}

pub fn all_held_items() -> Vec<i32> {
    let mut list = held_items::gen6_all_held_items();
    list.extend(GEN7_CONSUMABLES);
    list.extend([items::TERRAIN_EXTENDER, items::PROTECTIVE_PADS]);
    list
}

pub fn general_purpose_consumable_items() -> Vec<i32> {
    let mut list = held_items::gen6_general_purpose_consumable_items();
    list.extend([
        items::FIGY_BERRY,
        items::WIKI_BERRY,
        items::MAGO_BERRY,
        items::AGUAV_BERRY,
        items::IAPAPA_BERRY,
        items::ADRENALINE_ORB,
    ]);
    list
}

pub fn general_purpose_items() -> Vec<i32> {
    let mut list = held_items::gen6_general_purpose_items();
    list.push(items::PROTECTIVE_PADS);
    list
}

pub fn move_boosting_items() -> BTreeMap<i32, Vec<i32>> {
    let mut map = held_items::gen6_move_boosting_items();
    for terrain in [
        moves::ELECTRIC_TERRAIN,
        moves::GRASSY_TERRAIN,
        moves::MISTY_TERRAIN,
        moves::PSYCHIC_TERRAIN,
    ] {
        map.insert(terrain, vec![items::TERRAIN_EXTENDER]);
    }
    map.insert(moves::STRENGTH_SAP, vec![items::BIG_ROOT]);
    map
}

const SURGE_SEEDS: [(i32, i32); 4] = [
    (abilities::ELECTRIC_SURGE, items::ELECTRIC_SEED),
    (abilities::GRASSY_SURGE, items::GRASSY_SEED),
    (abilities::MISTY_SURGE, items::MISTY_SEED),
    (abilities::PSYCHIC_SURGE, items::PSYCHIC_SEED),
];

pub fn ability_boosting_items() -> BTreeMap<i32, Vec<i32>> {
    let mut map = held_items::gen6_ability_boosting_items();
    for (ability, _) in SURGE_SEEDS {
        map.insert(ability, vec![items::TERRAIN_EXTENDER]);
    }
    map
}

pub fn consumable_ability_boosting_item(ability: i32) -> Option<i32> {
    SURGE_SEEDS
        .iter()
        .find(|(candidate, _)| *candidate == ability)
        .map(|(_, seed)| *seed)
}
