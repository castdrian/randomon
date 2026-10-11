use std::collections::BTreeMap;
use std::path::Path;

use randomon_core::{
    Compatibility, EggMoves, HandlerCore, Movesets, OutputFile, Result, RomHandler, TrainerNameMode,
};
use randomon_ctr::held_items;
use randomon_ctr::rom::open_file_source;
use randomon_data::constants::{abilities, global, items, species};
use randomon_data::{
    Effectiveness, EncounterSet, IngameTrade, ItemList, MegaEvolutionId, MoveCategory, PickupItem,
    PokemonId, Shop, StatChange, StaticEncounter, TotemPokemon, Trainer, TrainerPokemon, Type,
    stat,
};
use randomon_log::LogWriter;
use randomon_settings::{MiscTweak, Settings};

use crate::constants::{self, POKEMON_COUNT};
use crate::handler::Gen6RomHandler;

impl Gen6RomHandler {
    fn sensible_items(
        &self,
        trainer_pokemon: &TrainerPokemon,
        consumable_only: bool,
        moveset: &[i32; 4],
    ) -> Vec<i32> {
        let mut items = held_items::gen6_general_purpose_consumable_items();
        let mut boost = 6;
        if !consumable_only {
            boost = 8;
            items.extend(held_items::gen6_general_purpose_items());
        }
        let move_boosting = held_items::gen6_move_boosting_items();
        let mut damaging_moves = 0;
        for &move_index in moveset {
            let Some(mv) = self.core.data.move_by_number(move_index) else {
                continue;
            };
            let kind = mv.kind;
            let physical = mv.category == MoveCategory::Physical;
            let special = mv.category == MoveCategory::Special;
            if physical || special {
                damaging_moves += 1;
                items.push(if physical {
                    items::LIECHI_BERRY
                } else {
                    items::PETAYA_BERRY
                });
                items.push(
                    kind.and_then(held_items::gen6_consumable_type_boosting_item)
                        .unwrap_or(0),
                );
                if !consumable_only {
                    items.extend(held_items::gen6_type_boosting_items(kind).unwrap_or_default());
                    if physical {
                        items.extend([items::CHOICE_BAND, items::MUSCLE_BAND]);
                    } else {
                        items.extend([items::WISE_GLASSES, items::CHOICE_SPECS]);
                    }
                }
            }
            if !consumable_only && let Some(boosting) = move_boosting.get(&move_index) {
                items.extend(boosting.iter().copied());
            }
        }
        if damaging_moves >= 2 {
            items.push(items::ASSAULT_VEST);
        }
        let pokemon = self.core.data.pk(trainer_pokemon.pokemon);
        let by_type = pokemon
            .primary_type
            .and_then(|primary| Effectiveness::against(primary, pokemon.secondary_type, 6, false))
            .unwrap_or_default();
        let effectiveness_of = |kind: Type| {
            by_type
                .iter()
                .find(|(candidate, _)| *candidate == kind)
                .map(|(_, effectiveness)| *effectiveness)
        };
        for &(kind, effectiveness) in &by_type {
            let berry = held_items::gen6_weakness_reducing_berry(kind).unwrap_or(0);
            match effectiveness {
                Effectiveness::Double => items.push(berry),
                Effectiveness::Quadruple => items.extend(std::iter::repeat_n(berry, boost)),
                _ => {}
            }
        }
        if effectiveness_of(Type::Normal) == Some(Effectiveness::Neutral) {
            items.push(items::CHILAN_BERRY);
        }
        let ability = self.ability_for_trainer_pokemon(trainer_pokemon);
        if ability == abilities::LEVITATE {
            items.retain(|&item| item != items::SHUCA_BERRY);
        } else if matches!(
            effectiveness_of(Type::Ground),
            Some(Effectiveness::Double | Effectiveness::Quadruple)
        ) {
            items.push(items::AIR_BALLOON);
        }
        if !consumable_only {
            if let Some(boosting) = held_items::gen6_ability_boosting_items().get(&ability) {
                items.extend(boosting.iter().copied());
            }
            if pokemon.primary_type == Some(Type::Poison)
                || pokemon.secondary_type == Some(Type::Poison)
            {
                items.push(items::BLACK_SLUDGE);
            }
            if let Some(species_items) = held_items::species_boosting_items(pokemon.number) {
                for _ in 0..boost {
                    items.extend(species_items.iter().copied());
                }
            }
            if !self
                .core
                .data
                .evolutions_from(trainer_pokemon.pokemon)
                .is_empty()
                && trainer_pokemon.level >= 20
            {
                items.push(items::EVIOLITE);
            }
        }
        items
    }

    fn ability_for_trainer_pokemon(&self, trainer_pokemon: &TrainerPokemon) -> i32 {
        if !(1..=3).contains(&trainer_pokemon.ability_slot) {
            return 0;
        }
        let pokemon = self.core.data.pk(trainer_pokemon.pokemon);
        [pokemon.ability1, pokemon.ability2, pokemon.ability3]
            [(trainer_pokemon.ability_slot - 1) as usize]
    }

    fn save_files(&mut self) -> Result<()> {
        self.save_pokemon_stats()?;
        self.save_moves()?;
        let code = self.code.clone();
        self.rom.get_mut().write_code(&code)?;
        self.write_garc(self.file("TextStrings"), &self.strings_garc)?;
        self.write_garc(self.file("StoryText"), &self.story_text_garc)
    }
}

impl RomHandler for Gen6RomHandler {
    fn core(&self) -> &HandlerCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HandlerCore {
        &mut self.core
    }

    fn save_rom(&mut self) -> Result<Vec<u8>> {
        self.save_rom_with_seed(0)
    }

    fn save_rom_with_seed(&mut self, seed: i64) -> Result<Vec<u8>> {
        self.save_files()?;
        let acronym = self.entry.acronym.clone();
        self.rom.get_mut().save_as_ncch_bytes(&acronym, seed)
    }

    fn save_rom_to_path(&mut self, path: &Path, seed: i64) -> Result<()> {
        self.save_files()?;
        let acronym = self.entry.acronym.clone();
        self.rom.get_mut().save_as_ncch(path, &acronym, seed)
    }

    fn save_rom_directory(&mut self) -> Result<Vec<OutputFile>> {
        self.save_files()?;
        self.rom.get_mut().layered_fs_files()
    }

    fn load_game_update(&mut self, path: &Path) -> Result<()> {
        self.rom.get_mut().load_update(open_file_source(path)?)?;
        self.loaded_rom()
    }

    fn loaded_filename(&self) -> &str {
        &self.file_name
    }

    fn has_game_update_loaded(&self) -> bool {
        self.rom.borrow().has_update()
    }

    fn game_update_version(&self) -> Option<String> {
        Some(self.game_version())
    }

    fn print_rom_diagnostics(&self, log: &mut LogWriter) {
        for line in self.core.deferred.settle(self.rom.borrow().diagnostics()) {
            log.println(&line);
        }
    }

    fn is_rom_valid(&self) -> bool {
        self.rom_is_valid()
    }

    fn get_pokemon(&self) -> Vec<Option<PokemonId>> {
        self.pokes[..=POKEMON_COUNT as usize].to_vec()
    }

    fn get_pokemon_incl_formes(&self) -> Vec<Option<PokemonId>> {
        self.pokes.clone()
    }

    fn get_alt_formes(&self) -> Vec<PokemonId> {
        self.pokes[POKEMON_COUNT as usize + 1..]
            .iter()
            .flatten()
            .copied()
            .collect()
    }

    fn get_mega_evolutions(&self) -> Vec<MegaEvolutionId> {
        self.mega_evolutions.clone()
    }

    fn get_alt_forme_of_pokemon(&self, pokemon: PokemonId, forme: i32) -> PokemonId {
        self.alt_forme_of(pokemon, forme)
    }

    fn get_irregular_formes(&self) -> Vec<PokemonId> {
        constants::irregular_formes(self.rom_type)
            .iter()
            .map(|&number| self.pk(number))
            .collect()
    }

    fn remove_evos_for_pokemon_pool(&mut self) {
        let outcome = self.remove_evos_for_pool();
        self.core.deferred.settle(outcome);
    }

    fn get_starters(&self) -> Vec<PokemonId> {
        self.core.deferred.settle(self.read_starters())
    }

    fn set_starters(&mut self, starters: &[PokemonId]) -> Result<bool> {
        self.write_starters(starters)?;
        Ok(true)
    }

    fn has_starter_alt_formes(&self) -> bool {
        true
    }

    fn starter_count(&self) -> i32 {
        if self.is_xy() { 6 } else { 12 }
    }

    fn supports_starter_held_items(&self) -> bool {
        true
    }

    fn get_starter_held_items(&self) -> Vec<i32> {
        self.core.deferred.settle(self.read_starter_held_items())
    }

    fn set_starter_held_items(&mut self, items: &[i32]) {
        let outcome = self.write_starter_held_items(items);
        self.core.deferred.settle(outcome);
    }

    fn get_updated_pokemon_stats(&self, generation: i32) -> BTreeMap<i32, StatChange> {
        let mut map = global::stat_changes(generation);
        match generation {
            7 => {
                map.insert(
                    species::gen6_formes::ALAKAZAM_MEGA,
                    StatChange::new(stat::SPDEF, &[105]),
                );
            }
            8 => {
                map.insert(
                    species::gen6_formes::AEGISLASH_B,
                    StatChange::new(stat::ATK | stat::SPATK, &[140, 140]),
                );
            }
            _ => {}
        }
        map
    }

    fn abilities_per_pokemon(&self) -> i32 {
        3
    }

    fn highest_ability_index(&self) -> i32 {
        constants::highest_ability_index(self.rom_type)
    }

    fn ability_name(&self, number: i32) -> String {
        self.ability_names
            .get(number as usize)
            .cloned()
            .unwrap_or_default()
    }

    fn get_ability_variations(&self) -> BTreeMap<i32, Vec<i32>> {
        held_items::gen5_ability_variations()
    }

    fn get_useless_abilities(&self) -> Vec<i32> {
        constants::USELESS_ABILITIES.to_vec()
    }

    fn get_ability_for_trainer_pokemon(&self, pokemon: &TrainerPokemon) -> i32 {
        self.ability_for_trainer_pokemon(pokemon)
    }

    fn has_mega_evolutions(&self) -> bool {
        true
    }

    fn get_encounters(&self, _use_time_of_day: bool) -> Vec<EncounterSet> {
        self.core.deferred.settle(self.read_encounters())
    }

    fn set_encounters(
        &mut self,
        _use_time_of_day: bool,
        encounters: &[EncounterSet],
    ) -> Result<()> {
        self.write_encounters(encounters)
    }

    fn has_time_based_encounters(&self) -> bool {
        false
    }

    fn has_wild_alt_formes(&self) -> bool {
        true
    }

    fn enable_guaranteed_pokemon_catching(&mut self) {
        let outcome = self.enable_guaranteed_catching();
        self.core.deferred.settle(outcome);
    }

    fn get_trainers(&self) -> Result<Vec<Trainer>> {
        self.read_trainers()
    }

    fn get_main_playthrough_trainers(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_elite_four_trainers(&self, _is_challenge_mode: bool) -> Vec<i32> {
        self.entry.array("EliteFourIndices").to_vec()
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
        self.sensible_items(pokemon, consumable_only, moveset)
    }

    fn get_all_consumable_held_items(&self) -> Vec<i32> {
        held_items::gen6_consumable_held_items()
    }

    fn get_all_held_items(&self) -> Vec<i32> {
        held_items::gen6_all_held_items()
    }

    fn has_physical_special_split(&self) -> bool {
        true
    }

    fn get_moves_learnt(&self) -> Movesets {
        self.core.deferred.settle(self.read_movesets())
    }

    fn set_moves_learnt(&mut self, movesets: &Movesets) -> Result<()> {
        self.write_movesets(movesets)
    }

    fn get_moves_banned_from_levelup(&self) -> Vec<i32> {
        constants::BANNED_MOVES.to_vec()
    }

    fn get_egg_moves(&self) -> EggMoves {
        self.core.deferred.settle(self.read_egg_moves())
    }

    fn set_egg_moves(&mut self, egg_moves: &EggMoves) -> Result<()> {
        self.write_egg_moves(egg_moves)
    }

    fn supports_four_starting_moves(&self) -> bool {
        true
    }

    fn get_static_pokemon(&self) -> Vec<StaticEncounter> {
        self.core.deferred.settle(self.read_static_pokemon())
    }

    fn set_static_pokemon(&mut self, statics: &[StaticEncounter]) -> Result<bool> {
        self.write_static_pokemon(statics)?;
        Ok(true)
    }

    fn can_change_static_pokemon(&self) -> bool {
        true
    }

    fn has_static_alt_formes(&self) -> bool {
        true
    }

    fn banned_for_static_pokemon(&self) -> Vec<PokemonId> {
        constants::ACTUALLY_COSMETIC_FORMS
            .iter()
            .filter(|&&number| number < self.total_pokemon())
            .map(|&number| self.pk(number))
            .collect()
    }

    fn force_swap_static_mega_evos(&self) -> bool {
        self.is_xy()
    }

    fn has_main_game_legendaries(&self) -> bool {
        true
    }

    fn get_main_game_legendaries(&self) -> Vec<i32> {
        self.entry.array("MainGameLegendaries").to_vec()
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
        self.read_tm_moves()
    }

    fn get_hm_moves(&self) -> Vec<i32> {
        self.read_hm_moves()
    }

    fn set_tm_moves(&mut self, moves: &[i32]) -> Result<()> {
        self.write_tm_moves(moves)
    }

    fn get_tm_count(&self) -> i32 {
        constants::TM_COUNT
    }

    fn get_hm_count(&self) -> i32 {
        constants::hm_count(self.rom_type)
    }

    fn get_tmhm_compatibility(&self) -> Compatibility {
        self.read_tmhm_compatibility()
    }

    fn set_tmhm_compatibility(&mut self, compatibility: &Compatibility) {
        self.write_tmhm_compatibility(compatibility);
    }

    fn has_move_tutors(&self) -> bool {
        self.is_oras()
    }

    fn get_move_tutor_moves(&self) -> Vec<i32> {
        self.read_move_tutor_moves()
    }

    fn set_move_tutor_moves(&mut self, moves: &[i32]) -> Result<()> {
        self.write_move_tutor_moves(moves)
    }

    fn get_move_tutor_compatibility(&self) -> Compatibility {
        self.read_move_tutor_compatibility()
    }

    fn set_move_tutor_compatibility(&mut self, compatibility: &Compatibility) {
        self.write_move_tutor_compatibility(compatibility);
    }

    fn can_change_trainer_text(&self) -> bool {
        true
    }

    fn get_trainer_names(&self) -> Vec<String> {
        self.core.deferred.settle(self.trainer_name_list())
    }

    fn set_trainer_names(&mut self, names: &[String]) {
        let index = self.entry.int("TrainerNamesTextOffset");
        let current = self.core.deferred.settle(self.strings(false, index));
        let mut new_names = vec![current[0].clone()];
        new_names.extend(names.iter().cloned());
        let outcome = self.set_strings(false, index, &new_names);
        self.core.deferred.settle(outcome);
        let outcome = self.write_strings_for_all_languages(&new_names, index);
        self.core.deferred.settle(outcome);
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
        self.core.deferred.settle(self.class_names())
    }

    fn set_trainer_class_names(&mut self, names: &[String]) {
        let index = self.entry.int("TrainerClassesTextOffset");
        let outcome = self.set_strings(false, index, names);
        self.core.deferred.settle(outcome);
        let outcome = self.write_strings_for_all_languages(names, index);
        self.core.deferred.settle(outcome);
    }

    fn fixed_trainer_class_names_length(&self) -> bool {
        false
    }

    fn max_trainer_class_name_length(&self) -> i32 {
        15
    }

    fn get_doubles_trainer_classes(&self) -> Vec<i32> {
        self.entry.array("DoublesTrainerClasses").to_vec()
    }

    fn allowed_items_mut(&mut self) -> &mut ItemList {
        &mut self.allowed_items
    }

    fn non_bad_items_mut(&mut self) -> &mut ItemList {
        &mut self.non_bad_items
    }

    fn get_evolution_items(&self) -> Option<Vec<i32>> {
        Some(constants::EVOLUTION_ITEMS.to_vec())
    }

    fn get_unique_no_sell_items(&self) -> Vec<i32> {
        constants::UNIQUE_NO_SELL_ITEMS.to_vec()
    }

    fn get_regular_shop_items(&self) -> Option<Vec<i32>> {
        Some(constants::regular_shop_items())
    }

    fn get_op_shop_items(&self) -> Option<Vec<i32>> {
        Some(constants::op_shop_items())
    }

    fn get_item_names(&self) -> Vec<String> {
        self.item_names.clone()
    }

    fn get_required_field_tms(&self) -> Vec<i32> {
        constants::required_field_tms(self.rom_type).to_vec()
    }

    fn get_current_field_tms(&self) -> Vec<i32> {
        self.core.deferred.settle(self.current_field_tms())
    }

    fn set_field_tms(&mut self, tms: &[i32]) -> Result<()> {
        self.write_field_tms(tms)
    }

    fn get_regular_field_items(&self) -> Vec<i32> {
        self.core.deferred.settle(self.regular_field_items())
    }

    fn set_regular_field_items(&mut self, items: &[i32]) -> Result<()> {
        self.write_regular_field_items(items)
    }

    fn has_shop_randomization(&self) -> bool {
        true
    }

    fn get_shop_items(&self) -> Option<BTreeMap<i32, Shop>> {
        Some(self.read_shop_items())
    }

    fn set_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()> {
        self.write_shop_items(shops)
    }

    fn set_shop_prices(&mut self) {
        let outcome = self.write_shop_prices();
        self.core.deferred.settle(outcome);
    }

    fn get_pickup_items(&self) -> Vec<PickupItem> {
        self.read_pickup_items()
    }

    fn set_pickup_items(&mut self, items: &[PickupItem]) {
        self.write_pickup_items(items);
    }

    fn get_ingame_trades(&self) -> Vec<IngameTrade> {
        self.core.deferred.settle(self.read_ingame_trades())
    }

    fn set_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        self.write_ingame_trades(trades)
    }

    fn has_dvs(&self) -> bool {
        false
    }

    fn remove_impossible_evolutions(&mut self, settings: &Settings) -> Result<()> {
        self.remove_impossible_evos(settings)
    }

    fn make_evolutions_easier(&mut self, settings: &Settings) -> Result<()> {
        self.make_evos_easier(settings)
    }

    fn remove_time_based_evolutions(&mut self) -> Result<()> {
        self.remove_time_evos()
    }

    fn get_field_moves(&self) -> Vec<i32> {
        if self.is_xy() {
            constants::FIELD_MOVES_XY.to_vec()
        } else {
            constants::FIELD_MOVES_ORAS.to_vec()
        }
    }

    fn get_early_required_hm_moves(&self) -> Vec<i32> {
        Vec::new()
    }

    fn rom_name(&self) -> String {
        format!("Pokemon {}", self.entry.name)
    }

    fn rom_code(&self) -> String {
        self.entry.rom_code.clone()
    }

    fn support_level(&self) -> String {
        "Complete".to_owned()
    }

    fn default_extension(&self) -> String {
        "cxi".to_owned()
    }

    fn internal_string_length(&self, text: &str) -> i32 {
        text.encode_utf16().count() as i32
    }

    fn randomize_intro_pokemon(&mut self) -> Result<()> {
        self.randomize_intro()
    }

    fn generation_of_pokemon(&self) -> i32 {
        6
    }

    fn misc_tweaks_available(&self) -> i32 {
        MiscTweak::FastestText.value()
            | MiscTweak::BanLuckyEgg.value()
            | MiscTweak::RetainAltFormes.value()
            | MiscTweak::NationalDexAtStart.value()
    }

    fn apply_misc_tweak(&mut self, tweak: MiscTweak) -> Result<()> {
        match tweak {
            MiscTweak::FastestText => self.apply_fastest_text(),
            MiscTweak::BanLuckyEgg => {
                self.allowed_items.ban_singles(&[items::LUCKY_EGG]);
                self.non_bad_items.ban_singles(&[items::LUCKY_EGG]);
            }
            MiscTweak::RetainAltFormes => {
                let _ = self.patch_forme_reversion();
            }
            MiscTweak::NationalDexAtStart => self.patch_for_national_dex(),
            _ => {}
        }
        Ok(())
    }

    fn is_effectiveness_updated(&self) -> bool {
        false
    }

    fn has_functional_formes(&self) -> bool {
        true
    }
}
