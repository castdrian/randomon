use randomon_settings::{
    AbilitiesMod, MoveTutorMovesMod, MoveTutorsCompatibilityMod, Settings, ShopItemsMod,
    StaticPokemonMod, TrainersMod,
};

use crate::handler::RomHandler;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TweakForRomFeedback {
    pub changed_starter: bool,
    pub removed_code_tweaks: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandlerTraits {
    pub generation: i32,
    pub is_gen1: bool,
    pub is_gen2: bool,
    pub is_gen3: bool,
    pub is_gen5: bool,
}

impl HandlerTraits {
    pub fn for_generation(generation: i32) -> Self {
        Self {
            generation,
            is_gen1: generation == 1,
            is_gen2: generation == 2,
            is_gen3: generation == 3,
            is_gen5: generation == 5,
        }
    }
}

pub fn tweak_for_rom(settings: &mut Settings, handler: &dyn RomHandler) -> TweakForRomFeedback {
    let traits = HandlerTraits::for_generation(handler.generation_of_pokemon());
    let mut feedback = TweakForRomFeedback::default();
    if settings.update_moves_legacy && traits.is_gen5 {
        settings.update_moves_legacy = false;
        settings.update_moves = false;
    }
    let rom_pokemon = if handler.has_starter_alt_formes() {
        handler.get_pokemon_incl_formes()
    } else {
        handler.get_pokemon()
    };
    let rom_starters = handler.get_starters();
    for starter in 0..3 {
        let custom = settings.custom_starters[starter];
        if custom < 0 || custom as usize >= rom_pokemon.len() {
            feedback.changed_starter = true;
            settings.custom_starters[starter] = match rom_starters.get(starter) {
                None => 1,
                Some(&original) => rom_pokemon
                    .iter()
                    .position(|&pokemon| pokemon == Some(original))
                    .map_or(-1, |index| index as i32),
            };
        }
    }
    if traits.is_gen1 || (traits.is_gen3 && !handler.is_rom_valid()) {
        settings.current_restrictions = None;
        settings.limit_pokemon = false;
    } else if let Some(restrictions) = settings.current_restrictions.as_mut() {
        restrictions.limit_to_gen(handler.generation_of_pokemon());
    }
    if handler.generation_of_pokemon() != 5 && settings.trainers_mod == TrainersMod::MainPlaythrough
    {
        settings.trainers_mod = TrainersMod::Random;
    }
    let old_tweaks = settings.current_misc_tweaks;
    settings.current_misc_tweaks &= handler.misc_tweaks_available();
    if old_tweaks != settings.current_misc_tweaks {
        feedback.removed_code_tweaks = true;
    }
    if handler.abilities_per_pokemon() == 0 {
        settings.abilities_mod = AbilitiesMod::Unchanged;
        settings.allow_wonder_guard = false;
    }
    if !handler.supports_starter_held_items() {
        settings.randomize_starters_held_items = false;
        settings.ban_bad_random_starter_held_items = false;
    }
    if !handler.supports_four_starting_moves() {
        settings.start_with_guaranteed_moves = false;
    }
    if traits.is_gen1 || traits.is_gen2 {
        settings.trainers_block_early_wonder_guard = false;
    }
    if !handler.has_time_based_encounters() {
        settings.use_time_based_encounters = false;
    }
    if traits.is_gen1 {
        settings.randomize_wild_pokemon_held_items = false;
        settings.ban_bad_random_wild_pokemon_held_items = false;
    }
    if !handler.can_change_static_pokemon() {
        settings.static_pokemon_mod = StaticPokemonMod::Unchanged;
    }
    if !handler.has_move_tutors() {
        settings.move_tutor_moves_mod = MoveTutorMovesMod::Unchanged;
        settings.move_tutors_compatibility_mod = MoveTutorsCompatibilityMod::Unchanged;
        settings.tutor_level_up_move_sanity = false;
        settings.keep_field_move_tutors = false;
    }
    if traits.is_gen1 {
        settings.randomize_in_game_trades_items = false;
        settings.randomize_in_game_trades_ivs = false;
        settings.randomize_in_game_trades_ots = false;
    }
    if !handler.has_physical_special_split() {
        settings.randomize_move_category = false;
    }
    if !handler.has_shop_randomization() {
        settings.shop_items_mod = ShopItemsMod::Unchanged;
    }
    feedback
}
