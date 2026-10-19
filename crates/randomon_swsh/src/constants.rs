use std::collections::BTreeMap;

use crate::DataVersion;
use randomon_data::constants::{abilities, items, moves, species};
use randomon_data::{ItemList, Trainer};

pub const HIGHEST_ABILITY_INDEX: i32 = 267;
pub const HIGHEST_ITEM_INDEX: usize = 1607;
pub const TM_COUNT: i32 = 100;
pub const TR_COUNT: i32 = 100;
pub const PARTY_SIZE: usize = 6;
pub const MAX_TRAINER_NAME_LENGTH: i32 = 12;
pub const MAX_TRAINER_CLASS_NAME_LENGTH: i32 = 15;

pub const MAX_GUARD: i32 = 743;
pub const MAX_ATTACKS: std::ops::RangeInclusive<i32> = 757..=774;

pub fn is_max_move(number: i32) -> bool {
    number == MAX_GUARD || MAX_ATTACKS.contains(&number)
}
pub const DYNAMAX_CRYSTALS: std::ops::RangeInclusive<i32> = 1279..=1578;
pub const STATIC_SPECIES_KEPT: &[i32] = &[species::ZACIAN, species::ZAMAZENTA, species::ETERNATUS];
pub const GIFT_SPECIES_KEPT: &[i32] = &[species::KUBFU, species::GLASTRIER, species::SPECTRIER];
pub const MAIN_GAME_LEGENDARIES: &[i32] = &[species::ZACIAN, species::ZAMAZENTA];

pub fn species_count(version: DataVersion) -> i32 {
    match version {
        DataVersion::Base => species::ETERNATUS,
        DataVersion::Expansion => species::CALYREX,
    }
}

pub const BATTLE_ONLY_FORMS: &[(i32, i32)] = &[
    (species::CHERRIM, 1),
    (species::GIRATINA, 1),
    (species::DARMANITAN, 1),
    (species::DARMANITAN, 3),
    (species::AEGISLASH, 1),
    (species::ZYGARDE, 2),
    (species::ZYGARDE, 3),
    (species::ZYGARDE, 4),
    (species::WISHIWASHI, 1),
    (species::MIMIKYU, 1),
    (species::CRAMORANT, 1),
    (species::CRAMORANT, 2),
    (species::EISCUE, 1),
    (species::MORPEKO, 1),
    (species::ZACIAN, 1),
    (species::ZAMAZENTA, 1),
    (species::ETERNATUS, 1),
];

pub const COSMETIC_FORM_SPECIES: &[i32] = &[
    species::PIKACHU,
    species::SHELLOS,
    species::GASTRODON,
    species::KELDEO,
    species::GENESECT,
    species::XERNEAS,
    species::ROCKRUFF,
    species::SILVALLY,
    species::MAGEARNA,
    species::SINISTEA,
    species::POLTEAGEIST,
    species::ALCREMIE,
    species::ZARUDE,
];

pub const IRREGULAR_FORMS: &[(i32, i32)] = &[
    (species::KYUREM, 1),
    (species::KYUREM, 2),
    (species::NECROZMA, 1),
    (species::NECROZMA, 2),
    (species::CALYREX, 1),
    (species::CALYREX, 2),
];

const ALOLAN_FORMS: &[i32] = &[
    species::RAICHU,
    species::SANDSHREW,
    species::SANDSLASH,
    species::VULPIX,
    species::NINETALES,
    species::DIGLETT,
    species::DUGTRIO,
    species::PERSIAN,
    species::EXEGGUTOR,
    species::MAROWAK,
];

const GALARIAN_FORMS: &[(i32, i32)] = &[
    (species::MEOWTH, 2),
    (species::PONYTA, 1),
    (species::RAPIDASH, 1),
    (species::SLOWPOKE, 1),
    (species::SLOWBRO, 2),
    (species::FARFETCHD, 1),
    (species::WEEZING, 1),
    (species::MR_MIME, 1),
    (species::ARTICUNO, 1),
    (species::ZAPDOS, 1),
    (species::MOLTRES, 1),
    (species::SLOWKING, 1),
    (species::CORSOLA, 1),
    (species::ZIGZAGOON, 1),
    (species::LINOONE, 1),
    (species::DARUMAKA, 1),
    (species::DARMANITAN, 2),
    (species::YAMASK, 1),
    (species::STUNFISK, 1),
];

const NAMED_FORM_SUFFIXES: &[(i32, i32, &str)] = &[
    (species::MEOWTH, 1, "-A"),
    (species::BASCULIN, 1, "-B"),
    (species::ZYGARDE, 1, "-10"),
    (species::LYCANROC, 1, "-M"),
    (species::LYCANROC, 2, "-D"),
    (species::NECROZMA, 1, "-DM"),
    (species::NECROZMA, 2, "-DW"),
    (species::TOXTRICITY, 1, "-L"),
    (species::INDEEDEE, 1, "-F"),
    (species::URSHIFU, 1, "-R"),
    (species::CALYREX, 1, "-I"),
    (species::CALYREX, 2, "-S"),
];

pub fn forme_suffix(base_forme: i32, forme_number: i32) -> String {
    if forme_number == 0 {
        return String::new();
    }
    if forme_number == 1 && ALOLAN_FORMS.contains(&base_forme) {
        return "-A".to_owned();
    }
    if GALARIAN_FORMS.contains(&(base_forme, forme_number)) {
        return "-G".to_owned();
    }
    if let Some((_, _, suffix)) = NAMED_FORM_SUFFIXES
        .iter()
        .find(|(number, forme, _)| *number == base_forme && *forme == forme_number)
    {
        return (*suffix).to_owned();
    }
    let known = randomon_gen7::constants::forme_suffix_by_base_forme(base_forme, forme_number);
    if known.is_empty() {
        format!("-{forme_number}")
    } else {
        known.to_owned()
    }
}

pub const BROKEN_ABILITIES: &[i32] = &[
    abilities::STANCE_CHANGE,
    abilities::DISGUISE,
    abilities::BATTLE_BOND,
    abilities::POWER_CONSTRUCT,
    abilities::RKS_SYSTEM,
    abilities::GULP_MISSILE,
    abilities::ICE_FACE,
    abilities::HUNGER_SWITCH,
    abilities::ZEN_MODE,
    abilities::SCHOOLING,
    abilities::AS_ONE_CHILLING_NEIGH,
    abilities::AS_ONE_GRIM_NEIGH,
];

pub const BANNED_MOVES: &[i32] = &[moves::DARK_VOID];

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
    let mut list = randomon_data::constants::gen4::EVOLUTION_ITEMS.to_vec();
    list.extend([
        items::ICE_STONE,
        items::WHIPPED_DREAM,
        items::SACHET,
        items::SWEET_APPLE,
        items::TART_APPLE,
        items::CRACKED_POT,
        items::CHIPPED_POT,
        items::GALARICA_CUFF,
        items::GALARICA_WREATH,
    ]);
    list.extend(items::STRAWBERRY_SWEET..=items::RIBBON_SWEET);
    list.sort_unstable();
    list.dedup();
    list
}

pub fn non_bad_bans(list: &mut ItemList) {
    list.ban_singles(&[
        items::LIGHT_BALL,
        items::ORAN_BERRY,
        items::QUICK_POWDER,
        items::SOUL_DEW,
        items::RUSTED_SWORD,
        items::RUSTED_SHIELD,
    ]);
    list.ban_range(items::FIGY_BERRY, 25);
    list.ban_range(items::LUCKY_PUNCH, 4);
    list.ban_range(items::STRAWBERRY_SWEET, 7);
    list.ban_range(items::LONELY_MINT, 21);
}

pub fn regular_shop_items() -> Vec<i32> {
    (items::ULTRA_BALL..=items::POKE_BALL)
        .chain(items::POTION..=items::REVIVE)
        .chain(items::SUPER_REPEL..=items::REPEL)
        .collect()
}

pub fn op_shop_items() -> Vec<i32> {
    let mut list = vec![
        items::RARE_CANDY,
        items::MASTER_BALL,
        items::BOTTLE_CAP,
        items::GOLD_BOTTLE_CAP,
        items::WISHING_PIECE,
        items::LUCKY_EGG,
    ];
    list.extend(items::TINY_MUSHROOM..=items::NUGGET);
    list.extend(items::EXP_CANDY_XS..=items::DYNAMAX_CANDY);
    list
}

pub const STARTER_LINES: [std::ops::RangeInclusive<i32>; 3] = [
    species::GROOKEY..=species::RILLABOOM,
    species::SCORBUNNY..=species::CINDERACE,
    species::SOBBLE..=species::INTELEON,
];

pub const GYM_LEADERS: [&[i32]; 8] = [
    &[32],
    &[36],
    &[37],
    &[77, 78],
    &[108],
    &[135, 136],
    &[107],
    &[144],
];
pub const GYM_TRAINER_CLASSES: [&[i32]; 8] = [
    &[76, 77],
    &[78],
    &[79, 80],
    &[81, 82, 83, 84],
    &[85],
    &[86, 87, 88, 89],
    &[179],
    &[92, 93],
];
pub const CHAMPION: &[i32] = &[149, 189, 190];
pub const CHAMPION_CUP: &[i32] = &[145, 153, 154, 155, 213];
pub const HOP_CLASSES: &[i32] = &[7, 8, 184, 188, 199, 241];
pub const THEMED_CLASSES: &[(&str, &[i32])] = &[
    ("THEMED:BEDE-STRONG", &[11, 12, 231]),
    ("THEMED:MARNIE-STRONG", &[13, 14, 15, 232]),
    ("THEMED:LEON-STRONG", &[6, 233]),
    ("THEMED:ROSE-STRONG", &[32]),
    ("THEMED:OLEANA-STRONG", &[30]),
    ("THEMED:SORDWARD-STRONG", &[74, 186, 247]),
    ("THEMED:SHIELBERT-STRONG", &[75, 187, 240]),
    ("THEMED:MUSTARD-STRONG", &[221, 222, 230]),
    ("THEMED:PEONY-STRONG", &[228, 246]),
];

pub fn starter_line(number: i32) -> Option<usize> {
    STARTER_LINES.iter().position(|line| line.contains(&number))
}

fn tag_ids(trainers: &mut [Trainer], ids: &[i32], tag: &str) {
    for trainer in trainers
        .iter_mut()
        .filter(|trainer| ids.contains(&trainer.index))
    {
        trainer.tag = Some(tag.to_owned());
    }
}

fn tag_classes(trainers: &mut [Trainer], classes: &[i32], tag: &str) {
    for trainer in trainers
        .iter_mut()
        .filter(|trainer| trainer.tag.is_none() && classes.contains(&trainer.trainerclass))
        .filter(|trainer| !trainer.pokemon.is_empty())
    {
        trainer.tag = Some(tag.to_owned());
    }
}

fn starter_member(trainer: &Trainer) -> Option<(usize, usize, i32)> {
    trainer
        .pokemon
        .iter()
        .enumerate()
        .find_map(|(position, pokemon)| {
            starter_line(pokemon.pokemon.number()).map(|line| (position, line, pokemon.level))
        })
}

fn tag_hop(trainers: &mut [Trainer]) {
    let mut levels: Vec<i32> = trainers
        .iter()
        .filter(|trainer| HOP_CLASSES.contains(&trainer.trainerclass))
        .filter_map(starter_member)
        .map(|(_, _, level)| level)
        .collect();
    levels.sort_unstable();
    levels.dedup();
    for trainer in trainers
        .iter_mut()
        .filter(|trainer| HOP_CLASSES.contains(&trainer.trainerclass))
    {
        let Some((position, line, level)) = starter_member(trainer) else {
            continue;
        };
        let Some(battle) = levels.iter().position(|&known| known == level) else {
            continue;
        };
        trainer.tag = Some(format!("RIVAL{}-{}", battle + 1, (line + 2) % 3));
        trainer.force_starter_position = position as i32;
    }
}

pub fn tag_trainers(trainers: &mut [Trainer]) {
    tag_hop(trainers);
    for (gym, leaders) in GYM_LEADERS.iter().enumerate() {
        tag_ids(trainers, leaders, &format!("GYM{}-LEADER", gym + 1));
    }
    tag_ids(trainers, CHAMPION, "CHAMPION");
    for (tag, classes) in THEMED_CLASSES {
        tag_classes(trainers, classes, tag);
    }
    for (gym, classes) in GYM_TRAINER_CLASSES.iter().enumerate() {
        tag_classes(trainers, classes, &format!("GYM{}", gym + 1));
    }
}
