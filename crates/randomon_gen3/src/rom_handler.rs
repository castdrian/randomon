use std::collections::BTreeMap;

use randomon_core::{
    Compatibility, EggMoves, HandlerCore, Movesets, Result, RomHandler, TrainerNameMode,
};
use randomon_data::constants::{gb, global, species};
use randomon_data::{
    EncounterSet, IngameTrade, ItemList, MegaEvolutionId, PickupItem, PokemonId, Shop, StatChange,
    StaticEncounter, TotemPokemon, Trainer, TrainerPokemon,
};
use randomon_log::LogWriter;
use randomon_settings::{MiscTweak, Settings};

use crate::constants as c;
use crate::handler::Gen3RomHandler;
use crate::items;
use crate::rom_entry::RomType;

const SPECIES_ITEM_WEIGHT: usize = 6;

impl Gen3RomHandler {
    fn sensible_held_items_for(
        &self,
        pokemon: &TrainerPokemon,
        consumable_only: bool,
        moveset: &[i32; 4],
    ) -> Vec<i32> {
        let mut items_list = c::GENERAL_PURPOSE_CONSUMABLE_ITEMS.to_vec();
        if !consumable_only {
            items_list.extend_from_slice(c::GENERAL_PURPOSE_ITEMS);
        }
        for &number in moveset {
            let Some(mv) = self.core.data.move_by_number(number) else {
                continue;
            };
            if mv.power <= 0 {
                continue;
            }
            if gb::is_physical_type(mv.kind) {
                items_list.push(items::LIECHI_BERRY);
                if !consumable_only {
                    items_list.extend_from_slice(c::type_boosting_items(mv.kind));
                    items_list.push(items::CHOICE_BAND);
                }
            } else {
                items_list.push(items::PETAYA_BERRY);
                if !consumable_only {
                    items_list.extend_from_slice(c::type_boosting_items(mv.kind));
                }
            }
        }
        if !consumable_only {
            let number = self.core.data.pk(pokemon.pokemon).number;
            if let Some(species_items) = c::species_boosting_items(number) {
                for _ in 0..SPECIES_ITEM_WEIGHT {
                    items_list.extend_from_slice(species_items);
                }
            }
        }
        items_list
    }

    fn file_name_only(&self) -> String {
        std::path::Path::new(&self.file_name)
            .file_name()
            .map_or_else(
                || self.file_name.clone(),
                |name| name.to_string_lossy().into_owned(),
            )
    }
}

impl RomHandler for Gen3RomHandler {
    fn core(&self) -> &HandlerCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HandlerCore {
        &mut self.core
    }

    fn save_rom(&mut self) -> Result<Vec<u8>> {
        self.save_pokemon_stats();
        self.save_moves();
        Ok(self.rom.bytes().to_vec())
    }

    fn loaded_filename(&self) -> &str {
        &self.file_name
    }

    fn print_rom_diagnostics(&self, log: &mut LogWriter) {
        log.println(&format!("File name: {}", self.file_name_only()));
        log.println(&format!("Original ROM CRC32: {:08X}", self.original_crc32));
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
        self.read_starters()
    }

    fn set_starters(&mut self, starters: &[PokemonId]) -> Result<bool> {
        Ok(self.write_starters(starters))
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
        self.read_starter_held_items()
    }

    fn set_starter_held_items(&mut self, items: &[i32]) {
        self.write_starter_held_items(items);
    }

    fn get_updated_pokemon_stats(&self, generation: i32) -> BTreeMap<i32, StatChange> {
        global::stat_changes(generation)
    }

    fn abilities_per_pokemon(&self) -> i32 {
        2
    }

    fn highest_ability_index(&self) -> i32 {
        c::HIGHEST_ABILITY_INDEX
    }

    fn ability_name(&self, number: i32) -> String {
        usize::try_from(number)
            .ok()
            .and_then(|index| self.ability_names.get(index))
            .cloned()
            .unwrap_or_default()
    }

    fn get_ability_variations(&self) -> BTreeMap<i32, Vec<i32>> {
        c::ability_variations()
    }

    fn get_useless_abilities(&self) -> Vec<i32> {
        c::USELESS_ABILITIES.to_vec()
    }

    fn get_ability_for_trainer_pokemon(&self, pokemon: &TrainerPokemon) -> i32 {
        self.core.data.pk(pokemon.pokemon).ability1
    }

    fn has_mega_evolutions(&self) -> bool {
        false
    }

    fn get_encounters(&self, _use_time_of_day: bool) -> Vec<EncounterSet> {
        self.read_encounters()
    }

    fn set_encounters(
        &mut self,
        _use_time_of_day: bool,
        encounters: &[EncounterSet],
    ) -> Result<()> {
        self.write_encounters(encounters)
    }

    fn has_wild_alt_formes(&self) -> bool {
        false
    }

    fn banned_for_wild_encounters(&self) -> Vec<PokemonId> {
        let unown = PokemonId::from_index(species::UNOWN as usize);
        if self.rom_type().is_frlg() && self.core.data.exists(unown) {
            vec![unown]
        } else {
            Vec::new()
        }
    }

    fn enable_guaranteed_pokemon_catching(&mut self) {
        self.guarantee_catching();
    }

    fn get_trainers(&self) -> Result<Vec<Trainer>> {
        Ok(self.read_trainers())
    }

    fn get_main_playthrough_trainers(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_elite_four_trainers(&self, _is_challenge_mode: bool) -> Vec<i32> {
        self.array("EliteFourIndices")
    }

    fn set_trainers(&mut self, trainers: &[Trainer], double_battle_mode: bool) -> Result<()> {
        self.write_trainers(trainers, double_battle_mode)
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
        c::CONSUMABLE_HELD_ITEMS.to_vec()
    }

    fn get_all_held_items(&self) -> Vec<i32> {
        c::all_held_items()
    }

    fn has_rival_final_battle(&self) -> bool {
        self.rom_type().is_frlg()
    }

    fn has_physical_special_split(&self) -> bool {
        false
    }

    fn get_moves_learnt(&self) -> Movesets {
        self.read_moves_learnt()
    }

    fn set_moves_learnt(&mut self, movesets: &Movesets) -> Result<()> {
        self.write_moves_learnt(movesets)
    }

    fn get_egg_moves(&self) -> EggMoves {
        self.read_egg_moves()
    }

    fn set_egg_moves(&mut self, egg_moves: &EggMoves) -> Result<()> {
        self.write_egg_moves(egg_moves);
        Ok(())
    }

    fn supports_four_starting_moves(&self) -> bool {
        true
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

    fn has_main_game_legendaries(&self) -> bool {
        self.entry.has_array("MainGameLegendaries")
    }

    fn get_main_game_legendaries(&self) -> Vec<i32> {
        self.array("MainGameLegendaries")
    }

    fn get_special_music_statics(&self) -> Vec<i32> {
        self.array("SpecialMusicStatics")
    }

    fn apply_correct_static_music(&mut self, changes: &BTreeMap<i32, i32>) {
        if self.apply_static_music(changes).is_err() {
            self.log("Couldn't apply the static Pokémon music fix.");
        }
    }

    fn has_static_music_fix(&self) -> bool {
        self.entry.tweak("NewIndexToMusicTweak").is_some()
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
        c::HM_MOVES.to_vec()
    }

    fn set_tm_moves(&mut self, moves: &[i32]) -> Result<()> {
        self.write_tm_moves(moves);
        Ok(())
    }

    fn get_tm_count(&self) -> i32 {
        c::TM_COUNT
    }

    fn get_hm_count(&self) -> i32 {
        c::HM_COUNT
    }

    fn get_tmhm_compatibility(&self) -> Compatibility {
        self.tmhm_compatibility()
    }

    fn set_tmhm_compatibility(&mut self, compatibility: &Compatibility) {
        self.write_tmhm_compatibility(compatibility);
    }

    fn has_move_tutors(&self) -> bool {
        self.has_tutors()
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
        true
    }

    fn get_trainer_names(&self) -> Vec<String> {
        self.read_trainer_names()
    }

    fn set_trainer_names(&mut self, names: &[String]) {
        self.write_trainer_names(names);
    }

    fn trainer_name_mode(&self) -> TrainerNameMode {
        TrainerNameMode::MaxLength
    }

    fn max_trainer_name_length(&self) -> i32 {
        self.value("TrainerNameLength") - 1
    }

    fn get_tc_name_lengths_by_trainer(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_trainer_class_names(&self) -> Vec<String> {
        self.read_trainer_class_names()
    }

    fn set_trainer_class_names(&mut self, names: &[String]) {
        self.write_trainer_class_names(names);
    }

    fn fixed_trainer_class_names_length(&self) -> bool {
        false
    }

    fn max_trainer_class_name_length(&self) -> i32 {
        self.value("TrainerClassNameLength") - 1
    }

    fn get_doubles_trainer_classes(&self) -> Vec<i32> {
        self.array("DoublesTrainerClasses")
    }

    fn allowed_items_mut(&mut self) -> &mut ItemList {
        &mut self.allowed_items
    }

    fn non_bad_items_mut(&mut self) -> &mut ItemList {
        &mut self.non_bad_items
    }

    fn get_evolution_items(&self) -> Option<Vec<i32>> {
        Some(c::EVOLUTION_ITEMS.to_vec())
    }

    fn get_x_items(&self) -> Vec<i32> {
        c::X_ITEMS.to_vec()
    }

    fn get_unique_no_sell_items(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_regular_shop_items(&self) -> Option<Vec<i32>> {
        Some(c::regular_shop_items())
    }

    fn get_op_shop_items(&self) -> Option<Vec<i32>> {
        Some(c::op_shop_items())
    }

    fn get_item_names(&self) -> Vec<String> {
        self.item_names.clone()
    }

    fn get_required_field_tms(&self) -> Vec<i32> {
        match self.rom_type() {
            RomType::FireRedLeafGreen => c::FRLG_REQUIRED_FIELD_TMS.to_vec(),
            RomType::Ruby | RomType::Sapphire => c::RS_REQUIRED_FIELD_TMS.to_vec(),
            RomType::Emerald => c::E_REQUIRED_FIELD_TMS.to_vec(),
        }
    }

    fn get_current_field_tms(&self) -> Vec<i32> {
        self.current_field_tms()
    }

    fn set_field_tms(&mut self, tms: &[i32]) -> Result<()> {
        self.write_field_tms(tms);
        Ok(())
    }

    fn get_regular_field_items(&self) -> Vec<i32> {
        self.regular_field_items()
    }

    fn set_regular_field_items(&mut self, items: &[i32]) -> Result<()> {
        self.write_regular_field_items(items);
        Ok(())
    }

    fn has_shop_randomization(&self) -> bool {
        true
    }

    fn get_shop_items(&self) -> Option<BTreeMap<i32, Shop>> {
        Some(self.shop_items())
    }

    fn set_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()> {
        self.write_shop_items(shops);
        Ok(())
    }

    fn set_shop_prices(&mut self) {
        self.write_shop_prices();
    }

    fn get_pickup_items(&self) -> Vec<PickupItem> {
        self.pickup_items()
    }

    fn set_pickup_items(&mut self, items: &[PickupItem]) {
        self.write_pickup_items(items);
    }

    fn get_ingame_trades(&self) -> Vec<IngameTrade> {
        self.ingame_trades()
    }

    fn set_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        self.write_ingame_trades(trades);
        Ok(())
    }

    fn has_dvs(&self) -> bool {
        false
    }

    fn remove_impossible_evolutions(&mut self, _settings: &Settings) -> Result<()> {
        self.remove_trade_and_frlg_impossible_evolutions();
        Ok(())
    }

    fn make_evolutions_easier(&mut self, _settings: &Settings) -> Result<()> {
        self.lower_happiness_requirements()
    }

    fn remove_time_based_evolutions(&mut self) -> Result<()> {
        self.remove_time_evolutions();
        Ok(())
    }

    fn get_field_moves(&self) -> Vec<i32> {
        if self.rom_type().is_frlg() {
            c::FRLG_FIELD_MOVES.to_vec()
        } else {
            c::RSE_FIELD_MOVES.to_vec()
        }
    }

    fn get_early_required_hm_moves(&self) -> Vec<i32> {
        if self.rom_type().is_frlg() {
            c::FRLG_EARLY_REQUIRED_HM_MOVES.to_vec()
        } else {
            c::RSE_EARLY_REQUIRED_HM_MOVES.to_vec()
        }
    }

    fn rom_name(&self) -> String {
        self.entry.name.clone()
    }

    fn rom_code(&self) -> String {
        self.entry.rom_code.clone()
    }

    fn support_level(&self) -> String {
        if self.value("StaticPokemonSupport") > 0 {
            "Complete".to_owned()
        } else {
            "No Static Pokemon".to_owned()
        }
    }

    fn default_extension(&self) -> String {
        "gba".to_owned()
    }

    fn internal_string_length(&self, text: &str) -> i32 {
        self.translate(text).len() as i32
    }

    fn randomize_intro_pokemon(&mut self) -> Result<()> {
        self.randomize_intro();
        Ok(())
    }

    fn generation_of_pokemon(&self) -> i32 {
        3
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
