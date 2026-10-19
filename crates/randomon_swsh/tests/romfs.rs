use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use randomon_core::{
    HandlerRegistry, RandomizationOptions, RomHandler, RomOutput, SwitchLayout,
    write_directory_output,
};
use randomon_java::RandomSource;
use randomon_settings::{
    AbilitiesMod, BaseStatisticsMod, EvolutionsMod, ExpCurve, FieldItemsMod, InGameTradesMod,
    MiscTweak, MoveTutorMovesMod, MoveTutorsCompatibilityMod, MovesetsMod, Settings, ShopItemsMod,
    StartersMod, StaticPokemonMod, TmsHmsCompatibilityMod, TmsMod, TrainersMod, TypesMod,
    WildPokemonMod,
};
use randomon_swsh::formats::items::pouch;
use randomon_swsh::formats::personal::PersonalTable;
use randomon_swsh::{GameData, GameFiles, SwshFactory, SwshRomHandler};

const SEEDS: [i64; 3] = [1, 424_242, 9_876_543_210];
const SHIELD_TITLE: &str = "01008DB008C2C000";
const SWORD_TITLE: &str = "0100ABF008968000";

fn romfs() -> Option<PathBuf> {
    std::env::var_os("RANDOMON_SWSH_ROMFS").map(PathBuf::from)
}

fn everything_on() -> Settings {
    Settings {
        rom_name: Some(randomon_swsh::ROM_NAME.to_owned()),
        selected_exp_curve: Some(ExpCurve::MediumFast),
        abilities_mod: AbilitiesMod::Randomize,
        starters_mod: StartersMod::RandomWithTwoEvolutions,
        allow_starter_alt_formes: true,
        types_mod: TypesMod::RandomFollowEvolutions,
        base_statistics_mod: BaseStatisticsMod::Shuffle,
        movesets_mod: MovesetsMod::RandomPreferSameType,
        start_with_guaranteed_moves: true,
        guaranteed_move_count: 4,
        evolution_moves_for_all: true,
        trainers_mod: TrainersMod::Random,
        rival_carries_starter_throughout: true,
        allow_trainer_alternate_formes: true,
        randomize_trainer_names: true,
        randomize_trainer_class_names: true,
        randomize_held_items_for_regular_trainer_pokemon: true,
        randomize_held_items_for_important_trainer_pokemon: true,
        randomize_held_items_for_boss_trainer_pokemon: true,
        randomize_starters_held_items: true,
        additional_regular_trainer_pokemon: 2,
        trainers_force_fully_evolved: true,
        trainers_force_fully_evolved_level: 40,
        double_battle_mode: true,
        wild_pokemon_mod: WildPokemonMod::AreaMapping,
        allow_wild_alt_formes: true,
        randomize_wild_pokemon_held_items: true,
        static_pokemon_mod: StaticPokemonMod::RandomMatching,
        allow_static_alt_formes: true,
        in_game_trades_mod: InGameTradesMod::RandomizeGivenAndRequested,
        randomize_in_game_trades_items: true,
        field_items_mod: FieldItemsMod::Random,
        shop_items_mod: ShopItemsMod::Random,
        balance_shop_prices: true,
        tms_mod: TmsMod::Random,
        tms_hms_compatibility_mod: TmsHmsCompatibilityMod::RandomPreferType,
        move_tutor_moves_mod: MoveTutorMovesMod::Random,
        move_tutors_compatibility_mod: MoveTutorsCompatibilityMod::RandomPreferType,
        evolutions_mod: EvolutionsMod::Random,
        evos_allow_alt_formes: true,
        change_impossible_evolutions: true,
        make_evolutions_easier: true,
        remove_time_based_evolutions: true,
        randomize_move_powers: true,
        randomize_move_types: true,
        randomize_move_accuracies: true,
        randomize_move_pps: true,
        current_misc_tweaks: MiscTweak::BanLuckyEgg.value(),
        ..Settings::default()
    }
}

fn load(path: &Path) -> Box<dyn RomHandler> {
    let source = RandomSource::with_seed(0).into_shared();
    HandlerRegistry::new()
        .with(SwshFactory)
        .load_file(path, "romfs", &source)
        .expect("loads the romfs")
}

fn usable_move(original: &GameData, move_number: i32) -> bool {
    usize::try_from(move_number)
        .ok()
        .and_then(|index| original.moves.get(index))
        .is_some_and(|record| {
            record.can_use_move && move_number != 743 && !(757..=774).contains(&move_number)
        })
}

struct Checker<'a> {
    personal: &'a PersonalTable,
    species_count: i32,
    original: &'a GameData,
}

impl Checker<'_> {
    fn placeable(&self, species: i32, form: i32, context: &str) {
        assert!(
            (1..=self.species_count).contains(&species),
            "{context}: species {species} out of range"
        );
        let entry = self.personal.get(species as usize).expect("personal entry");
        assert!(
            entry.is_present_in_game,
            "{context}: species {species} is not present in Galar"
        );
        if form != 0 {
            let index = entry
                .alternate_form_index(form as u16)
                .unwrap_or_else(|| panic!("{context}: species {species} has no form {form}"));
            let form_entry = self.personal.get(index).expect("form entry");
            assert!(
                form_entry.is_present_in_game,
                "{context}: species {species} form {form} is not present"
            );
        }
    }

    fn moves(&self, moves: &[i32], context: &str) {
        for &move_number in moves {
            assert!(
                move_number == 0 || usable_move(self.original, move_number),
                "{context}: move {move_number} is unusable"
            );
        }
    }

    fn can_dynamax(&self, species: i32, form: i32, context: &str) {
        let entry = self
            .personal
            .form_entry(species as u16, form as u16)
            .expect("personal entry");
        assert!(
            !entry.cannot_dynamax(),
            "{context}: species {species} form {form} cannot dynamax"
        );
    }

    fn level(&self, level: i32, context: &str) {
        assert!((1..=100).contains(&level), "{context}: level {level}");
    }
}

fn assert_game_is_consistent(files: &GameFiles, original: &GameData, species_count: i32) {
    let data = &files.data;
    let checker = Checker {
        personal: &data.personal,
        species_count,
        original,
    };
    for (index, trainer) in data.trainers.iter().enumerate().skip(1) {
        if trainer.team == original.trainers[index].team {
            continue;
        }
        assert!(trainer.team.len() <= 6);
        for member in &trainer.team {
            let context = format!("trainer {index}");
            checker.placeable(i32::from(member.species), i32::from(member.form), &context);
            checker.level(i32::from(member.level), &context);
            checker.moves(&member.moves.map(i32::from), &context);
            if member.can_dynamax {
                checker.can_dynamax(i32::from(member.species), i32::from(member.form), &context);
            }
        }
    }
    for (name, archive) in data.wild.by_file() {
        for table in &archive.tables {
            for sub_table in &table.sub_tables {
                for slot in sub_table.slots.iter().filter(|slot| slot.species != 0) {
                    checker.placeable(slot.species, i32::from(slot.form), name);
                }
                if !sub_table.is_empty() {
                    checker.level(i32::from(sub_table.level_min), name);
                    checker.level(i32::from(sub_table.level_max), name);
                }
            }
        }
    }
    for (index, entry) in data.statics.encounters.iter().enumerate() {
        if *entry == original.statics.encounters[index] {
            continue;
        }
        let context = format!("static {index}");
        checker.placeable(entry.species, i32::from(entry.form), &context);
        checker.level(i32::from(entry.level), &context);
        checker.moves(&entry.moves(), &context);
        assert!(
            !entry.can_gigantamax || entry.species == original.statics.encounters[index].species
        );
    }
    for (index, gift) in data.gifts.gifts.iter().enumerate() {
        let context = format!("gift {index}");
        checker.placeable(gift.species, i32::from(gift.form), &context);
        checker.level(i32::from(gift.level), &context);
    }
    for (index, trade) in data.trades.trades.iter().enumerate() {
        let context = format!("trade {index}");
        checker.placeable(trade.species, i32::from(trade.form), &context);
        checker.placeable(
            trade.required_species,
            i32::from(trade.required_form),
            &context,
        );
    }
    for (table, before) in data.nests.tables.iter().zip(&original.nests.tables) {
        for (entry, previous) in table.entries.iter().zip(&before.entries) {
            checker.placeable(entry.species, entry.form, "raid");
            if entry.species != previous.species {
                checker.can_dynamax(entry.species, entry.form, "raid");
                assert!(
                    !entry.is_gigantamax,
                    "raid {} kept a gigantamax flag",
                    entry.species
                );
            }
        }
    }
    if let Some(underground) = &data.underground {
        for entry in &underground.encounters {
            checker.placeable(entry.species, i32::from(entry.form), "dynamax adventure");
            checker.moves(
                &entry.moves().map(|move_number| move_number as i32),
                "dynamax adventure",
            );
        }
    }
    for (number, set) in data.evolutions.iter().enumerate() {
        if *set == original.evolutions[number] {
            continue;
        }
        for evolution in set.active() {
            let form = i32::from(evolution.form.max(0));
            checker.placeable(
                i32::from(evolution.species),
                form,
                &format!("evolution {number}"),
            );
        }
    }
    for (number, learnset) in data.learnsets.iter().enumerate() {
        if *learnset == original.learnsets[number] {
            continue;
        }
        let moves: Vec<i32> = learnset
            .moves
            .iter()
            .map(|entry| i32::from(entry.move_id))
            .collect();
        checker.moves(&moves, &format!("learnset {number}"));
        assert!(learnset.moves.iter().all(|entry| entry.level <= 100));
    }
    for (number, egg_moves) in data.egg_moves.iter().enumerate() {
        if *egg_moves == original.egg_moves[number] {
            continue;
        }
        let moves: Vec<i32> = egg_moves
            .moves
            .iter()
            .map(|&move_id| i32::from(move_id))
            .collect();
        checker.moves(&moves, &format!("egg moves {number}"));
    }
    let machines: Vec<i32> = data
        .items
        .tm_moves()
        .into_iter()
        .chain(data.items.tr_moves())
        .map(i32::from)
        .collect();
    checker.moves(&machines, "machines");
    assert_eq!(data.items.records[0], original.items.records[0]);
    assert_eq!(data.items.item_records, original.items.item_records);
    let shop_items = data
        .shops
        .single
        .iter()
        .flat_map(|shop| shop.items.iter())
        .chain(
            data.shops
                .multi
                .iter()
                .flat_map(|shop| shop.inventories.iter().flatten()),
        );
    let original_shop_items = original
        .shops
        .single
        .iter()
        .flat_map(|shop| shop.items.iter())
        .chain(
            original
                .shops
                .multi
                .iter()
                .flat_map(|shop| shop.inventories.iter().flatten()),
        );
    for (&item, &before) in shop_items.zip(original_shop_items) {
        if item == before {
            continue;
        }
        let item_pouch = data.items.pouch(item as u16);
        assert_ne!(item_pouch, Some(pouch::MACHINES), "shop item {item}");
        assert_ne!(item_pouch, Some(pouch::KEY), "shop item {item}");
        assert!(
            !(1279..=1578).contains(&item),
            "shop item {item} is a dynamax crystal"
        );
        assert_eq!(
            item_pouch,
            original.items.pouch(before as u16),
            "shop item {item}"
        );
    }
    for area in &data.placement {
        for zone in &area.archive.zones {
            for item in &zone.field_items {
                if let Some(&hash) = item.item_hashes.first() {
                    assert!(
                        hash == randomon_swsh::formats::hash::EMPTY_HASH
                            || data.item_hashes.item_of(hash).is_some()
                            || original
                                .placement
                                .iter()
                                .flat_map(|area| area.archive.zones.iter())
                                .flat_map(|zone| zone.field_items.iter())
                                .any(|before| before.item_hashes.first() == Some(&hash))
                    );
                }
            }
        }
    }
}

#[test]
fn the_romfs_exposes_the_full_catalog() {
    let Some(romfs) = romfs() else {
        return;
    };
    let handler = load(&romfs);
    let data = &handler.core().data;
    let personal = PersonalTable::parse(
        &std::fs::read(romfs.join(randomon_swsh::paths::PERSONAL)).expect("personal"),
    )
    .expect("personal");
    let pokemon = handler.get_pokemon();
    assert!(pokemon.len() == 891 || pokemon.len() == 899);
    for (number, slot) in pokemon.iter().enumerate().skip(1) {
        assert_eq!(slot.is_some(), personal.entries[number].is_present_in_game);
    }
    assert!(!handler.get_alt_formes().is_empty());
    for forme in handler.get_alt_formes() {
        assert!(personal.entries[forme.number() as usize].is_present_in_game);
        assert!(data.pk(forme).base_forme.is_some());
    }
    assert_eq!(handler.generation_of_pokemon(), 8);
    assert_eq!(handler.abilities_per_pokemon(), 3);
    assert_eq!(handler.get_tm_moves().len(), 100);
    assert_eq!(handler.get_move_tutor_moves().len(), 100);
    assert!(handler.has_move_tutors());
    let trainers = handler.get_trainers().expect("trainers");
    assert!(trainers.len() > 300);
    let tagged: BTreeSet<&str> = trainers
        .iter()
        .filter_map(|trainer| trainer.tag.as_deref())
        .collect();
    for tag in [
        "GYM1-LEADER",
        "GYM8-LEADER",
        "CHAMPION",
        "RIVAL1-0",
        "RIVAL2-2",
    ] {
        assert!(tagged.contains(tag), "missing tag {tag}: {tagged:?}");
    }
    let starters = handler.get_starters();
    assert_eq!(
        starters
            .iter()
            .map(|pokemon| pokemon.number())
            .collect::<Vec<_>>(),
        vec![810, 813, 816]
    );
    assert!(handler.get_encounters(false).len() > 400);
    assert!(handler.get_static_pokemon().len() > 200);
    assert!(handler.get_ingame_trades().len() >= 11);
    assert!(!handler.get_shop_items().expect("shops").is_empty());
    assert!(!handler.get_regular_field_items().is_empty());
    assert!(handler.get_current_field_tms().len() > 30);
    assert!(handler.is_switch_game());
    assert!(data.move_by_number(33).is_some());
    assert!(data.move_by_number(622).is_none());
    assert!(data.move_by_number(757).is_none());
    assert!(data.move_by_number(2).is_none());
}

fn output_romfs(output: &Path) -> PathBuf {
    let contents = output.join("atmosphere/contents");
    [SHIELD_TITLE, SWORD_TITLE]
        .iter()
        .map(|title| contents.join(title).join("romfs"))
        .find(|path| path.is_dir())
        .expect("a romfs folder in the mod")
}

#[test]
fn randomized_mods_load_back_with_consistent_data() {
    let Some(romfs) = romfs() else {
        return;
    };
    let settings = everything_on();
    let original = GameFiles::load(&romfs).expect("original");
    let species_count = if original.data.personal.len() > 1171 {
        898
    } else {
        890
    };
    for seed in SEEDS {
        let output = std::env::temp_dir().join(format!("randomon-swsh-mod-{seed}"));
        let _ = std::fs::remove_dir_all(&output);
        let options = RandomizationOptions {
            file_name: "romfs".to_owned(),
            switch_layout: SwitchLayout::Atmosphere,
            ..RandomizationOptions::default()
        };
        let result = HandlerRegistry::new()
            .with(SwshFactory)
            .randomize_file(&romfs, &settings, seed, &options)
            .expect("randomizes");
        let RomOutput::Directory(files) = &result.rom else {
            panic!("Switch games are written as mod folders");
        };
        assert!(!files.is_empty());
        let expected_titles: Vec<String> =
            match randomon_swsh::detect(&romfs).and_then(|detection| detection.game) {
                Some(game) => vec![game.title_id_text()],
                None => vec![SWORD_TITLE.to_owned(), SHIELD_TITLE.to_owned()],
            };
        for file in files.iter() {
            assert!(
                expected_titles.iter().any(|title| file
                    .path
                    .starts_with(&format!("atmosphere/contents/{title}/romfs/"))),
                "{}",
                file.path
            );
        }
        for title in &expected_titles {
            assert!(files.iter().any(|file| file.path.contains(title.as_str())));
        }
        write_directory_output(&output, files).expect("writes");
        let overlay = output_romfs(&output);
        let reloaded = GameFiles::load_layered(
            randomon_swsh::RomfsLayers::new(&romfs).with_overlay(&overlay),
            "English",
        )
        .expect("reloads the randomized romfs");
        assert_game_is_consistent(&reloaded, &original.data, species_count);
        let source = RandomSource::with_seed(seed).into_shared();
        let handler = SwshRomHandler::load_layers(&[romfs.clone(), overlay], "romfs", &source)
            .expect("handler reloads");
        let data = &handler.core().data;
        for trainer in handler.get_trainers().expect("trainers") {
            for pokemon in &trainer.pokemon {
                assert!((1..=100).contains(&pokemon.level));
                for &move_number in &pokemon.moves {
                    assert!(move_number == 0 || data.move_by_number(move_number).is_some());
                }
            }
        }
        for pokemon in handler.get_pokemon().into_iter().flatten() {
            let entry = data.pk(pokemon);
            for ability in [entry.ability1, entry.ability2, entry.ability3] {
                assert!((0..=267).contains(&ability));
            }
        }
        assert!(result.log.contains("--Static Pokemon--"));
        assert!(result.log.contains("Data version"));
        let _ = std::fs::remove_dir_all(&output);
    }
}

#[test]
fn reading_and_writing_everything_back_changes_nothing() {
    let Some(romfs) = romfs() else {
        return;
    };
    let mut handler = load(&romfs);
    let encounters = handler.get_encounters(false);
    handler
        .set_encounters(false, &encounters)
        .expect("encounters");
    let trainers = handler.get_trainers().expect("trainers");
    handler.set_trainers(&trainers, false).expect("trainers");
    let statics = handler.get_static_pokemon();
    handler.set_static_pokemon(&statics).expect("statics");
    let starters = handler.get_starters();
    handler.set_starters(&starters).expect("starters");
    let starter_items = handler.get_starter_held_items();
    assert_eq!(starter_items.len(), 3);
    handler.set_starter_held_items(&starter_items);
    let trades = handler.get_ingame_trades();
    handler.set_ingame_trades(&trades).expect("trades");
    let movesets = handler.get_moves_learnt();
    handler.set_moves_learnt(&movesets).expect("movesets");
    let egg_moves = handler.get_egg_moves();
    handler.set_egg_moves(&egg_moves).expect("egg moves");
    let tms = handler.get_tm_moves();
    handler.set_tm_moves(&tms).expect("tms");
    let tutors = handler.get_move_tutor_moves();
    handler.set_move_tutor_moves(&tutors).expect("tutors");
    let compatibility = handler.get_tmhm_compatibility();
    handler.set_tmhm_compatibility(&compatibility);
    let tutor_compatibility = handler.get_move_tutor_compatibility();
    handler.set_move_tutor_compatibility(&tutor_compatibility);
    let field_items = handler.get_regular_field_items();
    handler
        .set_regular_field_items(&field_items)
        .expect("field items");
    let field_tms = handler.get_current_field_tms();
    handler.set_field_tms(&field_tms).expect("field tms");
    let shops = handler.get_shop_items().expect("shops");
    handler.set_shop_items(&shops).expect("shops");
    let names = handler.get_trainer_names();
    handler.set_trainer_names(&names);
    let classes = handler.get_trainer_class_names();
    handler.set_trainer_class_names(&classes);
    let files = handler.save_rom_directory().expect("saves");
    assert_eq!(
        files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
}

fn alternative_modes() -> Vec<Settings> {
    let base = everything_on();
    vec![
        Settings {
            wild_pokemon_mod: WildPokemonMod::GlobalMapping,
            trainers_mod: TrainersMod::TypeThemedElite4Gyms,
            static_pokemon_mod: StaticPokemonMod::CompletelyRandom,
            evolutions_mod: EvolutionsMod::RandomEveryLevel,
            field_items_mod: FieldItemsMod::Shuffle,
            shop_items_mod: ShopItemsMod::Shuffle,
            ..base.clone()
        },
        Settings {
            wild_pokemon_mod: WildPokemonMod::Random,
            trainers_mod: TrainersMod::Distributed,
            static_pokemon_mod: StaticPokemonMod::SimilarStrength,
            movesets_mod: MovesetsMod::MetronomeOnly,
            starters_mod: StartersMod::CompletelyRandom,
            tms_hms_compatibility_mod: TmsHmsCompatibilityMod::Full,
            move_tutors_compatibility_mod: MoveTutorsCompatibilityMod::Full,
            field_items_mod: FieldItemsMod::RandomEven,
            ..base.clone()
        },
        Settings {
            trainers_mod: TrainersMod::TypeThemed,
            wild_pokemon_mod: WildPokemonMod::Unchanged,
            evolutions_mod: EvolutionsMod::Unchanged,
            types_mod: TypesMod::Unchanged,
            base_statistics_mod: BaseStatisticsMod::Random,
            ..base
        },
    ]
}

#[test]
fn alternative_modes_randomize_cleanly() {
    let Some(romfs) = romfs() else {
        return;
    };
    let original = GameFiles::load(&romfs).expect("original");
    let species_count = if original.data.personal.len() > 1171 {
        898
    } else {
        890
    };
    for (index, settings) in alternative_modes().into_iter().enumerate() {
        let output = std::env::temp_dir().join(format!("randomon-swsh-mode-{index}"));
        let _ = std::fs::remove_dir_all(&output);
        let options = RandomizationOptions {
            file_name: "romfs".to_owned(),
            switch_layout: SwitchLayout::Atmosphere,
            ..RandomizationOptions::default()
        };
        let result = HandlerRegistry::new()
            .with(SwshFactory)
            .randomize_file(&romfs, &settings, 77 + index as i64, &options)
            .expect("randomizes");
        let RomOutput::Directory(files) = &result.rom else {
            panic!("Switch games are written as mod folders");
        };
        write_directory_output(&output, files).expect("writes");
        let overlay = output_romfs(&output);
        let reloaded = GameFiles::load_layered(
            randomon_swsh::RomfsLayers::new(&romfs).with_overlay(&overlay),
            "English",
        )
        .expect("reloads the randomized romfs");
        assert_game_is_consistent(&reloaded, &original.data, species_count);
        let _ = std::fs::remove_dir_all(&output);
    }
}
