use std::collections::HashMap;

use randomon_settings::{GenRestrictions, JavaEnum, Settings};

use crate::json::{Json, normalize_key};

pub struct JavaFields {
    values: HashMap<String, Json>,
}

impl JavaFields {
    pub fn new(object: &Json) -> Self {
        Self {
            values: object.normalized_object(),
        }
    }

    fn value(&self, rust_name: &str) -> &Json {
        self.values
            .get(&normalize_key(rust_name))
            .unwrap_or_else(|| panic!("Java fields have no counterpart for {rust_name}"))
    }

    fn boolean(&self, rust_name: &str) -> bool {
        self.value(rust_name).as_bool()
    }

    fn int(&self, rust_name: &str) -> i32 {
        self.value(rust_name).as_i32()
    }

    fn enumeration<E: JavaEnum>(&self, rust_name: &str) -> E {
        let name = self.value(rust_name).as_string();
        E::from_java_name(&name)
            .unwrap_or_else(|| panic!("unknown {} constant {name}", E::TYPE_NAME))
    }

    fn optional_enum<E: JavaEnum>(&self, rust_name: &str) -> Option<E> {
        (!self.value(rust_name).is_null()).then(|| self.enumeration(rust_name))
    }

    fn optional_string(&self, rust_name: &str) -> Option<String> {
        let value = self.value(rust_name);
        (!value.is_null()).then(|| value.as_string())
    }

    fn optional_restrictions(&self, rust_name: &str) -> Option<GenRestrictions> {
        let value = self.value(rust_name);
        (!value.is_null()).then(|| GenRestrictions::from_state(value.as_i32()))
    }

    fn starters(&self, rust_name: &str) -> [i32; 3] {
        let items = self.value(rust_name).as_array();
        [items[0].as_i32(), items[1].as_i32(), items[2].as_i32()]
    }

    fn count(&self) -> usize {
        self.values.len()
    }
}

macro_rules! settings_field_mapping {
    (
        bools: [$($boolean:ident),* $(,)?],
        ints: [$($int:ident),* $(,)?],
        enums: [$($enumeration:ident),* $(,)?],
    ) => {
        const SPECIAL_FIELD_COUNT: usize = 4;
        const MAPPED_FIELD_COUNT: usize = [$(stringify!($boolean),)* $(stringify!($int),)* $(stringify!($enumeration),)*].len()
            + SPECIAL_FIELD_COUNT;

        pub fn settings_from_java(object: &Json) -> Settings {
            let fields = JavaFields::new(object);
            assert_eq!(fields.count(), MAPPED_FIELD_COUNT, "every Java field must be mapped");
            let mut settings = Settings::default();
            $(settings.$boolean = fields.boolean(stringify!($boolean));)*
            $(settings.$int = fields.int(stringify!($int));)*
            $(settings.$enumeration = fields.enumeration(stringify!($enumeration));)*
            settings.rom_name = fields.optional_string("rom_name");
            settings.current_restrictions = fields.optional_restrictions("current_restrictions");
            settings.selected_exp_curve = fields.optional_enum("selected_exp_curve");
            settings.custom_starters = fields.starters("custom_starters");
            settings
        }

        pub fn assert_matches_java(settings: &Settings, object: &Json, context: &str) {
            let fields = JavaFields::new(object);
            assert_eq!(fields.count(), MAPPED_FIELD_COUNT, "every Java field must be mapped");
            $(assert_eq!(settings.$boolean, fields.boolean(stringify!($boolean)), "{context}: {}", stringify!($boolean));)*
            $(assert_eq!(settings.$int, fields.int(stringify!($int)), "{context}: {}", stringify!($int));)*
            $(assert_eq!(settings.$enumeration, fields.enumeration(stringify!($enumeration)), "{context}: {}", stringify!($enumeration));)*
            assert_eq!(settings.rom_name, fields.optional_string("rom_name"), "{context}: rom_name");
            assert_eq!(
                settings.current_restrictions.map(GenRestrictions::to_state),
                fields.optional_restrictions("current_restrictions").map(GenRestrictions::to_state),
                "{context}: current_restrictions"
            );
            assert_eq!(settings.selected_exp_curve, fields.optional_enum("selected_exp_curve"), "{context}: selected_exp_curve");
            assert_eq!(settings.custom_starters, fields.starters("custom_starters"), "{context}: custom_starters");
        }
    };
}

settings_field_mapping! {
    bools: [
        updated_from_old_version,
        change_impossible_evolutions,
        make_evolutions_easier,
        remove_time_based_evolutions,
        race_mode,
        block_broken_moves,
        limit_pokemon,
        ban_irregular_alt_formes,
        dual_type_only,
        base_stats_follow_evolutions,
        base_stats_follow_mega_evolutions,
        assign_evo_stats_randomly,
        update_base_stats,
        standardize_exp_curves,
        allow_wonder_guard,
        abilities_follow_evolutions,
        abilities_follow_mega_evolutions,
        ban_trapping_abilities,
        ban_negative_abilities,
        ban_bad_abilities,
        weigh_duplicate_abilities_together,
        ensure_two_abilities,
        allow_starter_alt_formes,
        randomize_starters_held_items,
        limit_main_game_legendaries,
        limit600,
        ban_bad_random_starter_held_items,
        types_follow_mega_evolutions,
        evos_similar_strength,
        evos_same_typing,
        evos_max_three_stages,
        evos_force_change,
        evos_allow_alt_formes,
        randomize_move_powers,
        randomize_move_accuracies,
        randomize_move_pps,
        randomize_move_types,
        randomize_move_category,
        update_moves,
        update_moves_legacy,
        start_with_guaranteed_moves,
        reorder_damaging_moves,
        movesets_force_good_damaging,
        block_broken_moveset_moves,
        evolution_moves_for_all,
        rival_carries_starter_throughout,
        trainers_use_pokemon_of_similar_strength,
        trainers_match_typing_distribution,
        trainers_block_legendaries,
        trainers_block_early_wonder_guard,
        trainers_enforce_distribution,
        trainers_enforce_main_playthrough,
        randomize_trainer_names,
        randomize_trainer_class_names,
        trainers_force_fully_evolved,
        trainers_level_modified,
        allow_trainer_alternate_formes,
        swap_trainer_mega_evos,
        randomize_held_items_for_boss_trainer_pokemon,
        randomize_held_items_for_important_trainer_pokemon,
        randomize_held_items_for_regular_trainer_pokemon,
        consumable_items_only_for_trainer_pokemon,
        sensible_items_only_for_trainer_pokemon,
        highest_level_only_gets_items_for_trainer_pokemon,
        double_battle_mode,
        shiny_chance,
        better_trainer_movesets,
        use_time_based_encounters,
        block_wild_legendaries,
        use_minimum_catch_rate,
        randomize_wild_pokemon_held_items,
        ban_bad_random_wild_pokemon_held_items,
        balance_shaking_grass,
        wild_levels_modified,
        allow_wild_alt_formes,
        allow_static_alt_formes,
        swap_static_mega_evos,
        static_level_modified,
        correct_static_music,
        randomize_totem_held_items,
        totem_levels_modified,
        allow_totem_alt_formes,
        tm_level_up_move_sanity,
        keep_field_move_tms,
        full_hm_compat,
        tms_force_good_damaging,
        block_broken_tm_moves,
        tms_follow_evolutions,
        tutor_level_up_move_sanity,
        keep_field_move_tutors,
        tutors_force_good_damaging,
        block_broken_tutor_moves,
        tutor_follow_evolutions,
        randomize_in_game_trades_nicknames,
        randomize_in_game_trades_ots,
        randomize_in_game_trades_ivs,
        randomize_in_game_trades_items,
        ban_bad_random_field_items,
        ban_bad_random_shop_items,
        ban_regular_shop_items,
        ban_op_shop_items,
        balance_shop_prices,
        guarantee_evolution_items,
        guarantee_x_items,
        ban_bad_random_pickup_items,
    ],
    ints: [
        current_misc_tweaks,
        update_base_stats_to_generation,
        update_moves_to_generation,
        guaranteed_move_count,
        movesets_good_damaging_percent,
        trainers_force_fully_evolved_level,
        trainers_level_modifier,
        elite_four_unique_pokemon_number,
        additional_boss_trainer_pokemon,
        additional_important_trainer_pokemon,
        additional_regular_trainer_pokemon,
        minimum_catch_rate_level,
        wild_level_modifier,
        static_level_modifier,
        totem_level_modifier,
        tms_good_damaging_percent,
        tutors_good_damaging_percent,
    ],
    enums: [
        base_statistics_mod,
        exp_curve_mod,
        abilities_mod,
        starters_mod,
        types_mod,
        evolutions_mod,
        movesets_mod,
        trainers_mod,
        wild_pokemon_mod,
        wild_pokemon_restriction_mod,
        static_pokemon_mod,
        totem_pokemon_mod,
        ally_pokemon_mod,
        aura_mod,
        tms_mod,
        tms_hms_compatibility_mod,
        move_tutor_moves_mod,
        move_tutors_compatibility_mod,
        in_game_trades_mod,
        field_items_mod,
        shop_items_mod,
        pickup_items_mod,
    ],
}
