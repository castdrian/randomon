use std::collections::BTreeMap;

use randomon_core::{
    Compatibility, EggMoves, HandlerCore, Movesets, Result, RomHandler, TrainerNameMode,
};
use randomon_data::constants::global;
use randomon_data::{
    EncounterSet, IngameTrade, ItemList, MegaEvolutionId, PokemonId, Shop, StatChange,
    StaticEncounter, TotemPokemon, Trainer,
};
use randomon_gb::crc32;
use randomon_log::LogWriter;
use randomon_settings::{MiscTweak, Settings};

use crate::constants::{self as c, HM_COUNT, TM_COUNT};
use crate::handler::Gen2RomHandler;

fn starter_key(index: usize) -> String {
    format!("StarterOffsets{}", index + 1)
}

impl RomHandler for Gen2RomHandler {
    fn core(&self) -> &HandlerCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HandlerCore {
        &mut self.core
    }

    fn save_rom(&mut self) -> Result<Vec<u8>> {
        self.save_pokemon_stats()?;
        self.save_moves();
        Ok(self.rom.clone())
    }

    fn loaded_filename(&self) -> &str {
        &self.file_name
    }

    fn print_rom_diagnostics(&self, log: &mut LogWriter) {
        let file_name = std::path::Path::new(&self.file_name)
            .file_name()
            .map_or_else(
                || self.file_name.clone(),
                |name| name.to_string_lossy().into_owned(),
            );
        log.println(&format!("File name: {file_name}"));
        log.println(&format!(
            "Original ROM CRC32: {:08X}",
            crc32(&self.original_rom)
        ));
    }

    fn is_rom_valid(&self) -> bool {
        self.entry.expected_crc32 == self.actual_crc32
    }

    fn get_pokemon(&self) -> Vec<Option<PokemonId>> {
        self.pokemon_list.clone()
    }

    fn get_pokemon_incl_formes(&self) -> Vec<Option<PokemonId>> {
        self.pokemon_list.clone()
    }

    fn get_alt_formes(&self) -> Vec<PokemonId> {
        Vec::new()
    }

    fn get_mega_evolutions(&self) -> Vec<MegaEvolutionId> {
        Vec::new()
    }

    fn get_alt_forme_of_pokemon(&self, pokemon: PokemonId, _forme: i32) -> PokemonId {
        pokemon
    }

    fn get_irregular_formes(&self) -> Vec<PokemonId> {
        Vec::new()
    }

    fn remove_evos_for_pokemon_pool(&mut self) {
        self.remove_evolutions_outside_pool();
    }

    fn get_starters(&self) -> Vec<PokemonId> {
        (0..3)
            .map(|index| self.pokemon_at(self.array(&starter_key(index))[0]))
            .collect()
    }

    fn set_starters(&mut self, starters: &[PokemonId]) -> Result<bool> {
        if starters.len() != 3 {
            return Ok(false);
        }
        for (index, &starter) in starters.iter().enumerate() {
            let number = self.number_of(starter);
            for &offset in self.array(&starter_key(index)) {
                self.set_byte(offset, number);
            }
        }
        if self.value("CanChangeStarterText") > 0 {
            for (index, &offset) in self.array("StarterTextOffsets").iter().enumerate().take(3) {
                let name = self.core.data.pk(starters[index]).name.clone();
                self.write_variable_string(&format!("{name}?\\e"), offset, true);
            }
        }
        Ok(true)
    }

    fn has_starter_alt_formes(&self) -> bool {
        false
    }

    fn starter_count(&self) -> i32 {
        3
    }

    fn supports_starter_held_items(&self) -> bool {
        true
    }

    fn get_starter_held_items(&self) -> Vec<i32> {
        self.array("StarterHeldItems")
            .iter()
            .map(|&offset| self.int(offset))
            .collect()
    }

    fn set_starter_held_items(&mut self, items: &[i32]) {
        let offsets = self.array("StarterHeldItems");
        if items.len() != offsets.len() {
            return;
        }
        for (&offset, &item) in offsets.iter().zip(items) {
            self.set_byte(offset, item as u8);
        }
    }

    fn get_updated_pokemon_stats(&self, generation: i32) -> BTreeMap<i32, StatChange> {
        global::stat_changes(generation)
    }

    fn abilities_per_pokemon(&self) -> i32 {
        0
    }

    fn highest_ability_index(&self) -> i32 {
        0
    }

    fn get_ability_variations(&self) -> BTreeMap<i32, Vec<i32>> {
        BTreeMap::new()
    }

    fn has_mega_evolutions(&self) -> bool {
        false
    }

    fn get_encounters(&self, use_time_of_day: bool) -> Vec<EncounterSet> {
        self.read_encounters(use_time_of_day)
    }

    fn set_encounters(&mut self, use_time_of_day: bool, encounters: &[EncounterSet]) -> Result<()> {
        self.write_encounters(use_time_of_day, encounters)
    }

    fn has_time_based_encounters(&self) -> bool {
        true
    }

    fn has_wild_alt_formes(&self) -> bool {
        false
    }

    fn banned_for_wild_encounters(&self) -> Vec<PokemonId> {
        vec![self.unown()]
    }

    fn enable_guaranteed_pokemon_catching(&mut self) {
        let _ = self.guarantee_catching();
    }

    fn get_trainers(&self) -> Result<Vec<Trainer>> {
        self.read_trainers()
    }

    fn get_main_playthrough_trainers(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_elite_four_trainers(&self, _is_challenge_mode: bool) -> Vec<i32> {
        Vec::new()
    }

    fn set_trainers(&mut self, trainers: &[Trainer], _double_battle_mode: bool) -> Result<()> {
        self.write_trainers(trainers)
    }

    fn has_physical_special_split(&self) -> bool {
        false
    }

    fn get_moves_learnt(&self) -> Movesets {
        self.read_moves_learnt()
    }

    fn set_moves_learnt(&mut self, movesets: &Movesets) -> Result<()> {
        self.write_evos_and_moves_learnt(false, Some(movesets))
    }

    fn get_moves_banned_from_levelup(&self) -> Vec<i32> {
        c::BANNED_LEVELUP_MOVES.to_vec()
    }

    fn get_egg_moves(&self) -> EggMoves {
        self.read_egg_moves()
    }

    fn set_egg_moves(&mut self, egg_moves: &EggMoves) -> Result<()> {
        self.write_egg_moves(egg_moves);
        Ok(())
    }

    fn supports_four_starting_moves(&self) -> bool {
        self.value("SupportsFourStartingMoves") > 0
    }

    fn get_static_pokemon(&self) -> Vec<StaticEncounter> {
        self.read_static_pokemon()
    }

    fn set_static_pokemon(&mut self, statics: &[StaticEncounter]) -> Result<bool> {
        self.write_static_pokemon(statics)
    }

    fn can_change_static_pokemon(&self) -> bool {
        self.value("StaticPokemonSupport") > 0
    }

    fn has_static_alt_formes(&self) -> bool {
        false
    }

    fn banned_for_static_pokemon(&self) -> Vec<PokemonId> {
        vec![self.unown()]
    }

    fn has_main_game_legendaries(&self) -> bool {
        false
    }

    fn get_main_game_legendaries(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_special_music_statics(&self) -> Vec<i32> {
        Vec::new()
    }

    fn apply_correct_static_music(&mut self, _changes: &BTreeMap<i32, i32>) {}

    fn has_static_music_fix(&self) -> bool {
        false
    }

    fn get_totem_pokemon(&self) -> Vec<TotemPokemon> {
        Vec::new()
    }

    fn set_totem_pokemon(&mut self, _totems: &[TotemPokemon]) -> Result<()> {
        Ok(())
    }

    fn get_tm_moves(&self) -> Vec<i32> {
        self.tm_moves()
    }

    fn get_hm_moves(&self) -> Vec<i32> {
        self.hm_moves()
    }

    fn set_tm_moves(&mut self, moves: &[i32]) -> Result<()> {
        self.write_tm_moves(moves)
    }

    fn get_tm_count(&self) -> i32 {
        TM_COUNT
    }

    fn get_hm_count(&self) -> i32 {
        HM_COUNT
    }

    fn get_tmhm_compatibility(&self) -> Compatibility {
        self.tmhm_compatibility()
    }

    fn set_tmhm_compatibility(&mut self, compatibility: &Compatibility) {
        self.write_tmhm_compatibility(compatibility);
    }

    fn has_move_tutors(&self) -> bool {
        self.entry.is_crystal
    }

    fn get_move_tutor_moves(&self) -> Vec<i32> {
        self.tutor_moves()
    }

    fn set_move_tutor_moves(&mut self, moves: &[i32]) -> Result<()> {
        self.write_tutor_moves(moves);
        Ok(())
    }

    fn get_move_tutor_compatibility(&self) -> Compatibility {
        self.tutor_compatibility()
    }

    fn set_move_tutor_compatibility(&mut self, compatibility: &Compatibility) {
        self.write_tutor_compatibility(compatibility);
    }

    fn can_change_trainer_text(&self) -> bool {
        self.value("CanChangeTrainerText") > 0
    }

    fn get_trainer_names(&self) -> Vec<String> {
        self.read_trainer_names().unwrap_or_default()
    }

    fn set_trainer_names(&mut self, names: &[String]) {
        let _ = self.write_trainer_names(names);
    }

    fn trainer_name_mode(&self) -> TrainerNameMode {
        TrainerNameMode::MaxLengthWithClass
    }

    fn max_trainer_name_length(&self) -> i32 {
        c::MAX_TRAINER_NAME_LENGTH
    }

    fn max_sum_of_trainer_name_lengths(&self) -> i32 {
        self.value("MaxSumOfTrainerNameLengths")
    }

    fn get_tc_name_lengths_by_trainer(&self) -> Vec<i32> {
        self.class_name_lengths_by_trainer()
    }

    fn get_trainer_class_names(&self) -> Vec<String> {
        self.read_trainer_class_names()
    }

    fn set_trainer_class_names(&mut self, names: &[String]) {
        self.write_trainer_class_names(names);
    }

    fn fixed_trainer_class_names_length(&self) -> bool {
        true
    }

    fn get_doubles_trainer_classes(&self) -> Vec<i32> {
        self.array("DoublesTrainerClasses").to_vec()
    }

    fn allowed_items_mut(&mut self) -> &mut ItemList {
        &mut self.allowed_items
    }

    fn non_bad_items_mut(&mut self) -> &mut ItemList {
        &mut self.non_bad_items
    }

    fn get_evolution_items(&self) -> Option<Vec<i32>> {
        None
    }

    fn get_unique_no_sell_items(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_regular_shop_items(&self) -> Option<Vec<i32>> {
        None
    }

    fn get_op_shop_items(&self) -> Option<Vec<i32>> {
        None
    }

    fn get_item_names(&self) -> Vec<String> {
        self.item_names.clone()
    }

    fn get_required_field_tms(&self) -> Vec<i32> {
        c::REQUIRED_FIELD_TMS.to_vec()
    }

    fn get_current_field_tms(&self) -> Vec<i32> {
        self.current_field_tms()
    }

    fn set_field_tms(&mut self, tms: &[i32]) -> Result<()> {
        self.write_field_tms(tms)
    }

    fn get_regular_field_items(&self) -> Vec<i32> {
        self.regular_field_items()
    }

    fn set_regular_field_items(&mut self, items: &[i32]) -> Result<()> {
        self.write_regular_field_items(items)
    }

    fn has_shop_randomization(&self) -> bool {
        false
    }

    fn get_shop_items(&self) -> Option<BTreeMap<i32, Shop>> {
        None
    }

    fn set_shop_items(&mut self, _shops: &BTreeMap<i32, Shop>) -> Result<()> {
        Ok(())
    }

    fn set_shop_prices(&mut self) {}

    fn get_ingame_trades(&self) -> Vec<IngameTrade> {
        self.read_ingame_trades()
    }

    fn set_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        self.write_ingame_trades(trades)
    }

    fn has_dvs(&self) -> bool {
        true
    }

    fn get_sensible_held_items_for(
        &self,
        _pokemon: &randomon_data::TrainerPokemon,
        consumable_only: bool,
        moveset: &[i32; 4],
    ) -> Vec<i32> {
        crate::held_items::sensible_held_items(&self.core.data, consumable_only, moveset)
    }

    fn get_all_consumable_held_items(&self) -> Vec<i32> {
        crate::held_items::CONSUMABLE_HELD_ITEMS.to_vec()
    }

    fn get_all_held_items(&self) -> Vec<i32> {
        crate::held_items::all_held_items()
    }

    fn remove_impossible_evolutions(&mut self, _settings: &Settings) -> Result<()> {
        self.remove_trade_evolutions();
        Ok(())
    }

    fn make_evolutions_easier(&mut self, _settings: &Settings) -> Result<()> {
        self.lower_happiness_requirement()
    }

    fn remove_time_based_evolutions(&mut self) -> Result<()> {
        self.remove_time_evolutions();
        Ok(())
    }

    fn get_field_moves(&self) -> Vec<i32> {
        c::FIELD_MOVES.to_vec()
    }

    fn get_early_required_hm_moves(&self) -> Vec<i32> {
        c::EARLY_REQUIRED_HM_MOVES.to_vec()
    }

    fn get_game_breaking_moves(&self) -> Vec<i32> {
        c::BROKEN_MOVES.to_vec()
    }

    fn get_illegal_moves(&self) -> Vec<i32> {
        if self.is_viet_crystal {
            c::ILLEGAL_VIET_CRYSTAL_MOVES.to_vec()
        } else {
            Vec::new()
        }
    }

    fn rom_name(&self) -> String {
        if self.is_viet_crystal {
            c::VIET_CRYSTAL_ROM_NAME.to_owned()
        } else {
            format!("Pokemon {}", self.entry.name)
        }
    }

    fn rom_code(&self) -> String {
        self.entry
            .rom_code
            .clone()
            .unwrap_or_else(|| "null".to_owned())
    }

    fn support_level(&self) -> String {
        "Complete".to_owned()
    }

    fn default_extension(&self) -> String {
        "gbc".to_owned()
    }

    fn internal_string_length(&self, text: &str) -> i32 {
        self.translate(text).len() as i32
    }

    fn randomize_intro_pokemon(&mut self) -> Result<()> {
        self.randomize_intro();
        Ok(())
    }

    fn generation_of_pokemon(&self) -> i32 {
        2
    }

    fn write_check_value_to_rom(&mut self, value: i32) {
        let offset = self.value("CheckValueOffset");
        if offset > 0 {
            for (index, byte) in value.to_be_bytes().into_iter().enumerate() {
                self.set_byte(offset + index as i32, byte);
            }
        }
    }

    fn misc_tweaks_available(&self) -> i32 {
        self.available_misc_tweaks()
    }

    fn apply_misc_tweak(&mut self, tweak: MiscTweak) -> Result<()> {
        self.apply_tweak(tweak)
    }

    fn is_effectiveness_updated(&self) -> bool {
        self.effectiveness_updated
    }

    fn has_functional_formes(&self) -> bool {
        false
    }
}
