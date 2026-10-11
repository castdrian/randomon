use randomon_settings::{ExpCurve, GenRestrictions, MiscTweak, Settings};

use crate::dice::Dice;

#[derive(Clone, Copy, Debug)]
pub struct GameShape {
    pub generation: i32,
    pub species_count: i32,
}

pub fn random_settings(dice: &mut Dice, rom_name: &str, shape: GameShape) -> Settings {
    let mut settings = Settings {
        rom_name: Some(rom_name.to_owned()),
        selected_exp_curve: Some(ExpCurve::MediumFast),
        ..Settings::default()
    };
    general(dice, &mut settings);
    pokemon_traits(dice, &mut settings);
    starters(dice, &mut settings, shape);
    moves(dice, &mut settings);
    trainers(dice, &mut settings);
    wild(dice, &mut settings);
    statics_and_totems(dice, &mut settings);
    tms_and_tutors(dice, &mut settings);
    trades_and_items(dice, &mut settings);
    settings.current_misc_tweaks = random_misc_tweaks(dice);
    settings
}

fn general(dice: &mut Dice, s: &mut Settings) {
    s.change_impossible_evolutions = dice.coin();
    s.make_evolutions_easier = dice.coin();
    s.remove_time_based_evolutions = dice.coin();
    s.race_mode = dice.chance(20);
    s.block_broken_moves = dice.coin();
    s.ban_irregular_alt_formes = dice.coin();
    s.dual_type_only = dice.chance(20);
    if dice.chance(30) {
        s.limit_pokemon = true;
        s.current_restrictions = Some(GenRestrictions::from_state(dice.range(0, 255)));
    }
}

fn pokemon_traits(dice: &mut Dice, s: &mut Settings) {
    s.base_statistics_mod = dice.variant();
    s.base_stats_follow_evolutions = dice.coin();
    s.base_stats_follow_mega_evolutions = dice.coin();
    s.assign_evo_stats_randomly = dice.coin();
    if dice.chance(30) {
        s.update_base_stats = true;
        s.update_base_stats_to_generation = dice.range(6, 9);
    }
    if dice.chance(30) {
        s.standardize_exp_curves = true;
        s.exp_curve_mod = dice.variant();
        s.selected_exp_curve = Some(dice.variant());
    }
    s.abilities_mod = dice.variant();
    s.allow_wonder_guard = dice.coin();
    s.abilities_follow_evolutions = dice.coin();
    s.abilities_follow_mega_evolutions = dice.coin();
    s.ban_trapping_abilities = dice.coin();
    s.ban_negative_abilities = dice.coin();
    s.ban_bad_abilities = dice.coin();
    s.weigh_duplicate_abilities_together = dice.coin();
    s.ensure_two_abilities = dice.coin();
    s.types_mod = dice.variant();
    s.types_follow_mega_evolutions = dice.coin();
    s.evolutions_mod = dice.variant();
    s.evos_similar_strength = dice.coin();
    s.evos_same_typing = dice.coin();
    s.evos_max_three_stages = dice.coin();
    s.evos_force_change = dice.coin();
    s.evos_allow_alt_formes = dice.coin();
}

fn starters(dice: &mut Dice, s: &mut Settings, shape: GameShape) {
    s.starters_mod = dice.variant();
    s.allow_starter_alt_formes = dice.coin();
    let highest = shape.species_count.max(1);
    s.custom_starters = [
        dice.range(1, highest),
        dice.range(1, highest),
        dice.range(1, highest),
    ];
    if dice.chance(5) {
        s.custom_starters[dice.below(3) as usize] = highest + 50;
    }
    s.randomize_starters_held_items = dice.coin();
    s.ban_bad_random_starter_held_items = dice.coin();
    s.limit_main_game_legendaries = dice.coin();
    s.limit600 = dice.chance(30);
}

fn moves(dice: &mut Dice, s: &mut Settings) {
    s.randomize_move_powers = dice.coin();
    s.randomize_move_accuracies = dice.coin();
    s.randomize_move_pps = dice.coin();
    s.randomize_move_types = dice.coin();
    s.randomize_move_category = dice.coin();
    if dice.chance(30) {
        s.update_moves = true;
        s.update_moves_to_generation = dice.range(2, 9);
        s.update_moves_legacy = dice.chance(20);
    }
    s.movesets_mod = dice.variant();
    s.start_with_guaranteed_moves = dice.coin();
    s.guaranteed_move_count = dice.range(2, 4);
    s.reorder_damaging_moves = dice.coin();
    if dice.coin() {
        s.movesets_force_good_damaging = true;
        s.movesets_good_damaging_percent = dice.range(0, 20) * 5;
    }
    s.block_broken_moveset_moves = dice.coin();
    s.evolution_moves_for_all = dice.coin();
}

fn trainers(dice: &mut Dice, s: &mut Settings) {
    s.trainers_mod = dice.variant();
    s.rival_carries_starter_throughout = dice.coin();
    s.trainers_use_pokemon_of_similar_strength = dice.coin();
    s.trainers_match_typing_distribution = dice.coin();
    s.trainers_block_legendaries = dice.coin();
    s.trainers_block_early_wonder_guard = dice.coin();
    s.trainers_enforce_distribution = dice.coin();
    s.trainers_enforce_main_playthrough = dice.coin();
    s.randomize_trainer_names = dice.coin();
    s.randomize_trainer_class_names = dice.coin();
    if dice.chance(40) {
        s.trainers_force_fully_evolved = true;
        s.trainers_force_fully_evolved_level = dice.range(30, 65);
    }
    if dice.chance(40) {
        s.trainers_level_modified = true;
        s.trainers_level_modifier = dice.range(-50, 50);
    }
    s.elite_four_unique_pokemon_number = dice.range(0, 2);
    s.allow_trainer_alternate_formes = dice.coin();
    s.swap_trainer_mega_evos = dice.coin();
    s.additional_boss_trainer_pokemon = additional_count(dice);
    s.additional_important_trainer_pokemon = additional_count(dice);
    s.additional_regular_trainer_pokemon = additional_count(dice);
    s.randomize_held_items_for_boss_trainer_pokemon = dice.coin();
    s.randomize_held_items_for_important_trainer_pokemon = dice.coin();
    s.randomize_held_items_for_regular_trainer_pokemon = dice.coin();
    s.consumable_items_only_for_trainer_pokemon = dice.coin();
    s.sensible_items_only_for_trainer_pokemon = dice.coin();
    s.highest_level_only_gets_items_for_trainer_pokemon = dice.coin();
    s.double_battle_mode = dice.chance(30);
    s.shiny_chance = dice.coin();
    s.better_trainer_movesets = dice.coin();
}

fn additional_count(dice: &mut Dice) -> i32 {
    if dice.chance(40) { dice.range(1, 5) } else { 0 }
}

fn wild(dice: &mut Dice, s: &mut Settings) {
    s.wild_pokemon_mod = dice.variant();
    s.wild_pokemon_restriction_mod = dice.variant();
    s.use_time_based_encounters = dice.coin();
    s.block_wild_legendaries = dice.coin();
    if dice.chance(40) {
        s.use_minimum_catch_rate = true;
        s.minimum_catch_rate_level = dice.range(1, 5);
    }
    s.randomize_wild_pokemon_held_items = dice.coin();
    s.ban_bad_random_wild_pokemon_held_items = dice.coin();
    s.balance_shaking_grass = dice.coin();
    if dice.chance(40) {
        s.wild_levels_modified = true;
        s.wild_level_modifier = dice.range(-50, 50);
    }
    s.allow_wild_alt_formes = dice.coin();
}

fn statics_and_totems(dice: &mut Dice, s: &mut Settings) {
    s.static_pokemon_mod = dice.variant();
    s.allow_static_alt_formes = dice.coin();
    s.swap_static_mega_evos = dice.coin();
    if dice.chance(40) {
        s.static_level_modified = true;
        s.static_level_modifier = dice.range(-50, 50);
    }
    s.correct_static_music = dice.coin();
    s.totem_pokemon_mod = dice.variant();
    s.ally_pokemon_mod = dice.variant();
    s.aura_mod = dice.variant();
    s.randomize_totem_held_items = dice.coin();
    if dice.chance(40) {
        s.totem_levels_modified = true;
        s.totem_level_modifier = dice.range(-50, 50);
    }
    s.allow_totem_alt_formes = dice.coin();
}

fn tms_and_tutors(dice: &mut Dice, s: &mut Settings) {
    s.tms_mod = dice.variant();
    s.tm_level_up_move_sanity = dice.coin();
    s.keep_field_move_tms = dice.coin();
    s.full_hm_compat = dice.chance(30);
    if dice.coin() {
        s.tms_force_good_damaging = true;
        s.tms_good_damaging_percent = dice.range(0, 20) * 5;
    }
    s.block_broken_tm_moves = dice.coin();
    s.tms_follow_evolutions = dice.coin();
    s.tms_hms_compatibility_mod = dice.variant();
    s.move_tutor_moves_mod = dice.variant();
    s.tutor_level_up_move_sanity = dice.coin();
    s.keep_field_move_tutors = dice.coin();
    if dice.coin() {
        s.tutors_force_good_damaging = true;
        s.tutors_good_damaging_percent = dice.range(0, 20) * 5;
    }
    s.block_broken_tutor_moves = dice.coin();
    s.tutor_follow_evolutions = dice.coin();
    s.move_tutors_compatibility_mod = dice.variant();
}

fn trades_and_items(dice: &mut Dice, s: &mut Settings) {
    s.in_game_trades_mod = dice.variant();
    s.randomize_in_game_trades_nicknames = dice.coin();
    s.randomize_in_game_trades_ots = dice.coin();
    s.randomize_in_game_trades_ivs = dice.coin();
    s.randomize_in_game_trades_items = dice.coin();
    s.field_items_mod = dice.variant();
    s.ban_bad_random_field_items = dice.coin();
    s.shop_items_mod = dice.variant();
    s.ban_bad_random_shop_items = dice.coin();
    s.ban_regular_shop_items = dice.coin();
    s.ban_op_shop_items = dice.coin();
    s.balance_shop_prices = dice.coin();
    s.guarantee_evolution_items = dice.coin();
    s.guarantee_x_items = dice.coin();
    s.pickup_items_mod = dice.variant();
    s.ban_bad_random_pickup_items = dice.coin();
}

fn random_misc_tweaks(dice: &mut Dice) -> i32 {
    MiscTweak::ALL
        .iter()
        .filter(|_| dice.chance(35))
        .fold(0, |mask, tweak| mask | tweak.value())
}
