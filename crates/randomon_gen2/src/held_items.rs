use randomon_data::{GameData, Type};

use crate::items::*;

pub const CONSUMABLE_HELD_ITEMS: &[i32] = &[
    BERRY,
    GOLD_BERRY,
    BERRY_JUICE,
    MIRACLE_BERRY,
    MINT_BERRY,
    PRZ_CURE_BERRY,
    PSN_CURE_BERRY,
    ICE_BERRY,
    BURNT_BERRY,
    BITTER_BERRY,
    MYSTERY_BERRY,
    BERSERK_GENE,
];

const GENERAL_HELD_ITEMS: &[i32] = &[
    LEFTOVERS,
    QUICK_CLAW,
    KINGS_ROCK,
    SCOPE_LENS,
    FOCUS_BAND,
    BRIGHT_POWDER,
];

const TYPE_BOOSTERS: &[(Type, i32)] = &[
    (Type::Normal, PINK_BOW),
    (Type::Normal, POLKADOT_BOW),
    (Type::Fire, CHARCOAL),
    (Type::Water, MYSTIC_WATER),
    (Type::Grass, MIRACLE_SEED),
    (Type::Electric, MAGNET),
    (Type::Ice, NEVER_MELT_ICE),
    (Type::Fighting, BLACKBELT),
    (Type::Poison, POISON_BARB),
    (Type::Ground, SOFT_SAND),
    (Type::Flying, SHARP_BEAK),
    (Type::Psychic, TWISTED_SPOON),
    (Type::Bug, SILVER_POWDER),
    (Type::Rock, HARD_STONE),
    (Type::Ghost, SPELL_TAG),
    (Type::Dragon, DRAGON_FANG),
    (Type::Dark, BLACK_GLASSES),
    (Type::Steel, METAL_COAT),
];

pub fn all_held_items() -> Vec<i32> {
    let mut items = CONSUMABLE_HELD_ITEMS.to_vec();
    items.extend_from_slice(GENERAL_HELD_ITEMS);
    items.extend(TYPE_BOOSTERS.iter().map(|&(_, item)| item));
    items
}

pub fn sensible_held_items(data: &GameData, consumable_only: bool, moves: &[i32; 4]) -> Vec<i32> {
    let mut items = CONSUMABLE_HELD_ITEMS.to_vec();
    if consumable_only {
        return items;
    }
    items.extend_from_slice(GENERAL_HELD_ITEMS);
    for &number in moves {
        let Some(mv) = data.move_by_number(number) else {
            continue;
        };
        if mv.power <= 1 {
            continue;
        }
        for &(kind, item) in TYPE_BOOSTERS {
            if mv.kind == Some(kind) && !items.contains(&item) {
                items.push(item);
            }
        }
    }
    items
}
