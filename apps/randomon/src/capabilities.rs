use randomon_settings::{ExpCurve, JavaEnum, MiscTweak, RomTweakSupport, TrainersMod};

use crate::rom::Platform;

pub const HIGHEST_POKEMON_GENERATION: i32 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub generation: Option<i32>,
    pub is_yellow: bool,
    pub has_mega_evolutions: bool,
    pub has_starter_alt_formes: bool,
    pub supports_starter_held_items: bool,
    pub can_change_static_pokemon: bool,
    pub has_main_game_legendaries: bool,
    pub has_static_alt_formes: bool,
    pub force_swap_static_mega_evos: bool,
    pub has_static_music_fix: bool,
    pub has_physical_special_split: bool,
    pub supports_four_starting_moves: bool,
    pub has_functional_formes: bool,
    pub has_time_based_encounters: bool,
    pub has_wild_alt_formes: bool,
    pub has_move_tutors: bool,
    pub has_shop_randomization: bool,
    pub abilities_per_pokemon: i32,
    pub misc_tweaks_available: i32,
    pub species_count: usize,
}

impl Capabilities {
    pub fn preview() -> Self {
        Self {
            generation: None,
            is_yellow: false,
            has_mega_evolutions: true,
            has_starter_alt_formes: true,
            supports_starter_held_items: true,
            can_change_static_pokemon: true,
            has_main_game_legendaries: true,
            has_static_alt_formes: true,
            force_swap_static_mega_evos: false,
            has_static_music_fix: true,
            has_physical_special_split: true,
            supports_four_starting_moves: true,
            has_functional_formes: true,
            has_time_based_encounters: true,
            has_wild_alt_formes: true,
            has_move_tutors: true,
            has_shop_randomization: true,
            abilities_per_pokemon: 3,
            misc_tweaks_available: MiscTweak::ALL
                .iter()
                .fold(0, |mask, tweak| mask | tweak.value()),
            species_count: 807,
        }
    }

    pub fn provisional(platform: Platform) -> Self {
        Self::for_generation(match platform {
            Platform::GameBoy => 1,
            Platform::GameBoyColor => 2,
            Platform::GameBoyAdvance => 3,
            Platform::NintendoDs => 4,
            Platform::Nintendo3ds => 6,
        })
    }

    pub fn for_generation(generation: i32) -> Self {
        let generation = generation.clamp(1, 7);
        Self {
            generation: Some(generation),
            is_yellow: false,
            has_mega_evolutions: generation >= 6,
            has_starter_alt_formes: generation >= 6,
            supports_starter_held_items: matches!(generation, 2 | 3 | 4 | 6 | 7),
            can_change_static_pokemon: true,
            has_main_game_legendaries: generation >= 3,
            has_static_alt_formes: generation >= 6,
            force_swap_static_mega_evos: false,
            has_static_music_fix: matches!(generation, 3..=5),
            has_physical_special_split: generation >= 4,
            supports_four_starting_moves: true,
            has_functional_formes: generation >= 4,
            has_time_based_encounters: matches!(generation, 2 | 4 | 5 | 7),
            has_wild_alt_formes: generation >= 5,
            has_move_tutors: generation >= 2,
            has_shop_randomization: generation >= 3,
            abilities_per_pokemon: match generation {
                1 | 2 => 0,
                3 | 4 => 2,
                _ => 3,
            },
            misc_tweaks_available: tweak_support(generation).available_mask(),
            species_count: species_count(generation),
        }
    }

    pub fn has_game(&self) -> bool {
        self.generation.is_some()
    }

    pub fn generation_at_least(&self, generation: i32) -> bool {
        self.generation.is_none_or(|current| current >= generation)
    }

    pub fn generation_below(&self, generation: i32) -> bool {
        self.generation.is_none_or(|current| current < generation)
    }

    pub fn generation_is(&self, generation: i32) -> bool {
        self.generation.is_none_or(|current| current == generation)
    }

    pub fn generation_is_not(&self, generation: i32) -> bool {
        self.generation.is_none_or(|current| current != generation)
    }

    pub fn exp_curves(&self) -> &'static [ExpCurve] {
        const EARLY: &[ExpCurve] = &[
            ExpCurve::MediumFast,
            ExpCurve::MediumSlow,
            ExpCurve::Fast,
            ExpCurve::Slow,
        ];
        const LATER: &[ExpCurve] = &[
            ExpCurve::MediumFast,
            ExpCurve::MediumSlow,
            ExpCurve::Fast,
            ExpCurve::Slow,
            ExpCurve::Erratic,
            ExpCurve::Fluctuating,
        ];
        if self.generation_below(3) && self.has_game() {
            EARLY
        } else {
            LATER
        }
    }

    pub fn base_stat_update_generations(&self) -> Vec<i32> {
        let generation = self.generation.unwrap_or(1);
        let first = 6.max(generation + 1);
        let count = 4.min(HIGHEST_POKEMON_GENERATION - generation).max(0);
        (first..first + count).collect()
    }

    pub fn move_update_generations(&self) -> Vec<i32> {
        let generation = self.generation.unwrap_or(1);
        (generation + 1..=HIGHEST_POKEMON_GENERATION).collect()
    }

    pub fn trainer_modes(&self) -> Vec<TrainersMod> {
        TrainersMod::ALL
            .iter()
            .copied()
            .filter(|mode| *mode != TrainersMod::MainPlaythrough || self.generation_is(5))
            .collect()
    }

    pub fn starter_slots(&self) -> usize {
        if self.is_yellow { 2 } else { 3 }
    }

    pub fn has_misc_tweak(&self, tweak: MiscTweak) -> bool {
        tweak.is_set_in(self.misc_tweaks_available)
    }
}

fn tweak_support(generation: i32) -> RomTweakSupport {
    match generation {
        1 => RomTweakSupport::Gen1 {
            has_bw_exp_tweak: true,
            has_x_accuracy_nerf_tweak: true,
            has_crit_rate_tweak: true,
            has_text_delay_function: true,
            has_pc_potion: true,
            has_pikachu_evolution_jump: false,
            has_catching_tutorial: true,
        },
        2 => RomTweakSupport::Gen2 {
            has_bw_exp_tweak: true,
            has_text_delay_function: true,
            has_catching_tutorial: true,
        },
        3 => RomTweakSupport::Gen3 {
            has_run_indoors_tweak: true,
            has_fastest_text: true,
            has_catching_tutorial: true,
            has_pc_potion: true,
            is_fire_red_leaf_green: false,
        },
        4 => RomTweakSupport::Gen4 {
            has_fastest_text_tweak: true,
            has_national_dex_tweak: true,
            has_fast_distortion_world_tweak: false,
            is_platinum_or_heart_gold_soul_silver: true,
        },
        5 => RomTweakSupport::Gen5 {
            has_fastest_text_tweak: true,
            has_national_dex_tweak: true,
            is_black_white: false,
            is_black2_white2: true,
        },
        6 => RomTweakSupport::Gen6,
        _ => RomTweakSupport::Gen7,
    }
}

fn species_count(generation: i32) -> usize {
    match generation {
        1 => 151,
        2 => 251,
        3 => 386,
        4 => 493,
        5 => 649,
        6 => 721,
        _ => 807,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_lists_match_upstream_dropdowns() {
        let gen1 = Capabilities::for_generation(1);
        assert_eq!(gen1.base_stat_update_generations(), vec![6, 7, 8, 9]);
        assert_eq!(gen1.move_update_generations(), (2..=9).collect::<Vec<_>>());
        let gen7 = Capabilities::for_generation(7);
        assert_eq!(gen7.base_stat_update_generations(), vec![8, 9]);
        assert_eq!(gen7.move_update_generations(), vec![8, 9]);
    }

    #[test]
    fn early_games_offer_four_exp_curves() {
        assert_eq!(Capabilities::for_generation(2).exp_curves().len(), 4);
        assert_eq!(Capabilities::for_generation(3).exp_curves().len(), 6);
        assert_eq!(Capabilities::preview().exp_curves().len(), 6);
    }

    #[test]
    fn main_playthrough_trainers_are_gen5_only() {
        assert!(
            Capabilities::for_generation(5)
                .trainer_modes()
                .contains(&TrainersMod::MainPlaythrough)
        );
        assert!(
            !Capabilities::for_generation(4)
                .trainer_modes()
                .contains(&TrainersMod::MainPlaythrough)
        );
    }
}
