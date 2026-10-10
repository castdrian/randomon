use randomon_settings::Settings;

use super::visibility as v;
use crate::capabilities::Capabilities;

pub fn settings_for_output(
    state: &Settings,
    caps: &Capabilities,
    rom_name: Option<&str>,
) -> Settings {
    let mut out = state.clone();
    out.block_broken_moves = false;
    out.update_moves_legacy = false;
    out.updated_from_old_version = false;
    if let Some(name) = rom_name {
        out.rom_name = Some(name.to_owned());
    }
    if caps.has_game() {
        hide_unavailable(&mut out, caps);
    }
    out
}

fn hide_unavailable(s: &mut Settings, caps: &Capabilities) {
    s.ban_irregular_alt_formes &= v::generation_4_plus(caps);
    s.update_moves &= v::generation_below_8(caps);
    s.base_stats_follow_mega_evolutions &= v::mega_evolutions(caps);
    s.types_follow_mega_evolutions &= v::mega_evolutions(caps);
    s.randomize_starters_held_items &= v::starter_held_items(caps);
    s.ban_bad_random_starter_held_items &= v::starter_held_items(caps);
    s.allow_starter_alt_formes &= v::starter_alt_formes(caps);
    s.evos_allow_alt_formes &= v::generation_7_plus(caps);
    s.start_with_guaranteed_moves &= v::four_starting_moves(caps);
    s.evolution_moves_for_all &= v::generation_7_plus(caps);
    hide_unavailable_trainer_options(s, caps);
    s.randomize_wild_pokemon_held_items &= v::not_generation_1(caps);
    s.ban_bad_random_wild_pokemon_held_items &= v::not_generation_1(caps);
    s.balance_shaking_grass &= v::generation_5(caps);
    s.allow_wild_alt_formes &= v::wild_alt_formes(caps);
    s.limit_main_game_legendaries &= v::main_game_legendaries(caps);
    s.allow_static_alt_formes &= v::static_alt_formes(caps);
    s.swap_static_mega_evos &= v::static_mega_swaps(caps);
    s.correct_static_music &= v::static_music_fix(caps);
    s.full_hm_compat &= v::generation_below_7(caps);
}

fn hide_unavailable_trainer_options(s: &mut Settings, caps: &Capabilities) {
    let later_generation = v::generation_3_plus(caps);
    if !later_generation {
        s.elite_four_unique_pokemon_number = 0;
        s.additional_boss_trainer_pokemon = 0;
        s.additional_important_trainer_pokemon = 0;
        s.additional_regular_trainer_pokemon = 0;
    }
    s.allow_trainer_alternate_formes &= v::functional_formes(caps);
    s.swap_trainer_mega_evos &= v::mega_evolutions(caps);
    s.double_battle_mode &= later_generation;
    s.shiny_chance &= v::generation_7_plus(caps);
    s.better_trainer_movesets &= later_generation;
    s.randomize_held_items_for_boss_trainer_pokemon &= later_generation;
    s.randomize_held_items_for_important_trainer_pokemon &= later_generation;
    s.randomize_held_items_for_regular_trainer_pokemon &= later_generation;
    s.consumable_items_only_for_trainer_pokemon &= later_generation;
    s.sensible_items_only_for_trainer_pokemon &= later_generation;
    s.highest_level_only_gets_items_for_trainer_pokemon &= later_generation;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_options_are_left_out_like_upstream() {
        let state = Settings {
            randomize_move_category: true,
            shiny_chance: true,
            double_battle_mode: true,
            balance_shaking_grass: true,
            ..Settings::default()
        };
        let output = settings_for_output(&state, &Capabilities::for_generation(1), Some("Red (U)"));
        assert!(!output.shiny_chance);
        assert!(!output.double_battle_mode);
        assert!(!output.balance_shaking_grass);
        assert!(output.randomize_move_category);
        assert_eq!(output.rom_name.as_deref(), Some("Red (U)"));
    }

    #[test]
    fn previews_keep_every_option() {
        let state = Settings {
            shiny_chance: true,
            ..Settings::default()
        };
        assert!(settings_for_output(&state, &Capabilities::preview(), None).shiny_chance);
    }
}
