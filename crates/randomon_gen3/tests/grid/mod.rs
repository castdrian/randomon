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

const GEN3_TWEAKS: &[MiscTweak] = &[
    MiscTweak::FastestText,
    MiscTweak::RunningShoesIndoors,
    MiscTweak::RandomizePcPotion,
    MiscTweak::NationalDexAtStart,
    MiscTweak::UpdateTypeEffectiveness,
    MiscTweak::LowerCasePokemonNames,
    MiscTweak::RandomizeCatchingTutorial,
    MiscTweak::BanLuckyEgg,
    MiscTweak::BalanceStaticLevels,
    MiscTweak::RunWithoutRunningShoes,
];

const CASES: &[(&str, Tweak)] = &[
    ("defaults", |_| {}),
    ("stats_shuffle_follow", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Shuffle;
        s.base_stats_follow_evolutions = true;
    }),
    ("stats_random_follow_assign", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Random;
        s.base_stats_follow_evolutions = true;
        s.assign_evo_stats_randomly = true;
    }),
    ("stats_update_gen9", |s| {
        s.update_base_stats = true;
        s.update_base_stats_to_generation = 9;
    }),
    ("exp_curves_all", |s| {
        s.standardize_exp_curves = true;
        s.exp_curve_mod = ExpCurveMod::All;
        s.selected_exp_curve = Some(ExpCurve::Erratic);
    }),
    ("abilities_random", |s| {
        s.abilities_mod = AbilitiesMod::Randomize
    }),
    ("abilities_random_everything", |s| {
        s.abilities_mod = AbilitiesMod::Randomize;
        s.allow_wonder_guard = true;
        s.abilities_follow_evolutions = true;
        s.ban_trapping_abilities = true;
        s.ban_negative_abilities = true;
        s.ban_bad_abilities = true;
        s.weigh_duplicate_abilities_together = true;
        s.ensure_two_abilities = true;
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
    }),
    ("evolutions_every_level", |s| {
        s.evolutions_mod = EvolutionsMod::RandomEveryLevel
    }),
    ("evolutions_all_fixes", |s| {
        s.change_impossible_evolutions = true;
        s.make_evolutions_easier = true;
        s.remove_time_based_evolutions = true;
    }),
    ("starters_custom", |s| {
        s.starters_mod = StartersMod::Custom;
        s.custom_starters = [386, 25, 151];
    }),
    ("starters_custom_out_of_range", |s| {
        s.starters_mod = StartersMod::Custom;
        s.custom_starters = [900, -4, 3];
    }),
    ("starters_random_items", |s| {
        s.starters_mod = StartersMod::CompletelyRandom;
        s.randomize_starters_held_items = true;
        s.ban_bad_random_starter_held_items = true;
    }),
    ("starters_two_evos_rival", |s| {
        s.starters_mod = StartersMod::RandomWithTwoEvolutions;
        s.rival_carries_starter_throughout = true;
        s.trainers_mod = TrainersMod::Random;
    }),
    ("moves_everything", |s| {
        s.randomize_move_powers = true;
        s.randomize_move_accuracies = true;
        s.randomize_move_pps = true;
        s.randomize_move_types = true;
        s.update_moves = true;
        s.update_moves_to_generation = 9;
    }),
    ("movesets_random_options", |s| {
        s.movesets_mod = MovesetsMod::CompletelyRandom;
        s.start_with_guaranteed_moves = true;
        s.guaranteed_move_count = 4;
        s.movesets_force_good_damaging = true;
        s.movesets_good_damaging_percent = 50;
        s.block_broken_moveset_moves = true;
        s.evolution_moves_for_all = true;
    }),
    ("movesets_same_type_reorder", |s| {
        s.movesets_mod = MovesetsMod::RandomPreferSameType;
        s.reorder_damaging_moves = true;
    }),
    ("movesets_metronome", |s| {
        s.movesets_mod = MovesetsMod::MetronomeOnly
    }),
    ("tms_and_tutors_random", |s| {
        s.tms_mod = TmsMod::Random;
        s.keep_field_move_tms = true;
        s.tms_force_good_damaging = true;
        s.tms_good_damaging_percent = 40;
        s.move_tutor_moves_mod = MoveTutorMovesMod::Random;
        s.keep_field_move_tutors = true;
        s.tutors_force_good_damaging = true;
        s.tutors_good_damaging_percent = 40;
    }),
    ("tm_and_tutor_compatibility", |s| {
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::RandomPreferType;
        s.tm_level_up_move_sanity = true;
        s.tms_follow_evolutions = true;
        s.full_hm_compat = true;
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::CompletelyRandom;
        s.tutor_level_up_move_sanity = true;
        s.tutor_follow_evolutions = true;
    }),
    ("full_compatibility", |s| {
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::Full;
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::Full;
    }),
    ("trainers_type_themed_gyms", |s| {
        s.trainers_mod = TrainersMod::TypeThemedElite4Gyms;
        s.trainers_use_pokemon_of_similar_strength = true;
    }),
    ("trainers_distributed", |s| {
        s.trainers_mod = TrainersMod::Distributed;
        s.trainers_enforce_distribution = true;
    }),
    ("trainers_heavy", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.trainers_force_fully_evolved = true;
        s.trainers_force_fully_evolved_level = 30;
        s.trainers_level_modified = true;
        s.trainers_level_modifier = 50;
        s.additional_boss_trainer_pokemon = 5;
        s.additional_important_trainer_pokemon = 5;
        s.additional_regular_trainer_pokemon = 5;
        s.double_battle_mode = true;
    }),
    ("trainers_held_items", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.randomize_held_items_for_boss_trainer_pokemon = true;
        s.randomize_held_items_for_important_trainer_pokemon = true;
        s.randomize_held_items_for_regular_trainer_pokemon = true;
        s.sensible_items_only_for_trainer_pokemon = true;
    }),
    ("trainers_better_movesets", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.better_trainer_movesets = true;
    }),
    ("trainers_elite_unique_names", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.elite_four_unique_pokemon_number = 2;
        s.randomize_trainer_names = true;
        s.randomize_trainer_class_names = true;
    }),
    ("wild_random_catch_em_all", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::CatchEmAll;
        s.randomize_wild_pokemon_held_items = true;
        s.ban_bad_random_wild_pokemon_held_items = true;
    }),
    ("wild_area_type_themed", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::TypeThemeAreas;
    }),
    ("wild_global_similar_levels", |s| {
        s.wild_pokemon_mod = WildPokemonMod::GlobalMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::SimilarStrength;
        s.wild_levels_modified = true;
        s.wild_level_modifier = -30;
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = 5;
    }),
    ("statics_random_music", |s| {
        s.static_pokemon_mod = StaticPokemonMod::CompletelyRandom;
        s.correct_static_music = true;
        s.static_level_modified = true;
        s.static_level_modifier = 30;
    }),
    ("statics_matching_legendaries", |s| {
        s.static_pokemon_mod = StaticPokemonMod::RandomMatching;
        s.limit_main_game_legendaries = true;
        s.limit600 = true;
    }),
    ("statics_similar", |s| {
        s.static_pokemon_mod = StaticPokemonMod::SimilarStrength
    }),
    ("trades_everything", |s| {
        s.in_game_trades_mod = InGameTradesMod::RandomizeGivenAndRequested;
        s.randomize_in_game_trades_nicknames = true;
        s.randomize_in_game_trades_ots = true;
        s.randomize_in_game_trades_ivs = true;
        s.randomize_in_game_trades_items = true;
    }),
    ("field_items_random_even", |s| {
        s.field_items_mod = FieldItemsMod::RandomEven;
        s.ban_bad_random_field_items = true;
    }),
    ("field_items_shuffle", |s| {
        s.field_items_mod = FieldItemsMod::Shuffle
    }),
    ("shops_random_balanced", |s| {
        s.shop_items_mod = ShopItemsMod::Random;
        s.ban_bad_random_shop_items = true;
        s.ban_regular_shop_items = true;
        s.ban_op_shop_items = true;
        s.balance_shop_prices = true;
        s.guarantee_evolution_items = true;
        s.guarantee_x_items = true;
    }),
    ("shops_shuffle", |s| {
        s.shop_items_mod = ShopItemsMod::Shuffle
    }),
    ("pickup_random", |s| {
        s.pickup_items_mod = PickupItemsMod::Random;
        s.ban_bad_random_pickup_items = true;
    }),
    ("tweak_all", |s| {
        s.current_misc_tweaks = tweak_mask(GEN3_TWEAKS)
    }),
    ("limit_hoenn_relatives", |s| {
        s.limit_pokemon = true;
        s.current_restrictions = Some(GenRestrictions {
            allow_gen3: true,
            allow_evolutionary_relatives: true,
            ..GenRestrictions::default()
        });
    }),
    ("kitchen_sink", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Random;
        s.abilities_mod = AbilitiesMod::Randomize;
        s.types_mod = TypesMod::CompletelyRandom;
        s.evolutions_mod = EvolutionsMod::Random;
        s.starters_mod = StartersMod::CompletelyRandom;
        s.movesets_mod = MovesetsMod::CompletelyRandom;
        s.tms_mod = TmsMod::Random;
        s.move_tutor_moves_mod = MoveTutorMovesMod::Random;
        s.trainers_mod = TrainersMod::TypeThemed;
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.static_pokemon_mod = StaticPokemonMod::CompletelyRandom;
        s.in_game_trades_mod = InGameTradesMod::RandomizeGiven;
        s.field_items_mod = FieldItemsMod::Random;
        s.shop_items_mod = ShopItemsMod::Random;
        s.pickup_items_mod = PickupItemsMod::Random;
        s.current_misc_tweaks = tweak_mask(GEN3_TWEAKS);
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

fn fuzz_pokemon_traits(dice: &mut Dice, s: &mut Settings) {
    s.change_impossible_evolutions = dice.chance(40);
    s.make_evolutions_easier = dice.chance(40);
    s.remove_time_based_evolutions = dice.chance(30);
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
    s.abilities_mod = dice.pick(AbilitiesMod::ALL);
    s.allow_wonder_guard = dice.chance(30);
    s.abilities_follow_evolutions = dice.chance(50);
    s.ban_trapping_abilities = dice.chance(30);
    s.ban_negative_abilities = dice.chance(30);
    s.ban_bad_abilities = dice.chance(30);
    s.weigh_duplicate_abilities_together = dice.chance(30);
    s.ensure_two_abilities = dice.chance(30);
    s.types_mod = dice.pick(TypesMod::ALL);
    s.dual_type_only = dice.chance(15);
    s.evolutions_mod = dice.pick(EvolutionsMod::ALL);
    s.evos_similar_strength = dice.chance(40);
    s.evos_same_typing = dice.chance(40);
    s.evos_max_three_stages = dice.chance(40);
    s.evos_force_change = dice.chance(40);
    if dice.chance(15) {
        s.limit_pokemon = true;
        s.current_restrictions = Some(GenRestrictions {
            allow_gen1: dice.chance(60),
            allow_gen2: dice.chance(60),
            allow_gen3: dice.chance(60),
            allow_evolutionary_relatives: dice.chance(50),
            ..GenRestrictions::default()
        });
    }
}

fn fuzz_starters_and_moves(dice: &mut Dice, s: &mut Settings) {
    s.starters_mod = dice.pick(StartersMod::ALL);
    s.custom_starters = [dice.range(1, 386), dice.range(1, 386), dice.range(1, 386)];
    s.rival_carries_starter_throughout = dice.chance(40);
    s.randomize_starters_held_items = dice.chance(30);
    s.ban_bad_random_starter_held_items = dice.chance(50);
    s.randomize_move_powers = dice.chance(30);
    s.randomize_move_accuracies = dice.chance(30);
    s.randomize_move_pps = dice.chance(30);
    s.randomize_move_types = dice.chance(30);
    if dice.chance(25) {
        s.update_moves = true;
        s.update_moves_to_generation = dice.range(3, 9);
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
    s.move_tutor_moves_mod = dice.pick(MoveTutorMovesMod::ALL);
    s.keep_field_move_tutors = dice.chance(40);
    s.tutor_level_up_move_sanity = dice.chance(30);
    s.tutor_follow_evolutions = dice.chance(30);
    s.block_broken_tutor_moves = dice.chance(40);
    s.move_tutors_compatibility_mod = dice.pick(MoveTutorsCompatibilityMod::ALL);
}

fn fuzz_trainers(dice: &mut Dice, s: &mut Settings) {
    s.trainers_mod = dice.pick(TrainersMod::ALL);
    s.trainers_use_pokemon_of_similar_strength = dice.chance(40);
    s.trainers_match_typing_distribution = dice.chance(30);
    s.trainers_block_legendaries = dice.chance(60);
    s.trainers_block_early_wonder_guard = dice.chance(40);
    s.trainers_enforce_distribution = dice.chance(20);
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
    s.double_battle_mode = dice.chance(15);
    s.better_trainer_movesets = dice.chance(8);
    s.randomize_held_items_for_boss_trainer_pokemon = dice.chance(10);
    s.randomize_held_items_for_important_trainer_pokemon = dice.chance(10);
    s.randomize_held_items_for_regular_trainer_pokemon = dice.chance(10);
    s.consumable_items_only_for_trainer_pokemon = dice.chance(40);
    s.sensible_items_only_for_trainer_pokemon = dice.chance(40);
    s.highest_level_only_gets_items_for_trainer_pokemon = dice.chance(40);
    if dice.chance(10) {
        s.additional_boss_trainer_pokemon = dice.range(0, 5);
        s.additional_important_trainer_pokemon = dice.range(0, 5);
        s.additional_regular_trainer_pokemon = dice.range(0, 5);
    }
    s.elite_four_unique_pokemon_number = if dice.chance(20) { dice.range(1, 2) } else { 0 };
}

fn fuzz_world(dice: &mut Dice, s: &mut Settings) {
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
    s.limit_main_game_legendaries = dice.chance(30);
    s.limit600 = dice.chance(30);
    s.correct_static_music = dice.chance(40);
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
    s.shop_items_mod = dice.pick(ShopItemsMod::ALL);
    s.ban_bad_random_shop_items = dice.chance(40);
    s.ban_regular_shop_items = dice.chance(30);
    s.ban_op_shop_items = dice.chance(30);
    s.balance_shop_prices = dice.chance(40);
    s.guarantee_evolution_items = dice.chance(30);
    s.guarantee_x_items = dice.chance(30);
    s.pickup_items_mod = dice.pick(PickupItemsMod::ALL);
    s.ban_bad_random_pickup_items = dice.chance(40);
    s.current_misc_tweaks = GEN3_TWEAKS
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
            fuzz_trainers(&mut dice, &mut settings);
            fuzz_world(&mut dice, &mut settings);
            (format!("fuzz_{index:03}"), settings)
        })
        .collect()
}
