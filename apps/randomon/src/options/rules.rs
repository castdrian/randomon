use randomon_settings::{
    AbilitiesMod, AllyPokemonMod, BaseStatisticsMod, EvolutionsMod, ExpCurveMod, FieldItemsMod,
    InGameTradesMod, MoveTutorMovesMod, MoveTutorsCompatibilityMod, MovesetsMod, PickupItemsMod,
    Settings, ShopItemsMod, StartersMod, StaticPokemonMod, TmsHmsCompatibilityMod, TmsMod,
    TotemPokemonMod, TrainersMod, TypesMod, WildPokemonMod, WildPokemonRestrictionMod,
};

use crate::capabilities::Capabilities;

const FORCE_FULLY_EVOLVED_MINIMUM: i32 = 30;
const GUARANTEED_MOVES_MINIMUM: i32 = 2;
const MINIMUM_CATCH_RATE_MINIMUM: i32 = 1;

type Caps = Capabilities;

pub fn evolve_every_level(s: &Settings) -> bool {
    s.evolutions_mod == EvolutionsMod::RandomEveryLevel
}

pub fn follow_evolution_controls(s: &Settings) -> bool {
    !evolve_every_level(s)
}

pub fn follow_mega_evolution_controls(s: &Settings) -> bool {
    !(evolve_every_level(s) && !s.ban_irregular_alt_formes && s.evos_allow_alt_formes)
}

pub fn not_every_level(s: &Settings, _: &Caps) -> bool {
    !evolve_every_level(s)
}

pub fn base_stats_follow_evolutions(s: &Settings, _: &Caps) -> bool {
    s.base_statistics_mod != BaseStatisticsMod::Unchanged && follow_evolution_controls(s)
}

pub fn base_stats_follow_megas(s: &Settings, _: &Caps) -> bool {
    s.base_statistics_mod != BaseStatisticsMod::Unchanged && follow_mega_evolution_controls(s)
}

pub fn assign_evo_stats_randomly(s: &Settings, _: &Caps) -> bool {
    s.base_statistics_mod == BaseStatisticsMod::Random
        && (s.base_stats_follow_evolutions || s.base_stats_follow_mega_evolutions)
}

pub fn update_base_stats(_: &Settings, caps: &Caps) -> bool {
    caps.generation_below(crate::capabilities::HIGHEST_POKEMON_GENERATION)
}

pub fn standardize_exp_curves(s: &Settings, _: &Caps) -> bool {
    s.standardize_exp_curves
}

pub fn types_randomized(s: &Settings, _: &Caps) -> bool {
    s.types_mod != TypesMod::Unchanged
}

pub fn types_follow_megas(s: &Settings, caps: &Caps) -> bool {
    types_randomized(s, caps) && follow_mega_evolution_controls(s)
}

pub fn abilities_randomized(s: &Settings, _: &Caps) -> bool {
    s.abilities_mod == AbilitiesMod::Randomize
}

pub fn abilities_follow_evolutions(s: &Settings, caps: &Caps) -> bool {
    abilities_randomized(s, caps) && follow_evolution_controls(s)
}

pub fn abilities_follow_megas(s: &Settings, caps: &Caps) -> bool {
    abilities_randomized(s, caps) && follow_mega_evolution_controls(s)
}

pub fn evolutions_random(s: &Settings, _: &Caps) -> bool {
    s.evolutions_mod == EvolutionsMod::Random
}

pub fn evolutions_randomized(s: &Settings, _: &Caps) -> bool {
    s.evolutions_mod != EvolutionsMod::Unchanged
}

pub fn custom_starters(s: &Settings, _: &Caps) -> bool {
    s.starters_mod == StartersMod::Custom
}

pub fn starters_randomized(s: &Settings, _: &Caps) -> bool {
    s.starters_mod != StartersMod::Unchanged
}

pub fn starter_held_items_randomized(s: &Settings, _: &Caps) -> bool {
    s.randomize_starters_held_items
}

pub fn statics_randomized(s: &Settings, _: &Caps) -> bool {
    s.static_pokemon_mod != StaticPokemonMod::Unchanged
}

pub fn statics_similar_strength(s: &Settings, caps: &Caps) -> bool {
    s.static_pokemon_mod == StaticPokemonMod::SimilarStrength
        && crate::options::visibility::main_game_legendaries(caps)
}

pub fn trades_randomized(s: &Settings, _: &Caps) -> bool {
    s.in_game_trades_mod != InGameTradesMod::Unchanged
}

pub fn movesets_randomized(s: &Settings, _: &Caps) -> bool {
    !matches!(
        s.movesets_mod,
        MovesetsMod::Unchanged | MovesetsMod::MetronomeOnly
    )
}

pub fn trainers_randomized(s: &Settings, _: &Caps) -> bool {
    s.trainers_mod != TrainersMod::Unchanged
}

pub fn trainers_swap_megas(s: &Settings, caps: &Caps) -> bool {
    trainers_randomized(s, caps) && restrictions_allow_mega_swaps(s, caps, false)
}

pub fn restrictions_allow_mega_swaps(s: &Settings, caps: &Caps, include_elite_four: bool) -> bool {
    let type_themed = s.trainers_mod == TrainersMod::TypeThemed
        || (include_elite_four && s.trainers_mod == TrainersMod::TypeThemedElite4Gyms);
    s.current_restrictions.is_none_or(|restrictions| {
        restrictions
            .allow_trainer_swap_mega_evolvables(caps.force_swap_static_mega_evos, type_themed)
    })
}

pub fn better_trainer_movesets(_: &Settings, caps: &Caps) -> bool {
    caps.generation_at_least(3)
}

pub fn trainer_held_items_chosen(s: &Settings, _: &Caps) -> bool {
    s.randomize_held_items_for_boss_trainer_pokemon
        || s.randomize_held_items_for_important_trainer_pokemon
        || s.randomize_held_items_for_regular_trainer_pokemon
}

pub fn rival_carries_starter(s: &Settings, _: &Caps) -> bool {
    !evolve_every_level(s)
        && (s.starters_mod != StartersMod::Unchanged || s.trainers_mod != TrainersMod::Unchanged)
}

pub fn trainers_type_themed(s: &Settings, _: &Caps) -> bool {
    s.trainers_mod == TrainersMod::TypeThemed
}

pub fn totem_alt_formes(s: &Settings, _: &Caps) -> bool {
    s.totem_pokemon_mod != TotemPokemonMod::Unchanged
        || s.ally_pokemon_mod != AllyPokemonMod::Unchanged
}

pub fn wild_randomized(s: &Settings, _: &Caps) -> bool {
    s.wild_pokemon_mod != WildPokemonMod::Unchanged
}

pub fn wild_area_rules(s: &Settings, _: &Caps) -> bool {
    matches!(
        s.wild_pokemon_mod,
        WildPokemonMod::Random | WildPokemonMod::AreaMapping
    )
}

pub fn wild_random(s: &Settings, _: &Caps) -> bool {
    s.wild_pokemon_mod == WildPokemonMod::Random
}

pub fn wild_held_items(s: &Settings, caps: &Caps) -> bool {
    s.randomize_wild_pokemon_held_items && caps.generation_is_not(1)
}

pub fn not_metronome(s: &Settings, _: &Caps) -> bool {
    s.movesets_mod != MovesetsMod::MetronomeOnly
}

pub fn tm_level_up_sanity(s: &Settings, caps: &Caps) -> bool {
    not_metronome(s, caps)
        && (s.movesets_mod != MovesetsMod::Unchanged
            || s.tms_mod != TmsMod::Unchanged
            || s.tms_hms_compatibility_mod != TmsHmsCompatibilityMod::Unchanged)
}

pub fn tm_follow_evolutions(s: &Settings, caps: &Caps) -> bool {
    not_metronome(s, caps)
        && (s.tms_hms_compatibility_mod != TmsHmsCompatibilityMod::Unchanged
            || s.tm_level_up_move_sanity)
        && follow_evolution_controls(s)
}

pub fn tm_moves_randomized(s: &Settings, caps: &Caps) -> bool {
    not_metronome(s, caps) && s.tms_mod != TmsMod::Unchanged
}

pub fn full_hm_compatibility(s: &Settings, _: &Caps) -> bool {
    s.tms_hms_compatibility_mod != TmsHmsCompatibilityMod::Full
}

pub fn tutor_moves_radio(s: &Settings, caps: &Caps) -> bool {
    not_metronome(s, caps) && caps.has_move_tutors
}

pub fn tutor_compatibility_radio(_: &Settings, caps: &Caps) -> bool {
    caps.has_move_tutors
}

pub fn tutor_level_up_sanity(s: &Settings, caps: &Caps) -> bool {
    not_metronome(s, caps)
        && caps.has_move_tutors
        && (s.movesets_mod != MovesetsMod::Unchanged
            || s.move_tutor_moves_mod != MoveTutorMovesMod::Unchanged
            || s.move_tutors_compatibility_mod != MoveTutorsCompatibilityMod::Unchanged)
}

pub fn tutor_follow_evolutions(s: &Settings, caps: &Caps) -> bool {
    not_metronome(s, caps)
        && (s.move_tutors_compatibility_mod != MoveTutorsCompatibilityMod::Unchanged
            || s.tutor_level_up_move_sanity)
        && follow_evolution_controls(s)
}

pub fn tutor_moves_randomized(s: &Settings, caps: &Caps) -> bool {
    not_metronome(s, caps)
        && caps.has_move_tutors
        && s.move_tutor_moves_mod != MoveTutorMovesMod::Unchanged
}

pub fn field_items_randomized(s: &Settings, _: &Caps) -> bool {
    matches!(
        s.field_items_mod,
        FieldItemsMod::Random | FieldItemsMod::RandomEven
    )
}

pub fn shop_items_random(s: &Settings, _: &Caps) -> bool {
    s.shop_items_mod == ShopItemsMod::Random
}

pub fn pickup_items_random(s: &Settings, _: &Caps) -> bool {
    s.pickup_items_mod == PickupItemsMod::Random
}

pub fn normalize(s: &mut Settings, caps: &Caps) {
    normalize_evolution_dependants(s);
    normalize_pokemon_traits(s);
    normalize_starters_statics_trades(s);
    normalize_movesets(s);
    normalize_trainers(s, caps);
    normalize_totems(s);
    normalize_wild(s, caps);
    normalize_tms_and_tutors(s, caps);
    normalize_items(s);
    normalize_dropdowns(s, caps);
}

fn normalize_evolution_dependants(s: &mut Settings) {
    if !evolve_every_level(s) {
        return;
    }
    s.base_stats_follow_evolutions = false;
    if s.types_mod == TypesMod::RandomFollowEvolutions {
        s.types_mod = TypesMod::CompletelyRandom;
    }
    if s.starters_mod == StartersMod::RandomWithTwoEvolutions {
        s.starters_mod = StartersMod::CompletelyRandom;
    }
    s.abilities_follow_evolutions = false;
    s.tms_follow_evolutions = false;
    s.tutor_follow_evolutions = false;
    if !follow_mega_evolution_controls(s) {
        s.base_stats_follow_mega_evolutions = false;
        s.types_follow_mega_evolutions = false;
        s.abilities_follow_mega_evolutions = false;
    }
    s.change_impossible_evolutions = false;
    s.make_evolutions_easier = false;
    s.remove_time_based_evolutions = false;
    s.trainers_force_fully_evolved = false;
    s.trainers_force_fully_evolved_level = FORCE_FULLY_EVOLVED_MINIMUM;
}

fn normalize_pokemon_traits(s: &mut Settings) {
    if s.base_statistics_mod == BaseStatisticsMod::Unchanged {
        s.base_stats_follow_evolutions = false;
        s.base_stats_follow_mega_evolutions = false;
    }
    if !(s.base_statistics_mod == BaseStatisticsMod::Random
        && (s.base_stats_follow_evolutions || s.base_stats_follow_mega_evolutions))
    {
        s.assign_evo_stats_randomly = false;
    }
    if !s.standardize_exp_curves {
        s.exp_curve_mod = ExpCurveMod::Legendaries;
    }
    if s.types_mod == TypesMod::Unchanged {
        s.types_follow_mega_evolutions = false;
        s.dual_type_only = false;
    }
    if s.abilities_mod != AbilitiesMod::Randomize {
        s.allow_wonder_guard = false;
        s.abilities_follow_evolutions = false;
        s.ban_trapping_abilities = false;
        s.ban_negative_abilities = false;
        s.ban_bad_abilities = false;
        s.abilities_follow_mega_evolutions = false;
        s.weigh_duplicate_abilities_together = false;
        s.ensure_two_abilities = false;
    }
    match s.evolutions_mod {
        EvolutionsMod::Random => {}
        EvolutionsMod::RandomEveryLevel => {
            s.evos_similar_strength = false;
            s.evos_max_three_stages = false;
        }
        EvolutionsMod::Unchanged => {
            s.evos_similar_strength = false;
            s.evos_same_typing = false;
            s.evos_max_three_stages = false;
            s.evos_force_change = false;
            s.evos_allow_alt_formes = false;
        }
    }
}

fn normalize_starters_statics_trades(s: &mut Settings) {
    if s.starters_mod == StartersMod::Unchanged {
        s.allow_starter_alt_formes = false;
    }
    if !s.randomize_starters_held_items {
        s.ban_bad_random_starter_held_items = false;
    }
    if s.static_pokemon_mod == StaticPokemonMod::Unchanged {
        s.limit600 = false;
        s.allow_static_alt_formes = false;
        s.swap_static_mega_evos = false;
        s.correct_static_music = false;
    }
    if s.static_pokemon_mod != StaticPokemonMod::SimilarStrength {
        s.limit_main_game_legendaries = false;
    }
    if !s.static_level_modified {
        s.static_level_modifier = 0;
    }
    if s.in_game_trades_mod == InGameTradesMod::Unchanged {
        s.randomize_in_game_trades_items = false;
        s.randomize_in_game_trades_ivs = false;
        s.randomize_in_game_trades_nicknames = false;
        s.randomize_in_game_trades_ots = false;
    }
}

fn normalize_movesets(s: &mut Settings) {
    if matches!(
        s.movesets_mod,
        MovesetsMod::MetronomeOnly | MovesetsMod::Unchanged
    ) {
        s.start_with_guaranteed_moves = false;
        s.movesets_force_good_damaging = false;
        s.reorder_damaging_moves = false;
        s.block_broken_moveset_moves = false;
        s.evolution_moves_for_all = false;
    }
    if !s.start_with_guaranteed_moves {
        s.guaranteed_move_count = GUARANTEED_MOVES_MINIMUM;
    }
    if !s.movesets_force_good_damaging {
        s.movesets_good_damaging_percent = 0;
    }
}

fn normalize_trainers(s: &mut Settings, caps: &Caps) {
    if s.trainers_mod == TrainersMod::Unchanged {
        s.trainers_use_pokemon_of_similar_strength = false;
        s.trainers_block_legendaries = false;
        s.trainers_block_early_wonder_guard = false;
        s.allow_trainer_alternate_formes = false;
        s.swap_trainer_mega_evos = false;
        s.shiny_chance = false;
        s.double_battle_mode = false;
        s.additional_boss_trainer_pokemon = 0;
        s.additional_important_trainer_pokemon = 0;
        s.additional_regular_trainer_pokemon = 0;
        s.randomize_held_items_for_boss_trainer_pokemon = false;
        s.randomize_held_items_for_important_trainer_pokemon = false;
        s.randomize_held_items_for_regular_trainer_pokemon = false;
        s.consumable_items_only_for_trainer_pokemon = false;
        s.sensible_items_only_for_trainer_pokemon = false;
        s.highest_level_only_gets_items_for_trainer_pokemon = false;
        s.elite_four_unique_pokemon_number = 0;
    } else if !restrictions_allow_mega_swaps(s, caps, false) {
        s.swap_trainer_mega_evos = false;
    }
    if !s.trainers_force_fully_evolved {
        s.trainers_force_fully_evolved_level = FORCE_FULLY_EVOLVED_MINIMUM;
    }
    if !s.trainers_level_modified {
        s.trainers_level_modifier = 0;
    }
    if !rival_carries_starter(s, caps) {
        s.rival_carries_starter_throughout = false;
    }
    if s.trainers_mod != TrainersMod::TypeThemed {
        s.trainers_match_typing_distribution = false;
    }
}

fn normalize_totems(s: &mut Settings) {
    if s.totem_pokemon_mod == TotemPokemonMod::Unchanged
        && s.ally_pokemon_mod == AllyPokemonMod::Unchanged
    {
        s.allow_totem_alt_formes = false;
    }
    if !s.totem_levels_modified {
        s.totem_level_modifier = 0;
    }
}

fn normalize_wild(s: &mut Settings, caps: &Caps) {
    let restricted_rule = matches!(
        s.wild_pokemon_restriction_mod,
        WildPokemonRestrictionMod::CatchEmAll | WildPokemonRestrictionMod::TypeThemeAreas
    );
    match s.wild_pokemon_mod {
        WildPokemonMod::GlobalMapping if restricted_rule => {
            s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::None;
        }
        WildPokemonMod::Unchanged => {
            s.wild_pokemon_restriction_mod = WildPokemonRestrictionMod::None;
        }
        _ => {}
    }
    if s.wild_pokemon_mod == WildPokemonMod::Unchanged {
        s.use_time_based_encounters = false;
        s.block_wild_legendaries = false;
        s.allow_wild_alt_formes = false;
    }
    if !wild_held_items(s, caps) {
        s.ban_bad_random_wild_pokemon_held_items = false;
    }
    if !s.use_minimum_catch_rate {
        s.minimum_catch_rate_level = MINIMUM_CATCH_RATE_MINIMUM;
    }
    if !s.wild_levels_modified {
        s.wild_level_modifier = 0;
    }
}

fn normalize_tms_and_tutors(s: &mut Settings, caps: &Caps) {
    if s.movesets_mod == MovesetsMod::MetronomeOnly {
        s.tms_mod = TmsMod::Unchanged;
        s.move_tutor_moves_mod = MoveTutorMovesMod::Unchanged;
        clear_tm_extras(s);
        s.tm_level_up_move_sanity = false;
        s.tms_follow_evolutions = false;
        clear_tutor_extras(s);
        s.tutor_level_up_move_sanity = false;
        s.tutor_follow_evolutions = false;
    } else {
        if !tm_level_up_sanity(s, caps) {
            s.tm_level_up_move_sanity = false;
        }
        if !(s.tms_hms_compatibility_mod != TmsHmsCompatibilityMod::Unchanged
            || s.tm_level_up_move_sanity)
        {
            s.tms_follow_evolutions = false;
        }
        if s.tms_mod == TmsMod::Unchanged {
            clear_tm_extras(s);
        }
        if !tutor_level_up_sanity(s, caps) {
            s.tutor_level_up_move_sanity = false;
        }
        if !(s.move_tutors_compatibility_mod != MoveTutorsCompatibilityMod::Unchanged
            || s.tutor_level_up_move_sanity)
        {
            s.tutor_follow_evolutions = false;
        }
        if !tutor_moves_randomized(s, caps) {
            clear_tutor_extras(s);
        }
    }
    if !s.tms_force_good_damaging {
        s.tms_good_damaging_percent = 0;
    }
    if !s.tutors_force_good_damaging {
        s.tutors_good_damaging_percent = 0;
    }
}

fn clear_tm_extras(s: &mut Settings) {
    s.keep_field_move_tms = false;
    s.tms_force_good_damaging = false;
    s.block_broken_tm_moves = false;
}

fn clear_tutor_extras(s: &mut Settings) {
    s.keep_field_move_tutors = false;
    s.tutors_force_good_damaging = false;
    s.block_broken_tutor_moves = false;
}

fn normalize_items(s: &mut Settings) {
    if !matches!(
        s.field_items_mod,
        FieldItemsMod::Random | FieldItemsMod::RandomEven
    ) {
        s.ban_bad_random_field_items = false;
    }
    if s.shop_items_mod != ShopItemsMod::Random {
        s.ban_bad_random_shop_items = false;
        s.ban_regular_shop_items = false;
        s.ban_op_shop_items = false;
        s.balance_shop_prices = false;
        s.guarantee_evolution_items = false;
        s.guarantee_x_items = false;
    }
    if s.pickup_items_mod != PickupItemsMod::Random {
        s.ban_bad_random_pickup_items = false;
    }
}

fn normalize_dropdowns(s: &mut Settings, caps: &Caps) {
    let curves = caps.exp_curves();
    if !s
        .selected_exp_curve
        .is_some_and(|curve| curves.contains(&curve))
    {
        s.selected_exp_curve = curves.first().copied();
    }
    s.update_base_stats_to_generation = snap_to_list(
        s.update_base_stats_to_generation,
        &caps.base_stat_update_generations(),
    );
    s.update_moves_to_generation = snap_to_list(
        s.update_moves_to_generation,
        &caps.move_update_generations(),
    );
    if !caps.trainer_modes().contains(&s.trainers_mod) {
        s.trainers_mod = TrainersMod::Unchanged;
    }
}

fn snap_to_list(value: i32, options: &[i32]) -> i32 {
    match (options.first(), options.last()) {
        (Some(&first), Some(&last)) => value.clamp(first, last),
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalized(mut settings: Settings) -> Settings {
        normalize(&mut settings, &Capabilities::for_generation(5));
        settings
    }

    #[test]
    fn evolve_every_level_clears_follow_evolution_options() {
        let settings = normalized(Settings {
            evolutions_mod: EvolutionsMod::RandomEveryLevel,
            base_statistics_mod: BaseStatisticsMod::Random,
            base_stats_follow_evolutions: true,
            types_mod: TypesMod::RandomFollowEvolutions,
            starters_mod: StartersMod::RandomWithTwoEvolutions,
            change_impossible_evolutions: true,
            trainers_force_fully_evolved: true,
            trainers_force_fully_evolved_level: 50,
            ..Settings::default()
        });
        assert!(!settings.base_stats_follow_evolutions);
        assert_eq!(settings.types_mod, TypesMod::CompletelyRandom);
        assert_eq!(settings.starters_mod, StartersMod::CompletelyRandom);
        assert!(!settings.change_impossible_evolutions);
        assert!(!settings.trainers_force_fully_evolved);
        assert_eq!(settings.trainers_force_fully_evolved_level, 30);
    }

    #[test]
    fn unchanged_abilities_clear_wonder_guard_like_upstream() {
        let settings = normalized(Settings::default());
        assert!(!settings.allow_wonder_guard);
        assert!(!settings.trainers_block_legendaries);
        assert!(!settings.block_wild_legendaries);
    }

    #[test]
    fn metronome_resets_tm_and_tutor_moves() {
        let settings = normalized(Settings {
            movesets_mod: MovesetsMod::MetronomeOnly,
            tms_mod: TmsMod::Random,
            keep_field_move_tms: true,
            move_tutor_moves_mod: MoveTutorMovesMod::Random,
            ..Settings::default()
        });
        assert_eq!(settings.tms_mod, TmsMod::Unchanged);
        assert_eq!(settings.move_tutor_moves_mod, MoveTutorMovesMod::Unchanged);
        assert!(!settings.keep_field_move_tms);
    }

    #[test]
    fn global_mapping_drops_area_only_rules() {
        let settings = normalized(Settings {
            wild_pokemon_mod: WildPokemonMod::GlobalMapping,
            wild_pokemon_restriction_mod: WildPokemonRestrictionMod::CatchEmAll,
            ..Settings::default()
        });
        assert_eq!(
            settings.wild_pokemon_restriction_mod,
            WildPokemonRestrictionMod::None
        );
    }

    #[test]
    fn disabled_trainer_item_filters_keep_their_selection() {
        let settings = normalized(Settings {
            trainers_mod: TrainersMod::Random,
            consumable_items_only_for_trainer_pokemon: true,
            ..Settings::default()
        });
        assert!(settings.consumable_items_only_for_trainer_pokemon);
    }

    #[test]
    fn dropdowns_snap_into_the_game_lists() {
        let settings = normalized(Settings::default());
        assert_eq!(
            settings.selected_exp_curve,
            Some(randomon_settings::ExpCurve::MediumFast)
        );
        assert_eq!(settings.update_base_stats_to_generation, 6);
        assert_eq!(settings.update_moves_to_generation, 6);
    }
}
