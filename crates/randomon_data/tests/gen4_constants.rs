use randomon_data::constants::{gen4, items, species};
use randomon_data::{MultiBattleStatus, Trainer, Type};

const FIRST_SPRITE: i32 = 297;

#[test]
fn overworld_sprites_convert_like_upstream() {
    let expected: Vec<i32> = include_str!("vectors/gen4_overworld_species.txt")
        .trim()
        .split(',')
        .map(|value| value.parse().expect("number"))
        .collect();
    for (offset, &species_id) in expected.iter().enumerate() {
        let sprite = FIRST_SPRITE + offset as i32;
        assert_eq!(
            gen4::convert_overworld_sprite_to_species(sprite),
            species_id,
            "sprite {sprite}"
        );
    }
}

#[test]
fn type_bytes_round_trip_and_mystery_is_nine() {
    for byte in 0..=0x11 {
        match gen4::type_table(byte) {
            Some(kind) => assert_eq!(i32::from(gen4::type_to_byte(Some(kind))), byte),
            None => assert_eq!(byte, 0x09),
        }
    }
    assert_eq!(gen4::type_to_byte(None), 0x09);
    assert_eq!(gen4::type_table(0x12), None);
    assert_eq!(gen4::type_to_byte(Some(Type::Fairy)), 0);
}

#[test]
fn formes_map_to_upstream_indices() {
    assert_eq!(gen4::FORME_MAPPINGS.len(), 12);
    assert_eq!(gen4::FORME_MAPPINGS[0].internal_index, 496);
    assert_eq!(gen4::FORME_MAPPINGS[11].internal_index, 507);
    assert_eq!(
        gen4::absolute_poke_num_by_base_forme(species::ROTOM, 3),
        503
    );
    assert_eq!(
        gen4::absolute_poke_num_by_base_forme(species::PIKACHU, 0),
        species::PIKACHU
    );
    assert_eq!(
        gen4::absolute_poke_num_by_base_forme(species::PIKACHU, 255),
        0
    );
    assert_eq!(gen4::forme_suffix_by_base_forme(species::GIRATINA, 1), "-O");
    assert_eq!(gen4::forme_suffix_by_base_forme(species::GIRATINA, 2), "");
    assert_eq!(gen4::forme_count(gen4::TYPE_DP), 5);
    assert_eq!(gen4::forme_count(gen4::TYPE_HGSS), 12);
}

#[test]
fn item_lists_match_upstream_setup() {
    let allowed = gen4::allowed_items();
    assert!(allowed.is_tm(items::TM01));
    assert!(allowed.is_tm(items::TM92));
    assert!(!allowed.is_allowed(items::HM01));
    assert!(!allowed.is_allowed(items::GRISEOUS_ORB));
    assert!(allowed.is_allowed(items::MASTER_BALL));
    let non_bad = gen4::non_bad_items();
    assert!(!non_bad.is_allowed(items::ORAN_BERRY));
    assert!(non_bad.is_allowed(items::SITRUS_BERRY));
    assert_eq!(gen4::regular_shop_items().len(), 19);
    assert_eq!(gen4::op_shop_items().first(), Some(&items::RARE_CANDY));
    assert_eq!(gen4::balanced_item_price(items::MASTER_BALL), Some(300));
    assert_eq!(gen4::balanced_item_price(items::ENIGMA_STONE + 1), None);
    assert_eq!(gen4::shop_names(gen4::TYPE_HGSS).len(), 59);
}

#[test]
fn trainer_tags_are_bounds_safe() {
    let mut few: Vec<Trainer> = (0..10).map(|_| Trainer::new()).collect();
    gen4::tag_trainers_hgss(&mut few);
    gen4::set_multi_battle_status_hgss(&mut few);
    assert_eq!(few[0].tag.as_deref(), Some("RIVAL2-2"));
    let mut many: Vec<Trainer> = (0..1000).map(|_| Trainer::new()).collect();
    gen4::tag_trainers_pt(&mut many);
    gen4::set_multi_battle_status_pt(&mut many);
    assert_eq!(many[0xf7].tag.as_deref(), Some("RIVAL2-0"));
    assert_eq!(many[0xf8].tag.as_deref(), Some("RIVAL2-1"));
    assert_eq!(many[0xf6].tag.as_deref(), Some("RIVAL2-2"));
    assert_eq!(many[0x264].tag.as_deref(), Some("FRIEND1-1"));
    assert_eq!(many[0x266].tag.as_deref(), Some("FRIEND1-0"));
    assert_eq!(many[920].multi_battle_status, MultiBattleStatus::Always);
    assert_eq!(many[33].multi_battle_status, MultiBattleStatus::Potential);
}

#[test]
fn dex_maps_have_upstream_lengths() {
    assert_eq!(gen4::DPPT_OVERWORLD_DEX_MAPS.len(), 183);
    assert_eq!(gen4::DPPT_DUNGEON_DEX_MAPS.len(), 183);
    assert_eq!(gen4::HGSS_OVERWORLD_DEX_MAPS.len(), 142);
    assert_eq!(gen4::HGSS_DUNGEON_DEX_MAPS.len(), 142);
    assert_eq!(gen4::HGSS_HEADBUTT_OVERWORLD_DEX_MAPS.len(), 59);
    assert_eq!(gen4::HGSS_HEADBUTT_DUNGEON_DEX_MAPS.len(), 59);
}
