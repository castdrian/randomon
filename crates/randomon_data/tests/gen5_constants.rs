use randomon_data::constants::species::gen5_formes;
use randomon_data::constants::{abilities, gen4, gen5, items, moves, species};
use randomon_data::{MoveCategory, MultiBattleStatus, Trainer, Type};

fn trainers(count: usize) -> Vec<Trainer> {
    (0..count).map(|_| Trainer::new()).collect()
}

#[test]
fn list_sizes_match_upstream() {
    assert_eq!(gen5::BW1_MAIN_PLAYTHROUGH_TRAINERS.len(), 370);
    assert_eq!(gen5::BW2_MAIN_PLAYTHROUGH_TRAINERS.len(), 473);
    assert_eq!(gen5::BW1_WILD_FILE_TO_AREA_MAP.len(), 112);
    assert_eq!(gen5::BW2_WILD_FILE_TO_AREA_MAP.len(), 135);
    assert_eq!(gen5::BW1_SHOP_NAMES.len(), 26);
    assert_eq!(gen5::BW2_SHOP_NAMES.len(), 32);
    assert_eq!(gen5::BW2_HIDDEN_HOLLOW_UNOVA_POKEMON.len(), 53);
    assert_eq!(gen5::EVOLUTION_ITEMS.len(), 24);
    assert_eq!(gen5::main_game_shops(gen5::TYPE_BW).len(), 12);
    assert_eq!(gen5::main_game_shops(gen5::TYPE_BW2).len(), 17);
    assert!(gen5::main_game_shops(2).is_empty());
    assert_eq!(gen5::irregular_formes(gen5::TYPE_BW2).len(), 7);
    assert_eq!(gen4::consumable_held_items().len(), 49);
    assert_eq!(gen5::consumable_held_items().len(), 71);
    assert_eq!(
        gen5::all_held_items().len(),
        gen4::all_held_items().len() + 27
    );
    assert_eq!(gen5::general_purpose_consumable_items().len(), 28);
    assert_eq!(gen5::general_purpose_items().len(), 18);
    assert_eq!(gen5::ability_variations().len(), 8);
}

#[test]
fn spot_values_match_upstream() {
    assert_eq!(gen5::BW2_WILD_FILE_TO_AREA_MAP[95], -1);
    assert_eq!(gen5::BW1_WILD_FILE_TO_AREA_MAP[111], 54);
    assert_eq!(gen5::BW1_MAIN_PLAYTHROUGH_TRAINERS[369], 613);
    assert_eq!(gen5::BW2_MAIN_PLAYTHROUGH_TRAINERS[0], 4);
    assert_eq!(gen5::BW2_SHOP_NAMES[31], "Lentimas Secondary");
    assert_eq!(gen5::encounter_area_count(gen5::TYPE_BW2), 85);
    assert_eq!(gen5::area_data_entry_length(gen5::TYPE_BW), 249);
    assert_eq!(gen5::forme_offset(gen5::TYPE_BW2), 35);
    assert_eq!(gen5::forme_count(gen5::TYPE_BW), 18);
    assert_eq!(gen5::non_pokemon_battle_sprite_count(gen5::TYPE_BW2), 36);
    assert_eq!(gen5::balanced_item_price(items::MASTER_BALL), Some(300));
    assert_eq!(gen5::balanced_item_price(items::ELIXIR), Some(1500));
    assert_eq!(gen5::balanced_item_price(items::CASTELIACONE), Some(45));
    assert_eq!(gen5::balanced_item_price(items::REVEAL_GLASS), Some(0));
    assert_eq!(gen5::balanced_item_price(items::NONE), None);
    assert_eq!(
        gen5::ability_variations()[&abilities::MOLD_BREAKER],
        vec![
            abilities::MOLD_BREAKER,
            abilities::TURBOBLAZE,
            abilities::TERAVOLT
        ]
    );
}

#[test]
fn type_and_category_tables_round_trip() {
    assert_eq!(gen5::type_table(0x08), Some(Type::Steel));
    assert_eq!(gen5::type_table(0x10), Some(Type::Dark));
    assert_eq!(gen5::type_table(0x11), None);
    assert_eq!(gen5::type_table(-1), None);
    assert_eq!(gen5::type_to_byte(Some(Type::Steel)), 0x08);
    assert_eq!(gen5::type_to_byte(Some(Type::Fairy)), 0);
    assert_eq!(gen5::type_to_byte(None), 0);
    assert_eq!(
        gen5::move_category_from_index(0),
        Some(MoveCategory::Status)
    );
    assert_eq!(gen5::move_category_from_index(3), None);
    assert_eq!(gen5::move_category_to_byte(MoveCategory::Special), 2);
}

#[test]
fn forme_lookups_follow_the_java_fallbacks() {
    assert_eq!(
        gen5::absolute_poke_num_by_base_forme(species::DEOXYS, 1),
        gen5_formes::DEOXYS_A
    );
    assert_eq!(
        gen5::absolute_poke_num_by_base_forme(species::DEOXYS, 7),
        species::DEOXYS
    );
    assert_eq!(
        gen5::absolute_poke_num_by_base_forme(species::BULBASAUR, 255),
        0
    );
    assert_eq!(
        gen5::absolute_poke_num_by_base_forme(species::BULBASAUR, 3),
        species::BULBASAUR
    );
    assert_eq!(gen5::forme_suffix_by_base_forme(species::ROTOM, 3), "-Fr");
    assert_eq!(gen5::forme_suffix_by_base_forme(species::ROTOM, 9), "");
    assert_eq!(gen5::forme_suffix_by_base_forme(species::PIKACHU, 0), "");
    assert_eq!(gen5::forme_suffix_by_base_forme(species::PIKACHU, 1), "");
    assert_eq!(
        gen5::forme_suffix(gen5_formes::KYUREM_W + 35, gen5::TYPE_BW2),
        "-W"
    );
    assert_eq!(gen5::forme_suffix(gen5_formes::KYUREM_W, gen5::TYPE_BW), "");
    assert_eq!(
        gen5::forme_suffix(gen5_formes::ROTOM_FA, gen5::TYPE_BW),
        "-Fa"
    );
    assert_eq!(gen5::forme_suffix(gen5_formes::ROTOM_FA, 2), "");
}

#[test]
fn allowed_items_ban_key_items_and_tms() {
    let allowed = gen5::allowed_items();
    assert!(!allowed.is_allowed(items::NONE));
    assert!(!allowed.is_allowed(items::LIBERTY_PASS));
    assert!(!allowed.is_allowed(items::TM01));
    assert!(!allowed.is_allowed(items::TM95));
    assert!(!allowed.is_allowed(items::DIRE_HIT2));
    assert!(!allowed.is_allowed(items::DATA_CARD01));
    assert!(allowed.is_tm(items::TM01));
    assert!(allowed.is_tm(items::TM95));
    assert!(!allowed.is_tm(items::HM01));
    assert!(allowed.is_allowed(items::LEFTOVERS));
    assert!(!allowed.is_allowed(items::REVEAL_GLASS));
}

#[test]
fn non_bad_items_differ_between_bw1_and_bw2() {
    let bw1 = gen5::non_bad_items(gen5::TYPE_BW);
    let bw2 = gen5::non_bad_items(gen5::TYPE_BW2);
    assert!(bw2.is_allowed(items::RED_SHARD));
    assert!(!bw1.is_allowed(items::RED_SHARD));
    assert!(!bw1.is_allowed(items::GREEN_SHARD));
    for list in [&bw1, &bw2] {
        assert!(!list.is_allowed(items::ORAN_BERRY));
        assert!(!list.is_allowed(items::LIGHT_BALL));
        assert!(!list.is_allowed(items::MAIL12));
        assert!(list.is_allowed(items::SITRUS_BERRY));
    }
    assert_eq!(gen5::non_bad_items(7), bw1);
}

#[test]
fn shop_item_lists_keep_upstream_order() {
    let regular = gen5::regular_shop_items();
    assert_eq!(&regular[..3], &[2, 3, 4]);
    assert_eq!(regular.len(), 3 + 12 + 4);
    assert_eq!(regular[regular.len() - 1], items::REPEL);
    let op = gen5::op_shop_items();
    assert_eq!(
        &op[..4],
        &[
            items::LAVA_COOKIE,
            items::BERRY_JUICE,
            items::RARE_CANDY,
            items::OLD_GATEAU
        ]
    );
    assert_eq!(op[op.len() - 1], items::CASTELIACONE);
}

#[test]
fn boosting_items_layer_gen5_over_gen4() {
    assert_eq!(
        gen5::move_boosting_items(moves::TRICK),
        Some(vec![items::TOXIC_ORB, items::FLAME_ORB, items::RING_TARGET])
    );
    assert_eq!(
        gen5::move_boosting_items(moves::OUTRAGE),
        Some(vec![items::GRIP_CLAW])
    );
    assert_eq!(
        gen5::move_boosting_items(moves::WRAP),
        Some(vec![items::GRIP_CLAW, items::BINDING_BAND])
    );
    assert_eq!(
        gen5::move_boosting_items(moves::HORN_LEECH),
        Some(vec![items::BIG_ROOT])
    );
    assert_eq!(gen5::move_boosting_items(moves::POUND), None);
    assert_eq!(
        gen5::consumable_type_boosting_item(Type::Normal),
        Some(items::NORMAL_GEM)
    );
    assert_eq!(gen5::consumable_type_boosting_item(Type::Fairy), None);
    assert_eq!(
        gen5::weakness_reducing_berry(Type::Steel),
        Some(items::BABIRI_BERRY)
    );
    assert_eq!(
        gen5::type_boosting_items(Type::Water).map(|list| list.len()),
        Some(4)
    );
    assert_eq!(
        gen5::species_boosting_items(species::DITTO),
        Some(vec![items::METAL_POWDER, items::QUICK_POWDER])
    );
    assert_eq!(
        gen5::ability_boosting_items(abilities::GUTS),
        Some(vec![items::FLAME_ORB, items::TOXIC_ORB])
    );
}

#[test]
fn bw_tagging_marks_leaders_and_rivals() {
    let mut all = trainers(700);
    gen5::tag_trainers_bw(&mut all);
    assert_eq!(all[0x0C - 1].tag.as_deref(), Some("GYM1-LEADER"));
    assert_eq!(all[0x34].tag.as_deref(), Some("RIVAL1-0"));
    assert_eq!(all[0x35].tag.as_deref(), Some("RIVAL1-1"));
    assert_eq!(all[0x36].tag.as_deref(), Some("RIVAL1-2"));
    assert_eq!(all[63].tag.as_deref(), Some("NOTSTRONG"));
}

#[test]
fn bw2_tagging_marks_challenge_mode_teams() {
    let mut all = trainers(900);
    gen5::tag_trainers_bw2(&mut all);
    assert_eq!(all[0x2fc - 1].tag.as_deref(), Some("GYM1-LEADER"));
    assert_eq!(all[0x30d - 1].tag.as_deref(), Some("CHAMPION"));
    assert_eq!(all[0x32E - 1].tag.as_deref(), Some("GYM1"));
    assert_eq!(all[0x16b].tag.as_deref(), Some("FRIEND1-1"));
}

#[test]
fn multi_battle_status_respects_bounds_and_version() {
    let mut short = trainers(500);
    gen5::set_multi_battle_status_bw2(&mut short, true);
    assert_eq!(short[341].multi_battle_status, MultiBattleStatus::Always);
    assert_eq!(short[495].multi_battle_status, MultiBattleStatus::Always);

    let mut black = trainers(813);
    gen5::set_multi_battle_status_bw2(&mut black, true);
    assert_eq!(black[520].multi_battle_status, MultiBattleStatus::Always);
    assert_eq!(black[519].multi_battle_status, MultiBattleStatus::Potential);
    assert_eq!(black[513].multi_battle_status, MultiBattleStatus::Never);

    let mut white = trainers(813);
    gen5::set_multi_battle_status_bw2(&mut white, false);
    assert_eq!(white[520].multi_battle_status, MultiBattleStatus::Potential);
    assert_eq!(white[518].multi_battle_status, MultiBattleStatus::Always);
    assert_eq!(white[788].multi_battle_status, MultiBattleStatus::Never);

    let mut bw = trainers(402);
    gen5::set_multi_battle_status_bw(&mut bw);
    assert_eq!(bw[401].multi_battle_status, MultiBattleStatus::Always);
}
