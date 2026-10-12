use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use randomon_data::{
    EncounterSet, IngameTrade, MoveCategory, PokemonId, SosType, StaticEncounter, TotemPokemon,
};
use randomon_java::{FormatArg, SharedRandomSource};
use randomon_log::{LogWriter, format as jformat};
use randomon_settings::{
    EvolutionsMod, InGameTradesMod, MoveTutorMovesMod, MoveTutorsCompatibilityMod, MovesetsMod,
    PickupItemsMod, Settings, ShopItemsMod, StartersMod, StaticPokemonMod, TmsHmsCompatibilityMod,
    TmsMod, TrainersMod, TypesMod, VERSION, VERSION_STRING, WildPokemonMod,
};

use crate::api::{top_level_directories, write_directory_output_with_progress};
use crate::engine::util::rotate_check_value;
use crate::error::{RandomizationError, Result};
use crate::handler::{OutputFile, RomHandler};
use crate::progress::{ProgressPlan, Stage};
use crate::state::EvolutionUpdates;

const INVALID_ROM_LOADED: &str = "The ROM you loaded is not a clean, official ROM.\nRandomizing ROM hacks or bad ROM dumps is not supported and may cause issues.\n";
const SEPARATOR_LINE: &str = "------------------------------------------------------------------";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RomOutput {
    File(Vec<u8>),
    Directory(Vec<OutputFile>),
    Written(PathBuf),
}

#[derive(Clone, Debug)]
pub struct RandomizerRun {
    pub check_value: i32,
    pub rom: RomOutput,
}

pub struct Randomizer<'a> {
    pub settings: &'a Settings,
    pub handler: &'a mut dyn RomHandler,
    pub random_source: &'a SharedRandomSource,
    pub save_as_directory: bool,
    pub settings_string: String,
    pub is_gen1: bool,
    pub output_path: Option<&'a Path>,
    pub progress: &'a ProgressPlan,
}

fn log(handler: &mut dyn RomHandler) -> &mut LogWriter {
    &mut handler.core_mut().engine.log
}

fn add_to_cv(check_value: i32, values: &[i32]) -> i32 {
    rotate_check_value(check_value, values)
}

impl Randomizer<'_> {
    fn nl(&mut self) -> &'static str {
        log(self.handler).line_separator()
    }

    fn println(&mut self, text: &str) {
        log(self.handler).println(text);
    }

    fn print(&mut self, text: &str) {
        log(self.handler).print(text);
    }

    fn newline(&mut self) {
        log(self.handler).newline();
    }

    fn printf(&mut self, format: &str, args: &[FormatArg]) {
        log(self.handler).printf(format, args);
    }

    pub fn randomize(&mut self, seed: i64) -> Result<RandomizerRun> {
        let start = Instant::now();
        self.random_source.seed(seed);
        let settings = self.settings;
        let mut check_value = 0;
        self.println(&format!("Randomizer Version: {VERSION_STRING}"));
        self.println(&format!("Random Seed: {seed}"));
        let settings_line = format!("Settings String: {VERSION}{}", self.settings_string);
        self.println(&settings_line);
        self.newline();
        let nl = self.nl();

        let mut moves_updated = false;
        let mut moves_changed = false;
        let mut movesets_changed = false;
        let mut pokemon_traits_changed = false;
        let mut starters_changed = false;
        let mut evolutions_changed = false;
        let mut trainers_changed = false;
        let mut trainer_movesets_changed = false;
        let mut statics_changed = false;
        let mut totems_changed = false;
        let mut wilds_changed = false;
        let mut tm_moves_changed = false;
        let mut move_tutor_moves_changed = false;
        let mut trades_changed = false;
        let mut tms_hms_compat_changed = false;
        let mut tutor_compat_changed = false;
        let mut shops_changed = false;

        self.handler.set_pokemon_pool(Some(settings));
        self.progress.begin(Stage::Moves);
        if settings.limit_pokemon {
            self.handler.remove_evos_for_pokemon_pool();
        }
        if settings.update_moves {
            self.handler.init_move_updates();
            self.handler.update_moves(settings);
            moves_updated = true;
        }
        if moves_updated {
            self.log_move_updates();
        }
        if settings.randomize_move_powers {
            self.handler.randomize_move_powers();
            moves_changed = true;
        }
        if settings.randomize_move_accuracies {
            self.handler.randomize_move_accuracies();
            moves_changed = true;
        }
        if settings.randomize_move_pps {
            self.handler.randomize_move_pps();
            moves_changed = true;
        }
        if settings.randomize_move_types {
            self.handler.randomize_move_types();
            moves_changed = true;
        }
        if settings.randomize_move_category && self.handler.has_physical_special_split() {
            self.handler.randomize_move_category();
            moves_changed = true;
        }
        self.progress.begin(Stage::MiscTweaks);
        if settings.current_misc_tweaks != randomon_settings::NO_MISC_TWEAKS {
            self.handler.apply_misc_tweaks(settings)?;
        }
        self.progress.begin(Stage::Types);
        if settings.update_base_stats {
            self.handler.update_pokemon_stats(settings);
            pokemon_traits_changed = true;
        }
        if settings.standardize_exp_curves {
            self.handler.standardize_exp_curves(settings);
        }
        if settings.types_mod != TypesMod::Unchanged {
            self.handler.randomize_pokemon_types(settings)?;
            pokemon_traits_changed = true;
        }
        if settings.randomize_wild_pokemon_held_items {
            self.handler.randomize_wild_held_items(settings)?;
            pokemon_traits_changed = true;
        }
        self.progress.begin(Stage::Evolutions);
        if settings.evolutions_mod == EvolutionsMod::Random {
            self.handler.randomize_evolutions(settings)?;
            evolutions_changed = true;
        } else if settings.evolutions_mod == EvolutionsMod::RandomEveryLevel {
            self.handler.randomize_evolutions_every_level(settings)?;
            evolutions_changed = true;
        }
        if evolutions_changed {
            self.log_evolution_changes();
        }
        self.progress.begin(Stage::BaseStats);
        match settings.base_statistics_mod {
            randomon_settings::BaseStatisticsMod::Shuffle => {
                self.handler.shuffle_pokemon_stats(settings)?;
                pokemon_traits_changed = true;
            }
            randomon_settings::BaseStatisticsMod::Random => {
                self.handler.randomize_pokemon_stats(settings)?;
                pokemon_traits_changed = true;
            }
            randomon_settings::BaseStatisticsMod::Unchanged => {}
        }
        self.progress.begin(Stage::Abilities);
        if settings.abilities_mod == randomon_settings::AbilitiesMod::Randomize {
            self.handler.randomize_abilities(settings)?;
            pokemon_traits_changed = true;
        }
        if pokemon_traits_changed {
            self.log_pokemon_trait_changes();
        } else {
            self.println(&format!("Pokemon base stats & type: unchanged{nl}"));
        }
        for pokemon in self.handler.get_pokemon().into_iter().flatten() {
            let pk = self.handler.core().data.pk(pokemon);
            check_value = add_to_cv(
                check_value,
                &[
                    pk.hp,
                    pk.attack,
                    pk.defense,
                    pk.speed,
                    pk.spatk,
                    pk.spdef,
                    pk.ability1,
                    pk.ability2,
                    pk.ability3,
                ],
            );
        }
        if settings.change_impossible_evolutions {
            self.handler.remove_impossible_evolutions(settings)?;
        }
        if settings.make_evolutions_easier {
            self.handler.condense_level_evolutions(40, 30);
            self.handler.make_evolutions_easier(settings)?;
        }
        if settings.remove_time_based_evolutions {
            self.handler.remove_time_based_evolutions()?;
        }
        if settings.change_impossible_evolutions {
            self.println("--Removing Impossible Evolutions--");
            let impossible = self.handler.get_impossible_evo_updates().clone();
            let easier = self.handler.get_easier_evo_updates().clone();
            self.log_updated_evolutions(&impossible, Some(&easier));
        }
        if settings.make_evolutions_easier {
            self.println("--Making Evolutions Easier--");
            if !self.is_gen1 {
                self.println("Friendship evolutions now take 160 happiness (was 220).");
            }
            let easier = self.handler.get_easier_evo_updates().clone();
            self.log_updated_evolutions(&easier, None);
        }
        if settings.remove_time_based_evolutions {
            self.println("--Removing Timed-Based Evolutions--");
            let time_based = self.handler.get_time_based_evo_updates().clone();
            self.log_updated_evolutions(&time_based, None);
        }
        self.progress.begin(Stage::Starters);
        match settings.starters_mod {
            StartersMod::Custom => {
                self.handler.custom_starters(settings)?;
                starters_changed = true;
            }
            StartersMod::CompletelyRandom => {
                self.handler.randomize_starters(settings)?;
                starters_changed = true;
            }
            StartersMod::RandomWithTwoEvolutions => {
                self.handler.randomize_basic_two_evos_starters(settings)?;
                starters_changed = true;
            }
            StartersMod::Unchanged => {}
        }
        if settings.randomize_starters_held_items && !self.is_gen1 {
            self.handler.randomize_starter_held_items(settings)?;
        }
        if starters_changed {
            self.log_starters();
        }
        if moves_changed {
            self.log_move_changes();
        } else if !moves_updated {
            self.println(&format!("Move Data: Unchanged.{nl}"));
        }
        self.progress.begin(Stage::Movesets);
        if settings.movesets_mod != MovesetsMod::Unchanged
            && settings.movesets_mod != MovesetsMod::MetronomeOnly
        {
            self.handler.randomize_moves_learnt(settings)?;
            self.handler.randomize_egg_moves(settings)?;
            movesets_changed = true;
        }
        if settings.reorder_damaging_moves {
            self.handler.order_damaging_moves_by_damage()?;
            movesets_changed = true;
        }
        if movesets_changed {
            self.log_moveset_changes()?;
        } else if settings.movesets_mod == MovesetsMod::MetronomeOnly {
            self.println(&format!("Pokemon Movesets: Metronome Only.{nl}"));
        } else {
            self.println(&format!("Pokemon Movesets: Unchanged.{nl}"));
        }
        self.progress.begin(Stage::TmsAndTutors);
        if settings.movesets_mod != MovesetsMod::MetronomeOnly && settings.tms_mod == TmsMod::Random
        {
            self.handler.randomize_tm_moves(settings)?;
            tm_moves_changed = true;
        }
        if tm_moves_changed {
            check_value = self.log_tm_moves(check_value);
        } else if settings.movesets_mod == MovesetsMod::MetronomeOnly {
            self.println(&format!("TM Moves: Metronome Only.{nl}"));
        } else {
            self.println(&format!("TM Moves: Unchanged.{nl}"));
        }
        match settings.tms_hms_compatibility_mod {
            TmsHmsCompatibilityMod::CompletelyRandom | TmsHmsCompatibilityMod::RandomPreferType => {
                self.handler.randomize_tmhm_compatibility(settings)?;
                tms_hms_compat_changed = true;
            }
            TmsHmsCompatibilityMod::Full => {
                self.handler.full_tmhm_compatibility();
                tms_hms_compat_changed = true;
            }
            TmsHmsCompatibilityMod::Unchanged => {}
        }
        if settings.tm_level_up_move_sanity {
            self.handler.ensure_tm_compat_sanity();
            if settings.tms_follow_evolutions {
                self.handler.ensure_tm_evolution_sanity()?;
            }
            tms_hms_compat_changed = true;
        }
        if settings.full_hm_compat {
            self.handler.full_hm_compatibility();
            tms_hms_compat_changed = true;
        }
        if tms_hms_compat_changed {
            self.handler.copy_tm_compatibility_to_cosmetic_formes();
            self.log_tmhm_compatibility();
        }
        if self.handler.has_move_tutors() {
            let old_tutor_moves = self.handler.get_move_tutor_moves();
            if settings.movesets_mod != MovesetsMod::MetronomeOnly
                && settings.move_tutor_moves_mod == MoveTutorMovesMod::Random
            {
                self.handler.randomize_move_tutor_moves(settings)?;
                move_tutor_moves_changed = true;
            }
            if move_tutor_moves_changed {
                check_value = self.log_move_tutor_moves(check_value, &old_tutor_moves);
            } else if settings.movesets_mod == MovesetsMod::MetronomeOnly {
                self.println(&format!("Move Tutor Moves: Metronome Only.{nl}"));
            } else {
                self.println(&format!("Move Tutor Moves: Unchanged.{nl}"));
            }
            match settings.move_tutors_compatibility_mod {
                MoveTutorsCompatibilityMod::CompletelyRandom
                | MoveTutorsCompatibilityMod::RandomPreferType => {
                    self.handler.randomize_move_tutor_compatibility(settings)?;
                    tutor_compat_changed = true;
                }
                MoveTutorsCompatibilityMod::Full => {
                    self.handler.full_move_tutor_compatibility();
                    tutor_compat_changed = true;
                }
                MoveTutorsCompatibilityMod::Unchanged => {}
            }
            if settings.tutor_level_up_move_sanity {
                self.handler.ensure_move_tutor_compat_sanity();
                if settings.tutor_follow_evolutions {
                    self.handler.ensure_move_tutor_evolution_sanity()?;
                }
                tutor_compat_changed = true;
            }
            if tutor_compat_changed {
                self.handler
                    .copy_move_tutor_compatibility_to_cosmetic_formes();
                self.log_tutor_compatibility();
            }
        }
        self.progress.begin(Stage::Trainers);
        if settings.additional_regular_trainer_pokemon > 0
            || settings.additional_important_trainer_pokemon > 0
            || settings.additional_boss_trainer_pokemon > 0
        {
            self.handler.add_trainer_pokemon(settings)?;
            trainers_changed = true;
        }
        if settings.double_battle_mode {
            self.handler.double_battle_mode()?;
            trainers_changed = true;
        }
        match settings.trainers_mod {
            TrainersMod::Random
            | TrainersMod::Distributed
            | TrainersMod::MainPlaythrough
            | TrainersMod::TypeThemed
            | TrainersMod::TypeThemedElite4Gyms => {
                self.handler.randomize_trainer_pokes(settings)?;
                trainers_changed = true;
            }
            TrainersMod::Unchanged => {
                if settings.trainers_level_modified {
                    self.handler.only_change_trainer_levels(settings)?;
                    trainers_changed = true;
                }
            }
        }
        if (settings.trainers_mod != TrainersMod::Unchanged
            || settings.starters_mod != StartersMod::Unchanged)
            && settings.rival_carries_starter_throughout
        {
            self.handler.rival_carries_starter()?;
            trainers_changed = true;
        }
        if settings.trainers_force_fully_evolved {
            self.handler.force_fully_evolved_trainer_pokes(settings)?;
            trainers_changed = true;
        }
        if settings.better_trainer_movesets {
            self.handler.pick_trainer_movesets(settings)?;
            trainers_changed = true;
            trainer_movesets_changed = true;
        }
        if settings.randomize_held_items_for_boss_trainer_pokemon
            || settings.randomize_held_items_for_important_trainer_pokemon
            || settings.randomize_held_items_for_regular_trainer_pokemon
        {
            self.handler.randomize_trainer_held_items(settings)?;
            trainers_changed = true;
        }
        let original_trainer_names = self.trainer_names()?;
        let mut trainer_names_changed = false;
        if self.handler.can_change_trainer_text() {
            if settings.randomize_trainer_class_names {
                self.handler.randomize_trainer_class_names(settings)?;
                trainers_changed = true;
                trainer_names_changed = true;
            }
            if settings.randomize_trainer_names {
                self.handler.randomize_trainer_names(settings)?;
                trainers_changed = true;
                trainer_names_changed = true;
            }
        }
        if trainers_changed {
            self.log_trainer_changes(
                &original_trainer_names,
                trainer_names_changed,
                trainer_movesets_changed,
            )?;
        } else {
            self.println(&format!("Trainers: Unchanged.{nl}"));
        }
        if settings.movesets_mod == MovesetsMod::MetronomeOnly {
            self.handler.metronome_only_mode()?;
        }
        for trainer in self.handler.get_trainers()? {
            for tp in &trainer.pokemon {
                let number = self.handler.core().data.pk(tp.pokemon).number;
                check_value = add_to_cv(check_value, &[tp.level, number]);
            }
        }
        self.progress.begin(Stage::Statics);
        if self.handler.can_change_static_pokemon() {
            let old_statics = self.handler.get_static_pokemon();
            if settings.static_pokemon_mod != StaticPokemonMod::Unchanged {
                self.handler.randomize_static_pokemon(settings)?;
                statics_changed = true;
            } else if settings.static_level_modified {
                self.handler.only_change_static_levels(settings)?;
                statics_changed = true;
            }
            if statics_changed {
                check_value = self.log_static_pokemon(check_value, &old_statics);
            } else {
                self.println(&format!("Static Pokemon: Unchanged.{nl}"));
            }
        }
        if self.handler.generation_of_pokemon() == 7 {
            let old_totems = self.handler.get_totem_pokemon();
            if settings.totem_pokemon_mod != randomon_settings::TotemPokemonMod::Unchanged
                || settings.ally_pokemon_mod != randomon_settings::AllyPokemonMod::Unchanged
                || settings.aura_mod != randomon_settings::AuraMod::Unchanged
                || settings.randomize_totem_held_items
                || settings.totem_levels_modified
            {
                self.handler.randomize_totem_pokemon(settings)?;
                totems_changed = true;
            }
            if totems_changed {
                check_value = self.log_totem_pokemon(check_value, &old_totems);
            } else {
                self.println(&format!("Totem Pokemon: Unchanged.{nl}"));
            }
        }
        self.progress.begin(Stage::WildEncounters);
        if settings.use_minimum_catch_rate {
            self.handler.change_catch_rates(settings);
        }
        match settings.wild_pokemon_mod {
            WildPokemonMod::Random => {
                self.handler.random_encounters(settings)?;
                wilds_changed = true;
            }
            WildPokemonMod::AreaMapping => {
                self.handler.area_1to1_encounters(settings)?;
                wilds_changed = true;
            }
            WildPokemonMod::GlobalMapping => {
                self.handler.game_1to1_encounters(settings)?;
                wilds_changed = true;
            }
            WildPokemonMod::Unchanged => {
                if settings.wild_levels_modified {
                    self.handler.only_change_wild_levels(settings)?;
                    wilds_changed = true;
                }
            }
        }
        if wilds_changed {
            self.log_wild_pokemon_changes();
        } else {
            self.println(&format!("Wild Pokemon: Unchanged.{nl}"));
        }
        let use_time_based = settings.use_time_based_encounters
            || (settings.wild_pokemon_mod == WildPokemonMod::Unchanged
                && settings.wild_levels_modified);
        for set in self.handler.get_encounters(use_time_based) {
            for encounter in &set.encounters {
                let number = self.handler.core().data.pk(encounter.pokemon).number;
                check_value = add_to_cv(check_value, &[encounter.level, number]);
            }
        }
        self.progress.begin(Stage::Trades);
        let old_trades = self.handler.get_ingame_trades();
        if matches!(
            settings.in_game_trades_mod,
            InGameTradesMod::RandomizeGiven | InGameTradesMod::RandomizeGivenAndRequested
        ) {
            self.handler.randomize_ingame_trades(settings)?;
            trades_changed = true;
        }
        if trades_changed {
            self.log_trades(&old_trades);
        }
        self.progress.begin(Stage::Items);
        match settings.field_items_mod {
            randomon_settings::FieldItemsMod::Shuffle => self.handler.shuffle_field_items()?,
            randomon_settings::FieldItemsMod::Random
            | randomon_settings::FieldItemsMod::RandomEven => {
                self.handler.randomize_field_items(settings)?
            }
            randomon_settings::FieldItemsMod::Unchanged => {}
        }
        match settings.shop_items_mod {
            ShopItemsMod::Shuffle => {
                self.handler.shuffle_shop_items()?;
                shops_changed = true;
            }
            ShopItemsMod::Random => {
                self.handler.randomize_shop_items(settings)?;
                shops_changed = true;
            }
            ShopItemsMod::Unchanged => {}
        }
        if shops_changed {
            self.log_shops();
        }
        if settings.pickup_items_mod == PickupItemsMod::Random {
            self.handler.randomize_pickup_items(settings)?;
            self.log_pickup_items();
        }
        self.handler.randomize_intro_pokemon()?;
        self.handler.write_check_value_to_rom(check_value);
        self.handler.core().deferred.check()?;
        self.progress.begin(Stage::Saving);
        let rom = self.save_output(seed)?;
        self.handler.core().deferred.check()?;
        self.progress.begin(Stage::WritingLog);
        let mut game_name = self.handler.rom_name();
        if self.handler.has_game_update_loaded() {
            game_name = format!(
                "{game_name} ({})",
                self.handler.game_update_version().unwrap_or_default()
            );
        }
        self.println(SEPARATOR_LINE);
        self.println(&format!("Randomization of {game_name} completed."));
        self.println(&format!("Time elapsed: {}ms", start.elapsed().as_millis()));
        let calls = self.random_source.calls_since_seed();
        self.println(&format!("RNG Calls: {calls}"));
        self.println(SEPARATOR_LINE);
        self.newline();
        self.println("--ROM Diagnostics--");
        if !self.handler.is_rom_valid() {
            self.println(INVALID_ROM_LOADED);
        }
        let mut diagnostics = LogWriter::new(nl);
        self.handler.print_rom_diagnostics(&mut diagnostics);
        let diagnostics = diagnostics.into_string();
        self.print(&diagnostics);
        let _ = (totems_changed, shops_changed);
        Ok(RandomizerRun { check_value, rom })
    }

    fn save_output(&mut self, seed: i64) -> Result<RomOutput> {
        let progress = self.progress;
        let report_saving = progress.stage_reporter(Stage::Saving);
        if self.save_as_directory {
            let files = self.handler.save_rom_directory()?;
            return match self.output_path {
                Some(root) => {
                    write_directory_output_with_progress(root, &files, &report_saving).map_err(
                        |error| RandomizationError::Io(format!("{}: {error}", root.display())),
                    )?;
                    Ok(RomOutput::Written(written_directory_root(root, &files)))
                }
                None => Ok(RomOutput::Directory(files)),
            };
        }
        match self.output_path {
            Some(path) => {
                self.handler
                    .save_rom_to_path_with_progress(path, seed, &report_saving)?;
                Ok(RomOutput::Written(path.to_path_buf()))
            }
            None => {
                report_saving(0.0, "Building the ROM image");
                Ok(RomOutput::File(self.handler.save_rom_with_seed(seed)?))
            }
        }
    }

    fn move_name(&self, number: i32) -> String {
        self.handler
            .core()
            .data
            .move_name(number)
            .unwrap_or_default()
            .to_owned()
    }

    fn log_move_tutor_moves(&mut self, mut check_value: i32, old_moves: &[i32]) -> i32 {
        self.println("--Move Tutor Moves--");
        let new_moves = self.handler.get_move_tutor_moves();
        let nl = self.nl();
        for (index, &new_move) in new_moves.iter().enumerate() {
            let old_name = self.move_name(old_moves[index]);
            let new_name = self.move_name(new_move);
            self.printf(
                &format!("%-10s -> %-10s{nl}"),
                &[old_name.into(), new_name.into()],
            );
            check_value = add_to_cv(check_value, &[new_move]);
        }
        self.newline();
        check_value
    }

    fn log_tm_moves(&mut self, mut check_value: i32) -> i32 {
        self.println("--TM Moves--");
        let tm_moves = self.handler.get_tm_moves();
        let nl = self.nl();
        for (index, &tm_move) in tm_moves.iter().enumerate() {
            let name = self.move_name(tm_move);
            self.printf(
                &format!("TM%02d %s{nl}"),
                &[(index as i32 + 1).into(), name.into()],
            );
            check_value = add_to_cv(check_value, &[tm_move]);
        }
        self.newline();
        check_value
    }

    fn full_name(&self, pokemon: PokemonId) -> String {
        self.handler.core().data.full_name(pokemon)
    }

    fn log_trades(&mut self, old_trades: &[IngameTrade]) {
        self.println("--In-Game Trades--");
        let new_trades = self.handler.get_ingame_trades();
        let nl = self.nl();
        for (old, new) in old_trades.iter().zip(&new_trades) {
            let requested = |trade: &IngameTrade| {
                trade
                    .requested_pokemon
                    .map_or_else(|| "Any".to_owned(), |pk| self.full_name(pk))
            };
            let args: Vec<FormatArg> = vec![
                requested(old).into(),
                old.nickname.clone().into(),
                self.full_name(old.given_pokemon).into(),
                requested(new).into(),
                new.nickname.clone().into(),
                self.full_name(new.given_pokemon).into(),
            ];
            self.printf(
                &format!("Trade %-11s -> %-11s the %-11s        ->      %-11s -> %-15s the %s{nl}"),
                &args,
            );
        }
        self.newline();
    }

    fn log_moveset_changes(&mut self) -> Result<()> {
        self.println("--Pokemon Movesets--");
        let nl = self.nl();
        let movesets = self.handler.get_moves_learnt();
        let egg_moves = self.handler.get_egg_moves();
        let all = self.handler.get_pokemon_incl_formes();
        let data = &self.handler.core().data;
        let mut entries: Vec<String> = Vec::new();
        let mut counter = 1;
        for pokemon in all.into_iter().flatten() {
            let pk = data.pk(pokemon);
            if pk.actually_cosmetic {
                continue;
            }
            let evolution = match data.evolutions_from(pokemon).first() {
                Some(&evo) => format!(" -> {}", data.full_name(data.evo(evo).to)),
                None => " ->  (no evolution)".to_owned(),
            };
            let mut text = jformat("%03d %s", &[counter.into(), pk.full_name().into()]);
            text.push_str(&evolution);
            text.push_str(nl);
            let stat_lines: Vec<(&str, i32)> = if self.is_gen1 {
                vec![
                    ("HP   %-3d", pk.hp),
                    ("ATK  %-3d", pk.attack),
                    ("DEF  %-3d", pk.defense),
                    ("SPEC %-3d", pk.special),
                    ("SPE  %-3d", pk.speed),
                ]
            } else {
                vec![
                    ("HP  %-3d", pk.hp),
                    ("ATK %-3d", pk.attack),
                    ("DEF %-3d", pk.defense),
                    ("SPA %-3d", pk.spatk),
                    ("SPD %-3d", pk.spdef),
                    ("SPE %-3d", pk.speed),
                ]
            };
            for (format, value) in stat_lines {
                text.push_str(&jformat(format, &[value.into()]));
                text.push_str(nl);
            }
            counter += 1;
            let moves =
                movesets
                    .get(&pk.number)
                    .ok_or(crate::error::RandomizationError::NullPointer(
                        "moveData.get",
                    ))?;
            for learnt in moves {
                match data.move_by_number(learnt.move_number) {
                    Some(mv) if learnt.level == 0 => {
                        text.push_str("Learned upon evolution: ");
                        text.push_str(&mv.name);
                        text.push_str(nl);
                    }
                    Some(mv) => {
                        text.push_str("Level ");
                        text.push_str(&jformat("%-2d", &[learnt.level.into()]));
                        text.push_str(": ");
                        text.push_str(&mv.name);
                        text.push_str(nl);
                    }
                    None => {
                        text.push_str(&format!("invalid move at level{}", learnt.level));
                    }
                }
            }
            if let Some(eggs) = egg_moves.get(&pk.number).filter(|eggs| !eggs.is_empty()) {
                text.push_str("Egg Moves:");
                text.push_str(nl);
                for &egg in eggs {
                    text.push_str(" - ");
                    text.push_str(data.move_name(egg).unwrap_or_default());
                    text.push_str(nl);
                }
            }
            entries.push(text);
        }
        entries.sort_by(|first, second| randomon_java::string_compare_to(first, second));
        for entry in entries {
            self.println(&entry);
        }
        self.newline();
        Ok(())
    }

    fn log_move_updates(&mut self) {
        self.println("--Move Updates--");
        let updates = self
            .handler
            .core()
            .engine
            .move_updates
            .clone()
            .unwrap_or_default();
        for (number, changes) in updates {
            let Some(mv) = self.handler.core().data.move_by_number(number).cloned() else {
                continue;
            };
            let mut non_type_changes: Vec<String> = Vec::new();
            if changes[0] {
                non_type_changes.push(jformat("%d power", &[mv.power.into()]));
            }
            if changes[1] {
                non_type_changes.push(jformat("%d PP", &[mv.pp.into()]));
            }
            if changes[2] {
                non_type_changes.push(jformat("%.00f%% accuracy", &[mv.hitratio.into()]));
            }
            let mut text = format!("Made {}", mv.name);
            if changes[3] {
                let kind = mv
                    .kind
                    .map_or_else(|| "null".to_owned(), |kind| kind.java_name().to_owned());
                text.push_str(&format!(" be {kind}-type"));
                if !non_type_changes.is_empty() {
                    text.push_str(" and");
                }
            }
            if changes[4] {
                text.push_str(match mv.category {
                    MoveCategory::Physical => " a Physical move",
                    MoveCategory::Special => " a Special move",
                    MoveCategory::Status => " a Status move",
                });
            }
            if !non_type_changes.is_empty() {
                text.push_str(" have ");
                match non_type_changes.len() {
                    3 => text.push_str(&format!(
                        "{}, {} and {}",
                        non_type_changes[0], non_type_changes[1], non_type_changes[2]
                    )),
                    2 => text.push_str(&format!(
                        "{} and {}",
                        non_type_changes[0], non_type_changes[1]
                    )),
                    _ => text.push_str(&non_type_changes[0]),
                }
            }
            self.println(&text);
        }
        self.newline();
    }

    fn log_evolution_changes(&mut self) {
        self.println("--Randomized Evolutions--");
        let nl = self.nl();
        let all = self.handler.get_pokemon_incl_formes();
        let mut lines: Vec<(String, String)> = Vec::new();
        {
            let data = &self.handler.core().data;
            for pokemon in all.into_iter().flatten() {
                let pk = data.pk(pokemon);
                if pk.actually_cosmetic {
                    continue;
                }
                let evolutions = data.evolutions_from(pokemon);
                if evolutions.is_empty() {
                    continue;
                }
                let name_of = |index: usize| {
                    let evo = data.evo(evolutions[index]);
                    format!("{}{}", data.pk(evo.to).name, evo.forme_suffix)
                };
                let mut text = name_of(0);
                for index in 1..evolutions.len() {
                    if index == evolutions.len() - 1 {
                        text.push_str(" and ");
                    } else {
                        text.push_str(", ");
                    }
                    text.push_str(&name_of(index));
                }
                lines.push((pk.full_name(), text));
            }
        }
        for (name, text) in lines {
            self.printf(&format!("%-15s -> %-15s{nl}"), &[name.into(), text.into()]);
        }
        self.newline();
    }

    fn log_pokemon_trait_changes(&mut self) {
        let all = self.handler.get_pokemon_incl_formes();
        let item_names = self.handler.get_item_names();
        let nl = self.nl();
        self.println("--Pokemon Base Stats & Types--");
        let type_string = |pk: &randomon_data::Pokemon| {
            let mut text = pk
                .primary_type
                .map_or_else(|| "???".to_owned(), |kind| kind.java_name().to_owned());
            if let Some(secondary) = pk.secondary_type {
                text.push('/');
                text.push_str(secondary.java_name());
            }
            text
        };
        if self.is_gen1 {
            self.println("NUM|NAME      |TYPE             |  HP| ATK| DEF| SPE|SPEC");
            let rows: Vec<Vec<FormatArg>> = all
                .iter()
                .flatten()
                .map(|&pokemon| {
                    let pk = self.handler.core().data.pk(pokemon);
                    vec![
                        pk.number.into(),
                        pk.full_name().into(),
                        type_string(pk).into(),
                        pk.hp.into(),
                        pk.attack.into(),
                        pk.defense.into(),
                        pk.speed.into(),
                        pk.special.into(),
                    ]
                })
                .collect();
            for row in rows {
                self.printf(&format!("%3d|%-10s|%-17s|%4d|%4d|%4d|%4d|%4d{nl}"), &row);
            }
        } else {
            let generation = self.handler.generation_of_pokemon();
            let (name_space, name_format, ability_space, ability_format) = match generation {
                5 => ("         ", "%-13s", "    ", "%-12s"),
                6 => ("            ", "%-16s", "      ", "%-14s"),
                generation if generation >= 7 => ("            ", "%-16s", "        ", "%-16s"),
                _ => ("      ", "%-13s", "    ", "%-12s"),
            };
            self.print(&format!(
                "NUM|NAME{name_space}|TYPE             |  HP| ATK| DEF|SATK|SDEF| SPD"
            ));
            let abilities = self.handler.abilities_per_pokemon();
            for index in 0..abilities {
                self.print(&format!("|ABILITY{}{ability_space}", index + 1));
            }
            self.print("|ITEM");
            self.newline();
            let mut counter = 0;
            for pokemon in all.into_iter().flatten() {
                let pk = self.handler.core().data.pk(pokemon).clone();
                if pk.actually_cosmetic {
                    continue;
                }
                counter += 1;
                self.printf(
                    &format!("%3d|{name_format}|%-17s|%4d|%4d|%4d|%4d|%4d|%4d"),
                    &[
                        counter.into(),
                        pk.full_name().into(),
                        type_string(&pk).into(),
                        pk.hp.into(),
                        pk.attack.into(),
                        pk.defense.into(),
                        pk.spatk.into(),
                        pk.spdef.into(),
                        pk.speed.into(),
                    ],
                );
                if abilities > 0 {
                    let first = self.handler.ability_name(pk.ability1);
                    let second = if pk.ability1 == pk.ability2 {
                        "--".to_owned()
                    } else {
                        self.handler.ability_name(pk.ability2)
                    };
                    self.printf(
                        &format!("|{ability_format}|{ability_format}"),
                        &[first.into(), second.into()],
                    );
                    if abilities > 2 {
                        let third = self.handler.ability_name(pk.ability3);
                        self.printf(&format!("|{ability_format}"), &[third.into()]);
                    }
                }
                self.print("|");
                let item = |index: i32| item_names.get(index as usize).cloned().unwrap_or_default();
                if pk.guaranteed_held_item > 0 {
                    self.print(&format!("{} (100%)", item(pk.guaranteed_held_item)));
                } else {
                    let mut item_count = 0;
                    if pk.common_held_item > 0 {
                        item_count += 1;
                        self.print(&format!("{} (common)", item(pk.common_held_item)));
                    }
                    if pk.rare_held_item > 0 {
                        if item_count > 0 {
                            self.print(", ");
                        }
                        item_count += 1;
                        self.print(&format!("{} (rare)", item(pk.rare_held_item)));
                    }
                    if pk.dark_grass_held_item > 0 {
                        if item_count > 0 {
                            self.print(", ");
                        }
                        self.print(&format!(
                            "{} (dark grass only)",
                            item(pk.dark_grass_held_item)
                        ));
                    }
                }
                self.newline();
            }
        }
        self.newline();
    }

    fn log_tmhm_compatibility(&mut self) {
        self.println("--TM Compatibility--");
        let compat = self.handler.get_tmhm_compatibility();
        let mut moves = self.handler.get_tm_moves();
        moves.extend(self.handler.get_hm_moves());
        self.log_compatibility(&compat, &moves, true);
    }

    fn log_tutor_compatibility(&mut self) {
        self.println("--Move Tutor Compatibility--");
        let compat = self.handler.get_move_tutor_compatibility();
        let moves = self.handler.get_move_tutor_moves();
        self.log_compatibility(&compat, &moves, false);
    }

    fn log_compatibility(
        &mut self,
        compat: &crate::handler::Compatibility,
        moves: &[i32],
        include_tm_number: bool,
    ) {
        let tm_count = self.handler.get_tm_count() as usize;
        let name_format = if self.handler.generation_of_pokemon() >= 6 {
            "%-17s"
        } else {
            "%-14s"
        };
        for (&pokemon, flags) in compat {
            let pk = self.handler.core().data.pk(pokemon).clone();
            if pk.actually_cosmetic {
                continue;
            }
            self.printf(
                &format!("%3d {name_format}"),
                &[pk.number.into(), format!("{} ", pk.full_name()).into()],
            );
            for index in 1..flags.len() {
                let mut name = self.move_name(moves[index - 1]);
                if name.is_empty() {
                    name = "(BLANK)".to_owned();
                }
                let length = randomon_data::text::utf16_length(&name);
                if flags[index] {
                    if include_tm_number {
                        if index <= tm_count {
                            self.printf(
                                &format!("|TM%02d %{length}s "),
                                &[(index as i32).into(), name.into()],
                            );
                        } else {
                            self.printf(
                                &format!("|HM%02d %{length}s "),
                                &[((index - tm_count) as i32).into(), name.into()],
                            );
                        }
                    } else {
                        self.printf(&format!("|%{length}s "), &[name.into()]);
                    }
                } else if include_tm_number {
                    self.printf(&format!("| %{}s ", length + 4), &["-".into()]);
                } else {
                    self.printf(&format!("| %{}s ", length as i64 - 1), &["-".into()]);
                }
            }
            self.println("|");
        }
        self.println("");
    }

    fn log_updated_evolutions(
        &mut self,
        updates: &EvolutionUpdates,
        others: Option<&EvolutionUpdates>,
    ) {
        for (key, update) in updates {
            if others.is_some_and(|others| others.contains_key(key)) {
                self.println(&format!(
                    "{} (Overwritten by \"Make Evolutions Easier\", see below)",
                    update.describe()
                ));
            } else {
                self.println(&update.describe());
            }
        }
        self.newline();
    }

    fn log_starters(&mut self) {
        match self.settings.starters_mod {
            StartersMod::Custom => self.println("--Custom Starters--"),
            StartersMod::CompletelyRandom => self.println("--Random Starters--"),
            StartersMod::RandomWithTwoEvolutions => self.println("--Random 2-Evolution Starters--"),
            StartersMod::Unchanged => {}
        }
        let starters = self.handler.get_picked_starters();
        for (index, starter) in starters.into_iter().enumerate() {
            let name = self.full_name(starter);
            self.println(&format!("Set starter {} to {name}", index + 1));
        }
        self.newline();
    }

    fn log_wild_pokemon_changes(&mut self) {
        self.println("--Wild Pokemon--");
        let settings = self.settings;
        let use_time_based = settings.use_time_based_encounters
            || (settings.wild_pokemon_mod == WildPokemonMod::Unchanged
                && settings.wild_levels_modified);
        let encounters: Vec<EncounterSet> = self.handler.get_encounters(use_time_based);
        let whitespace = if self.handler.generation_of_pokemon() == 7 {
            "%-31s"
        } else {
            "%-25s"
        };
        for (index, set) in encounters.iter().enumerate() {
            self.print(&format!("Set #{} ", index + 1));
            if let Some(name) = &set.display_name {
                self.print(&format!("- {name} "));
            }
            self.print(&format!("(rate={})", set.rate));
            self.newline();
            for encounter in &set.encounters {
                let mut text = String::new();
                if encounter.is_sos {
                    text.push_str(match encounter.sos_type {
                        Some(SosType::Rain) => "Rain SOS: ",
                        Some(SosType::Hail) => "Hail SOS: ",
                        Some(SosType::Sand) => "Sand SOS: ",
                        _ => "  SOS: ",
                    });
                }
                let pk = self.handler.core().data.pk(encounter.pokemon).clone();
                text.push_str(&pk.full_name());
                text.push_str(" Lv");
                if encounter.max_level > 0 && encounter.max_level != encounter.level {
                    text.push_str(&format!("s {}-{}", encounter.level, encounter.max_level));
                } else {
                    text.push_str(&encounter.level.to_string());
                }
                self.print(&jformat(whitespace, &[text.into()]));
                let stats = if self.is_gen1 {
                    jformat(
                        "HP %-3d ATK %-3d DEF %-3d SPECIAL %-3d SPEED %-3d",
                        &[
                            pk.hp.into(),
                            pk.attack.into(),
                            pk.defense.into(),
                            pk.special.into(),
                            pk.speed.into(),
                        ],
                    )
                } else {
                    jformat(
                        "HP %-3d ATK %-3d DEF %-3d SPATK %-3d SPDEF %-3d SPEED %-3d",
                        &[
                            pk.hp.into(),
                            pk.attack.into(),
                            pk.defense.into(),
                            pk.spatk.into(),
                            pk.spdef.into(),
                            pk.speed.into(),
                        ],
                    )
                };
                self.print(&stats);
                self.newline();
            }
            self.newline();
        }
        self.newline();
    }

    fn log_trainer_changes(
        &mut self,
        original_names: &[String],
        names_changed: bool,
        log_movesets: bool,
    ) -> Result<()> {
        self.println("--Trainers Pokemon--");
        let trainers = self.handler.get_trainers()?;
        let item_names = self.handler.get_item_names();
        for trainer in &trainers {
            self.print(&format!("#{} ", trainer.index));
            let original = original_names.get(trainer.index as usize).cloned().ok_or(
                crate::error::RandomizationError::IndexOutOfBounds("originalTrainerNames"),
            )?;
            let current = trainer.current_name().to_owned();
            if !current.is_empty() {
                if names_changed {
                    self.printf("(%s => %s)", &[original.into(), current.into()]);
                } else {
                    self.printf("(%s)", &[current.into()]);
                }
            }
            if trainer.offset != 0 {
                self.printf("@%X", &[trainer.offset.into()]);
            }
            let item_of = |index: i32| item_names.get(index as usize).cloned().unwrap_or_default();
            if log_movesets {
                self.newline();
                for tp in &trainer.pokemon {
                    let name = self.handler.core().data.pk(tp.pokemon).name.clone();
                    self.printf(
                        &tp.description_template(&name),
                        &[item_of(tp.held_item).into()],
                    );
                    let ability = self
                        .handler
                        .ability_name(self.handler.get_ability_for_trainer_pokemon(tp));
                    self.print(&format!(", Ability: {ability}"));
                    self.print(" - ");
                    let mut first = true;
                    for &mv in &tp.moves {
                        if mv != 0 {
                            if !first {
                                self.print(", ");
                            }
                            let name = self.move_name(mv);
                            self.print(&name);
                            first = false;
                        }
                    }
                    self.newline();
                }
            } else {
                self.print(" - ");
                for (index, tp) in trainer.pokemon.iter().enumerate() {
                    if index > 0 {
                        self.print(", ");
                    }
                    let name = self.handler.core().data.pk(tp.pokemon).name.clone();
                    self.printf(
                        &tp.description_template(&name),
                        &[item_of(tp.held_item).into()],
                    );
                }
            }
            self.newline();
        }
        self.newline();
        Ok(())
    }

    fn log_static_pokemon(&mut self, mut check_value: i32, old_statics: &[StaticEncounter]) -> i32 {
        let new_statics = self.handler.get_static_pokemon();
        self.println("--Static Pokemon--");
        let print_level = self.settings.static_level_modified;
        let mut seen: BTreeMap<String, i32> = BTreeMap::new();
        for (old, new) in old_statics.iter().zip(&new_statics) {
            let new_number = self.handler.core().data.pk(new.pkmn).number;
            check_value = add_to_cv(check_value, &[new_number]);
            let old_text = old.describe(&self.full_name(old.pkmn), print_level);
            self.print(&old_text);
            match seen.get_mut(&old_text) {
                Some(amount) => {
                    *amount += 1;
                    let amount = *amount;
                    self.print(&format!("({amount})"));
                }
                None => {
                    seen.insert(old_text.clone(), 1);
                }
            }
            let new_text = new.describe(&self.full_name(new.pkmn), print_level);
            self.println(&format!(" => {new_text}"));
        }
        self.newline();
        check_value
    }

    fn log_totem_pokemon(&mut self, mut check_value: i32, old_totems: &[TotemPokemon]) -> i32 {
        let new_totems = self.handler.get_totem_pokemon();
        let item_names = self.handler.get_item_names();
        self.println("--Totem Pokemon--");
        for (old, new) in old_totems.iter().zip(&new_totems) {
            let number = self.handler.core().data.pk(new.encounter.pkmn).number;
            check_value = add_to_cv(check_value, &[number]);
            let old_name = self.full_name(old.encounter.pkmn);
            self.println(&format!("{old_name} =>"));
            let mut template = format!(
                "{}@%s Lv{}\n    Aura: {}\n",
                self.full_name(new.encounter.pkmn),
                new.encounter.level,
                new.aura.describe()
            );
            for (index, ally) in new.allies.values().enumerate() {
                template.push_str(&format!(
                    "    Ally {}: {}\n",
                    index + 1,
                    ally.describe(&self.full_name(ally.pkmn), true)
                ));
            }
            template.push('\n');
            let item = item_names
                .get(new.encounter.held_item as usize)
                .cloned()
                .unwrap_or_default();
            self.printf(&template, &[item.into()]);
        }
        self.newline();
        check_value
    }

    fn log_move_changes(&mut self) {
        self.println("--Move Data--");
        self.print("NUM|NAME           |TYPE    |POWER|ACC.|PP");
        let split = self.handler.has_physical_special_split();
        if split {
            self.print(" |CATEGORY");
        }
        self.newline();
        let moves: Vec<randomon_data::Move> = self
            .handler
            .core()
            .data
            .move_ids()
            .into_iter()
            .flatten()
            .map(|id| self.handler.core().data.mv(id).clone())
            .collect();
        for mv in moves {
            let kind = mv
                .kind
                .map_or_else(|| "???".to_owned(), |kind| kind.java_name().to_owned());
            self.printf(
                "%3d|%-15s|%-8s|%5d|%4d|%3d",
                &[
                    mv.internal_id.into(),
                    mv.name.clone().into(),
                    kind.into(),
                    mv.power.into(),
                    (mv.hitratio as i32).into(),
                    mv.pp.into(),
                ],
            );
            if split {
                self.printf("| %s", &[mv.category.java_name().into()]);
            }
            self.newline();
        }
        self.newline();
    }

    fn log_shops(&mut self) {
        let item_names = self.handler.get_item_names();
        self.println("--Shops--");
        let shops = self.handler.get_shop_items().unwrap_or_default();
        for shop in shops.values() {
            self.printf("%s", &[shop.name.clone().into()]);
            self.newline();
            for &item in &shop.items {
                let name = item_names.get(item as usize).cloned().unwrap_or_default();
                self.printf("- %5s", &[name.into()]);
                self.newline();
            }
            self.newline();
        }
        self.newline();
    }

    fn log_pickup_items(&mut self) {
        let items = self.handler.get_pickup_items();
        let item_names = self.handler.get_item_names();
        self.println("--Pickup Items--");
        for range in 0..10usize {
            let start = range * 10 + 1;
            let end = (range + 1) * 10;
            self.printf("Level %s-%s", &[(start as i32).into(), (end as i32).into()]);
            self.newline();
            let mut by_probability: BTreeMap<i32, Vec<String>> = BTreeMap::new();
            for item in &items {
                let probability = item.probabilities.get(range).copied().unwrap_or(0);
                let name = item_names
                    .get(item.item as usize)
                    .cloned()
                    .unwrap_or_default();
                if let Some(list) = by_probability.get_mut(&probability) {
                    list.push(name);
                } else if probability > 0 {
                    by_probability.insert(probability, vec![name]);
                }
            }
            for (probability, names) in by_probability.iter().rev() {
                self.printf(
                    "%d%%: %s",
                    &[(*probability).into(), names.join(", ").into()],
                );
                self.newline();
            }
            self.newline();
        }
        self.newline();
    }

    fn trainer_names(&self) -> Result<Vec<String>> {
        let mut names = vec![String::new()];
        for trainer in self.handler.get_trainers()? {
            names.push(trainer.current_name().to_owned());
        }
        Ok(names)
    }
}

fn changes_moves(settings: &Settings, handler: &dyn RomHandler) -> bool {
    settings.update_moves
        || settings.randomize_move_powers
        || settings.randomize_move_accuracies
        || settings.randomize_move_pps
        || settings.randomize_move_types
        || (settings.randomize_move_category && handler.has_physical_special_split())
}

fn changes_tms_or_tutors(settings: &Settings, handler: &dyn RomHandler) -> bool {
    let metronome = settings.movesets_mod == MovesetsMod::MetronomeOnly;
    let tms = (!metronome && settings.tms_mod == TmsMod::Random)
        || settings.tms_hms_compatibility_mod != TmsHmsCompatibilityMod::Unchanged
        || settings.tm_level_up_move_sanity
        || settings.full_hm_compat;
    let tutors = handler.has_move_tutors()
        && ((!metronome && settings.move_tutor_moves_mod == MoveTutorMovesMod::Random)
            || settings.move_tutors_compatibility_mod != MoveTutorsCompatibilityMod::Unchanged
            || settings.tutor_level_up_move_sanity);
    tms || tutors
}

fn changes_trainers(settings: &Settings, handler: &dyn RomHandler) -> bool {
    settings.additional_regular_trainer_pokemon > 0
        || settings.additional_important_trainer_pokemon > 0
        || settings.additional_boss_trainer_pokemon > 0
        || settings.double_battle_mode
        || settings.trainers_mod != TrainersMod::Unchanged
        || settings.trainers_level_modified
        || (settings.rival_carries_starter_throughout
            && settings.starters_mod != StartersMod::Unchanged)
        || settings.trainers_force_fully_evolved
        || settings.better_trainer_movesets
        || settings.randomize_held_items_for_boss_trainer_pokemon
        || settings.randomize_held_items_for_important_trainer_pokemon
        || settings.randomize_held_items_for_regular_trainer_pokemon
        || (handler.can_change_trainer_text()
            && (settings.randomize_trainer_class_names || settings.randomize_trainer_names))
        || settings.movesets_mod == MovesetsMod::MetronomeOnly
}

fn changes_statics(settings: &Settings, handler: &dyn RomHandler) -> bool {
    let statics = handler.can_change_static_pokemon()
        && (settings.static_pokemon_mod != StaticPokemonMod::Unchanged
            || settings.static_level_modified);
    let totems = handler.generation_of_pokemon() == 7
        && (settings.totem_pokemon_mod != randomon_settings::TotemPokemonMod::Unchanged
            || settings.ally_pokemon_mod != randomon_settings::AllyPokemonMod::Unchanged
            || settings.aura_mod != randomon_settings::AuraMod::Unchanged
            || settings.randomize_totem_held_items
            || settings.totem_levels_modified);
    statics || totems
}

fn changes_items(settings: &Settings) -> bool {
    settings.field_items_mod != randomon_settings::FieldItemsMod::Unchanged
        || settings.shop_items_mod != ShopItemsMod::Unchanged
        || settings.pickup_items_mod == PickupItemsMod::Random
}

pub(crate) fn planned_stages(settings: &Settings, handler: &dyn RomHandler) -> Vec<Stage> {
    let is_gen1 = handler.generation_of_pokemon() == 1;
    let candidates = [
        (Stage::Moves, changes_moves(settings, handler)),
        (
            Stage::MiscTweaks,
            settings.current_misc_tweaks != randomon_settings::NO_MISC_TWEAKS,
        ),
        (
            Stage::Types,
            settings.types_mod != TypesMod::Unchanged
                || settings.update_base_stats
                || settings.standardize_exp_curves
                || settings.randomize_wild_pokemon_held_items,
        ),
        (
            Stage::Evolutions,
            settings.evolutions_mod != EvolutionsMod::Unchanged,
        ),
        (
            Stage::BaseStats,
            settings.base_statistics_mod != randomon_settings::BaseStatisticsMod::Unchanged,
        ),
        (
            Stage::Abilities,
            settings.abilities_mod == randomon_settings::AbilitiesMod::Randomize
                || settings.change_impossible_evolutions
                || settings.make_evolutions_easier
                || settings.remove_time_based_evolutions,
        ),
        (
            Stage::Starters,
            settings.starters_mod != StartersMod::Unchanged
                || (settings.randomize_starters_held_items && !is_gen1),
        ),
        (
            Stage::Movesets,
            (settings.movesets_mod != MovesetsMod::Unchanged
                && settings.movesets_mod != MovesetsMod::MetronomeOnly)
                || settings.reorder_damaging_moves,
        ),
        (
            Stage::TmsAndTutors,
            changes_tms_or_tutors(settings, handler),
        ),
        (Stage::Trainers, changes_trainers(settings, handler)),
        (Stage::Statics, changes_statics(settings, handler)),
        (
            Stage::WildEncounters,
            settings.wild_pokemon_mod != WildPokemonMod::Unchanged
                || settings.wild_levels_modified
                || settings.use_minimum_catch_rate,
        ),
        (
            Stage::Trades,
            matches!(
                settings.in_game_trades_mod,
                InGameTradesMod::RandomizeGiven | InGameTradesMod::RandomizeGivenAndRequested
            ),
        ),
        (Stage::Items, changes_items(settings)),
        (Stage::Saving, true),
        (Stage::WritingLog, true),
    ];
    candidates
        .into_iter()
        .filter_map(|(stage, runs)| runs.then_some(stage))
        .collect()
}

fn written_directory_root(root: &Path, files: &[OutputFile]) -> PathBuf {
    match top_level_directories(files).as_slice() {
        [only] => root.join(only),
        _ => root.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str) -> OutputFile {
        OutputFile {
            path: path.to_owned(),
            bytes: Vec::new(),
        }
    }

    #[test]
    fn a_single_title_folder_is_reported_as_the_written_output() {
        let root = Path::new("/out/Output");
        let layered = [
            file("0004000000055D00/romfs/"),
            file("0004000000055D00/code.bin"),
        ];
        assert_eq!(
            written_directory_root(root, &layered),
            PathBuf::from("/out/Output/0004000000055D00")
        );
        let mixed = [file("a/one"), file("b/two")];
        assert_eq!(written_directory_root(root, &mixed), root.to_path_buf());
    }
}
