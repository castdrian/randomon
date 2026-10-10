use randomon_data::constants::gen1;
use randomon_data::text::{camel_case, java_equals_ignore_case};
use randomon_data::{
    Effectiveness, EvolutionType, GameData, ItemList, MoveLearnt, PokemonId, StatLayout, Type,
    moves_at_level,
};
use randomon_java::{JavaHash, JavaRandom};

#[test]
fn evolution_type_indices_match_the_upstream_reverse_table() {
    assert_eq!(EvolutionType::from_index(1, 1), Some(EvolutionType::Level));
    assert_eq!(EvolutionType::from_index(1, 2), Some(EvolutionType::Stone));
    assert_eq!(EvolutionType::from_index(1, 3), Some(EvolutionType::Trade));
    assert_eq!(EvolutionType::from_index(2, 3), Some(EvolutionType::Trade));
    assert_eq!(
        EvolutionType::from_index(2, 4),
        Some(EvolutionType::Happiness)
    );
    assert_eq!(EvolutionType::Stone.to_index(1), 2);
    assert_eq!(EvolutionType::TradeItem.to_index(1), -1);
    assert!(EvolutionType::LevelNightUltra.uses_level());
    assert!(!EvolutionType::Stone.uses_level());
}

#[test]
fn gen1_effectiveness_is_only_used_for_lists() {
    assert!(Effectiveness::against(Type::Fire, None, 1, false).is_none());
    assert_eq!(
        Effectiveness::not_very_effective(Type::Normal, 1, false),
        vec![Type::Rock, Type::Ghost]
    );
    assert_eq!(
        Effectiveness::super_effective(Type::Ghost, 1, false),
        vec![Type::Ghost]
    );
    assert_eq!(
        Effectiveness::super_effective(Type::Ghost, 1, true),
        vec![Type::Psychic, Type::Ghost]
    );
    let against =
        Effectiveness::against(Type::Grass, Some(Type::Poison), 3, false).expect("gen 3 table");
    let fire = against
        .iter()
        .find(|(kind, _)| *kind == Type::Fire)
        .expect("fire present");
    assert_eq!(fire.1, Effectiveness::Double);
    let grass = against
        .iter()
        .find(|(kind, _)| *kind == Type::Grass)
        .expect("grass present");
    assert_eq!(grass.1, Effectiveness::Quarter);
}

#[test]
fn moves_at_level_keeps_the_last_four() {
    let movepool: Vec<MoveLearnt> = [(1, 1), (2, 1), (3, 5), (1, 7), (4, 9), (5, 11), (6, 20)]
        .iter()
        .map(|&(move_number, level)| MoveLearnt { move_number, level })
        .collect();
    assert_eq!(moves_at_level(&movepool, 11, 0), [2, 3, 4, 5]);
    assert_eq!(moves_at_level(&movepool, 5, 0), [1, 2, 3, 0]);
}

#[test]
fn camel_case_follows_upstream_rules() {
    assert_eq!(camel_case("MR.MIME"), "Mr.Mime");
    assert_eq!(camel_case("FARFETCH'D"), "Farfetch'd");
    assert_eq!(camel_case("NIDORAN♂"), "Nidoran♂");
    assert!(java_equals_ignore_case("Pikachu", "PIKACHU"));
}

#[test]
fn gen1_allowed_items_ban_key_items_and_mark_tms() {
    let items: ItemList = gen1::allowed_items();
    assert!(items.is_allowed(gen1::TMS_START_INDEX));
    assert!(items.is_tm(gen1::TMS_START_INDEX));
    assert!(!items.is_allowed(gen1::HMS_START_INDEX));
    assert!(!items.is_allowed(0));
    assert!(!items.is_allowed(300));
    let mut random = JavaRandom::new(7);
    for _ in 0..50 {
        let item = items.random_non_tm(&mut random);
        assert!(items.is_allowed(item) && !items.is_tm(item));
    }
}

#[test]
fn pokemon_hash_matches_java() {
    assert_eq!(PokemonId::from_index(25).java_hash_code(), 56);
}

#[test]
fn shuffled_stats_copy_up_evolutions() {
    let mut data = GameData::new();
    let base = data.create_pokemon(StatLayout::Gen1, 1);
    let evolved = data.create_pokemon(StatLayout::Gen1, 2);
    let set = |data: &mut GameData, id, values: [i32; 5]| {
        let pk = data.pk_mut(id);
        pk.hp = values[0];
        pk.attack = values[1];
        pk.defense = values[2];
        pk.special = values[3];
        pk.speed = values[4];
    };
    set(&mut data, base, [10, 20, 30, 40, 50]);
    set(&mut data, evolved, [11, 21, 31, 41, 51]);
    let mut random = JavaRandom::new(3);
    data.pk_mut(base).shuffle_stats(&mut random);
    let order = data.pk(base).shuffled_stats_order.clone();
    data.pk_mut(evolved)
        .copy_shuffled_stats_up_evolution(&order);
    let base_stats = data.pk(base).stats();
    let evolved_stats = data.pk(evolved).stats();
    assert_eq!(base_stats.bst(), 150);
    assert_eq!(evolved_stats.bst(), 155);
    assert_eq!(evolved_stats.hp - base_stats.hp, 1);
    assert_eq!(evolved_stats.speed - base_stats.speed, 1);
}
