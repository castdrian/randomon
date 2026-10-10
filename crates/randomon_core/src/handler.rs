use std::collections::BTreeMap;

use randomon_data::{
    EncounterSet, IngameTrade, ItemList, MegaEvolutionId, MoveLearnt, PickupItem, PokemonId, Shop,
    StatChange, StaticEncounter, TotemPokemon, Trainer, TrainerPokemon, Type,
};
use randomon_log::LogWriter;
use randomon_settings::{MiscTweak, Settings};

use crate::engine;
use crate::error::Result;
use crate::state::{EvolutionUpdates, HandlerCore};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainerNameMode {
    SameLength,
    MaxLength,
    MaxLengthWithClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

pub type Compatibility = BTreeMap<PokemonId, Vec<bool>>;
pub type Movesets = BTreeMap<i32, Vec<MoveLearnt>>;
pub type EggMoves = BTreeMap<i32, Vec<i32>>;

pub trait RomHandler {
    fn core(&self) -> &HandlerCore;
    fn core_mut(&mut self) -> &mut HandlerCore;

    fn save_rom(&mut self) -> Result<Vec<u8>>;
    fn save_rom_directory(&mut self) -> Result<Vec<OutputFile>> {
        Ok(Vec::new())
    }
    fn loaded_filename(&self) -> &str;
    fn has_game_update_loaded(&self) -> bool {
        false
    }
    fn game_update_version(&self) -> Option<String> {
        None
    }
    fn print_rom_diagnostics(&self, log: &mut LogWriter);
    fn is_rom_valid(&self) -> bool;

    fn get_pokemon(&self) -> Vec<Option<PokemonId>>;
    fn get_pokemon_incl_formes(&self) -> Vec<Option<PokemonId>>;
    fn get_alt_formes(&self) -> Vec<PokemonId>;
    fn get_mega_evolutions(&self) -> Vec<MegaEvolutionId>;
    fn get_alt_forme_of_pokemon(&self, pokemon: PokemonId, forme: i32) -> PokemonId;
    fn get_irregular_formes(&self) -> Vec<PokemonId>;

    fn set_pokemon_pool(&mut self, settings: Option<&Settings>) {
        engine::pool::set_pokemon_pool(self, settings)
    }
    fn remove_evos_for_pokemon_pool(&mut self);

    fn get_starters(&self) -> Vec<PokemonId>;
    fn set_starters(&mut self, starters: &[PokemonId]) -> Result<bool>;
    fn has_starter_alt_formes(&self) -> bool;
    fn starter_count(&self) -> i32;
    fn custom_starters(&mut self, settings: &Settings) -> Result<()> {
        engine::starters::custom_starters(self, settings)
    }
    fn randomize_starters(&mut self, settings: &Settings) -> Result<()> {
        engine::starters::randomize_starters(self, settings)
    }
    fn randomize_basic_two_evos_starters(&mut self, settings: &Settings) -> Result<()> {
        engine::starters::randomize_basic_two_evos_starters(self, settings)
    }
    fn get_picked_starters(&self) -> Vec<PokemonId> {
        self.core().engine.picked_starters.clone()
    }
    fn supports_starter_held_items(&self) -> bool;
    fn get_starter_held_items(&self) -> Vec<i32>;
    fn set_starter_held_items(&mut self, items: &[i32]);
    fn randomize_starter_held_items(&mut self, settings: &Settings) -> Result<()> {
        engine::items::randomize_starter_held_items(self, settings)
    }

    fn shuffle_pokemon_stats(&mut self, settings: &Settings) -> Result<()> {
        engine::stats::shuffle_pokemon_stats(self, settings)
    }
    fn randomize_pokemon_stats(&mut self, settings: &Settings) -> Result<()> {
        engine::stats::randomize_pokemon_stats(self, settings)
    }
    fn update_pokemon_stats(&mut self, settings: &Settings) {
        engine::stats::update_pokemon_stats(self, settings)
    }
    fn get_updated_pokemon_stats(&self, generation: i32) -> BTreeMap<i32, StatChange>;
    fn standardize_exp_curves(&mut self, settings: &Settings) {
        engine::stats::standardize_exp_curves(self, settings)
    }

    fn type_in_game(&self, kind: Type) -> bool {
        !kind.is_hack_only() && !(kind == Type::Fairy && self.generation_of_pokemon() < 6)
    }
    fn randomize_pokemon_types(&mut self, settings: &Settings) -> Result<()> {
        engine::types::randomize_pokemon_types(self, settings)
    }

    fn abilities_per_pokemon(&self) -> i32;
    fn highest_ability_index(&self) -> i32;
    fn ability_name(&self, _number: i32) -> String {
        String::new()
    }
    fn randomize_abilities(&mut self, settings: &Settings) -> Result<()> {
        engine::abilities::randomize_abilities(self, settings)
    }
    fn get_ability_variations(&self) -> BTreeMap<i32, Vec<i32>>;
    fn get_useless_abilities(&self) -> Vec<i32> {
        Vec::new()
    }
    fn get_ability_for_trainer_pokemon(&self, _pokemon: &TrainerPokemon) -> i32 {
        0
    }
    fn has_mega_evolutions(&self) -> bool;

    fn get_encounters(&self, use_time_of_day: bool) -> Vec<EncounterSet>;
    fn set_encounters(&mut self, use_time_of_day: bool, encounters: &[EncounterSet]) -> Result<()>;
    fn random_encounters(&mut self, settings: &Settings) -> Result<()> {
        engine::wild::random_encounters(self, settings)
    }
    fn area_1to1_encounters(&mut self, settings: &Settings) -> Result<()> {
        engine::wild::area_1to1_encounters(self, settings)
    }
    fn game_1to1_encounters(&mut self, settings: &Settings) -> Result<()> {
        engine::wild::game_1to1_encounters(self, settings)
    }
    fn only_change_wild_levels(&mut self, settings: &Settings) -> Result<()> {
        engine::wild::only_change_wild_levels(self, settings)
    }
    fn has_time_based_encounters(&self) -> bool {
        false
    }
    fn has_wild_alt_formes(&self) -> bool;
    fn banned_for_wild_encounters(&self) -> Vec<PokemonId> {
        Vec::new()
    }
    fn randomize_wild_held_items(&mut self, settings: &Settings) -> Result<()> {
        engine::items::randomize_wild_held_items(self, settings)
    }
    fn change_catch_rates(&mut self, settings: &Settings) {
        engine::stats::change_catch_rates(self, settings)
    }
    fn minimum_catch_rate(&mut self, rate_non_legendary: i32, rate_legendary: i32) {
        engine::stats::minimum_catch_rate(self, rate_non_legendary, rate_legendary)
    }
    fn enable_guaranteed_pokemon_catching(&mut self);

    fn get_trainers(&self) -> Result<Vec<Trainer>>;
    fn get_main_playthrough_trainers(&self) -> Vec<i32>;
    fn get_elite_four_trainers(&self, is_challenge_mode: bool) -> Vec<i32>;
    fn set_trainers(&mut self, trainers: &[Trainer], double_battle_mode: bool) -> Result<()>;
    fn randomize_trainer_pokes(&mut self, settings: &Settings) -> Result<()> {
        engine::trainers::randomize_trainer_pokes(self, settings)
    }
    fn randomize_trainer_held_items(&mut self, settings: &Settings) -> Result<()> {
        engine::trainers::randomize_trainer_held_items(self, settings)
    }
    fn get_sensible_held_items_for(
        &self,
        _pokemon: &TrainerPokemon,
        _consumable_only: bool,
        _moveset: &[i32; 4],
    ) -> Vec<i32> {
        vec![0]
    }
    fn get_all_consumable_held_items(&self) -> Vec<i32> {
        vec![0]
    }
    fn get_all_held_items(&self) -> Vec<i32> {
        vec![0]
    }
    fn rival_carries_starter(&mut self) -> Result<()> {
        engine::trainers::rival_carries_starter(self)
    }
    fn has_rival_final_battle(&self) -> bool {
        false
    }
    fn force_fully_evolved_trainer_pokes(&mut self, settings: &Settings) -> Result<()> {
        engine::trainers::force_fully_evolved_trainer_pokes(self, settings)
    }
    fn only_change_trainer_levels(&mut self, settings: &Settings) -> Result<()> {
        engine::trainers::only_change_trainer_levels(self, settings)
    }
    fn add_trainer_pokemon(&mut self, settings: &Settings) -> Result<()> {
        engine::trainers::add_trainer_pokemon(self, settings)
    }
    fn double_battle_mode(&mut self) -> Result<()> {
        engine::trainers::double_battle_mode(self)
    }
    fn pick_trainer_movesets(&mut self, settings: &Settings) -> Result<()> {
        engine::trainer_movesets::pick_trainer_movesets(self, settings)
    }

    fn randomize_move_powers(&mut self) {
        engine::move_data::randomize_move_powers(self)
    }
    fn randomize_move_pps(&mut self) {
        engine::move_data::randomize_move_pps(self)
    }
    fn randomize_move_accuracies(&mut self) {
        engine::move_data::randomize_move_accuracies(self)
    }
    fn randomize_move_types(&mut self) {
        engine::move_data::randomize_move_types(self)
    }
    fn has_physical_special_split(&self) -> bool;
    fn randomize_move_category(&mut self) {
        engine::move_data::randomize_move_category(self)
    }
    fn update_moves(&mut self, settings: &Settings) {
        engine::move_data::update_moves(self, settings)
    }
    fn init_move_updates(&mut self) {
        self.core_mut().engine.move_updates = Some(BTreeMap::new());
    }

    fn get_moves_learnt(&self) -> Movesets;
    fn set_moves_learnt(&mut self, movesets: &Movesets) -> Result<()>;
    fn get_moves_banned_from_levelup(&self) -> Vec<i32> {
        Vec::new()
    }
    fn get_egg_moves(&self) -> EggMoves;
    fn set_egg_moves(&mut self, egg_moves: &EggMoves) -> Result<()>;
    fn randomize_moves_learnt(&mut self, settings: &Settings) -> Result<()> {
        engine::movesets::randomize_moves_learnt(self, settings)
    }
    fn randomize_egg_moves(&mut self, settings: &Settings) -> Result<()> {
        engine::movesets::randomize_egg_moves(self, settings)
    }
    fn order_damaging_moves_by_damage(&mut self) -> Result<()> {
        engine::movesets::order_damaging_moves_by_damage(self)
    }
    fn metronome_only_mode(&mut self) -> Result<()> {
        engine::movesets::metronome_only_mode(self)
    }
    fn supports_four_starting_moves(&self) -> bool;

    fn get_static_pokemon(&self) -> Vec<StaticEncounter>;
    fn set_static_pokemon(&mut self, statics: &[StaticEncounter]) -> Result<bool>;
    fn randomize_static_pokemon(&mut self, settings: &Settings) -> Result<()> {
        engine::statics::randomize_static_pokemon(self, settings)
    }
    fn can_change_static_pokemon(&self) -> bool;
    fn has_static_alt_formes(&self) -> bool;
    fn banned_for_static_pokemon(&self) -> Vec<PokemonId> {
        Vec::new()
    }
    fn force_swap_static_mega_evos(&self) -> bool {
        false
    }
    fn only_change_static_levels(&mut self, settings: &Settings) -> Result<()> {
        engine::statics::only_change_static_levels(self, settings)
    }
    fn has_main_game_legendaries(&self) -> bool;
    fn get_main_game_legendaries(&self) -> Vec<i32>;
    fn get_special_music_statics(&self) -> Vec<i32>;
    fn apply_correct_static_music(&mut self, changes: &BTreeMap<i32, i32>);
    fn has_static_music_fix(&self) -> bool;

    fn get_totem_pokemon(&self) -> Vec<TotemPokemon>;
    fn set_totem_pokemon(&mut self, totems: &[TotemPokemon]) -> Result<()>;
    fn randomize_totem_pokemon(&mut self, settings: &Settings) -> Result<()> {
        engine::statics::randomize_totem_pokemon(self, settings)
    }

    fn get_tm_moves(&self) -> Vec<i32>;
    fn get_hm_moves(&self) -> Vec<i32>;
    fn set_tm_moves(&mut self, moves: &[i32]) -> Result<()>;
    fn randomize_tm_moves(&mut self, settings: &Settings) -> Result<()> {
        engine::tms::randomize_tm_moves(self, settings)
    }
    fn get_tm_count(&self) -> i32;
    fn get_hm_count(&self) -> i32;
    fn get_tmhm_compatibility(&self) -> Compatibility;
    fn set_tmhm_compatibility(&mut self, compatibility: &Compatibility);
    fn randomize_tmhm_compatibility(&mut self, settings: &Settings) -> Result<()> {
        engine::tms::randomize_tmhm_compatibility(self, settings)
    }
    fn full_tmhm_compatibility(&mut self) {
        engine::tms::full_tmhm_compatibility(self)
    }
    fn ensure_tm_compat_sanity(&mut self) {
        engine::tms::ensure_tm_compat_sanity(self)
    }
    fn ensure_tm_evolution_sanity(&mut self) -> Result<()> {
        engine::tms::ensure_tm_evolution_sanity(self)
    }
    fn full_hm_compatibility(&mut self) {
        engine::tms::full_hm_compatibility(self)
    }
    fn copy_tm_compatibility_to_cosmetic_formes(&mut self) {
        engine::tms::copy_tm_compatibility_to_cosmetic_formes(self)
    }

    fn has_move_tutors(&self) -> bool;
    fn get_move_tutor_moves(&self) -> Vec<i32>;
    fn set_move_tutor_moves(&mut self, moves: &[i32]) -> Result<()>;
    fn randomize_move_tutor_moves(&mut self, settings: &Settings) -> Result<()> {
        engine::tms::randomize_move_tutor_moves(self, settings)
    }
    fn get_move_tutor_compatibility(&self) -> Compatibility;
    fn set_move_tutor_compatibility(&mut self, compatibility: &Compatibility);
    fn randomize_move_tutor_compatibility(&mut self, settings: &Settings) -> Result<()> {
        engine::tms::randomize_move_tutor_compatibility(self, settings)
    }
    fn full_move_tutor_compatibility(&mut self) {
        engine::tms::full_move_tutor_compatibility(self)
    }
    fn ensure_move_tutor_compat_sanity(&mut self) {
        engine::tms::ensure_move_tutor_compat_sanity(self)
    }
    fn ensure_move_tutor_evolution_sanity(&mut self) -> Result<()> {
        engine::tms::ensure_move_tutor_evolution_sanity(self)
    }
    fn copy_move_tutor_compatibility_to_cosmetic_formes(&mut self) {
        engine::tms::copy_move_tutor_compatibility_to_cosmetic_formes(self)
    }

    fn can_change_trainer_text(&self) -> bool;
    fn get_trainer_names(&self) -> Vec<String>;
    fn set_trainer_names(&mut self, names: &[String]);
    fn trainer_name_mode(&self) -> TrainerNameMode;
    fn max_trainer_name_length(&self) -> i32 {
        i32::MAX
    }
    fn max_sum_of_trainer_name_lengths(&self) -> i32 {
        i32::MAX
    }
    fn get_tc_name_lengths_by_trainer(&self) -> Vec<i32>;
    fn randomize_trainer_names(&mut self, settings: &Settings) -> Result<()> {
        engine::names::randomize_trainer_names(self, settings)
    }
    fn get_trainer_class_names(&self) -> Vec<String>;
    fn set_trainer_class_names(&mut self, names: &[String]);
    fn fixed_trainer_class_names_length(&self) -> bool;
    fn max_trainer_class_name_length(&self) -> i32 {
        i32::MAX
    }
    fn randomize_trainer_class_names(&mut self, settings: &Settings) -> Result<()> {
        engine::names::randomize_trainer_class_names(self, settings)
    }
    fn get_doubles_trainer_classes(&self) -> Vec<i32>;

    fn allowed_items_mut(&mut self) -> &mut ItemList;
    fn non_bad_items_mut(&mut self) -> &mut ItemList;
    fn get_evolution_items(&self) -> Option<Vec<i32>>;
    fn get_x_items(&self) -> Vec<i32> {
        randomon_data::constants::global::X_ITEMS.to_vec()
    }
    fn get_unique_no_sell_items(&self) -> Vec<i32>;
    fn get_regular_shop_items(&self) -> Option<Vec<i32>>;
    fn get_op_shop_items(&self) -> Option<Vec<i32>>;
    fn get_item_names(&self) -> Vec<String>;

    fn get_required_field_tms(&self) -> Vec<i32>;
    fn get_current_field_tms(&self) -> Vec<i32>;
    fn set_field_tms(&mut self, tms: &[i32]) -> Result<()>;
    fn get_regular_field_items(&self) -> Vec<i32>;
    fn set_regular_field_items(&mut self, items: &[i32]) -> Result<()>;
    fn shuffle_field_items(&mut self) -> Result<()> {
        engine::items::shuffle_field_items(self)
    }
    fn randomize_field_items(&mut self, settings: &Settings) -> Result<()> {
        engine::items::randomize_field_items(self, settings)
    }

    fn has_shop_randomization(&self) -> bool;
    fn shuffle_shop_items(&mut self) -> Result<()> {
        engine::items::shuffle_shop_items(self)
    }
    fn randomize_shop_items(&mut self, settings: &Settings) -> Result<()> {
        engine::items::randomize_shop_items(self, settings)
    }
    fn get_shop_items(&self) -> Option<BTreeMap<i32, Shop>>;
    fn set_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()>;
    fn set_shop_prices(&mut self);

    fn get_pickup_items(&self) -> Vec<PickupItem> {
        Vec::new()
    }
    fn set_pickup_items(&mut self, _items: &[PickupItem]) {}
    fn randomize_pickup_items(&mut self, settings: &Settings) -> Result<()> {
        engine::items::randomize_pickup_items(self, settings)
    }

    fn get_ingame_trades(&self) -> Vec<IngameTrade>;
    fn set_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()>;
    fn randomize_ingame_trades(&mut self, settings: &Settings) -> Result<()> {
        engine::trades::randomize_ingame_trades(self, settings)
    }
    fn has_dvs(&self) -> bool;
    fn max_trade_nickname_length(&self) -> i32 {
        10
    }
    fn max_trade_ot_name_length(&self) -> i32 {
        7
    }

    fn remove_impossible_evolutions(&mut self, settings: &Settings) -> Result<()>;
    fn condense_level_evolutions(&mut self, max_level: i32, max_intermediate_level: i32) {
        engine::evolutions::condense_level_evolutions(self, max_level, max_intermediate_level)
    }
    fn make_evolutions_easier(&mut self, settings: &Settings) -> Result<()>;
    fn remove_time_based_evolutions(&mut self) -> Result<()>;
    fn get_impossible_evo_updates(&self) -> &EvolutionUpdates {
        &self.core().engine.impossible_evolution_updates
    }
    fn get_easier_evo_updates(&self) -> &EvolutionUpdates {
        &self.core().engine.easier_evolution_updates
    }
    fn get_time_based_evo_updates(&self) -> &EvolutionUpdates {
        &self.core().engine.time_based_evolution_updates
    }
    fn randomize_evolutions(&mut self, settings: &Settings) -> Result<()> {
        engine::evolutions::randomize_evolutions(self, settings)
    }
    fn randomize_evolutions_every_level(&mut self, settings: &Settings) -> Result<()> {
        engine::evolutions::randomize_evolutions_every_level(self, settings)
    }
    fn alt_formes_can_have_different_evolutions(&self) -> bool {
        false
    }

    fn get_game_breaking_moves(&self) -> Vec<i32> {
        vec![49, 82]
    }
    fn get_illegal_moves(&self) -> Vec<i32> {
        Vec::new()
    }
    fn get_field_moves(&self) -> Vec<i32>;
    fn get_early_required_hm_moves(&self) -> Vec<i32>;

    fn is_yellow(&self) -> bool {
        false
    }
    fn rom_name(&self) -> String;
    fn rom_code(&self) -> String;
    fn support_level(&self) -> String;
    fn default_extension(&self) -> String;
    fn internal_string_length(&self, text: &str) -> i32;
    fn randomize_intro_pokemon(&mut self) -> Result<()>;
    fn generation_of_pokemon(&self) -> i32;
    fn write_check_value_to_rom(&mut self, _value: i32) {}

    fn misc_tweaks_available(&self) -> i32 {
        0
    }
    fn apply_misc_tweaks(&mut self, settings: &Settings) -> Result<()> {
        engine::misc::apply_misc_tweaks(self, settings)
    }
    fn apply_misc_tweak(&mut self, _tweak: MiscTweak) -> Result<()> {
        Ok(())
    }
    fn is_effectiveness_updated(&self) -> bool;

    fn has_functional_formes(&self) -> bool;
    fn get_ability_dependent_formes(&self) -> Vec<PokemonId> {
        engine::pool::ability_dependent_formes(self)
    }
    fn get_banned_formes_for_player_pokemon(&self) -> Vec<PokemonId> {
        engine::pool::banned_formes_for_player_pokemon(self)
    }
    fn get_banned_formes_for_trainer_pokemon(&self) -> Vec<PokemonId> {
        Vec::new()
    }
}
