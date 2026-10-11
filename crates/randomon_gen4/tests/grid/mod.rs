use randomon_settings::*;

pub const SEEDS: &[i64] = &[1, -987_654_321, 281_474_976_710_655];
pub const DEFAULT_FUZZ_CASES: usize = 16;

type Tweak = fn(&mut Settings);

pub fn base(rom_name: &str) -> Settings {
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

fn gen_restrictions(gen1: bool, gen4: bool) -> GenRestrictions {
    GenRestrictions {
        allow_gen1: gen1,
        allow_gen4: gen4,
        ..GenRestrictions::default()
    }
}

const CASES: &[(&str, Tweak)] = &[
    ("defaults", |_| {}),
    ("stats_shuffle_follow", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Shuffle;
        s.base_stats_follow_evolutions = true;
    }),
    ("stats_random_assign", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Random;
        s.base_stats_follow_evolutions = true;
        s.assign_evo_stats_randomly = true;
    }),
    ("stats_update_exp_curves", |s| {
        s.update_base_stats = true;
        s.update_base_stats_to_generation = 9;
        s.standardize_exp_curves = true;
        s.exp_curve_mod = ExpCurveMod::StrongLegendaries;
        s.selected_exp_curve = Some(ExpCurve::Fast);
    }),
    ("abilities_random", |s| {
        s.abilities_mod = AbilitiesMod::Randomize
    }),
    ("abilities_random_bans", |s| {
        s.abilities_mod = AbilitiesMod::Randomize;
        s.abilities_follow_evolutions = true;
        s.ban_trapping_abilities = true;
        s.ban_negative_abilities = true;
        s.ban_bad_abilities = true;
        s.ensure_two_abilities = true;
    }),
    ("abilities_wonder_guard_weighted", |s| {
        s.abilities_mod = AbilitiesMod::Randomize;
        s.allow_wonder_guard = true;
        s.weigh_duplicate_abilities_together = true;
    }),
    ("types_follow_dual", |s| {
        s.types_mod = TypesMod::RandomFollowEvolutions;
        s.dual_type_only = true;
    }),
    ("types_random", |s| s.types_mod = TypesMod::CompletelyRandom),
    ("evolutions_random_strict", |s| {
        s.evolutions_mod = EvolutionsMod::Random;
        s.evos_similar_strength = true;
        s.evos_same_typing = true;
        s.evos_max_three_stages = true;
        s.evos_force_change = true;
        s.evos_allow_alt_formes = true;
    }),
    ("evolutions_every_level", |s| {
        s.evolutions_mod = EvolutionsMod::RandomEveryLevel;
        s.evos_same_typing = true;
    }),
    ("evolutions_impossible_easier_time", |s| {
        s.change_impossible_evolutions = true;
        s.make_evolutions_easier = true;
        s.remove_time_based_evolutions = true;
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.movesets_mod = MovesetsMod::RandomPreferSameType;
    }),
    ("starters_custom", |s| {
        s.starters_mod = StartersMod::Custom;
        s.custom_starters = [487, 133, 385];
    }),
    ("starters_random_rival", |s| {
        s.starters_mod = StartersMod::CompletelyRandom;
        s.rival_carries_starter_throughout = true;
        s.trainers_mod = TrainersMod::Random;
    }),
    ("starters_two_evos", |s| {
        s.starters_mod = StartersMod::RandomWithTwoEvolutions;
        s.allow_starter_alt_formes = true;
    }),
    ("moves_data", |s| {
        s.randomize_move_powers = true;
        s.randomize_move_accuracies = true;
        s.randomize_move_pps = true;
        s.randomize_move_types = true;
        s.randomize_move_category = true;
    }),
    ("moves_update_gen9", |s| {
        s.update_moves = true;
        s.update_moves_to_generation = 9;
    }),
    ("movesets_same_type_options", |s| {
        s.movesets_mod = MovesetsMod::RandomPreferSameType;
        s.start_with_guaranteed_moves = true;
        s.guaranteed_move_count = 4;
        s.reorder_damaging_moves = true;
        s.evolution_moves_for_all = true;
    }),
    ("movesets_random_good_damaging", |s| {
        s.movesets_mod = MovesetsMod::CompletelyRandom;
        s.movesets_force_good_damaging = true;
        s.movesets_good_damaging_percent = 50;
        s.block_broken_moveset_moves = true;
    }),
    ("movesets_metronome", |s| {
        s.movesets_mod = MovesetsMod::MetronomeOnly
    }),
    ("tms_random_compat_type", |s| {
        s.tms_mod = TmsMod::Random;
        s.keep_field_move_tms = true;
        s.tms_force_good_damaging = true;
        s.tms_good_damaging_percent = 40;
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::RandomPreferType;
        s.tms_follow_evolutions = true;
    }),
    ("tms_compat_random_sanity", |s| {
        s.movesets_mod = MovesetsMod::CompletelyRandom;
        s.tms_mod = TmsMod::Random;
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::CompletelyRandom;
        s.tm_level_up_move_sanity = true;
        s.full_hm_compat = true;
        s.block_broken_tm_moves = true;
    }),
    ("tms_compat_full", |s| {
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::Full
    }),
    ("tutors_random_compat_type", |s| {
        s.move_tutor_moves_mod = MoveTutorMovesMod::Random;
        s.keep_field_move_tutors = true;
        s.tutors_force_good_damaging = true;
        s.tutors_good_damaging_percent = 30;
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::RandomPreferType;
        s.tutor_follow_evolutions = true;
    }),
    ("tutors_compat_random_sanity", |s| {
        s.movesets_mod = MovesetsMod::RandomPreferSameType;
        s.move_tutor_moves_mod = MoveTutorMovesMod::Random;
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::CompletelyRandom;
        s.tutor_level_up_move_sanity = true;
        s.block_broken_tutor_moves = true;
    }),
    ("tutors_compat_full", |s| {
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::Full
    }),
    ("trainers_random", |s| s.trainers_mod = TrainersMod::Random),
    ("trainers_distributed", |s| {
        s.trainers_mod = TrainersMod::Distributed;
        s.trainers_enforce_distribution = true;
    }),
    ("trainers_main_playthrough", |s| {
        s.trainers_mod = TrainersMod::MainPlaythrough;
        s.trainers_enforce_main_playthrough = true;
    }),
    ("trainers_type_themed_weighted", |s| {
        s.trainers_mod = TrainersMod::TypeThemed;
        s.trainers_match_typing_distribution = true;
    }),
    ("trainers_type_themed_gyms", |s| {
        s.trainers_mod = TrainersMod::TypeThemedElite4Gyms;
        s.allow_trainer_alternate_formes = true;
    }),
    ("trainers_similar_strength_levels", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.trainers_use_pokemon_of_similar_strength = true;
        s.trainers_block_legendaries = true;
        s.trainers_block_early_wonder_guard = true;
        s.trainers_level_modified = true;
        s.trainers_level_modifier = 30;
    }),
    ("trainers_fully_evolved_unique", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.trainers_force_fully_evolved = true;
        s.trainers_force_fully_evolved_level = 35;
        s.elite_four_unique_pokemon_number = 2;
    }),
    ("trainers_names", |s| {
        s.randomize_trainer_names = true;
        s.randomize_trainer_class_names = true;
    }),
    ("trainers_additional_doubles", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.additional_boss_trainer_pokemon = 2;
        s.additional_important_trainer_pokemon = 1;
        s.additional_regular_trainer_pokemon = 1;
        s.double_battle_mode = true;
    }),
    ("trainers_better_movesets", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.better_trainer_movesets = true;
    }),
    ("trainers_held_items", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.randomize_held_items_for_boss_trainer_pokemon = true;
        s.randomize_held_items_for_important_trainer_pokemon = true;
        s.randomize_held_items_for_regular_trainer_pokemon = true;
        s.highest_level_only_gets_items_for_trainer_pokemon = true;
    }),
    ("trainers_held_items_consumable", |s| {
        s.randomize_held_items_for_boss_trainer_pokemon = true;
        s.randomize_held_items_for_regular_trainer_pokemon = true;
        s.consumable_items_only_for_trainer_pokemon = true;
    }),
    ("trainers_levels_only_shiny", |s| {
        s.trainers_level_modified = true;
        s.trainers_level_modifier = -20;
        s.shiny_chance = true;
    }),
    ("wild_random_time", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.use_time_based_encounters = true;
        s.allow_wild_alt_formes = true;
    }),
    ("wild_random_similar_held", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::SimilarStrength;
        s.randomize_wild_pokemon_held_items = true;
        s.ban_bad_random_wild_pokemon_held_items = true;
        s.balance_shaking_grass = true;
    }),
    ("wild_area_catch_em_all_time", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::CatchEmAll;
        s.use_time_based_encounters = true;
    }),
    ("wild_area_type_themed", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::TypeThemeAreas;
        s.block_wild_legendaries = true;
    }),
    ("wild_global_levels_catch_rate", |s| {
        s.wild_pokemon_mod = WildPokemonMod::GlobalMapping;
        s.wild_levels_modified = true;
        s.wild_level_modifier = 25;
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = 2;
    }),
    ("catch_rate_guaranteed", |s| {
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = 5;
    }),
    ("statics_matching_music", |s| {
        s.static_pokemon_mod = StaticPokemonMod::RandomMatching;
        s.correct_static_music = true;
    }),
    ("statics_random_alt_formes_levels", |s| {
        s.static_pokemon_mod = StaticPokemonMod::CompletelyRandom;
        s.allow_static_alt_formes = true;
        s.static_level_modified = true;
        s.static_level_modifier = -15;
    }),
    ("statics_similar_limits_music", |s| {
        s.static_pokemon_mod = StaticPokemonMod::SimilarStrength;
        s.limit_main_game_legendaries = true;
        s.limit600 = true;
        s.correct_static_music = true;
    }),
    ("trades_given", |s| {
        s.in_game_trades_mod = InGameTradesMod::RandomizeGiven
    }),
    ("trades_everything", |s| {
        s.in_game_trades_mod = InGameTradesMod::RandomizeGivenAndRequested;
        s.randomize_in_game_trades_nicknames = true;
        s.randomize_in_game_trades_ots = true;
        s.randomize_in_game_trades_ivs = true;
        s.randomize_in_game_trades_items = true;
    }),
    ("field_items_shuffle", |s| {
        s.field_items_mod = FieldItemsMod::Shuffle
    }),
    ("field_items_random_ban_bad", |s| {
        s.field_items_mod = FieldItemsMod::Random;
        s.ban_bad_random_field_items = true;
    }),
    ("field_items_random_even", |s| {
        s.field_items_mod = FieldItemsMod::RandomEven
    }),
    ("shops_shuffle", |s| {
        s.shop_items_mod = ShopItemsMod::Shuffle
    }),
    ("shops_random_bans_prices", |s| {
        s.shop_items_mod = ShopItemsMod::Random;
        s.ban_bad_random_shop_items = true;
        s.ban_regular_shop_items = true;
        s.ban_op_shop_items = true;
        s.balance_shop_prices = true;
        s.guarantee_evolution_items = true;
        s.guarantee_x_items = true;
    }),
    ("pickup_random_ban_bad", |s| {
        s.pickup_items_mod = PickupItemsMod::Random;
        s.ban_bad_random_pickup_items = true;
    }),
    ("tweak_fastest_text", |s| {
        s.current_misc_tweaks = tweak_mask(&[MiscTweak::FastestText])
    }),
    ("tweak_lucky_egg_names_tutorial", |s| {
        s.current_misc_tweaks = tweak_mask(&[
            MiscTweak::BanLuckyEgg,
            MiscTweak::LowerCasePokemonNames,
            MiscTweak::RandomizeCatchingTutorial,
        ]);
        s.field_items_mod = FieldItemsMod::Random;
        s.shop_items_mod = ShopItemsMod::Random;
    }),
    ("tweak_national_dex_running", |s| {
        s.current_misc_tweaks = tweak_mask(&[
            MiscTweak::NationalDexAtStart,
            MiscTweak::RunWithoutRunningShoes,
        ])
    }),
    ("tweak_type_effectiveness", |s| {
        s.current_misc_tweaks = tweak_mask(&[MiscTweak::UpdateTypeEffectiveness]);
        s.trainers_mod = TrainersMod::TypeThemed;
    }),
    ("tweak_bars_distortion_rotom", |s| {
        s.current_misc_tweaks = tweak_mask(&[
            MiscTweak::FasterHpAndExpBars,
            MiscTweak::FastDistortionWorld,
            MiscTweak::UpdateRotomFormeTyping,
        ])
    }),
    ("intro_and_starter_items", |s| {
        s.randomize_starters_held_items = true;
        s.ban_bad_random_starter_held_items = true;
        s.starters_mod = StartersMod::CompletelyRandom;
    }),
    ("limit_pokemon_gen1_gen4", |s| {
        s.limit_pokemon = true;
        s.current_restrictions = Some(gen_restrictions(true, true));
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.trainers_mod = TrainersMod::Random;
        s.static_pokemon_mod = StaticPokemonMod::RandomMatching;
    }),
    ("race_mode_ban_irregular", |s| {
        s.race_mode = true;
        s.ban_irregular_alt_formes = true;
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.allow_wild_alt_formes = true;
    }),
    ("kitchen_sink", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Random;
        s.base_stats_follow_evolutions = true;
        s.abilities_mod = AbilitiesMod::Randomize;
        s.types_mod = TypesMod::RandomFollowEvolutions;
        s.evolutions_mod = EvolutionsMod::Random;
        s.change_impossible_evolutions = true;
        s.starters_mod = StartersMod::RandomWithTwoEvolutions;
        s.randomize_move_powers = true;
        s.movesets_mod = MovesetsMod::RandomPreferSameType;
        s.tms_mod = TmsMod::Random;
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::RandomPreferType;
        s.move_tutor_moves_mod = MoveTutorMovesMod::Random;
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::RandomPreferType;
        s.trainers_mod = TrainersMod::MainPlaythrough;
        s.rival_carries_starter_throughout = true;
        s.randomize_trainer_names = true;
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.use_time_based_encounters = true;
        s.static_pokemon_mod = StaticPokemonMod::RandomMatching;
        s.correct_static_music = true;
        s.in_game_trades_mod = InGameTradesMod::RandomizeGivenAndRequested;
        s.field_items_mod = FieldItemsMod::RandomEven;
        s.shop_items_mod = ShopItemsMod::Random;
        s.pickup_items_mod = PickupItemsMod::Random;
        s.current_misc_tweaks = tweak_mask(&[
            MiscTweak::FastestText,
            MiscTweak::NationalDexAtStart,
            MiscTweak::UpdateTypeEffectiveness,
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

const GEN4_TWEAKS: &[MiscTweak] = &[
    MiscTweak::FastestText,
    MiscTweak::BanLuckyEgg,
    MiscTweak::UpdateTypeEffectiveness,
    MiscTweak::NationalDexAtStart,
    MiscTweak::RunWithoutRunningShoes,
    MiscTweak::LowerCasePokemonNames,
    MiscTweak::RandomizeCatchingTutorial,
    MiscTweak::FasterHpAndExpBars,
    MiscTweak::FastDistortionWorld,
    MiscTweak::UpdateRotomFormeTyping,
];

fn fuzz_pokemon(dice: &mut Dice, s: &mut Settings) {
    s.change_impossible_evolutions = dice.chance(40);
    s.make_evolutions_easier = dice.chance(40);
    s.remove_time_based_evolutions = dice.chance(40);
    s.base_statistics_mod = dice.pick(BaseStatisticsMod::ALL);
    s.base_stats_follow_evolutions = dice.chance(50);
    s.abilities_mod = dice.pick(AbilitiesMod::ALL);
    s.abilities_follow_evolutions = dice.chance(50);
    s.ban_bad_abilities = dice.chance(30);
    s.types_mod = dice.pick(TypesMod::ALL);
    s.evolutions_mod = dice.pick(EvolutionsMod::ALL);
    s.evos_similar_strength = dice.chance(40);
    s.evos_same_typing = dice.chance(40);
    s.starters_mod = dice.pick(StartersMod::ALL);
    s.custom_starters = [dice.range(1, 493), dice.range(1, 493), dice.range(1, 493)];
    s.rival_carries_starter_throughout = dice.chance(40);
}

fn fuzz_moves(dice: &mut Dice, s: &mut Settings) {
    s.randomize_move_powers = dice.chance(30);
    s.randomize_move_types = dice.chance(30);
    s.randomize_move_category = dice.chance(30);
    s.movesets_mod = dice.pick(MovesetsMod::ALL);
    s.start_with_guaranteed_moves = dice.chance(40);
    s.guaranteed_move_count = dice.range(2, 4);
    s.tms_mod = dice.pick(TmsMod::ALL);
    s.tms_hms_compatibility_mod = dice.pick(TmsHmsCompatibilityMod::ALL);
    s.move_tutor_moves_mod = dice.pick(MoveTutorMovesMod::ALL);
    s.move_tutors_compatibility_mod = dice.pick(MoveTutorsCompatibilityMod::ALL);
}

fn fuzz_world(dice: &mut Dice, s: &mut Settings) {
    s.trainers_mod = dice.pick(TrainersMod::ALL);
    s.trainers_use_pokemon_of_similar_strength = dice.chance(40);
    s.randomize_trainer_names = dice.chance(30);
    s.randomize_held_items_for_regular_trainer_pokemon = dice.chance(30);
    s.wild_pokemon_mod = dice.pick(WildPokemonMod::ALL);
    s.wild_pokemon_restriction_mod = dice.pick(WildPokemonRestrictionMod::ALL);
    s.use_time_based_encounters = dice.chance(50);
    s.static_pokemon_mod = dice.pick(StaticPokemonMod::ALL);
    s.correct_static_music = dice.chance(50);
    s.in_game_trades_mod = dice.pick(InGameTradesMod::ALL);
    s.field_items_mod = dice.pick(FieldItemsMod::ALL);
    s.shop_items_mod = dice.pick(ShopItemsMod::ALL);
    s.pickup_items_mod = dice.pick(PickupItemsMod::ALL);
    s.current_misc_tweaks = GEN4_TWEAKS
        .iter()
        .filter(|_| dice.chance(25))
        .fold(0, |mask, tweak| mask | tweak.value());
}

pub fn fuzz_grid(rom_name: &str, count: usize) -> Vec<(String, Settings)> {
    (0..count)
        .map(|index| {
            let mut dice = Dice::new(index as u64 + 1);
            let mut settings = base(rom_name);
            fuzz_pokemon(&mut dice, &mut settings);
            fuzz_moves(&mut dice, &mut settings);
            fuzz_world(&mut dice, &mut settings);
            (format!("fuzz_{index:03}"), settings)
        })
        .collect()
}
