use std::collections::BTreeMap;

use randomon_core::{HandlerRegistry, RomHandler, RomHandlerFactory};
use randomon_data::constants::species;
use randomon_data::{EvolutionType, MoveLearnt, PokemonId, Trainer, TrainerPokemon};
use randomon_gen3::synthetic::{self, SyntheticOptions};
use randomon_gen3::{Gen3Factory, items};
use randomon_java::RandomSource;
use randomon_settings::MiscTweak;

const SAMPLE_ENTRIES: &[&str] = &[
    "Ruby (U)",
    "Sapphire (E)",
    "Emerald (U)",
    "Fire Red (U) 1.0",
    "Fire Red (U) 1.1",
    "Leaf Green (U) 1.0",
    "Emerald (G)",
    "Ruby (J)",
    "Emerald (J)",
    "Fire Red (J)",
    "Emerald (T-Eng)",
];

fn load(bytes: Vec<u8>) -> Box<dyn RomHandler> {
    let source = RandomSource::with_seed(7).into_shared();
    Gen3Factory
        .create(bytes, "synthetic.gba", &source)
        .unwrap_or_else(|error| panic!("synthetic ROM loads: {error}"))
}

fn load_entry(name: &str) -> Box<dyn RomHandler> {
    load(synthetic::build(name))
}

fn reload(handler: &mut Box<dyn RomHandler>) -> Box<dyn RomHandler> {
    load(handler.save_rom().expect("ROM saves"))
}

fn pokemon(dex: i32) -> PokemonId {
    PokemonId::from_index(dex as usize)
}

fn number_of(handler: &dyn RomHandler, id: PokemonId) -> i32 {
    handler.core().data.pk(id).number
}

fn numbers(handler: &dyn RomHandler, ids: &[PokemonId]) -> Vec<i32> {
    ids.iter().map(|&id| number_of(handler, id)).collect()
}

fn is_japanese(name: &str) -> bool {
    name.contains("(J)") || name.contains("T-Eng")
}

fn is_frlg(name: &str) -> bool {
    name.starts_with("Fire Red") || name.starts_with("Leaf Green")
}

#[test]
fn every_entry_loads_from_a_synthetic_rom() {
    for name in synthetic::entry_names() {
        let bytes = synthetic::build(name);
        assert!(Gen3Factory.is_loadable(&bytes), "{name} is detected");
        let handler = load(bytes);
        assert_eq!(handler.rom_name(), name);
        assert_eq!(handler.generation_of_pokemon(), 3);
        assert_eq!(handler.default_extension(), "gba");
        let list = handler.get_pokemon();
        assert_eq!(list.len(), 387, "{name} has the full dex");
        assert!(list[0].is_none());
        for (number, id) in list.iter().enumerate().skip(1) {
            let id = id.expect("every slot is filled");
            let pk = handler.core().data.pk(id);
            assert_eq!(pk.number, number as i32);
            assert_eq!(
                pk.name,
                synthetic::pokemon_name(number as i32, is_japanese(name))
            );
        }
        let expected_valid = synthetic::expected_crc32(name) >= 0;
        assert_eq!(handler.is_rom_valid(), expected_valid, "{name} checksum");
    }
}

#[test]
fn registry_detects_generation_three_and_rejects_other_data() {
    let bytes = synthetic::build("Emerald (U)");
    let registry = HandlerRegistry::new().with(Gen3Factory);
    assert_eq!(registry.detect_generation(&bytes), Some(3));
    assert_eq!(registry.detect_generation(&[0u8; 4096]), None);
    let mut wrong_size = bytes.clone();
    wrong_size.truncate(bytes.len() - 1);
    assert_eq!(registry.detect_generation(&wrong_size), None);
    let mut unknown_code = bytes;
    unknown_code[0xAC..0xB0].copy_from_slice(b"ZZZZ");
    assert_eq!(registry.detect_generation(&unknown_code), None);
    let source = RandomSource::with_seed(1).into_shared();
    assert!(Gen3Factory.create(unknown_code, "x.gba", &source).is_err());
}

#[test]
fn unsaved_roms_round_trip_without_drift() {
    for name in SAMPLE_ENTRIES {
        let mut handler = load_entry(name);
        let first = handler.save_rom().expect("saves");
        let mut reloaded = load(first.clone());
        let second = reloaded.save_rom().expect("saves again");
        assert!(first == second, "{name} saves identically twice");
    }
}

#[test]
fn starters_follow_each_game_layout() {
    for name in SAMPLE_ENTRIES {
        let mut handler = load_entry(name);
        let expected = if is_frlg(name) {
            vec![1, 4, 7]
        } else {
            vec![252, 255, 258]
        };
        assert_eq!(numbers(handler.as_ref(), &handler.get_starters()), expected);
        let new_starters = [pokemon(species::DEOXYS), pokemon(25), pokemon(151)];
        assert!(handler.set_starters(&new_starters).expect("sets"));
        assert!(!handler.set_starters(&new_starters[..2]).expect("rejects"));
        handler.set_starter_held_items(&[items::LEFTOVERS]);
        let reloaded = reload(&mut handler);
        assert_eq!(
            numbers(reloaded.as_ref(), &reloaded.get_starters()),
            vec![species::DEOXYS, 25, 151],
            "{name}"
        );
        assert_eq!(reloaded.get_starter_held_items(), vec![items::LEFTOVERS]);
    }
}

#[test]
fn small_and_large_starter_items_use_the_right_encoding() {
    let mut handler = load_entry("Emerald (U)");
    handler.set_starter_held_items(&[items::POTION]);
    let reloaded = reload(&mut handler);
    assert_eq!(reloaded.get_starter_held_items(), vec![items::POTION]);
    let mut handler = reloaded;
    handler.set_starter_held_items(&[items::TM50]);
    let reloaded = reload(&mut handler);
    assert_eq!(reloaded.get_starter_held_items(), vec![items::TM50]);
}

#[test]
fn frlg_starter_texts_are_rewritten() {
    let mut handler = load_entry("Fire Red (U) 1.1");
    let starters = [pokemon(152), pokemon(155), pokemon(158)];
    handler.set_starters(&starters).expect("sets");
    let bytes = handler.save_rom().expect("saves");
    let first_name = synthetic::pokemon_name(152, false);
    let needle: Vec<u8> = synthetic::encode_english(&format!("{first_name} is your choice."));
    assert!(
        bytes
            .windows(needle.len())
            .any(|window| window == needle.as_slice())
    );
}

#[test]
fn pokemon_stats_and_names_round_trip() {
    for name in SAMPLE_ENTRIES {
        let mut handler = load_entry(name);
        for (index, id) in handler.get_pokemon().into_iter().flatten().enumerate() {
            let pk = handler.core_mut().data.pk_mut(id);
            pk.hp = 1 + (index % 200) as i32;
            pk.speed = 255;
            pk.ability2 = 0;
            pk.guaranteed_held_item = if index % 2 == 0 { items::LEFTOVERS } else { 0 };
            pk.common_held_item = items::ORAN_BERRY;
            pk.rare_held_item = items::NUGGET;
            pk.secondary_type = None;
            pk.name = if is_japanese(name) {
                "PXY".to_owned()
            } else {
                "RENAMED".to_owned()
            };
        }
        let reloaded = reload(&mut handler);
        for (index, id) in reloaded.get_pokemon().into_iter().flatten().enumerate() {
            let pk = reloaded.core().data.pk(id);
            assert_eq!(pk.hp, 1 + (index % 200) as i32, "{name}");
            assert_eq!(pk.speed, 255);
            assert_eq!(pk.ability2, pk.ability1, "ability2 mirrors ability1");
            assert!(pk.secondary_type.is_none());
            if index % 2 == 0 && pk.number != species::DEOXYS {
                assert_eq!(pk.guaranteed_held_item, items::LEFTOVERS);
            } else if pk.number != species::DEOXYS {
                assert_eq!(pk.common_held_item, items::ORAN_BERRY);
                assert_eq!(pk.rare_held_item, items::NUGGET);
            }
        }
    }
}

#[test]
fn deoxys_uses_hardcoded_stats_where_the_game_does() {
    let handler = load_entry("Emerald (U)");
    let deoxys = handler.core().data.pk(pokemon(species::DEOXYS));
    assert_eq!(
        [
            deoxys.hp,
            deoxys.attack,
            deoxys.defense,
            deoxys.speed,
            deoxys.spatk,
            deoxys.spdef
        ],
        [50, 180, 20, 150, 180, 20]
    );
    let mut handler = handler;
    handler
        .core_mut()
        .data
        .pk_mut(pokemon(species::DEOXYS))
        .attack = 99;
    let reloaded = reload(&mut handler);
    assert_eq!(reloaded.core().data.pk(pokemon(species::DEOXYS)).attack, 99);
}

#[test]
fn evolutions_are_read_with_split_rules() {
    let handler = load_entry("Emerald (U)");
    let data = &handler.core().data;
    let eevee = data.evolutions_from(pokemon(species::EEVEE));
    assert_eq!(eevee.len(), 5);
    assert!(eevee.iter().all(|&evo| !data.evo(evo).carry_stats));
    let nincada = data.evolutions_from(pokemon(species::NINCADA));
    let ninjask = nincada
        .iter()
        .find(|&&evo| data.evo(evo).kind == EvolutionType::LevelCreateExtra)
        .expect("ninjask evolution");
    assert!(data.evo(*ninjask).carry_stats);
    assert_eq!(data.evolutions_from(pokemon(1)).len(), 1);
    assert_eq!(data.evo(data.evolutions_from(pokemon(1))[0]).to, pokemon(2));
}

#[test]
fn impossible_and_time_based_evolutions_are_replaced() {
    let mut handler = load_entry("Fire Red (U) 1.1");
    let settings = randomon_settings::Settings::default();
    handler
        .remove_impossible_evolutions(&settings)
        .expect("removes");
    handler.remove_time_based_evolutions().expect("removes");
    assert!(!handler.get_impossible_evo_updates().is_empty());
    let reloaded = reload(&mut handler);
    let data = &reloaded.core().data;
    let kinds = |dex: i32| -> Vec<(EvolutionType, i32)> {
        data.evolutions_from(pokemon(dex))
            .iter()
            .map(|&evo| (data.evo(evo).kind, data.evo(evo).extra_info))
            .collect()
    };
    assert!(kinds(species::HAUNTER).contains(&(EvolutionType::Level, 37)));
    assert!(kinds(species::SLOWPOKE).contains(&(EvolutionType::Stone, items::WATER_STONE)));
    assert!(kinds(species::SEADRA).contains(&(EvolutionType::Level, 40)));
    assert!(kinds(species::CLAMPERL).contains(&(EvolutionType::Level, 30)));
    assert!(kinds(species::CLAMPERL).contains(&(EvolutionType::Stone, items::WATER_STONE)));
    assert!(kinds(species::FEEBAS).contains(&(EvolutionType::Level, 35)));
    assert!(kinds(species::EEVEE).contains(&(EvolutionType::Stone, items::SUN_STONE)));
    assert!(kinds(species::EEVEE).contains(&(EvolutionType::Stone, items::MOON_STONE)));
}

#[test]
fn movesets_grow_into_free_space_and_shrink_in_place() {
    for name in ["Ruby (U)", "Emerald (U)", "Fire Red (U) 1.1"] {
        let mut handler = load_entry(name);
        let mut movesets = handler.get_moves_learnt();
        assert_eq!(movesets[&1][0].move_number, 0x150, "high move bit decodes");
        let long: Vec<MoveLearnt> = (1..=20)
            .map(|level| MoveLearnt {
                move_number: 300 + level,
                level,
            })
            .collect();
        movesets.insert(1, long.clone());
        movesets.insert(
            4,
            vec![MoveLearnt {
                move_number: 33,
                level: 1,
            }],
        );
        handler.set_moves_learnt(&movesets).expect("writes");
        let reloaded = reload(&mut handler);
        let read = reloaded.get_moves_learnt();
        assert_eq!(read[&1], long, "{name}");
        assert_eq!(
            read[&4],
            vec![MoveLearnt {
                move_number: 33,
                level: 1
            }]
        );
        assert_eq!(read[&7], movesets[&7]);
    }
}

#[test]
fn egg_moves_round_trip() {
    let mut handler = load_entry("Sapphire (E)");
    let mut egg_moves = handler.get_egg_moves();
    assert!(!egg_moves.is_empty());
    for moves in egg_moves.values_mut() {
        for mv in moves.iter_mut() {
            *mv = 100;
        }
    }
    handler.set_egg_moves(&egg_moves).expect("writes");
    let reloaded = reload(&mut handler);
    assert_eq!(reloaded.get_egg_moves(), egg_moves);
}

fn full_trainer(template: &Trainer, pokemon_number: i32) -> Trainer {
    let mut trainer = template.clone();
    trainer.poketype = 3;
    trainer.pokemon = (0..6)
        .map(|slot| {
            let mut tp = TrainerPokemon::new(pokemon(pokemon_number + slot), 50 + slot);
            tp.held_item = items::SITRUS_BERRY;
            tp.moves = [1, 2, 3, 4];
            tp.ivs = 31;
            tp
        })
        .collect();
    trainer
}

#[test]
fn trainers_grow_teams_into_free_space() {
    for name in SAMPLE_ENTRIES {
        let mut handler = load_entry(name);
        let mut trainers = handler.get_trainers().expect("reads");
        let regular = trainers.len();
        let template = trainers[0].clone();
        trainers[0] = full_trainer(&template, 10);
        trainers[1].pokemon.truncate(1);
        trainers[1].poketype = 0;
        handler.set_trainers(&trainers, true).expect("writes");
        let reloaded = reload(&mut handler);
        let read = reloaded.get_trainers().expect("reads again");
        assert_eq!(read.len(), regular, "{name}");
        assert_eq!(read[0].pokemon.len(), 6);
        assert_eq!(read[0].poketype, 3);
        for (slot, tp) in read[0].pokemon.iter().enumerate() {
            assert_eq!(number_of(reloaded.as_ref(), tp.pokemon), 10 + slot as i32);
            assert_eq!(tp.level, 50 + slot as i32);
            assert_eq!(tp.held_item, items::SITRUS_BERRY);
            assert_eq!(tp.moves, [1, 2, 3, 4]);
            assert_eq!(tp.ivs, 31);
        }
        assert_eq!(read[1].pokemon.len(), 1);
    }
}

#[test]
fn trainer_tags_and_multi_battles_are_assigned() {
    let emerald = load_entry("Emerald (U)");
    let trainers = emerald.get_trainers().expect("reads");
    let count = trainers.len();
    let steven = &trainers[count - 1];
    assert_eq!(steven.pokemon.len(), 3);
    assert_eq!(steven.poketype, 1);
    assert_eq!(trainers[803].tag.as_deref(), Some("UBER"));
    assert_eq!(trainers[0x109 - 1].tag.as_deref(), Some("GYM1-LEADER"));
    assert_eq!(
        trainers[24].multi_battle_status,
        randomon_data::MultiBattleStatus::Always
    );
    let fire_red = load_entry("Fire Red (U) 1.1");
    let trainers = fire_red.get_trainers().expect("reads");
    assert_eq!(trainers[0x19E - 1].tag.as_deref(), Some("GYM1-LEADER"));
    assert_eq!(trainers[0x148 - 1].tag.as_deref(), Some("RIVAL1-0"));
    assert!(fire_red.has_rival_final_battle());
}

#[test]
fn trainer_names_and_classes_round_trip() {
    let mut handler = load_entry("Ruby (U)");
    let names: Vec<String> = handler
        .get_trainer_names()
        .iter()
        .map(|_| "NEWNAME".to_owned())
        .collect();
    handler.set_trainer_names(&names);
    let classes: Vec<String> = handler
        .get_trainer_class_names()
        .iter()
        .map(|_| "COOL".to_owned())
        .collect();
    handler.set_trainer_class_names(&classes);
    let reloaded = reload(&mut handler);
    assert_eq!(reloaded.get_trainer_names(), names);
    assert_eq!(reloaded.get_trainer_class_names(), classes);
    assert_eq!(reloaded.max_trainer_name_length(), 11);
}

#[test]
fn wild_encounters_are_deduplicated_and_round_trip() {
    for name in SAMPLE_ENTRIES {
        let mut handler = load_entry(name);
        let mut sets = handler.get_encounters(false);
        assert_eq!(sets.len(), 84, "{name} keeps one copy of shared areas");
        assert!(sets[0].display_name_contains(" Grass/Cave"));
        for set in &mut sets {
            for encounter in &mut set.encounters {
                encounter.pokemon = pokemon(150);
                encounter.level = 77;
                encounter.max_level = 80;
            }
        }
        handler.set_encounters(false, &sets).expect("writes");
        let reloaded = reload(&mut handler);
        for set in reloaded.get_encounters(false) {
            for encounter in set.encounters {
                assert_eq!(number_of(reloaded.as_ref(), encounter.pokemon), 150);
                assert_eq!((encounter.level, encounter.max_level), (77, 80));
            }
        }
    }
}

#[test]
fn frlg_bans_trappers_and_unown_from_the_wild() {
    let handler = load_entry("Leaf Green (U) 1.0");
    let sets = handler.get_encounters(false);
    let trappers: Vec<PokemonId> = handler
        .get_pokemon()
        .into_iter()
        .flatten()
        .filter(|&id| {
            let pk = handler.core().data.pk(id);
            [23, 42, 71].contains(&pk.ability1) || [23, 42, 71].contains(&pk.ability2)
        })
        .collect();
    assert!(!trappers.is_empty());
    for set in &sets[55..60] {
        for &trapper in &trappers {
            assert!(set.is_banned(trapper));
        }
    }
    assert!(!sets[0].is_banned(trappers[0]));
    assert_eq!(
        handler.banned_for_wild_encounters(),
        vec![pokemon(species::UNOWN)]
    );
}

fn expected_static_count(name: &str) -> usize {
    let base = synthetic::static_entry_count(name);
    let hardcoded = usize::from(synthetic::has_tweak(name, "StaticFirstBattleTweak"))
        .max(usize::from(synthetic::has_tweak(name, "GhostMarowakTweak")));
    let roamers = synthetic::roamer_count(name);
    let roamer_statics = if name.starts_with("Emerald") {
        0
    } else if is_frlg(name) {
        if synthetic::has_tweak(name, "RoamingPokemonTweak") {
            roamers
        } else {
            0
        }
    } else {
        roamers.min(1)
    };
    base + hardcoded + roamer_statics
}

#[test]
fn statics_roamers_and_hardcoded_battles_round_trip() {
    for name in SAMPLE_ENTRIES {
        let mut handler = load_entry(name);
        if !handler.can_change_static_pokemon() {
            continue;
        }
        let mut statics = handler.get_static_pokemon();
        assert_eq!(statics.len(), expected_static_count(name), "{name}");
        for (index, encounter) in statics.iter_mut().enumerate() {
            encounter.pkmn = pokemon(100 + index as i32);
            encounter.level = 33;
            for linked in &mut encounter.linked_encounters {
                linked.pkmn = pokemon(100 + index as i32);
                linked.level = 33;
            }
        }
        assert!(
            handler.set_static_pokemon(&statics).expect("writes"),
            "{name}"
        );
        assert!(!handler.set_static_pokemon(&statics[1..]).expect("rejects"));
        let reloaded = reload(&mut handler);
        let read = reloaded.get_static_pokemon();
        assert_eq!(read.len(), statics.len());
        for (index, encounter) in read.iter().enumerate() {
            assert_eq!(
                number_of(reloaded.as_ref(), encounter.pkmn),
                100 + index as i32,
                "{name} static {index}"
            );
            if !encounter.is_egg
                && index < synthetic::static_entry_count(name)
                && synthetic::static_has_level(name, index)
            {
                assert_eq!(encounter.level, 33, "{name} static {index} level");
            }
        }
    }
}

#[test]
fn emerald_roamers_follow_their_southern_island_partner() {
    let handler = load_entry("Emerald (U)");
    let statics = handler.get_static_pokemon();
    let linked: Vec<i32> = statics
        .iter()
        .flat_map(|encounter| &encounter.linked_encounters)
        .map(|encounter| number_of(handler.as_ref(), encounter.pkmn))
        .collect();
    assert_eq!(linked, vec![species::LATIOS, species::LATIAS]);
}

#[test]
fn patched_roamers_read_their_hardcoded_species() {
    let ruby = load_entry("Ruby (U)");
    let statics = ruby.get_static_pokemon();
    let roamer = statics.last().expect("ruby has a roamer");
    assert_eq!(number_of(ruby.as_ref(), roamer.pkmn), species::LATIOS);
    let fire_red = load_entry("Fire Red (U) 1.1");
    let statics = fire_red.get_static_pokemon();
    let roamers: Vec<i32> = statics[statics.len() - 3..]
        .iter()
        .map(|encounter| number_of(fire_red.as_ref(), encounter.pkmn))
        .collect();
    assert_eq!(
        roamers,
        vec![species::RAIKOU, species::ENTEI, species::SUICUNE]
    );
}

#[test]
fn static_music_fix_rewrites_the_pool() {
    let mut handler = load_entry("Emerald (U)");
    assert!(handler.has_static_music_fix());
    let mut changes = BTreeMap::new();
    changes.insert(species::KYOGRE, 25);
    changes.insert(species::DEOXYS, 26);
    changes.insert(999, 1);
    handler.apply_correct_static_music(&changes);
    let bytes = handler.save_rom().expect("saves");
    let pool = 0xFE014C;
    let word = |offset: usize| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    assert_eq!(word(pool + 4 * 3), 25);
    assert_eq!(word(pool + 4 * 6), 26);
    assert_eq!(word(pool), 151);
}

#[test]
fn tm_moves_update_tables_texts_and_descriptions() {
    for name in ["Ruby (U)", "Emerald (U)", "Fire Red (U) 1.1", "Ruby (J)"] {
        let mut handler = load_entry(name);
        let tms: Vec<i32> = (0..50).map(|index| 200 + index).collect();
        handler.set_tm_moves(&tms).expect("writes");
        let reloaded = reload(&mut handler);
        assert_eq!(reloaded.get_tm_moves(), tms, "{name}");
        assert_eq!(reloaded.get_tm_count(), 50);
        assert_eq!(reloaded.get_hm_moves().len(), 8);
    }
    let mut handler = load_entry("Ruby (U)");
    let tms: Vec<i32> = (0..50).map(|index| 200 + index).collect();
    handler.set_tm_moves(&tms).expect("writes");
    let bytes = handler.save_rom().expect("saves");
    let move_name = synthetic::encode_english("contains\\nMOVE 202.");
    assert!(
        bytes
            .windows(move_name.len())
            .any(|window| window == move_name.as_slice()),
        "TM03 text names its new move"
    );
}

#[test]
fn move_tutors_round_trip_on_emerald_and_frlg() {
    for name in ["Emerald (U)", "Fire Red (U) 1.1", "Ruby (U)"] {
        let mut handler = load_entry(name);
        let tutors = handler.get_move_tutor_moves();
        if name.starts_with("Ruby") {
            assert!(!handler.has_move_tutors());
            assert!(tutors.is_empty());
            continue;
        }
        let replaced: Vec<i32> = tutors.iter().map(|_| 99).collect();
        handler.set_move_tutor_moves(&replaced).expect("writes");
        let mut compatibility = handler.get_move_tutor_compatibility();
        for flags in compatibility.values_mut() {
            flags.iter_mut().for_each(|flag| *flag = true);
        }
        handler.set_move_tutor_compatibility(&compatibility);
        let reloaded = reload(&mut handler);
        assert_eq!(reloaded.get_move_tutor_moves(), replaced, "{name}");
        let read = reloaded.get_move_tutor_compatibility();
        assert!(
            read.values()
                .all(|flags| flags.iter().skip(1).all(|&flag| flag))
        );
    }
}

#[test]
fn tm_compatibility_round_trips() {
    let mut handler = load_entry("Sapphire (E)");
    let mut compatibility = handler.get_tmhm_compatibility();
    for (index, flags) in compatibility.values_mut().enumerate() {
        for (slot, flag) in flags.iter_mut().enumerate().skip(1) {
            *flag = (slot + index) % 3 == 0;
        }
    }
    handler.set_tmhm_compatibility(&compatibility);
    let reloaded = reload(&mut handler);
    assert_eq!(reloaded.get_tmhm_compatibility(), compatibility);
}

#[test]
fn field_items_are_found_and_replaced() {
    for name in ["Ruby (U)", "Emerald (U)", "Leaf Green (U) 1.0"] {
        let mut handler = load_entry(name);
        let tms = handler.get_current_field_tms();
        assert!(tms.contains(&4), "{name} has the TM04 item ball");
        assert_eq!(tms.iter().filter(|&&tm| tm == 4).count(), 1);
        let regular = handler.get_regular_field_items();
        assert!(regular.contains(&items::RARE_CANDY));
        assert!(!regular.contains(&items::MACH_BIKE));
        let new_tms: Vec<i32> = tms.iter().map(|tm| 51 - tm).collect();
        handler.set_field_tms(&new_tms).expect("writes");
        let new_items: Vec<i32> = regular.iter().map(|_| items::NUGGET).collect();
        handler.set_regular_field_items(&new_items).expect("writes");
        let reloaded = reload(&mut handler);
        assert_eq!(reloaded.get_current_field_tms(), new_tms);
        assert!(
            reloaded
                .get_regular_field_items()
                .iter()
                .all(|&item| item == items::NUGGET)
        );
    }
}

#[test]
fn shops_prices_pickups_and_trades_round_trip() {
    for name in ["Ruby (U)", "Emerald (U)", "Fire Red (U) 1.1"] {
        let mut handler = load_entry(name);
        let mut shops = handler.get_shop_items().expect("has shops");
        assert!(!shops.is_empty());
        for shop in shops.values_mut() {
            shop.items
                .iter_mut()
                .for_each(|item| *item = items::ULTRA_BALL);
        }
        handler.set_shop_items(&shops).expect("writes");
        handler.set_shop_prices();
        let mut pickups = handler.get_pickup_items();
        assert!(!pickups.is_empty(), "{name} pickup table");
        for range in 0..10 {
            let total: i32 = pickups.iter().map(|item| item.probabilities[range]).sum();
            assert_eq!(total, 100);
        }
        pickups
            .iter_mut()
            .for_each(|item| item.item = items::NUGGET);
        handler.set_pickup_items(&pickups);
        let mut trades = handler.get_ingame_trades();
        assert!(!trades.is_empty());
        for trade in &mut trades {
            trade.nickname = "BUDDY".to_owned();
            trade.ot_name = "PAL".to_owned();
            trade.given_pokemon = pokemon(133);
            trade.requested_pokemon = Some(pokemon(134));
            trade.ivs = vec![31; 6];
            trade.item = items::LEFTOVERS;
        }
        handler.set_ingame_trades(&trades).expect("writes");
        let reloaded = reload(&mut handler);
        let read_shops = reloaded.get_shop_items().expect("has shops");
        assert!(
            read_shops
                .values()
                .all(|shop| shop.items.iter().all(|&item| item == items::ULTRA_BALL))
        );
        assert!(
            reloaded
                .get_pickup_items()
                .iter()
                .all(|item| item.item == items::NUGGET)
        );
        for trade in reloaded.get_ingame_trades() {
            assert_eq!(trade.nickname, "BUDDY");
            assert_eq!(trade.ot_name, "PAL");
            assert_eq!(number_of(reloaded.as_ref(), trade.given_pokemon), 133);
            assert_eq!(
                trade
                    .requested_pokemon
                    .map(|id| number_of(reloaded.as_ref(), id)),
                Some(134)
            );
            assert_eq!(trade.ivs, vec![31; 6]);
            assert_eq!(trade.item, items::LEFTOVERS);
        }
    }
}

#[test]
fn moves_round_trip_with_clamped_accuracy() {
    let mut handler = load_entry("Emerald (U)");
    for number in 1..=354 {
        if let Some(mv) = handler
            .core_mut()
            .data
            .move_ids()
            .get(number)
            .copied()
            .flatten()
        {
            let mv = handler.core_mut().data.mv_mut(mv);
            mv.power = 77;
            mv.hitratio = 140.0;
            mv.pp = 9;
        }
    }
    let reloaded = reload(&mut handler);
    let mv = reloaded
        .core()
        .data
        .move_by_number(10)
        .expect("move exists");
    assert_eq!((mv.power, mv.hitratio, mv.pp), (77, 100.0, 9));
}

#[test]
fn every_misc_tweak_applies_without_errors() {
    for name in SAMPLE_ENTRIES {
        let mut handler = load_entry(name);
        let available = handler.misc_tweaks_available();
        handler.set_pokemon_pool(None);
        for tweak in MiscTweak::ALL {
            if tweak.is_set_in(available) {
                handler.apply_misc_tweak(*tweak).expect("tweak applies");
            }
        }
        handler.randomize_intro_pokemon().expect("intro randomizes");
        handler.enable_guaranteed_pokemon_catching();
        handler
            .make_evolutions_easier(&randomon_settings::Settings::default())
            .expect("easier");
        let mut reloaded = reload(&mut handler);
        assert!(reloaded.save_rom().is_ok(), "{name}");
        if MiscTweak::UpdateTypeEffectiveness.is_set_in(available) {
            assert!(handler.is_effectiveness_updated());
        }
    }
}

#[test]
fn national_dex_patches_report_success() {
    for name in ["Ruby (U)", "Emerald (U)", "Fire Red (U) 1.1"] {
        let mut handler = load_entry(name);
        handler.core_mut().engine.handler_logging = true;
        handler
            .apply_misc_tweak(MiscTweak::NationalDexAtStart)
            .expect("applies");
        let log = handler.core().engine.log.as_str().to_owned();
        assert!(log.contains("Patch successful!"), "{name}: {log}");
    }
}

#[test]
fn fire_red_10_hack_detection_reads_hack_pointers() {
    let options = SyntheticOptions {
        forge_crc: false,
        hack_pointers: true,
        jambo_movesets: false,
    };
    let handler = load(synthetic::build_with("Fire Red (U) 1.0", options));
    assert!(!handler.is_rom_valid());
    assert_eq!(handler.get_pokemon().into_iter().flatten().count(), 386);
    assert!(!handler.get_trainers().expect("reads").is_empty());
    assert_eq!(handler.get_moves_learnt()[&1][0].move_number, 0x150);
    let jambo = SyntheticOptions {
        jambo_movesets: true,
        ..options
    };
    let mut handler = load(synthetic::build_with("Fire Red (U) 1.0", jambo));
    let mut movesets = handler.get_moves_learnt();
    assert_eq!(movesets[&1][0].move_number, 0x150);
    movesets.insert(
        2,
        (1..=15)
            .map(|level| MoveLearnt {
                move_number: 340 + level,
                level,
            })
            .collect(),
    );
    handler.set_moves_learnt(&movesets).expect("writes");
    let bytes = handler.save_rom().expect("saves");
    let reloaded = load(bytes);
    assert_eq!(reloaded.get_moves_learnt()[&2], movesets[&2]);
}

#[test]
fn unofficial_emerald_translation_is_normalised() {
    let mut handler = load_entry("Emerald (T-Eng)");
    assert_eq!(handler.rom_code(), "BPET");
    let bytes = handler.save_rom().expect("saves");
    assert_eq!(&bytes[0xAC..0xB0], b"BPET");
    assert_eq!(bytes[0xBD], 0x66);
}

#[test]
fn support_levels_follow_static_support() {
    for name in SAMPLE_ENTRIES {
        let handler = load_entry(name);
        let expected = if handler.can_change_static_pokemon() {
            "Complete"
        } else {
            "No Static Pokemon"
        };
        assert_eq!(handler.support_level(), expected, "{name}");
    }
    assert_eq!(load_entry("Ruby (U)").support_level(), "Complete");
}
