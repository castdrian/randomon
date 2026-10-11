use randomon_settings::*;

pub const SEEDS: &[i64] = &[1, 42, 123_456_789, -987_654_321, 281_474_976_710_655];

type Tweak = fn(&mut Settings);

fn base(rom_name: &str) -> Settings {
    Settings {
        rom_name: Some(rom_name.to_owned()),
        selected_exp_curve: Some(ExpCurve::MediumFast),
        custom_starters: [1, 1, 1],
        ..Settings::default()
    }
}

fn tweak_mask(tweaks: &[MiscTweak]) -> i32 {
    tweaks.iter().fold(0, |mask, tweak| mask | tweak.value())
}

const CASES: &[(&str, Tweak)] = &[
    ("defaults", |_| {}),
    ("stats_shuffle", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Shuffle
    }),
    ("stats_shuffle_follow", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Shuffle;
        s.base_stats_follow_evolutions = true;
    }),
    ("stats_random", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Random
    }),
    ("stats_random_follow", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Random;
        s.base_stats_follow_evolutions = true;
    }),
    ("stats_random_follow_assign", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Random;
        s.base_stats_follow_evolutions = true;
        s.assign_evo_stats_randomly = true;
    }),
    ("stats_update_gen6", |s| {
        s.update_base_stats = true;
        s.update_base_stats_to_generation = 6;
    }),
    ("stats_update_gen9", |s| {
        s.update_base_stats = true;
        s.update_base_stats_to_generation = 9;
    }),
    ("exp_curves_legendaries", |s| {
        s.standardize_exp_curves = true;
        s.exp_curve_mod = ExpCurveMod::Legendaries;
        s.selected_exp_curve = Some(ExpCurve::MediumFast);
    }),
    ("exp_curves_strong", |s| {
        s.standardize_exp_curves = true;
        s.exp_curve_mod = ExpCurveMod::StrongLegendaries;
        s.selected_exp_curve = Some(ExpCurve::Fast);
    }),
    ("exp_curves_all", |s| {
        s.standardize_exp_curves = true;
        s.exp_curve_mod = ExpCurveMod::All;
        s.selected_exp_curve = Some(ExpCurve::Slow);
    }),
    ("types_follow", |s| {
        s.types_mod = TypesMod::RandomFollowEvolutions
    }),
    ("types_random", |s| s.types_mod = TypesMod::CompletelyRandom),
    ("types_random_dual", |s| {
        s.types_mod = TypesMod::CompletelyRandom;
        s.dual_type_only = true;
    }),
    ("types_follow_dual", |s| {
        s.types_mod = TypesMod::RandomFollowEvolutions;
        s.dual_type_only = true;
    }),
    ("evolutions_random", |s| {
        s.evolutions_mod = EvolutionsMod::Random
    }),
    ("evolutions_random_strict", |s| {
        s.evolutions_mod = EvolutionsMod::Random;
        s.evos_similar_strength = true;
        s.evos_same_typing = true;
        s.evos_max_three_stages = true;
        s.evos_force_change = true;
    }),
    ("evolutions_every_level", |s| {
        s.evolutions_mod = EvolutionsMod::RandomEveryLevel
    }),
    ("evolutions_every_level_typed", |s| {
        s.evolutions_mod = EvolutionsMod::RandomEveryLevel;
        s.evos_same_typing = true;
        s.evos_force_change = true;
    }),
    ("evolutions_impossible_easier", |s| {
        s.change_impossible_evolutions = true;
        s.make_evolutions_easier = true;
    }),
    ("starters_custom", |s| {
        s.starters_mod = StartersMod::Custom;
        s.custom_starters = [26, 1, 5];
    }),
    ("starters_custom_random_slot", |s| {
        s.starters_mod = StartersMod::Custom;
        s.custom_starters = [1, 100, 1];
    }),
    ("starters_random", |s| {
        s.starters_mod = StartersMod::CompletelyRandom
    }),
    ("starters_two_evos", |s| {
        s.starters_mod = StartersMod::RandomWithTwoEvolutions
    }),
    ("starters_random_rival", |s| {
        s.starters_mod = StartersMod::CompletelyRandom;
        s.rival_carries_starter_throughout = true;
    }),
    ("starters_two_evos_rival", |s| {
        s.starters_mod = StartersMod::RandomWithTwoEvolutions;
        s.rival_carries_starter_throughout = true;
        s.trainers_mod = TrainersMod::Random;
    }),
    ("moves_all_stats", |s| {
        s.randomize_move_powers = true;
        s.randomize_move_accuracies = true;
        s.randomize_move_pps = true;
        s.randomize_move_types = true;
    }),
    ("moves_powers", |s| s.randomize_move_powers = true),
    ("moves_accuracies", |s| s.randomize_move_accuracies = true),
    ("moves_update_gen2", |s| {
        s.update_moves = true;
        s.update_moves_to_generation = 2;
    }),
    ("moves_update_gen9", |s| {
        s.update_moves = true;
        s.update_moves_to_generation = 9;
    }),
    ("movesets_same_type", |s| {
        s.movesets_mod = MovesetsMod::RandomPreferSameType
    }),
    ("movesets_random", |s| {
        s.movesets_mod = MovesetsMod::CompletelyRandom
    }),
    ("movesets_random_options", |s| {
        s.movesets_mod = MovesetsMod::CompletelyRandom;
        s.start_with_guaranteed_moves = true;
        s.guaranteed_move_count = 4;
        s.reorder_damaging_moves = true;
        s.movesets_force_good_damaging = true;
        s.movesets_good_damaging_percent = 40;
        s.block_broken_moveset_moves = true;
    }),
    ("movesets_same_type_evolution_moves", |s| {
        s.movesets_mod = MovesetsMod::RandomPreferSameType;
        s.evolution_moves_for_all = true;
        s.start_with_guaranteed_moves = true;
        s.guaranteed_move_count = 2;
    }),
    ("movesets_reorder_only", |s| s.reorder_damaging_moves = true),
    ("movesets_metronome", |s| {
        s.movesets_mod = MovesetsMod::MetronomeOnly
    }),
    ("tms_random", |s| s.tms_mod = TmsMod::Random),
    ("tms_random_options", |s| {
        s.tms_mod = TmsMod::Random;
        s.keep_field_move_tms = true;
        s.tms_force_good_damaging = true;
        s.tms_good_damaging_percent = 60;
        s.block_broken_tm_moves = true;
    }),
    ("tm_compat_random", |s| {
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::CompletelyRandom
    }),
    ("tm_compat_prefer_type_follow", |s| {
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::RandomPreferType;
        s.tms_follow_evolutions = true;
    }),
    ("tm_compat_full", |s| {
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::Full
    }),
    ("tm_compat_sanity", |s| {
        s.tms_mod = TmsMod::Random;
        s.movesets_mod = MovesetsMod::CompletelyRandom;
        s.tm_level_up_move_sanity = true;
        s.tms_follow_evolutions = true;
        s.full_hm_compat = true;
    }),
    ("trainers_random", |s| s.trainers_mod = TrainersMod::Random),
    ("trainers_distributed", |s| {
        s.trainers_mod = TrainersMod::Distributed
    }),
    ("trainers_main_playthrough", |s| {
        s.trainers_mod = TrainersMod::MainPlaythrough
    }),
    ("trainers_type_themed", |s| {
        s.trainers_mod = TrainersMod::TypeThemed
    }),
    ("trainers_type_themed_weighted", |s| {
        s.trainers_mod = TrainersMod::TypeThemed;
        s.trainers_match_typing_distribution = true;
    }),
    ("trainers_type_themed_gyms", |s| {
        s.trainers_mod = TrainersMod::TypeThemedElite4Gyms
    }),
    ("trainers_similar_strength", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.trainers_use_pokemon_of_similar_strength = true;
        s.trainers_block_legendaries = false;
    }),
    ("trainers_levels_only", |s| {
        s.trainers_level_modified = true;
        s.trainers_level_modifier = 25;
    }),
    ("trainers_levels_negative", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.trainers_level_modified = true;
        s.trainers_level_modifier = -30;
    }),
    ("trainers_fully_evolved", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.trainers_force_fully_evolved = true;
        s.trainers_force_fully_evolved_level = 30;
    }),
    ("trainers_names", |s| {
        s.randomize_trainer_names = true;
        s.randomize_trainer_class_names = true;
    }),
    ("trainers_additional", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.additional_boss_trainer_pokemon = 2;
        s.additional_important_trainer_pokemon = 1;
        s.additional_regular_trainer_pokemon = 3;
    }),
    ("trainers_double_battle", |s| {
        s.double_battle_mode = true;
        s.trainers_mod = TrainersMod::Random;
    }),
    ("trainers_elite_unique", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.elite_four_unique_pokemon_number = 2;
    }),
    ("trainers_better_movesets", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.better_trainer_movesets = true;
    }),
    ("trainers_held_items", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.randomize_held_items_for_boss_trainer_pokemon = true;
        s.randomize_held_items_for_regular_trainer_pokemon = true;
    }),
    ("wild_random", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random
    }),
    ("wild_random_similar", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::SimilarStrength;
    }),
    ("wild_random_catch_em_all", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::CatchEmAll;
    }),
    ("wild_random_type_themed", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::TypeThemeAreas;
        s.block_wild_legendaries = false;
    }),
    ("wild_area_mapping", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping
    }),
    ("wild_area_similar", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::SimilarStrength;
    }),
    ("wild_area_catch_em_all", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::CatchEmAll;
    }),
    ("wild_area_type_themed", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::TypeThemeAreas;
    }),
    ("wild_global_mapping", |s| {
        s.wild_pokemon_mod = WildPokemonMod::GlobalMapping
    }),
    ("wild_global_similar", |s| {
        s.wild_pokemon_mod = WildPokemonMod::GlobalMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::SimilarStrength;
        s.block_wild_legendaries = false;
    }),
    ("wild_levels_only", |s| {
        s.wild_levels_modified = true;
        s.wild_level_modifier = 50;
    }),
    ("wild_random_levels_catch_rate", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.wild_levels_modified = true;
        s.wild_level_modifier = -20;
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = 3;
    }),
    ("catch_rate_guaranteed", |s| {
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = 5;
    }),
    ("statics_matching", |s| {
        s.static_pokemon_mod = StaticPokemonMod::RandomMatching
    }),
    ("statics_random", |s| {
        s.static_pokemon_mod = StaticPokemonMod::CompletelyRandom
    }),
    ("statics_similar", |s| {
        s.static_pokemon_mod = StaticPokemonMod::SimilarStrength;
        s.limit600 = true;
    }),
    ("statics_levels", |s| {
        s.static_level_modified = true;
        s.static_level_modifier = 30;
    }),
    ("statics_random_levels", |s| {
        s.static_pokemon_mod = StaticPokemonMod::CompletelyRandom;
        s.static_level_modified = true;
        s.static_level_modifier = -10;
    }),
    ("trades_given", |s| {
        s.in_game_trades_mod = InGameTradesMod::RandomizeGiven
    }),
    ("trades_given_requested_names", |s| {
        s.in_game_trades_mod = InGameTradesMod::RandomizeGivenAndRequested;
        s.randomize_in_game_trades_nicknames = true;
    }),
    ("field_items_shuffle", |s| {
        s.field_items_mod = FieldItemsMod::Shuffle
    }),
    ("field_items_random", |s| {
        s.field_items_mod = FieldItemsMod::Random
    }),
    ("field_items_random_even", |s| {
        s.field_items_mod = FieldItemsMod::RandomEven
    }),
    ("tweak_bw_exp", |s| {
        s.current_misc_tweaks = tweak_mask(&[MiscTweak::BwExpPatch])
    }),
    ("tweak_fastest_text", |s| {
        s.current_misc_tweaks = tweak_mask(&[MiscTweak::FastestText])
    }),
    ("tweak_lower_case", |s| {
        s.current_misc_tweaks = tweak_mask(&[MiscTweak::LowerCasePokemonNames]);
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.static_pokemon_mod = StaticPokemonMod::CompletelyRandom;
    }),
    ("tweak_type_effectiveness", |s| {
        s.current_misc_tweaks = tweak_mask(&[MiscTweak::UpdateTypeEffectiveness])
    }),
    ("tweak_catching_tutorial", |s| {
        s.current_misc_tweaks = tweak_mask(&[MiscTweak::RandomizeCatchingTutorial])
    }),
    ("tweak_ban_lucky_egg", |s| {
        s.current_misc_tweaks = tweak_mask(&[MiscTweak::BanLuckyEgg]);
        s.field_items_mod = FieldItemsMod::Random;
        s.randomize_wild_pokemon_held_items = true;
    }),
    ("tweak_all", |s| {
        s.current_misc_tweaks = tweak_mask(&[
            MiscTweak::BwExpPatch,
            MiscTweak::NerfXAccuracy,
            MiscTweak::FixCritRate,
            MiscTweak::FastestText,
            MiscTweak::RandomizePcPotion,
            MiscTweak::AllowPikachuEvolution,
            MiscTweak::LowerCasePokemonNames,
            MiscTweak::UpdateTypeEffectiveness,
            MiscTweak::RandomizeCatchingTutorial,
            MiscTweak::BanLuckyEgg,
            MiscTweak::NationalDexAtStart,
        ]);
    }),
    ("intro_and_guaranteed_catch", |s| {
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = 5;
        s.current_misc_tweaks = tweak_mask(&[MiscTweak::RandomizeCatchingTutorial]);
    }),
    ("evolutions_remove_time_based", |s| {
        s.remove_time_based_evolutions = true
    }),
    ("evolutions_all_fixes", |s| {
        s.change_impossible_evolutions = true;
        s.make_evolutions_easier = true;
        s.remove_time_based_evolutions = true;
        s.evolutions_mod = EvolutionsMod::Random;
    }),
    ("limit_gen1", |s| {
        s.limit_pokemon = true;
        s.current_restrictions = Some(GenRestrictions {
            allow_gen1: true,
            ..GenRestrictions::default()
        });
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.trainers_mod = TrainersMod::Random;
        s.starters_mod = StartersMod::CompletelyRandom;
        s.static_pokemon_mod = StaticPokemonMod::RandomMatching;
    }),
    ("limit_gen2_relatives", |s| {
        s.limit_pokemon = true;
        s.current_restrictions = Some(GenRestrictions {
            allow_gen2: true,
            allow_evolutionary_relatives: true,
            ..GenRestrictions::default()
        });
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.trainers_mod = TrainersMod::TypeThemed;
        s.evolutions_mod = EvolutionsMod::Random;
    }),
    ("starters_held_items", |s| {
        s.randomize_starters_held_items = true;
        s.starters_mod = StartersMod::CompletelyRandom;
    }),
    ("starters_held_items_ban_bad", |s| {
        s.randomize_starters_held_items = true;
        s.ban_bad_random_starter_held_items = true;
    }),
    ("wild_time_based", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.use_time_based_encounters = true;
    }),
    ("wild_time_based_area", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.use_time_based_encounters = true;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::TypeThemeAreas;
    }),
    ("wild_time_based_global_catch_em_all", |s| {
        s.wild_pokemon_mod = WildPokemonMod::GlobalMapping;
        s.use_time_based_encounters = true;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::CatchEmAll;
    }),
    ("wild_time_based_levels", |s| {
        s.use_time_based_encounters = true;
        s.wild_levels_modified = true;
        s.wild_level_modifier = -35;
    }),
    ("wild_held_items", |s| {
        s.randomize_wild_pokemon_held_items = true
    }),
    ("wild_held_items_ban_bad", |s| {
        s.randomize_wild_pokemon_held_items = true;
        s.ban_bad_random_wild_pokemon_held_items = true;
        s.wild_pokemon_mod = WildPokemonMod::Random;
    }),
    ("trainers_held_items_all", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.randomize_held_items_for_boss_trainer_pokemon = true;
        s.randomize_held_items_for_important_trainer_pokemon = true;
        s.randomize_held_items_for_regular_trainer_pokemon = true;
    }),
    ("trainers_held_items_consumable_highest", |s| {
        s.randomize_held_items_for_boss_trainer_pokemon = true;
        s.randomize_held_items_for_important_trainer_pokemon = true;
        s.consumable_items_only_for_trainer_pokemon = true;
        s.highest_level_only_gets_items_for_trainer_pokemon = true;
    }),
    ("trainers_held_items_sensible", |s| {
        s.trainers_mod = TrainersMod::Distributed;
        s.randomize_held_items_for_regular_trainer_pokemon = true;
        s.sensible_items_only_for_trainer_pokemon = true;
    }),
    ("trainers_better_movesets_items", |s| {
        s.trainers_mod = TrainersMod::TypeThemed;
        s.better_trainer_movesets = true;
        s.randomize_held_items_for_boss_trainer_pokemon = true;
        s.movesets_mod = MovesetsMod::RandomPreferSameType;
    }),
    ("trainers_better_movesets_only", |s| {
        s.better_trainer_movesets = true
    }),
    ("trainers_rival_starter", |s| {
        s.starters_mod = StartersMod::Custom;
        s.custom_starters = [155, 158, 152];
        s.rival_carries_starter_throughout = true;
        s.trainers_mod = TrainersMod::Random;
        s.trainers_force_fully_evolved = true;
        s.trainers_force_fully_evolved_level = 40;
    }),
    ("trainers_additional_held", |s| {
        s.trainers_mod = TrainersMod::TypeThemedElite4Gyms;
        s.additional_boss_trainer_pokemon = 3;
        s.additional_important_trainer_pokemon = 2;
        s.randomize_held_items_for_boss_trainer_pokemon = true;
    }),
    ("trainers_shiny", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.shiny_chance = true;
    }),
    ("trades_full", |s| {
        s.in_game_trades_mod = InGameTradesMod::RandomizeGivenAndRequested;
        s.randomize_in_game_trades_nicknames = true;
        s.randomize_in_game_trades_ots = true;
        s.randomize_in_game_trades_ivs = true;
        s.randomize_in_game_trades_items = true;
    }),
    ("trades_given_items", |s| {
        s.in_game_trades_mod = InGameTradesMod::RandomizeGiven;
        s.randomize_in_game_trades_items = true;
        s.randomize_in_game_trades_ivs = true;
    }),
    ("field_items_ban_bad", |s| {
        s.field_items_mod = FieldItemsMod::Random;
        s.ban_bad_random_field_items = true;
    }),
    ("field_items_even_ban_bad", |s| {
        s.field_items_mod = FieldItemsMod::RandomEven;
        s.ban_bad_random_field_items = true;
    }),
    ("movesets_egg_moves", |s| {
        s.movesets_mod = MovesetsMod::CompletelyRandom;
        s.block_broken_moveset_moves = true;
        s.movesets_force_good_damaging = true;
        s.movesets_good_damaging_percent = 25;
    }),
    ("statics_similar_limit_legendaries", |s| {
        s.static_pokemon_mod = StaticPokemonMod::SimilarStrength;
        s.limit_main_game_legendaries = true;
        s.limit600 = true;
    }),
    ("statics_matching_levels", |s| {
        s.static_pokemon_mod = StaticPokemonMod::RandomMatching;
        s.static_level_modified = true;
        s.static_level_modifier = 45;
    }),
    ("tutors_ignored_on_gold", |s| {
        s.move_tutor_moves_mod = MoveTutorMovesMod::Random;
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::CompletelyRandom;
    }),
    ("kitchen_sink", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Random;
        s.base_stats_follow_evolutions = true;
        s.types_mod = TypesMod::RandomFollowEvolutions;
        s.evolutions_mod = EvolutionsMod::Random;
        s.evos_similar_strength = true;
        s.change_impossible_evolutions = true;
        s.make_evolutions_easier = true;
        s.starters_mod = StartersMod::RandomWithTwoEvolutions;
        s.randomize_move_powers = true;
        s.randomize_move_types = true;
        s.movesets_mod = MovesetsMod::RandomPreferSameType;
        s.start_with_guaranteed_moves = true;
        s.guaranteed_move_count = 4;
        s.tms_mod = TmsMod::Random;
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::RandomPreferType;
        s.tm_level_up_move_sanity = true;
        s.trainers_mod = TrainersMod::TypeThemed;
        s.rival_carries_starter_throughout = true;
        s.trainers_force_fully_evolved = true;
        s.trainers_level_modified = true;
        s.trainers_level_modifier = 10;
        s.randomize_trainer_names = true;
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::TypeThemeAreas;
        s.static_pokemon_mod = StaticPokemonMod::RandomMatching;
        s.in_game_trades_mod = InGameTradesMod::RandomizeGivenAndRequested;
        s.randomize_in_game_trades_nicknames = true;
        s.field_items_mod = FieldItemsMod::RandomEven;
        s.current_misc_tweaks =
            tweak_mask(&[MiscTweak::FastestText, MiscTweak::LowerCasePokemonNames]);
    }),
    ("kitchen_sink_shuffle", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Shuffle;
        s.types_mod = TypesMod::CompletelyRandom;
        s.evolutions_mod = EvolutionsMod::RandomEveryLevel;
        s.starters_mod = StartersMod::CompletelyRandom;
        s.update_moves = true;
        s.update_moves_to_generation = 5;
        s.randomize_move_accuracies = true;
        s.randomize_move_pps = true;
        s.movesets_mod = MovesetsMod::CompletelyRandom;
        s.reorder_damaging_moves = true;
        s.tms_mod = TmsMod::Random;
        s.keep_field_move_tms = true;
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::Full;
        s.trainers_mod = TrainersMod::Distributed;
        s.trainers_use_pokemon_of_similar_strength = true;
        s.randomize_trainer_class_names = true;
        s.wild_pokemon_mod = WildPokemonMod::GlobalMapping;
        s.wild_levels_modified = true;
        s.wild_level_modifier = 15;
        s.static_pokemon_mod = StaticPokemonMod::SimilarStrength;
        s.field_items_mod = FieldItemsMod::Shuffle;
        s.current_misc_tweaks = tweak_mask(&[
            MiscTweak::UpdateTypeEffectiveness,
            MiscTweak::RandomizePcPotion,
        ]);
    }),
];

pub fn settings_grid(rom_name: &str) -> Vec<(String, Settings)> {
    CASES
        .iter()
        .map(|(name, tweak)| {
            let mut settings = base(rom_name);
            tweak(&mut settings);
            ((*name).to_owned(), settings)
        })
        .collect()
}

struct Dice {
    state: u64,
}

impl Dice {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    fn pick<T: Copy>(&mut self, options: &[T]) -> T {
        options[self.below(options.len() as u64) as usize]
    }

    fn range(&mut self, low: i32, high: i32) -> i32 {
        low + self.below((high - low + 1) as u64) as i32
    }
}

const GEN2_TWEAKS: &[MiscTweak] = &[
    MiscTweak::BwExpPatch,
    MiscTweak::FastestText,
    MiscTweak::LowerCasePokemonNames,
    MiscTweak::UpdateTypeEffectiveness,
    MiscTweak::RandomizeCatchingTutorial,
    MiscTweak::BanLuckyEgg,
];

fn fuzz_pokemon_traits(dice: &mut Dice, s: &mut Settings) {
    s.change_impossible_evolutions = dice.chance(40);
    s.make_evolutions_easier = dice.chance(40);
    s.base_statistics_mod = dice.pick(BaseStatisticsMod::ALL);
    s.base_stats_follow_evolutions = dice.chance(50);
    s.assign_evo_stats_randomly = dice.chance(30);
    if dice.chance(20) {
        s.update_base_stats = true;
        s.update_base_stats_to_generation = dice.range(6, 9);
    }
    if dice.chance(20) {
        s.standardize_exp_curves = true;
        s.exp_curve_mod = dice.pick(ExpCurveMod::ALL);
        s.selected_exp_curve = Some(dice.pick(ExpCurve::ALL));
    }
    s.types_mod = dice.pick(TypesMod::ALL);
    s.dual_type_only = dice.chance(15);
    s.evolutions_mod = dice.pick(EvolutionsMod::ALL);
    s.evos_similar_strength = dice.chance(40);
    s.evos_same_typing = dice.chance(40);
    s.evos_max_three_stages = dice.chance(40);
    s.evos_force_change = dice.chance(40);
    s.remove_time_based_evolutions = dice.chance(30);
    if dice.chance(15) {
        s.limit_pokemon = true;
        s.current_restrictions = Some(GenRestrictions {
            allow_gen1: dice.chance(60),
            allow_gen2: dice.chance(60),
            allow_evolutionary_relatives: dice.chance(50),
            ..GenRestrictions::default()
        });
    }
}

fn fuzz_starters_and_moves(dice: &mut Dice, s: &mut Settings) {
    s.starters_mod = dice.pick(StartersMod::ALL);
    s.custom_starters = [dice.range(1, 251), dice.range(1, 251), dice.range(1, 251)];
    s.rival_carries_starter_throughout = dice.chance(40);
    s.randomize_starters_held_items = dice.chance(30);
    s.ban_bad_random_starter_held_items = dice.chance(50);
    s.randomize_move_powers = dice.chance(30);
    s.randomize_move_accuracies = dice.chance(30);
    s.randomize_move_pps = dice.chance(30);
    s.randomize_move_types = dice.chance(30);
    if dice.chance(25) {
        s.update_moves = true;
        s.update_moves_to_generation = dice.range(2, 9);
    }
    s.movesets_mod = dice.pick(MovesetsMod::ALL);
    s.start_with_guaranteed_moves = dice.chance(40);
    s.guaranteed_move_count = dice.range(2, 4);
    s.reorder_damaging_moves = dice.chance(30);
    if dice.chance(30) {
        s.movesets_force_good_damaging = true;
        s.movesets_good_damaging_percent = dice.range(0, 20) * 5;
    }
    s.block_broken_moveset_moves = dice.chance(40);
    s.evolution_moves_for_all = dice.chance(20);
    s.tms_mod = dice.pick(TmsMod::ALL);
    s.keep_field_move_tms = dice.chance(40);
    if dice.chance(30) {
        s.tms_force_good_damaging = true;
        s.tms_good_damaging_percent = dice.range(0, 20) * 5;
    }
    s.block_broken_tm_moves = dice.chance(40);
    s.tms_hms_compatibility_mod = dice.pick(TmsHmsCompatibilityMod::ALL);
    s.tm_level_up_move_sanity = dice.chance(30);
    s.tms_follow_evolutions = dice.chance(30);
    s.full_hm_compat = dice.chance(20);
}

fn fuzz_encounters(dice: &mut Dice, s: &mut Settings) {
    s.trainers_mod = dice.pick(TrainersMod::ALL);
    s.trainers_use_pokemon_of_similar_strength = dice.chance(40);
    s.trainers_match_typing_distribution = dice.chance(30);
    s.trainers_block_legendaries = dice.chance(60);
    if dice.chance(30) {
        s.trainers_level_modified = true;
        s.trainers_level_modifier = dice.range(-50, 50);
    }
    if dice.chance(30) {
        s.trainers_force_fully_evolved = true;
        s.trainers_force_fully_evolved_level = dice.range(30, 65);
    }
    s.randomize_trainer_names = dice.chance(30);
    s.randomize_trainer_class_names = dice.chance(30);
    s.shiny_chance = dice.chance(10);
    s.better_trainer_movesets = dice.chance(8);
    s.randomize_held_items_for_boss_trainer_pokemon = dice.chance(5);
    s.randomize_held_items_for_important_trainer_pokemon = dice.chance(5);
    s.randomize_held_items_for_regular_trainer_pokemon = dice.chance(5);
    s.consumable_items_only_for_trainer_pokemon = dice.chance(40);
    s.sensible_items_only_for_trainer_pokemon = dice.chance(40);
    s.highest_level_only_gets_items_for_trainer_pokemon = dice.chance(40);
    if dice.chance(8) {
        s.additional_boss_trainer_pokemon = dice.range(0, 5);
        s.additional_important_trainer_pokemon = dice.range(0, 5);
        s.additional_regular_trainer_pokemon = dice.range(0, 5);
    }
    s.elite_four_unique_pokemon_number = if dice.chance(20) { dice.range(1, 2) } else { 0 };
    s.use_time_based_encounters = dice.chance(40);
    s.randomize_wild_pokemon_held_items = dice.chance(30);
    s.ban_bad_random_wild_pokemon_held_items = dice.chance(50);
    s.wild_pokemon_mod = dice.pick(WildPokemonMod::ALL);
    s.wild_pokemon_restriction_mod = dice.pick(WildPokemonRestrictionMod::ALL);
    s.block_wild_legendaries = dice.chance(60);
    if dice.chance(30) {
        s.wild_levels_modified = true;
        s.wild_level_modifier = dice.range(-50, 50);
    }
    if dice.chance(30) {
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = dice.range(1, 5);
    }
    s.static_pokemon_mod = dice.pick(StaticPokemonMod::ALL);
    s.limit600 = dice.chance(30);
    if dice.chance(30) {
        s.static_level_modified = true;
        s.static_level_modifier = dice.range(-50, 50);
    }
    s.in_game_trades_mod = dice.pick(InGameTradesMod::ALL);
    s.randomize_in_game_trades_nicknames = dice.chance(50);
    s.randomize_in_game_trades_ots = dice.chance(50);
    s.randomize_in_game_trades_ivs = dice.chance(50);
    s.randomize_in_game_trades_items = dice.chance(50);
    s.field_items_mod = dice.pick(FieldItemsMod::ALL);
    s.ban_bad_random_field_items = dice.chance(50);
    s.current_misc_tweaks = GEN2_TWEAKS
        .iter()
        .filter(|_| dice.chance(25))
        .fold(0, |mask, tweak| mask | tweak.value());
}

pub fn fuzz_grid(rom_name: &str, count: usize) -> Vec<(String, Settings)> {
    (0..count)
        .map(|index| {
            let mut dice = Dice::new(index as u64 + 1);
            let mut settings = base(rom_name);
            fuzz_pokemon_traits(&mut dice, &mut settings);
            fuzz_starters_and_moves(&mut dice, &mut settings);
            fuzz_encounters(&mut dice, &mut settings);
            (format!("fuzz_{index:03}"), settings)
        })
        .collect()
}
