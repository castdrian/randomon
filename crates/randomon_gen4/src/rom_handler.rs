use std::collections::BTreeMap;

use randomon_core::{
    Compatibility, EggMoves, HandlerCore, Movesets, Result, RomHandler, TrainerNameMode,
};
use randomon_data::constants::{gen4, global, species};
use randomon_data::{
    EncounterSet, IngameTrade, ItemList, MegaEvolutionId, PickupItem, PokemonId, Shop, StatChange,
    StaticEncounter, TotemPokemon, Trainer, TrainerPokemon,
};
use randomon_log::LogWriter;
use randomon_settings::{MiscTweak, Settings};

use crate::handler::Gen4RomHandler;

fn recover<T: Default>(context: &str, result: Result<T>) -> T {
    result.unwrap_or_else(|error| {
        log::warn!("{context}: {error}");
        T::default()
    })
}

fn report(context: &str, result: Result<()>) {
    if let Err(error) = result {
        log::warn!("{context}: {error}");
    }
}

impl RomHandler for Gen4RomHandler {
    fn core(&self) -> &HandlerCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HandlerCore {
        &mut self.core
    }

    fn save_rom(&mut self) -> Result<Vec<u8>> {
        self.save_rom_bytes()
    }

    fn loaded_filename(&self) -> &str {
        &self.file_name
    }

    fn print_rom_diagnostics(&self, log: &mut LogWriter) {
        self.rom.borrow().print_diagnostics(log);
    }

    fn is_rom_valid(&self) -> bool {
        self.rom_is_valid()
    }

    fn get_pokemon(&self) -> Vec<Option<PokemonId>> {
        self.pokemon_list.clone()
    }

    fn get_pokemon_incl_formes(&self) -> Vec<Option<PokemonId>> {
        self.pokemon_list_incl_formes.clone()
    }

    fn get_alt_formes(&self) -> Vec<PokemonId> {
        self.pokemon_list_incl_formes
            .iter()
            .skip(gen4::POKEMON_COUNT as usize + 1)
            .flatten()
            .copied()
            .collect()
    }

    fn get_mega_evolutions(&self) -> Vec<MegaEvolutionId> {
        Vec::new()
    }

    fn get_alt_forme_of_pokemon(&self, pokemon: PokemonId, forme: i32) -> PokemonId {
        self.alt_forme_of(pokemon, forme)
    }

    fn get_irregular_formes(&self) -> Vec<PokemonId> {
        Vec::new()
    }

    fn remove_evos_for_pokemon_pool(&mut self) {
        let result = self.remove_evos_for_pokemon_pool_impl();
        report("removeEvosForPokemonPool", result);
    }

    fn get_starters(&self) -> Vec<PokemonId> {
        recover("getStarters", self.starters())
    }

    fn set_starters(&mut self, starters: &[PokemonId]) -> Result<bool> {
        self.set_starters_impl(starters)
    }

    fn has_starter_alt_formes(&self) -> bool {
        false
    }

    fn starter_count(&self) -> i32 {
        3
    }

    fn supports_starter_held_items(&self) -> bool {
        !self.entry.is_hgss()
    }

    fn get_starter_held_items(&self) -> Vec<i32> {
        recover("getStarterHeldItems", self.starter_held_items())
    }

    fn set_starter_held_items(&mut self, items: &[i32]) {
        let result = self.set_starter_held_items_impl(items);
        report("setStarterHeldItems", result);
    }

    fn get_updated_pokemon_stats(&self, generation: i32) -> BTreeMap<i32, StatChange> {
        global::stat_changes(generation)
    }

    fn abilities_per_pokemon(&self) -> i32 {
        2
    }

    fn highest_ability_index(&self) -> i32 {
        gen4::HIGHEST_ABILITY_INDEX
    }

    fn ability_name(&self, number: i32) -> String {
        usize::try_from(number)
            .ok()
            .and_then(|index| self.ability_names.get(index))
            .cloned()
            .unwrap_or_default()
    }

    fn get_ability_variations(&self) -> BTreeMap<i32, Vec<i32>> {
        gen4::ability_variations()
    }

    fn get_useless_abilities(&self) -> Vec<i32> {
        gen4::USELESS_ABILITIES.to_vec()
    }

    fn get_ability_for_trainer_pokemon(&self, pokemon: &TrainerPokemon) -> i32 {
        self.ability_for_trainer_pokemon(pokemon)
    }

    fn has_mega_evolutions(&self) -> bool {
        false
    }

    fn get_encounters(&self, use_time_of_day: bool) -> Vec<EncounterSet> {
        recover("getEncounters", self.encounters(use_time_of_day))
    }

    fn set_encounters(&mut self, use_time_of_day: bool, encounters: &[EncounterSet]) -> Result<()> {
        self.set_encounters_impl(use_time_of_day, encounters)
    }

    fn has_time_based_encounters(&self) -> bool {
        self.entry.is_hgss()
    }

    fn has_wild_alt_formes(&self) -> bool {
        false
    }

    fn banned_for_wild_encounters(&self) -> Vec<PokemonId> {
        vec![self.pk(species::UNOWN)]
    }

    fn enable_guaranteed_pokemon_catching(&mut self) {
        let result = self.enable_guaranteed_catching();
        report("enableGuaranteedPokemonCatching", result);
    }

    fn get_trainers(&self) -> Result<Vec<Trainer>> {
        self.trainers()
    }

    fn get_main_playthrough_trainers(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_elite_four_trainers(&self, _is_challenge_mode: bool) -> Vec<i32> {
        self.entry.array_or_empty("EliteFourIndices").to_vec()
    }

    fn set_trainers(&mut self, trainers: &[Trainer], double_battle_mode: bool) -> Result<()> {
        self.set_trainers_impl(trainers, double_battle_mode)
    }

    fn get_sensible_held_items_for(
        &self,
        pokemon: &TrainerPokemon,
        consumable_only: bool,
        moveset: &[i32; 4],
    ) -> Vec<i32> {
        self.sensible_held_items_for(pokemon, consumable_only, moveset)
    }

    fn get_all_consumable_held_items(&self) -> Vec<i32> {
        gen4::consumable_held_items()
    }

    fn get_all_held_items(&self) -> Vec<i32> {
        gen4::all_held_items()
    }

    fn has_physical_special_split(&self) -> bool {
        true
    }

    fn get_moves_learnt(&self) -> Movesets {
        recover("getMovesLearnt", self.moves_learnt())
    }

    fn set_moves_learnt(&mut self, movesets: &Movesets) -> Result<()> {
        self.set_moves_learnt_impl(movesets)
    }

    fn get_egg_moves(&self) -> EggMoves {
        recover("getEggMoves", self.egg_moves())
    }

    fn set_egg_moves(&mut self, egg_moves: &EggMoves) -> Result<()> {
        self.set_egg_moves_impl(egg_moves)
    }

    fn supports_four_starting_moves(&self) -> bool {
        true
    }

    fn get_static_pokemon(&self) -> Vec<StaticEncounter> {
        recover("getStaticPokemon", self.static_pokemon())
    }

    fn set_static_pokemon(&mut self, statics: &[StaticEncounter]) -> Result<bool> {
        self.set_static_pokemon_impl(statics)
    }

    fn can_change_static_pokemon(&self) -> bool {
        self.entry.static_pokemon_support
    }

    fn has_static_alt_formes(&self) -> bool {
        false
    }

    fn has_main_game_legendaries(&self) -> bool {
        true
    }

    fn get_main_game_legendaries(&self) -> Vec<i32> {
        self.entry.array_or_empty("MainGameLegendaries").to_vec()
    }

    fn get_special_music_statics(&self) -> Vec<i32> {
        self.entry.array_or_empty("SpecialMusicStatics").to_vec()
    }

    fn apply_correct_static_music(&mut self, changes: &BTreeMap<i32, i32>) {
        let result = self.apply_correct_static_music_impl(changes);
        report("applyCorrectStaticMusic", result);
    }

    fn has_static_music_fix(&self) -> bool {
        self.entry.tweak("NewIndexToMusicTweak").is_some() || self.entry.is_hgss()
    }

    fn get_totem_pokemon(&self) -> Vec<TotemPokemon> {
        Vec::new()
    }

    fn set_totem_pokemon(&mut self, _totems: &[TotemPokemon]) -> Result<()> {
        Ok(())
    }

    fn get_tm_moves(&self) -> Vec<i32> {
        recover("getTMMoves", self.tm_moves())
    }

    fn get_hm_moves(&self) -> Vec<i32> {
        recover("getHMMoves", self.hm_moves())
    }

    fn set_tm_moves(&mut self, moves: &[i32]) -> Result<()> {
        self.set_tm_moves_impl(moves)
    }

    fn get_tm_count(&self) -> i32 {
        gen4::TM_COUNT
    }

    fn get_hm_count(&self) -> i32 {
        gen4::HM_COUNT
    }

    fn get_tmhm_compatibility(&self) -> Compatibility {
        recover("getTMHMCompatibility", self.tmhm_compatibility())
    }

    fn set_tmhm_compatibility(&mut self, compatibility: &Compatibility) {
        let result = self.set_tmhm_compatibility_impl(compatibility);
        report("setTMHMCompatibility", result);
    }

    fn has_move_tutors(&self) -> bool {
        self.has_tutors()
    }

    fn get_move_tutor_moves(&self) -> Vec<i32> {
        recover("getMoveTutorMoves", self.move_tutor_moves())
    }

    fn set_move_tutor_moves(&mut self, moves: &[i32]) -> Result<()> {
        self.set_move_tutor_moves_impl(moves)
    }

    fn get_move_tutor_compatibility(&self) -> Compatibility {
        recover("getMoveTutorCompatibility", self.move_tutor_compatibility())
    }

    fn set_move_tutor_compatibility(&mut self, compatibility: &Compatibility) {
        let result = self.set_move_tutor_compatibility_impl(compatibility);
        report("setMoveTutorCompatibility", result);
    }

    fn can_change_trainer_text(&self) -> bool {
        true
    }

    fn get_trainer_names(&self) -> Vec<String> {
        recover("getTrainerNames", self.trainer_names())
    }

    fn set_trainer_names(&mut self, names: &[String]) {
        let result = self.set_trainer_names_impl(names);
        report("setTrainerNames", result);
    }

    fn trainer_name_mode(&self) -> TrainerNameMode {
        TrainerNameMode::MaxLength
    }

    fn max_trainer_name_length(&self) -> i32 {
        10
    }

    fn get_tc_name_lengths_by_trainer(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_trainer_class_names(&self) -> Vec<String> {
        recover("getTrainerClassNames", self.trainer_class_names())
    }

    fn set_trainer_class_names(&mut self, names: &[String]) {
        let result = self.set_trainer_class_names_impl(names);
        report("setTrainerClassNames", result);
    }

    fn fixed_trainer_class_names_length(&self) -> bool {
        false
    }

    fn max_trainer_class_name_length(&self) -> i32 {
        12
    }

    fn get_doubles_trainer_classes(&self) -> Vec<i32> {
        self.entry.array_or_empty("DoublesTrainerClasses").to_vec()
    }

    fn allowed_items_mut(&mut self) -> &mut ItemList {
        &mut self.allowed_items
    }

    fn non_bad_items_mut(&mut self) -> &mut ItemList {
        &mut self.non_bad_items
    }

    fn get_evolution_items(&self) -> Option<Vec<i32>> {
        Some(gen4::EVOLUTION_ITEMS.to_vec())
    }

    fn get_unique_no_sell_items(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_regular_shop_items(&self) -> Option<Vec<i32>> {
        Some(self.regular_shop_items.clone())
    }

    fn get_op_shop_items(&self) -> Option<Vec<i32>> {
        Some(self.op_shop_items.clone())
    }

    fn get_item_names(&self) -> Vec<String> {
        self.item_names.clone()
    }

    fn get_required_field_tms(&self) -> Vec<i32> {
        match self.rom_type() {
            gen4::TYPE_DP => gen4::DP_REQUIRED_FIELD_TMS.to_vec(),
            gen4::TYPE_PLAT => gen4::PT_REQUIRED_FIELD_TMS.to_vec(),
            _ => Vec::new(),
        }
    }

    fn get_current_field_tms(&self) -> Vec<i32> {
        recover("getCurrentFieldTMs", self.current_field_tms())
    }

    fn set_field_tms(&mut self, tms: &[i32]) -> Result<()> {
        self.set_field_tms_impl(tms)
    }

    fn get_regular_field_items(&self) -> Vec<i32> {
        recover("getRegularFieldItems", self.regular_field_items())
    }

    fn set_regular_field_items(&mut self, items: &[i32]) -> Result<()> {
        self.set_regular_field_items_impl(items)
    }

    fn has_shop_randomization(&self) -> bool {
        true
    }

    fn get_shop_items(&self) -> Option<BTreeMap<i32, Shop>> {
        Some(recover("getShopItems", self.shop_items()))
    }

    fn set_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()> {
        self.set_shop_items_impl(shops)
    }

    fn set_shop_prices(&mut self) {
        let result = self.set_shop_prices_impl();
        report("setShopPrices", result);
    }

    fn get_pickup_items(&self) -> Vec<PickupItem> {
        recover("getPickupItems", self.pickup_items())
    }

    fn set_pickup_items(&mut self, items: &[PickupItem]) {
        let result = self.set_pickup_items_impl(items);
        report("setPickupItems", result);
    }

    fn get_ingame_trades(&self) -> Vec<IngameTrade> {
        recover("getIngameTrades", self.ingame_trades())
    }

    fn set_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        self.set_ingame_trades_impl(trades)
    }

    fn has_dvs(&self) -> bool {
        false
    }

    fn remove_impossible_evolutions(&mut self, settings: &Settings) -> Result<()> {
        self.remove_impossible_evolutions_impl(settings)
    }

    fn make_evolutions_easier(&mut self, settings: &Settings) -> Result<()> {
        self.make_evolutions_easier_impl(settings)
    }

    fn remove_time_based_evolutions(&mut self) -> Result<()> {
        self.remove_time_based_evolutions_impl()
    }

    fn get_field_moves(&self) -> Vec<i32> {
        if self.entry.is_hgss() {
            gen4::HGSS_FIELD_MOVES.to_vec()
        } else {
            gen4::DPPT_FIELD_MOVES.to_vec()
        }
    }

    fn get_early_required_hm_moves(&self) -> Vec<i32> {
        if self.entry.is_hgss() {
            gen4::HGSS_EARLY_REQUIRED_HM_MOVES.to_vec()
        } else {
            gen4::DPPT_EARLY_REQUIRED_HM_MOVES.to_vec()
        }
    }

    fn rom_name(&self) -> String {
        format!("Pokemon {}", self.entry.name)
    }

    fn rom_code(&self) -> String {
        self.entry.rom_code.clone()
    }

    fn support_level(&self) -> String {
        if self.entry.static_pokemon_support {
            "Complete".to_owned()
        } else {
            "No Static Pokemon".to_owned()
        }
    }

    fn default_extension(&self) -> String {
        "nds".to_owned()
    }

    fn internal_string_length(&self, text: &str) -> i32 {
        text.encode_utf16().count() as i32
    }

    fn randomize_intro_pokemon(&mut self) -> Result<()> {
        self.randomize_intro()
    }

    fn generation_of_pokemon(&self) -> i32 {
        4
    }

    fn misc_tweaks_available(&self) -> i32 {
        self.tweaks_available()
    }

    fn apply_misc_tweak(&mut self, tweak: MiscTweak) -> Result<()> {
        self.apply_tweak(tweak)
    }

    fn is_effectiveness_updated(&self) -> bool {
        self.effectiveness_updated
    }

    fn has_functional_formes(&self) -> bool {
        !self.entry.is_dp()
    }

    fn get_banned_formes_for_trainer_pokemon(&self) -> Vec<PokemonId> {
        self.banned_formes_for_trainers()
    }
}
