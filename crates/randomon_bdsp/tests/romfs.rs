use std::collections::BTreeSet;
use std::path::PathBuf;

use randomon_bdsp::{BdspFactory, EXEFS_PATCH_FOLDER};
use randomon_core::{
    HandlerRegistry, RandomizationOptions, RomHandler, RomOutput, SwitchLayout,
    write_directory_output,
};
use randomon_java::RandomSource;
use randomon_settings::{
    AbilitiesMod, BaseStatisticsMod, EvolutionsMod, ExpCurve, FieldItemsMod, InGameTradesMod,
    MiscTweak, MovesetsMod, PickupItemsMod, Settings, ShopItemsMod, StartersMod, StaticPokemonMod,
    TmsHmsCompatibilityMod, TmsMod, TrainersMod, TypesMod, WildPokemonMod,
};

const SEEDS: [i64; 3] = [1, 424_242, 9_876_543_210];

fn romfs() -> Option<PathBuf> {
    std::env::var_os("RANDOMON_BDSP_ROMFS").map(PathBuf::from)
}

fn everything_on() -> Settings {
    Settings {
        rom_name: Some(randomon_bdsp::ROM_NAME.to_owned()),
        selected_exp_curve: Some(ExpCurve::MediumFast),
        abilities_mod: AbilitiesMod::Randomize,
        starters_mod: StartersMod::RandomWithTwoEvolutions,
        types_mod: TypesMod::RandomFollowEvolutions,
        base_statistics_mod: BaseStatisticsMod::Shuffle,
        movesets_mod: MovesetsMod::RandomPreferSameType,
        trainers_mod: TrainersMod::Random,
        rival_carries_starter_throughout: true,
        randomize_trainer_names: true,
        randomize_trainer_class_names: true,
        randomize_held_items_for_regular_trainer_pokemon: true,
        wild_pokemon_mod: WildPokemonMod::AreaMapping,
        use_time_based_encounters: true,
        randomize_wild_pokemon_held_items: true,
        static_pokemon_mod: StaticPokemonMod::RandomMatching,
        in_game_trades_mod: InGameTradesMod::RandomizeGivenAndRequested,
        field_items_mod: FieldItemsMod::Random,
        shop_items_mod: ShopItemsMod::Random,
        balance_shop_prices: true,
        pickup_items_mod: PickupItemsMod::Random,
        tms_mod: TmsMod::Random,
        tms_hms_compatibility_mod: TmsHmsCompatibilityMod::RandomPreferType,
        evolutions_mod: EvolutionsMod::Random,
        change_impossible_evolutions: true,
        randomize_move_powers: true,
        current_misc_tweaks: [
            MiscTweak::FollowingPokemonSpeeds,
            MiscTweak::TrueSizePokemon,
            MiscTweak::RemoveFogAndWeather,
            MiscTweak::BikeEverywhere,
            MiscTweak::SixtyFps,
            MiscTweak::FasterGameSpeed,
        ]
        .iter()
        .fold(0, |mask, tweak| mask | tweak.value()),
        ..Settings::default()
    }
}

fn load(path: &std::path::Path) -> Box<dyn RomHandler> {
    let source = RandomSource::with_seed(0).into_shared();
    HandlerRegistry::new()
        .with(BdspFactory)
        .load_file(path, "romfs", &source)
        .expect("loads the romfs")
}

#[test]
fn the_romfs_exposes_the_full_catalog() {
    let Some(romfs) = romfs() else {
        return;
    };
    let handler = load(&romfs);
    assert_eq!(handler.get_pokemon().len(), 494);
    assert_eq!(handler.generation_of_pokemon(), 8);
    assert_eq!(handler.abilities_per_pokemon(), 3);
    assert_eq!(handler.get_tm_moves().len(), 100);
    assert!(handler.get_trainers().expect("trainers").len() > 700);
    assert!(handler.get_encounters(true).len() > 300);
    assert!(handler.get_static_pokemon().len() > 20);
    assert_eq!(handler.get_ingame_trades().len(), 4);
    assert!(!handler.get_shop_items().expect("shops").is_empty());
    assert!(!handler.get_regular_field_items().is_empty());
    assert!(!handler.get_current_field_tms().is_empty());
    assert_eq!(handler.get_pickup_items().len(), 30);
    assert!(handler.is_switch_game());
}

#[test]
fn randomized_mods_load_back_with_consistent_data() {
    let Some(romfs) = romfs() else {
        return;
    };
    let settings = everything_on();
    for seed in SEEDS {
        let output = std::env::temp_dir().join(format!("randomon-bdsp-mod-{seed}"));
        let _ = std::fs::remove_dir_all(&output);
        let options = RandomizationOptions {
            file_name: "romfs".to_owned(),
            switch_layout: SwitchLayout::Atmosphere,
            ..RandomizationOptions::default()
        };
        let result = HandlerRegistry::new()
            .with(BdspFactory)
            .randomize_file(&romfs, &settings, seed, &options)
            .expect("randomizes");
        let RomOutput::Directory(files) = &result.rom else {
            panic!("Switch games are written as mod folders");
        };
        write_directory_output(&output, files).expect("writes");
        assert!(
            output
                .join("atmosphere/exefs_patches")
                .join(EXEFS_PATCH_FOLDER)
                .is_dir()
        );
        let randomized = load(&output.join("atmosphere/contents/0100000011D90000/romfs"));
        let data = &randomized.core().data;
        let valid = |species: i32| (1..=493).contains(&species);
        for trainer in randomized.get_trainers().expect("trainers") {
            for pokemon in &trainer.pokemon {
                assert!(valid(data.base_number(pokemon.pokemon)));
                assert!((1..=100).contains(&pokemon.level));
                for &move_number in &pokemon.moves {
                    assert!(move_number == 0 || data.move_by_number(move_number).is_some());
                }
            }
        }
        for set in randomized.get_encounters(true) {
            for encounter in &set.encounters {
                assert!(valid(data.base_number(encounter.pokemon)));
            }
        }
        for pokemon in randomized.get_pokemon().into_iter().flatten() {
            let entry = data.pk(pokemon);
            for ability in [entry.ability1, entry.ability2, entry.ability3] {
                assert!((0..=267).contains(&ability));
            }
        }
        let tms: BTreeSet<i32> = randomized.get_tm_moves().into_iter().collect();
        assert!(
            tms.iter()
                .all(|&move_number| data.move_by_number(move_number).is_some())
        );
        assert!(result.log.contains("--Static Pokemon--"));
        let _ = std::fs::remove_dir_all(&output);
    }
}
