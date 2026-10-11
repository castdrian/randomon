use randomon_settings::*;

pub type Tweak = fn(&mut Settings);

pub fn base(rom_name: &str) -> Settings {
    Settings {
        rom_name: Some(rom_name.to_owned()),
        selected_exp_curve: Some(ExpCurve::MediumFast),
        custom_starters: [1, 1, 1],
        ..Settings::default()
    }
}

pub fn tweak_mask(tweaks: &[MiscTweak]) -> i32 {
    tweaks.iter().fold(0, |mask, tweak| mask | tweak.value())
}

pub const SHARED_CASES: &[(&str, Tweak)] = &[
    ("defaults", |_| {}),
    ("stats_shuffle_follow_megas", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Shuffle;
        s.base_stats_follow_evolutions = true;
        s.base_stats_follow_mega_evolutions = true;
    }),
    ("stats_random_follow_assign", |s| {
        s.base_statistics_mod = BaseStatisticsMod::Random;
        s.base_stats_follow_evolutions = true;
        s.base_stats_follow_mega_evolutions = true;
        s.assign_evo_stats_randomly = true;
    }),
    ("stats_update_gen7", |s| {
        s.update_base_stats = true;
        s.update_base_stats_to_generation = 7;
        s.standardize_exp_curves = true;
        s.exp_curve_mod = ExpCurveMod::StrongLegendaries;
        s.selected_exp_curve = Some(ExpCurve::Fast);
    }),
    ("stats_update_gen9", |s| {
        s.update_base_stats = true;
        s.update_base_stats_to_generation = 9;
    }),
    ("abilities_random", |s| {
        s.abilities_mod = AbilitiesMod::Randomize;
        s.abilities_follow_evolutions = true;
        s.abilities_follow_mega_evolutions = true;
        s.ban_trapping_abilities = true;
        s.ban_negative_abilities = true;
        s.ban_bad_abilities = true;
    }),
    ("abilities_random_wonder_guard", |s| {
        s.abilities_mod = AbilitiesMod::Randomize;
        s.allow_wonder_guard = true;
        s.weigh_duplicate_abilities_together = true;
        s.ensure_two_abilities = true;
    }),
    ("types_follow_megas", |s| {
        s.types_mod = TypesMod::RandomFollowEvolutions;
        s.types_follow_mega_evolutions = true;
        s.dual_type_only = true;
    }),
    ("types_random", |s| s.types_mod = TypesMod::CompletelyRandom),
    ("evolutions_random_alt_formes", |s| {
        s.evolutions_mod = EvolutionsMod::Random;
        s.evos_similar_strength = true;
        s.evos_same_typing = true;
        s.evos_force_change = true;
        s.evos_allow_alt_formes = true;
    }),
    ("evolutions_random_three_stages", |s| {
        s.evolutions_mod = EvolutionsMod::Random;
        s.evos_max_three_stages = true;
        s.evos_force_change = true;
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
        s.movesets_mod = MovesetsMod::CompletelyRandom;
    }),
    ("starters_random_alt_formes_items", |s| {
        s.starters_mod = StartersMod::CompletelyRandom;
        s.allow_starter_alt_formes = true;
        s.randomize_starters_held_items = true;
        s.ban_bad_random_starter_held_items = true;
    }),
    ("starters_two_evos_rival", |s| {
        s.starters_mod = StartersMod::RandomWithTwoEvolutions;
        s.rival_carries_starter_throughout = true;
        s.trainers_mod = TrainersMod::Random;
    }),
    ("starters_custom", |s| {
        s.starters_mod = StartersMod::Custom;
        s.custom_starters = [252, 255, 258];
    }),
    ("moves_all", |s| {
        s.randomize_move_powers = true;
        s.randomize_move_accuracies = true;
        s.randomize_move_pps = true;
        s.randomize_move_types = true;
        s.randomize_move_category = true;
    }),
    ("moves_update_gen7_legacy", |s| {
        s.update_moves = true;
        s.update_moves_to_generation = 7;
        s.update_moves_legacy = true;
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
        s.movesets_force_good_damaging = true;
        s.movesets_good_damaging_percent = 40;
        s.block_broken_moveset_moves = true;
        s.evolution_moves_for_all = true;
    }),
    ("movesets_random", |s| {
        s.movesets_mod = MovesetsMod::CompletelyRandom
    }),
    ("movesets_metronome", |s| {
        s.movesets_mod = MovesetsMod::MetronomeOnly
    }),
    ("tms_tutors_random", |s| {
        s.tms_mod = TmsMod::Random;
        s.keep_field_move_tms = true;
        s.tms_force_good_damaging = true;
        s.tms_good_damaging_percent = 60;
        s.block_broken_tm_moves = true;
        s.move_tutor_moves_mod = MoveTutorMovesMod::Random;
        s.keep_field_move_tutors = true;
        s.tutors_force_good_damaging = true;
        s.tutors_good_damaging_percent = 40;
        s.block_broken_tutor_moves = true;
    }),
    ("tm_tutor_compat_prefer_type", |s| {
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::RandomPreferType;
        s.tms_follow_evolutions = true;
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::RandomPreferType;
        s.tutor_follow_evolutions = true;
    }),
    ("tm_tutor_compat_random_sanity", |s| {
        s.tms_mod = TmsMod::Random;
        s.move_tutor_moves_mod = MoveTutorMovesMod::Random;
        s.movesets_mod = MovesetsMod::CompletelyRandom;
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::CompletelyRandom;
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::CompletelyRandom;
        s.tm_level_up_move_sanity = true;
        s.tutor_level_up_move_sanity = true;
        s.full_hm_compat = true;
    }),
    ("tm_tutor_compat_full", |s| {
        s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::Full;
        s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::Full;
    }),
    ("trainers_random_megas_formes", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.allow_trainer_alternate_formes = true;
        s.swap_trainer_mega_evos = true;
        s.trainers_use_pokemon_of_similar_strength = true;
    }),
    ("trainers_type_themed_gyms", |s| {
        s.trainers_mod = TrainersMod::TypeThemedElite4Gyms;
        s.trainers_block_early_wonder_guard = true;
    }),
    ("trainers_distributed_main", |s| {
        s.trainers_mod = TrainersMod::MainPlaythrough;
        s.trainers_enforce_distribution = true;
    }),
    ("trainers_levels_evolved_names", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.trainers_level_modified = true;
        s.trainers_level_modifier = 30;
        s.trainers_force_fully_evolved = true;
        s.trainers_force_fully_evolved_level = 40;
        s.randomize_trainer_names = true;
        s.randomize_trainer_class_names = true;
    }),
    ("trainers_additional_double", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.additional_boss_trainer_pokemon = 2;
        s.additional_important_trainer_pokemon = 1;
        s.additional_regular_trainer_pokemon = 1;
        s.double_battle_mode = true;
        s.elite_four_unique_pokemon_number = 2;
    }),
    ("trainers_held_items_consumable", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.randomize_held_items_for_boss_trainer_pokemon = true;
        s.randomize_held_items_for_important_trainer_pokemon = true;
        s.randomize_held_items_for_regular_trainer_pokemon = true;
        s.consumable_items_only_for_trainer_pokemon = true;
        s.highest_level_only_gets_items_for_trainer_pokemon = true;
    }),
    ("trainers_held_items_all", |s| {
        s.randomize_held_items_for_boss_trainer_pokemon = true;
        s.randomize_held_items_for_regular_trainer_pokemon = true;
    }),
    ("trainers_better_movesets", |s| {
        s.trainers_mod = TrainersMod::Random;
        s.better_trainer_movesets = true;
        s.shiny_chance = true;
    }),
    ("wild_random_alt_formes_items", |s| {
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.allow_wild_alt_formes = true;
        s.randomize_wild_pokemon_held_items = true;
        s.ban_bad_random_wild_pokemon_held_items = true;
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = 4;
    }),
    ("wild_area_similar", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::SimilarStrength;
        s.wild_levels_modified = true;
        s.wild_level_modifier = 20;
    }),
    ("wild_area_catch_em_all", |s| {
        s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::CatchEmAll;
    }),
    ("wild_global_type_themed", |s| {
        s.wild_pokemon_mod = WildPokemonMod::GlobalMapping;
        s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::TypeThemeAreas;
        s.block_wild_legendaries = false;
    }),
    ("statics_matching_megas", |s| {
        s.static_pokemon_mod = StaticPokemonMod::RandomMatching;
        s.allow_static_alt_formes = true;
        s.swap_static_mega_evos = true;
        s.limit_main_game_legendaries = true;
    }),
    ("statics_similar_levels", |s| {
        s.static_pokemon_mod = StaticPokemonMod::SimilarStrength;
        s.limit600 = true;
        s.static_level_modified = true;
        s.static_level_modifier = -20;
    }),
    ("statics_random", |s| {
        s.static_pokemon_mod = StaticPokemonMod::CompletelyRandom
    }),
    ("trades_all", |s| {
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
    ("shops_random_prices", |s| {
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
    ("limit_pokemon_irregular", |s| {
        s.limit_pokemon = true;
        s.current_restrictions = Some(GenRestrictions::from_state(0b0010_0111));
        s.ban_irregular_alt_formes = true;
        s.wild_pokemon_mod = WildPokemonMod::Random;
        s.trainers_mod = TrainersMod::Random;
    }),
    ("catch_guaranteed", |s| {
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = 5;
    }),
];

pub fn misc_tweak_cases(tweaks: &[MiscTweak]) -> Vec<(String, i32)> {
    let mut cases: Vec<(String, i32)> = tweaks
        .iter()
        .map(|tweak| {
            (
                format!("tweak_{}", tweak.constant_name().to_lowercase()),
                tweak.value(),
            )
        })
        .collect();
    cases.push(("tweak_all".to_owned(), tweak_mask(tweaks)));
    cases
}

pub fn kitchen_sink(s: &mut Settings) {
    s.base_statistics_mod = BaseStatisticsMod::Random;
    s.base_stats_follow_evolutions = true;
    s.abilities_mod = AbilitiesMod::Randomize;
    s.types_mod = TypesMod::RandomFollowEvolutions;
    s.evolutions_mod = EvolutionsMod::Random;
    s.change_impossible_evolutions = true;
    s.starters_mod = StartersMod::RandomWithTwoEvolutions;
    s.randomize_starters_held_items = true;
    s.randomize_move_powers = true;
    s.movesets_mod = MovesetsMod::RandomPreferSameType;
    s.tms_mod = TmsMod::Random;
    s.move_tutor_moves_mod = MoveTutorMovesMod::Random;
    s.tms_hms_compatibility_mod = TmsHmsCompatibilityMod::RandomPreferType;
    s.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::RandomPreferType;
    s.trainers_mod = TrainersMod::TypeThemed;
    s.rival_carries_starter_throughout = true;
    s.randomize_trainer_names = true;
    s.wild_pokemon_mod = WildPokemonMod::AreaMapping;
    s.static_pokemon_mod = StaticPokemonMod::RandomMatching;
    s.in_game_trades_mod = InGameTradesMod::RandomizeGivenAndRequested;
    s.field_items_mod = FieldItemsMod::Random;
    s.shop_items_mod = ShopItemsMod::Random;
    s.pickup_items_mod = PickupItemsMod::Random;
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
    s.base_stats_follow_mega_evolutions = dice.chance(50);
    s.assign_evo_stats_randomly = dice.chance(30);
    if dice.chance(20) {
        s.update_base_stats = true;
        s.update_base_stats_to_generation = dice.range(7, 9);
    }
    if dice.chance(20) {
        s.standardize_exp_curves = true;
        s.exp_curve_mod = dice.pick(ExpCurveMod::ALL);
        s.selected_exp_curve = Some(dice.pick(ExpCurve::ALL));
    }
    s.abilities_mod = dice.pick(AbilitiesMod::ALL);
    s.allow_wonder_guard = dice.chance(30);
    s.abilities_follow_evolutions = dice.chance(50);
    s.abilities_follow_mega_evolutions = dice.chance(50);
    s.ban_trapping_abilities = dice.chance(30);
    s.ban_negative_abilities = dice.chance(30);
    s.ban_bad_abilities = dice.chance(30);
    s.weigh_duplicate_abilities_together = dice.chance(30);
    s.ensure_two_abilities = dice.chance(30);
    s.types_mod = dice.pick(TypesMod::ALL);
    s.types_follow_mega_evolutions = dice.chance(50);
    s.dual_type_only = dice.chance(15);
    s.evolutions_mod = dice.pick(EvolutionsMod::ALL);
    s.evos_similar_strength = dice.chance(40);
    s.evos_same_typing = dice.chance(40);
    s.evos_max_three_stages = dice.chance(30);
    s.evos_force_change = dice.chance(40);
    s.evos_allow_alt_formes = !s.evos_max_three_stages && dice.chance(40);
}

fn fuzz_starters_and_moves(dice: &mut Dice, s: &mut Settings) {
    s.starters_mod = dice.pick(StartersMod::ALL);
    s.custom_starters = [dice.range(1, 721), dice.range(1, 721), dice.range(1, 721)];
    s.allow_starter_alt_formes = dice.chance(40);
    s.randomize_starters_held_items = dice.chance(40);
    s.ban_bad_random_starter_held_items = dice.chance(40);
    s.rival_carries_starter_throughout = dice.chance(40);
    s.randomize_move_powers = dice.chance(30);
    s.randomize_move_accuracies = dice.chance(30);
    s.randomize_move_pps = dice.chance(30);
    s.randomize_move_types = dice.chance(30);
    s.randomize_move_category = dice.chance(30);
    if dice.chance(25) {
        s.update_moves = true;
        s.update_moves_to_generation = dice.range(7, 9);
        s.update_moves_legacy = dice.chance(50);
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
    s.block_broken_tutor_moves = dice.chance(40);
    s.move_tutors_compatibility_mod = dice.pick(MoveTutorsCompatibilityMod::ALL);
    s.tutor_level_up_move_sanity = dice.chance(30);
    s.tutor_follow_evolutions = dice.chance(30);
}

fn fuzz_trainers(dice: &mut Dice, s: &mut Settings) {
    s.trainers_mod = dice.pick(TrainersMod::ALL);
    s.trainers_use_pokemon_of_similar_strength = dice.chance(40);
    s.trainers_match_typing_distribution = dice.chance(30);
    s.trainers_block_legendaries = dice.chance(60);
    s.trainers_block_early_wonder_guard = dice.chance(30);
    s.allow_trainer_alternate_formes = dice.chance(40);
    s.swap_trainer_mega_evos = dice.chance(40);
    if dice.chance(30) {
        s.trainers_level_modified = true;
        s.trainers_level_modifier = dice.range(-50, 50);
    }
    if dice.chance(30) {
        s.trainers_force_fully_evolved = true;
        s.trainers_force_fully_evolved_level = dice.range(30, 65);
    }
    s.additional_boss_trainer_pokemon = dice.range(0, 2);
    s.additional_important_trainer_pokemon = dice.range(0, 2);
    s.additional_regular_trainer_pokemon = dice.range(0, 1);
    s.randomize_held_items_for_boss_trainer_pokemon = dice.chance(30);
    s.randomize_held_items_for_regular_trainer_pokemon = dice.chance(30);
    s.consumable_items_only_for_trainer_pokemon = dice.chance(50);
    s.highest_level_only_gets_items_for_trainer_pokemon = dice.chance(30);
    s.randomize_trainer_names = dice.chance(30);
    s.randomize_trainer_class_names = dice.chance(30);
    s.double_battle_mode = dice.chance(15);
    s.better_trainer_movesets = dice.chance(20);
    s.shiny_chance = dice.chance(10);
}

fn fuzz_world(dice: &mut Dice, s: &mut Settings) {
    s.wild_pokemon_mod = dice.pick(WildPokemonMod::ALL);
    s.wild_pokemon_restriction_mod = dice.pick(WildPokemonRestrictionMod::ALL);
    s.block_wild_legendaries = dice.chance(60);
    s.allow_wild_alt_formes = dice.chance(40);
    s.randomize_wild_pokemon_held_items = dice.chance(30);
    if dice.chance(30) {
        s.wild_levels_modified = true;
        s.wild_level_modifier = dice.range(-50, 50);
    }
    if dice.chance(30) {
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = dice.range(1, 5);
    }
    s.static_pokemon_mod = dice.pick(StaticPokemonMod::ALL);
    s.allow_static_alt_formes = dice.chance(40);
    s.swap_static_mega_evos = dice.chance(40);
    s.limit600 = dice.chance(30);
    s.limit_main_game_legendaries = dice.chance(30);
    if dice.chance(30) {
        s.static_level_modified = true;
        s.static_level_modifier = dice.range(-50, 50);
    }
    s.totem_pokemon_mod = dice.pick(TotemPokemonMod::ALL);
    s.ally_pokemon_mod = dice.pick(AllyPokemonMod::ALL);
    s.aura_mod = dice.pick(AuraMod::ALL);
    s.randomize_totem_held_items = dice.chance(40);
    s.allow_totem_alt_formes = dice.chance(40);
    if dice.chance(30) {
        s.totem_levels_modified = true;
        s.totem_level_modifier = dice.range(-30, 30);
    }
    s.in_game_trades_mod = dice.pick(InGameTradesMod::ALL);
    s.randomize_in_game_trades_nicknames = dice.chance(50);
    s.randomize_in_game_trades_ivs = dice.chance(50);
    s.randomize_in_game_trades_items = dice.chance(50);
    s.field_items_mod = dice.pick(FieldItemsMod::ALL);
    s.ban_bad_random_field_items = dice.chance(40);
    s.shop_items_mod = dice.pick(ShopItemsMod::ALL);
    s.ban_bad_random_shop_items = dice.chance(40);
    s.ban_regular_shop_items = dice.chance(30);
    s.ban_op_shop_items = dice.chance(30);
    s.balance_shop_prices = dice.chance(40);
    s.guarantee_evolution_items = dice.chance(40);
    s.guarantee_x_items = dice.chance(40);
    s.pickup_items_mod = dice.pick(PickupItemsMod::ALL);
    s.ban_bad_random_pickup_items = dice.chance(40);
}

pub fn fuzz_grid(rom_name: &str, count: usize, tweaks: &[MiscTweak]) -> Vec<(String, Settings)> {
    (0..count)
        .map(|index| {
            let mut dice = Dice::new(index as u64 + 1);
            let mut settings = base(rom_name);
            fuzz_pokemon_traits(&mut dice, &mut settings);
            fuzz_starters_and_moves(&mut dice, &mut settings);
            fuzz_trainers(&mut dice, &mut settings);
            fuzz_world(&mut dice, &mut settings);
            settings.current_misc_tweaks = tweaks
                .iter()
                .filter(|_| dice.chance(30))
                .fold(0, |mask, tweak| mask | tweak.value());
            (format!("fuzz_{index:03}"), settings)
        })
        .collect()
}
