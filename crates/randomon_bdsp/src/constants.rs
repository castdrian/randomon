use std::collections::BTreeMap;

use randomon_data::constants::{abilities, items, moves, species};
use randomon_data::{ItemList, Trainer};

pub const SPECIES_COUNT: i32 = 493;
pub const PERSONAL_COUNT: i32 = 559;
pub const MOVE_COUNT: i32 = 826;
pub const HIGHEST_ABILITY_INDEX: i32 = 267;
pub const HIGHEST_ITEM_INDEX: usize = 1822;
pub const TM_COUNT: i32 = 100;
pub const PARTY_SIZE: usize = 6;
pub const EVOLUTIONS_PER_SPECIES: usize = 8;

pub const BROKEN_ABILITIES: &[i32] = &[
    abilities::STANCE_CHANGE,
    abilities::DISGUISE,
    abilities::BATTLE_BOND,
    abilities::POWER_CONSTRUCT,
    abilities::RKS_SYSTEM,
    abilities::GULP_MISSILE,
    abilities::ICE_FACE,
    abilities::HUNGER_SWITCH,
    abilities::AS_ONE_CHILLING_NEIGH,
    abilities::AS_ONE_GRIM_NEIGH,
];

pub const MAIN_GAME_LEGENDARIES: &[i32] = &[species::DIALGA, species::PALKIA];

pub const IRREGULAR_FORMES: &[i32] = &[521, 522, 523];

pub const ACTUALLY_COSMETIC_SPECIES: &[i32] = &[
    species::UNOWN,
    species::BURMY,
    species::MOTHIM,
    species::CHERRIM,
    species::SHELLOS,
    species::GASTRODON,
    species::ARCEUS,
];

pub const BANNED_MOVES: &[i32] = &[moves::DARK_VOID];

pub const RANDOMIZABLE_POCKETS: &[i32] = &[0, 1, 2, 3, 4, 6];
pub const UNRANDOMIZABLE_ITEMS: &[i32] = &[items::SAFARI_BALL];

pub fn useless_abilities() -> Vec<i32> {
    let mut useless = randomon_gen7::constants::USELESS_ABILITIES.to_vec();
    useless.extend(BROKEN_ABILITIES);
    useless.sort_unstable();
    useless.dedup();
    useless
}

pub fn ability_variations() -> BTreeMap<i32, Vec<i32>> {
    randomon_gen7::constants::ability_variations()
}

pub fn evolution_items() -> Vec<i32> {
    randomon_data::constants::gen4::EVOLUTION_ITEMS.to_vec()
}

pub fn non_bad_bans(list: &mut ItemList) {
    list.ban_singles(&[
        items::ODD_KEYSTONE,
        items::GRISEOUS_ORB,
        items::SOUL_DEW,
        items::LIGHT_BALL,
        items::ORAN_BERRY,
        items::QUICK_POWDER,
        items::SHOAL_SALT,
        items::SHOAL_SHELL,
    ]);
    list.ban_range(items::GROWTH_MULCH, 4);
    list.ban_range(items::ADAMANT_ORB, 2);
    list.ban_range(items::MAIL1, 12);
    list.ban_range(items::FIGY_BERRY, 25);
    list.ban_range(items::LUCKY_PUNCH, 4);
    list.ban_range(items::RED_SCARF, 5);
    list.ban_range(items::BLUE_FLUTE, 5);
}

pub fn regular_shop_items() -> Vec<i32> {
    (items::ULTRA_BALL..=items::POKE_BALL)
        .chain(items::POTION..=items::REVIVE)
        .chain(items::SUPER_REPEL..=items::REPEL)
        .chain([items::HONEY])
        .collect()
}

pub fn op_shop_items() -> Vec<i32> {
    let mut list = vec![
        items::LAVA_COOKIE,
        items::BERRY_JUICE,
        items::RARE_CANDY,
        items::OLD_GATEAU,
    ];
    list.extend(items::TINY_MUSHROOM..=items::NUGGET);
    list.extend(items::LANSAT_BERRY..=items::ROWAP_BERRY);
    list.push(items::LUCKY_EGG);
    list
}

pub fn forme_suffix(base_forme: i32, forme_number: i32) -> &'static str {
    randomon_gen7::constants::forme_suffix_by_base_forme(base_forme, forme_number)
}

fn tag_one(trainers: &mut [Trainer], id: i32, tag: &str) {
    if let Some(trainer) = trainers.iter_mut().find(|trainer| trainer.index == id) {
        trainer.tag = Some(tag.to_owned());
    }
}

fn tag_all(trainers: &mut [Trainer], ids: &[i32], tag: &str) {
    for &id in ids {
        tag_one(trainers, id, tag);
    }
}

fn starter_line(trainer: &Trainer) -> Option<usize> {
    trainer.pokemon.iter().find_map(|pokemon| {
        let number = pokemon.pokemon.number();
        (species::TURTWIG..=species::EMPOLEON)
            .contains(&number)
            .then(|| ((number - species::TURTWIG) / 3) as usize)
    })
}

fn tag_rival(trainers: &mut [Trainer], tag: &str, ids: [i32; 3]) {
    for (position, id) in ids.into_iter().enumerate() {
        let Some(trainer) = trainers.iter_mut().find(|trainer| trainer.index == id) else {
            continue;
        };
        let line = starter_line(trainer).unwrap_or(position);
        trainer.tag = Some(format!("{tag}-{}", (line + 2) % 3));
    }
}

pub const RIVAL_BATTLES: [(&str, [i32; 3]); 8] = [
    ("RIVAL1", [176, 177, 178]),
    ("RIVAL2", [351, 352, 353]),
    ("RIVAL3", [354, 355, 356]),
    ("RIVAL4", [357, 358, 359]),
    ("RIVAL5", [481, 493, 494]),
    ("RIVAL6", [360, 361, 362]),
    ("RIVAL7", [663, 664, 665]),
    ("RIVAL8", [666, 667, 668]),
];

pub const GYM_LEADERS: [i32; 8] = [175, 240, 241, 242, 243, 244, 179, 245];
pub const GYM_LEADER_REMATCHES: [i32; 8] = [676, 677, 678, 679, 680, 681, 682, 683];
pub const ELITE_FOUR: [i32; 5] = [185, 186, 187, 188, 191];
pub const ELITE_FOUR_REMATCHES: [[i32; 5]; 2] =
    [[696, 697, 698, 699, 700], [701, 702, 703, 704, 705]];

pub fn tag_trainers(trainers: &mut [Trainer]) {
    for (gym, (&leader, &rematch)) in GYM_LEADERS.iter().zip(&GYM_LEADER_REMATCHES).enumerate() {
        tag_all(
            trainers,
            &[leader, rematch],
            &format!("GYM{}-LEADER", gym + 1),
        );
    }
    for (index, &member) in ELITE_FOUR[..4].iter().enumerate() {
        tag_one(trainers, member, &format!("ELITE{}", index + 1));
    }
    tag_one(trainers, ELITE_FOUR[4], "CHAMPION");
    for (tag, ids) in RIVAL_BATTLES {
        tag_rival(trainers, tag, ids);
    }
    tag_all(trainers, &[308, 309], "THEMED:CYRUS-LEADER");
    tag_all(trainers, &[220, 310], "THEMED:MARS-STRONG");
    tag_all(trainers, &[311, 312], "THEMED:JUPITER-STRONG");
    tag_all(trainers, &[313, 314], "THEMED:SATURN-STRONG");
}

pub fn elite_four_trainers(post_game: bool) -> Vec<i32> {
    if post_game {
        ELITE_FOUR_REMATCHES.iter().flatten().copied().collect()
    } else {
        ELITE_FOUR.to_vec()
    }
}

pub fn set_forced_rival_starter_positions(trainers: &mut [Trainer]) {
    for trainer in trainers.iter_mut() {
        let Some(tag) = trainer.tag.as_deref() else {
            continue;
        };
        if !tag.starts_with("RIVAL") {
            continue;
        }
        let position = trainer.pokemon.iter().position(|pokemon| {
            let number = pokemon.pokemon.number();
            (species::TURTWIG..=species::EMPOLEON).contains(&number)
        });
        if let Some(position) = position {
            trainer.force_starter_position = position as i32;
        }
    }
}
