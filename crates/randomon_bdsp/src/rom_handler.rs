use std::collections::BTreeMap;

use randomon_core::{
    Compatibility, EggMoves, HandlerCore, Movesets, OutputFile, Result, RomHandler, SwitchLayout,
    TrainerNameMode,
};
use randomon_data::constants::gen4;
use randomon_data::constants::{abilities, global, items};
use randomon_data::{
    Effectiveness, EncounterSet, IngameTrade, ItemList, MegaEvolutionId, MoveCategory, PickupItem,
    PokemonId, Shop, StatChange, StaticEncounter, TotemPokemon, Trainer, TrainerPokemon, Type,
};
use randomon_log::LogWriter;
use randomon_settings::{MiscTweak, Settings};

use crate::constants;
use crate::handler::BdspRomHandler;
use crate::tweaks::FASTER_GAME_SPEED;

impl BdspRomHandler {
    fn add_move_items(&self, items_list: &mut Vec<i32>, move_number: i32, consumable_only: bool) {
        let Some(mv) = self.core.data.move_by_number(move_number) else {
            return;
        };
        let type_items = || {
            mv.kind
                .and_then(gen4::type_boosting_items)
                .unwrap_or_default()
        };
        match mv.category {
            MoveCategory::Physical => {
                items_list.push(items::LIECHI_BERRY);
                if !consumable_only {
                    items_list.extend(type_items());
                    items_list.extend([items::CHOICE_BAND, items::MUSCLE_BAND]);
                }
            }
            MoveCategory::Special => {
                items_list.push(items::PETAYA_BERRY);
                if !consumable_only {
                    items_list.extend(type_items());
                    items_list.extend([items::WISE_GLASSES, items::CHOICE_SPECS]);
                }
            }
            MoveCategory::Status => {}
        }
        if !consumable_only && let Some(boosting) = gen4::move_boosting_items(move_number) {
            items_list.extend(boosting);
        }
    }

    fn sensible_items(
        &self,
        pokemon: &TrainerPokemon,
        consumable_only: bool,
        moveset: &[i32; 4],
    ) -> Vec<i32> {
        let mut items_list = gen4::general_purpose_consumable_items();
        let boost = if consumable_only { 6 } else { 8 };
        if !consumable_only {
            items_list.extend(gen4::general_purpose_items());
        }
        for &move_number in moveset {
            self.add_move_items(&mut items_list, move_number, consumable_only);
        }
        let pk = self.core.data.pk(pokemon.pokemon);
        let by_type = pk
            .primary_type
            .and_then(|primary| Effectiveness::against(primary, pk.secondary_type, 6, false))
            .unwrap_or_default();
        for &(kind, effect) in &by_type {
            let berry = gen4::weakness_reducing_berry(kind);
            match effect {
                Effectiveness::Double => items_list.extend(berry),
                Effectiveness::Quadruple => {
                    for _ in 0..boost {
                        items_list.extend(berry);
                    }
                }
                _ => {}
            }
        }
        let normal = by_type
            .iter()
            .find(|(attacker, _)| *attacker == Type::Normal)
            .map(|(_, effect)| *effect);
        if normal == Some(Effectiveness::Neutral) {
            items_list.push(items::CHILAN_BERRY);
        }
        let ability = self.ability_for_trainer_pokemon(pokemon);
        if ability == abilities::LEVITATE {
            items_list.retain(|&item| item != items::SHUCA_BERRY);
        }
        if !consumable_only {
            if let Some(boosting) = gen4::ability_boosting_items(ability) {
                items_list.extend(boosting);
            }
            if pk.primary_type == Some(Type::Poison) || pk.secondary_type == Some(Type::Poison) {
                items_list.push(items::BLACK_SLUDGE);
            }
            if let Some(species_items) = gen4::species_boosting_items(pk.number) {
                for _ in 0..boost {
                    items_list.extend(species_items.iter().copied());
                }
            }
        }
        items_list
    }

    fn tweak_mask(&self) -> i32 {
        let mut mask = MiscTweak::BanLuckyEgg.value()
            | MiscTweak::FollowingPokemonSpeeds.value()
            | MiscTweak::TrueSizePokemon.value()
            | MiscTweak::RemoveFogAndWeather.value()
            | MiscTweak::BikeEverywhere.value();
        if self.has_engine_settings() {
            mask |= MiscTweak::SixtyFps.value() | MiscTweak::FasterGameSpeed.value();
        }
        mask
    }
}

impl RomHandler for BdspRomHandler {
    fn core(&self) -> &HandlerCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HandlerCore {
        &mut self.core
    }

    fn save_rom(&mut self) -> Result<Vec<u8>> {
        Err(randomon_core::RandomizationError::Io(
            "Switch games are saved as a mod folder".to_owned(),
        ))
    }

    fn save_rom_directory(&mut self) -> Result<Vec<OutputFile>> {
        self.save_files()
    }

    fn is_switch_game(&self) -> bool {
        true
    }

    fn set_switch_layout(&mut self, layout: SwitchLayout) {
        self.layout = layout;
    }

    fn loaded_filename(&self) -> &str {
        &self.file_name
    }

    fn print_rom_diagnostics(&self, log: &mut LogWriter) {
        log.println("Game: Pokemon Brilliant Diamond / Shining Pearl (romfs)");
        log.println(&format!(
            "Engine tweaks available: {}",
            if self.has_engine_settings() {
                "yes"
            } else {
                "no (globalgamemanagers missing)"
            }
        ));
    }

    fn is_rom_valid(&self) -> bool {
        true
    }

    fn get_pokemon(&self) -> Vec<Option<PokemonId>> {
        self.pokes[..=constants::SPECIES_COUNT as usize].to_vec()
    }

    fn get_pokemon_incl_formes(&self) -> Vec<Option<PokemonId>> {
        self.pokes.clone()
    }

    fn get_alt_formes(&self) -> Vec<PokemonId> {
        self.pokes[constants::SPECIES_COUNT as usize + 1..]
            .iter()
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
        constants::IRREGULAR_FORMES
            .iter()
            .map(|&number| self.pk(number))
            .collect()
    }

    fn remove_evos_for_pokemon_pool(&mut self) {
        self.remove_evos_for_pool();
    }

    fn get_starters(&self) -> Vec<PokemonId> {
        self.starters
            .iter()
            .map(|&number| self.pk(number))
            .collect()
    }

    fn set_starters(&mut self, starters: &[PokemonId]) -> Result<bool> {
        self.write_starters(starters)?;
        Ok(true)
    }

    fn has_starter_alt_formes(&self) -> bool {
        false
    }

    fn starter_count(&self) -> i32 {
        3
    }

    fn supports_starter_held_items(&self) -> bool {
        false
    }

    fn get_starter_held_items(&self) -> Vec<i32> {
        Vec::new()
    }

    fn set_starter_held_items(&mut self, _items: &[i32]) {}

    fn get_updated_pokemon_stats(&self, generation: i32) -> BTreeMap<i32, StatChange> {
        global::stat_changes(generation)
    }

    fn abilities_per_pokemon(&self) -> i32 {
        3
    }

    fn highest_ability_index(&self) -> i32 {
        constants::HIGHEST_ABILITY_INDEX
    }

    fn ability_name(&self, number: i32) -> String {
        self.ability_names
            .get(number as usize)
            .cloned()
            .unwrap_or_default()
    }

    fn get_ability_variations(&self) -> BTreeMap<i32, Vec<i32>> {
        constants::ability_variations()
    }

    fn get_useless_abilities(&self) -> Vec<i32> {
        constants::useless_abilities()
    }

    fn get_ability_for_trainer_pokemon(&self, pokemon: &TrainerPokemon) -> i32 {
        self.ability_for_trainer_pokemon(pokemon)
    }

    fn has_mega_evolutions(&self) -> bool {
        false
    }

    fn get_encounters(&self, use_time_of_day: bool) -> Vec<EncounterSet> {
        self.core
            .deferred
            .settle(self.read_encounters(use_time_of_day).map_err(Into::into))
    }

    fn set_encounters(&mut self, use_time_of_day: bool, encounters: &[EncounterSet]) -> Result<()> {
        Ok(self.write_encounters(use_time_of_day, encounters)?)
    }

    fn has_time_based_encounters(&self) -> bool {
        true
    }

    fn has_wild_alt_formes(&self) -> bool {
        false
    }

    fn enable_guaranteed_pokemon_catching(&mut self) {
        for pokemon in self.pokes.clone().into_iter().flatten() {
            self.core.data.pk_mut(pokemon).catch_rate = 255;
        }
    }

    fn get_trainers(&self) -> Result<Vec<Trainer>> {
        Ok(self.read_trainers()?)
    }

    fn get_main_playthrough_trainers(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_elite_four_trainers(&self, is_challenge_mode: bool) -> Vec<i32> {
        constants::elite_four_trainers(is_challenge_mode)
    }

    fn set_trainers(&mut self, trainers: &[Trainer], double_battle_mode: bool) -> Result<()> {
        Ok(self.write_trainers(trainers, double_battle_mode)?)
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
        gen4::consumable_held_items()
    }

    fn get_all_held_items(&self) -> Vec<i32> {
        gen4::all_held_items()
    }

    fn has_rival_final_battle(&self) -> bool {
        true
    }

    fn has_physical_special_split(&self) -> bool {
        true
    }

    fn get_moves_learnt(&self) -> Movesets {
        self.core
            .deferred
            .settle(self.read_movesets().map_err(Into::into))
    }

    fn set_moves_learnt(&mut self, movesets: &Movesets) -> Result<()> {
        Ok(self.write_movesets(movesets)?)
    }

    fn get_moves_banned_from_levelup(&self) -> Vec<i32> {
        constants::BANNED_MOVES.to_vec()
    }

    fn get_egg_moves(&self) -> EggMoves {
        self.core
            .deferred
            .settle(self.read_egg_moves().map_err(Into::into))
    }

    fn set_egg_moves(&mut self, egg_moves: &EggMoves) -> Result<()> {
        Ok(self.write_egg_moves(egg_moves)?)
    }

    fn supports_four_starting_moves(&self) -> bool {
        true
    }

    fn get_static_pokemon(&self) -> Vec<StaticEncounter> {
        self.read_static_pokemon()
    }

    fn set_static_pokemon(&mut self, statics: &[StaticEncounter]) -> Result<bool> {
        self.write_static_pokemon(statics)?;
        Ok(true)
    }

    fn can_change_static_pokemon(&self) -> bool {
        true
    }

    fn has_static_alt_formes(&self) -> bool {
        false
    }

    fn has_main_game_legendaries(&self) -> bool {
        true
    }

    fn get_main_game_legendaries(&self) -> Vec<i32> {
        constants::MAIN_GAME_LEGENDARIES.to_vec()
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
        self.core
            .deferred
            .settle(self.read_tm_moves().map_err(Into::into))
    }

    fn get_hm_moves(&self) -> Vec<i32> {
        Vec::new()
    }

    fn set_tm_moves(&mut self, moves: &[i32]) -> Result<()> {
        Ok(self.write_tm_moves(moves)?)
    }

    fn get_tm_count(&self) -> i32 {
        constants::TM_COUNT
    }

    fn get_hm_count(&self) -> i32 {
        0
    }

    fn get_tmhm_compatibility(&self) -> Compatibility {
        self.core
            .deferred
            .settle(self.read_tm_compatibility().map_err(Into::into))
    }

    fn set_tmhm_compatibility(&mut self, compatibility: &Compatibility) {
        let outcome = self
            .write_tm_compatibility(compatibility)
            .map_err(Into::into);
        self.core.deferred.settle(outcome);
    }

    fn has_move_tutors(&self) -> bool {
        false
    }

    fn get_move_tutor_moves(&self) -> Vec<i32> {
        Vec::new()
    }

    fn set_move_tutor_moves(&mut self, _moves: &[i32]) -> Result<()> {
        Ok(())
    }

    fn get_move_tutor_compatibility(&self) -> Compatibility {
        Compatibility::new()
    }

    fn set_move_tutor_compatibility(&mut self, _compatibility: &Compatibility) {}

    fn can_change_trainer_text(&self) -> bool {
        true
    }

    fn get_trainer_names(&self) -> Vec<String> {
        self.core
            .deferred
            .settle(self.trainer_name_list().map_err(Into::into))
    }

    fn set_trainer_names(&mut self, names: &[String]) {
        let outcome = self.set_trainer_name_list(names).map_err(Into::into);
        self.core.deferred.settle(outcome);
    }

    fn trainer_name_mode(&self) -> TrainerNameMode {
        TrainerNameMode::MaxLength
    }

    fn max_trainer_name_length(&self) -> i32 {
        12
    }

    fn get_tc_name_lengths_by_trainer(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_trainer_class_names(&self) -> Vec<String> {
        self.core
            .deferred
            .settle(self.class_names().map_err(Into::into))
    }

    fn set_trainer_class_names(&mut self, names: &[String]) {
        let outcome = self.set_class_names(names).map_err(Into::into);
        self.core.deferred.settle(outcome);
    }

    fn fixed_trainer_class_names_length(&self) -> bool {
        false
    }

    fn max_trainer_class_name_length(&self) -> i32 {
        15
    }

    fn get_doubles_trainer_classes(&self) -> Vec<i32> {
        Vec::new()
    }

    fn allowed_items_mut(&mut self) -> &mut ItemList {
        &mut self.allowed_items
    }

    fn non_bad_items_mut(&mut self) -> &mut ItemList {
        &mut self.non_bad_items
    }

    fn get_evolution_items(&self) -> Option<Vec<i32>> {
        Some(constants::evolution_items())
    }

    fn get_unique_no_sell_items(&self) -> Vec<i32> {
        Vec::new()
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
        Vec::new()
    }

    fn get_current_field_tms(&self) -> Vec<i32> {
        self.core
            .deferred
            .settle(self.current_field_tms().map_err(Into::into))
    }

    fn set_field_tms(&mut self, tms: &[i32]) -> Result<()> {
        Ok(self.write_field_tms(tms)?)
    }

    fn get_regular_field_items(&self) -> Vec<i32> {
        self.core
            .deferred
            .settle(self.regular_field_items().map_err(Into::into))
    }

    fn set_regular_field_items(&mut self, items: &[i32]) -> Result<()> {
        Ok(self.write_regular_field_items(items)?)
    }

    fn has_shop_randomization(&self) -> bool {
        true
    }

    fn get_shop_items(&self) -> Option<BTreeMap<i32, Shop>> {
        Some(
            self.core
                .deferred
                .settle(self.read_shop_items().map_err(Into::into)),
        )
    }

    fn set_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()> {
        Ok(self.write_shop_items(shops)?)
    }

    fn set_shop_prices(&mut self) {
        let outcome = self.write_shop_prices().map_err(Into::into);
        self.core.deferred.settle(outcome);
    }

    fn get_pickup_items(&self) -> Vec<PickupItem> {
        self.core
            .deferred
            .settle(self.read_pickup_items().map_err(Into::into))
    }

    fn set_pickup_items(&mut self, items: &[PickupItem]) {
        let outcome = self.write_pickup_items(items).map_err(Into::into);
        self.core.deferred.settle(outcome);
    }

    fn get_ingame_trades(&self) -> Vec<IngameTrade> {
        self.core
            .deferred
            .settle(self.read_ingame_trades().map_err(Into::into))
    }

    fn set_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        Ok(self.write_ingame_trades(trades)?)
    }

    fn has_dvs(&self) -> bool {
        false
    }

    fn remove_impossible_evolutions(&mut self, settings: &Settings) -> Result<()> {
        self.remove_impossible_evos(settings)
    }

    fn make_evolutions_easier(&mut self, settings: &Settings) -> Result<()> {
        self.make_evos_easier(settings);
        Ok(())
    }

    fn remove_time_based_evolutions(&mut self) -> Result<()> {
        self.remove_time_evos();
        Ok(())
    }

    fn alt_formes_can_have_different_evolutions(&self) -> bool {
        true
    }

    fn get_field_moves(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_early_required_hm_moves(&self) -> Vec<i32> {
        Vec::new()
    }

    fn rom_name(&self) -> String {
        "Pokemon Brilliant Diamond / Shining Pearl".to_owned()
    }

    fn rom_code(&self) -> String {
        "BDSP".to_owned()
    }

    fn support_level(&self) -> String {
        "Complete".to_owned()
    }

    fn default_extension(&self) -> String {
        String::new()
    }

    fn internal_string_length(&self, text: &str) -> i32 {
        text.chars().count() as i32
    }

    fn randomize_intro_pokemon(&mut self) -> Result<()> {
        Ok(())
    }

    fn generation_of_pokemon(&self) -> i32 {
        8
    }

    fn misc_tweaks_available(&self) -> i32 {
        self.tweak_mask()
    }

    fn apply_misc_tweak(&mut self, tweak: MiscTweak) -> Result<()> {
        match tweak {
            MiscTweak::BanLuckyEgg => {
                self.allowed_items.ban_singles(&[items::LUCKY_EGG]);
                self.non_bad_items.ban_singles(&[items::LUCKY_EGG]);
            }
            MiscTweak::FollowingPokemonSpeeds => self.apply_following_speeds()?,
            MiscTweak::TrueSizePokemon => self.apply_true_size()?,
            MiscTweak::RemoveFogAndWeather => self.remove_weather()?,
            MiscTweak::BikeEverywhere => self.allow_bike_everywhere()?,
            MiscTweak::SixtyFps => self.enable_sixty_fps()?,
            MiscTweak::FasterGameSpeed => self.speed_up_game(FASTER_GAME_SPEED)?,
            _ => {}
        }
        Ok(())
    }

    fn is_effectiveness_updated(&self) -> bool {
        true
    }

    fn has_functional_formes(&self) -> bool {
        true
    }
}
