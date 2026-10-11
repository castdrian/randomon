use std::cmp::Ordering;

use randomon_core::engine::evolutions::{
    add_evo_update_condensed, add_evo_update_happiness, add_evo_update_held_item,
    add_evo_update_level, add_evo_update_party, add_evo_update_stone,
};
use randomon_core::{RandomizationError, Result, RomHandler};
use randomon_data::constants::{gen5, items, species};
use randomon_data::{Evolution, EvolutionId, EvolutionType, PokemonId};
use randomon_java::{JavaHash, JavaHashSet};
use randomon_nds::{find, read_word, write_full_int, write_word};
use randomon_settings::{MovesetsMod, Settings, WildPokemonMod};

use crate::handler::Gen5RomHandler;

#[derive(Clone, Debug)]
struct ExtraEvolution(Evolution);

impl PartialEq for ExtraEvolution {
    fn eq(&self, other: &Self) -> bool {
        self.0.java_equals(&other.0)
    }
}

impl Eq for ExtraEvolution {}

impl JavaHash for ExtraEvolution {
    fn java_hash_code(&self) -> i32 {
        self.0.java_hash_code()
    }

    fn java_compare_to(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

const EVOLUTION_SLOTS: i32 = 7;
const HAPPINESS_THRESHOLD: u8 = 220;
const EASIER_HAPPINESS_THRESHOLD: u8 = 160;

impl Gen5RomHandler {
    fn all_pokemon_ids(&self) -> Vec<PokemonId> {
        self.pokemon_list_incl_formes
            .iter()
            .flatten()
            .copied()
            .collect()
    }

    pub(crate) fn populate_evolutions(&mut self) -> Result<()> {
        let evolutions = self.read_narc("PokemonEvolutions")?;
        for number in 1..=gen5::POKEMON_COUNT {
            let pokemon = self.pk(number);
            let entry = evolutions
                .files
                .get(number as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("evoNARC.files"))?;
            for slot in 0..EVOLUTION_SLOTS {
                let method = read_word(entry, slot * 6);
                let target = read_word(entry, slot * 6 + 4);
                if !(1..=gen5::EVOLUTION_METHOD_COUNT).contains(&method) || target < 1 {
                    continue;
                }
                let kind = EvolutionType::from_index(5, method)
                    .ok_or(RandomizationError::NullPointer("EvolutionType.fromIndex"))?;
                if kind == EvolutionType::LevelHighBeauty {
                    continue;
                }
                let extra_info = read_word(entry, slot * 6 + 2);
                let target = self.pk(target);
                let evolution = Evolution::new(pokemon, target, true, kind, extra_info);
                let data = &mut self.core.data;
                if !data.evolution_list_contains(data.evolutions_from(pokemon), &evolution) {
                    let id = data.new_evolution(evolution);
                    data.evolutions_from_mut(pokemon).push(id);
                    data.evolutions_to_mut(target).push(id);
                }
            }
            let data = &mut self.core.data;
            if data.evolutions_from(pokemon).len() > 1 {
                for evo in data.evolutions_from(pokemon).to_vec() {
                    if data.evo(evo).kind != EvolutionType::LevelCreateExtra {
                        data.evo_mut(evo).carry_stats = false;
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn write_evolutions(&mut self) -> Result<()> {
        let mut evolutions = self.read_narc("PokemonEvolutions")?;
        for number in 1..=gen5::POKEMON_COUNT {
            let pokemon = self.pk(number);
            if number == species::NINCADA && self.entry.tweak("ShedinjaEvolutionTweak").is_some() {
                self.write_shedinja_evolution()?;
            }
            let entry = evolutions
                .files
                .get_mut(number as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("evoNARC.files"))?;
            let data = &self.core.data;
            let mut written = 0;
            for &evo in data.evolutions_from(pokemon) {
                let evolution = data.evo(evo);
                write_word(entry, written * 6, evolution.kind.to_index(5));
                write_word(entry, written * 6 + 2, evolution.extra_info);
                write_word(entry, written * 6 + 4, data.pk(evolution.to).number);
                written += 1;
                if written == EVOLUTION_SLOTS {
                    break;
                }
            }
            while written < EVOLUTION_SLOTS {
                write_word(entry, written * 6, 0);
                write_word(entry, written * 6 + 2, 0);
                write_word(entry, written * 6 + 4, 0);
                written += 1;
            }
        }
        self.write_narc("PokemonEvolutions", &evolutions)
    }

    fn write_shedinja_evolution(&mut self) -> Result<()> {
        let nincada = self.pk(species::NINCADA);
        let data = &self.core.data;
        if data.evolutions_from(nincada).len() < 2 {
            return Ok(());
        }
        let extra_evolution = data
            .pk(data.evo(data.evolutions_from(nincada)[1]).to)
            .number;
        let overlay_number = self.entry.int("EvolutionOvlNumber");
        let mut overlay = self.read_overlay(overlay_number)?;
        self.generic_ips_patch(&mut overlay, "ShedinjaEvolutionOvlTweak")?;
        self.write_overlay(overlay_number, &overlay)?;
        let mut arm9 = std::mem::take(&mut self.arm9);
        let patched = self.generic_ips_patch(&mut arm9, "ShedinjaEvolutionTweak");
        self.arm9 = arm9;
        patched?;
        let offset = self.entry.int("ShedinjaSpeciesOffset");
        if offset > 0 {
            write_full_int(&mut self.arm9, offset, extra_evolution);
        }
        Ok(())
    }

    fn add_extra_evolutions(&mut self, pokemon: PokemonId, extra: &JavaHashSet<ExtraEvolution>) {
        let data = &mut self.core.data;
        let created: Vec<EvolutionId> = extra
            .iter()
            .map(|evolution| data.new_evolution(evolution.0.clone()))
            .collect();
        data.evolutions_from_mut(pokemon)
            .extend(created.iter().copied());
        for id in created {
            let target = data.evo(id).to;
            data.evolutions_to_mut(target).push(id);
        }
    }

    fn item_name(&self, item: i32) -> Result<String> {
        self.item_names
            .get(item as usize)
            .cloned()
            .ok_or(RandomizationError::IndexOutOfBounds("itemNames"))
    }

    pub(crate) fn remove_impossible_evolutions_impl(&mut self, settings: &Settings) -> Result<()> {
        let change_move_evos = settings.movesets_mod != MovesetsMod::Unchanged;
        let movesets = self.get_moves_learnt();
        let mut extra_evolutions: JavaHashSet<ExtraEvolution> = JavaHashSet::new();
        for pokemon in self.all_pokemon_ids() {
            extra_evolutions.clear();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                if change_move_evos && self.core.data.evo(evo).kind == EvolutionType::LevelWithMove
                {
                    let move_number = self.core.data.evo(evo).extra_info;
                    let from = self.core.data.pk(self.core.data.evo(evo).from).number;
                    let learnt = movesets
                        .get(&from)
                        .ok_or(RandomizationError::NullPointer("movesets.get"))?;
                    let mut level_learnt_at = learnt
                        .iter()
                        .find(|learnt| learnt.move_number == move_number)
                        .map_or(1, |learnt| learnt.level);
                    if level_learnt_at == 1 {
                        level_learnt_at = 45;
                    }
                    let evolution = self.core.data.evo_mut(evo);
                    evolution.kind = EvolutionType::Level;
                    evolution.extra_info = level_learnt_at;
                    let core = &mut self.core;
                    add_evo_update_level(
                        &core.data,
                        &mut core.engine.impossible_evolution_updates,
                        evo,
                    );
                }
                if self.core.data.evo(evo).kind == EvolutionType::Trade {
                    let evolution = self.core.data.evo_mut(evo);
                    evolution.kind = EvolutionType::Level;
                    evolution.extra_info = 37;
                    let core = &mut self.core;
                    add_evo_update_level(
                        &core.data,
                        &mut core.engine.impossible_evolution_updates,
                        evo,
                    );
                }
                if self.core.data.evo(evo).kind == EvolutionType::TradeItem {
                    let item = self.core.data.evo(evo).extra_info;
                    let from = self.core.data.evo(evo).from;
                    if self.core.data.pk(from).number == species::SLOWPOKE {
                        let evolution = self.core.data.evo_mut(evo);
                        evolution.kind = EvolutionType::Stone;
                        evolution.extra_info = items::WATER_STONE;
                        let name = self.item_name(items::WATER_STONE)?;
                        let core = &mut self.core;
                        add_evo_update_stone(
                            &core.data,
                            &mut core.engine.impossible_evolution_updates,
                            evo,
                            &name,
                        );
                    } else {
                        let name = self.item_name(item)?;
                        let core = &mut self.core;
                        add_evo_update_held_item(
                            &core.data,
                            &mut core.engine.impossible_evolution_updates,
                            evo,
                            &name,
                        );
                        let evolution = core.data.evo_mut(evo);
                        evolution.kind = EvolutionType::LevelItemDay;
                        let extra = Evolution::new(
                            evolution.from,
                            evolution.to,
                            true,
                            EvolutionType::LevelItemNight,
                            item,
                        );
                        extra_evolutions.add(ExtraEvolution(extra));
                    }
                }
                if self.core.data.evo(evo).kind == EvolutionType::TradeSpecial {
                    let from = self.core.data.pk(self.core.data.evo(evo).from).number;
                    let partner = if from == species::KARRABLAST {
                        species::SHELMET
                    } else {
                        species::KARRABLAST
                    };
                    let evolution = self.core.data.evo_mut(evo);
                    evolution.kind = EvolutionType::LevelWithOther;
                    evolution.extra_info = partner;
                    let partner_name = self.core.data.full_name(self.pk(partner));
                    let core = &mut self.core;
                    add_evo_update_party(
                        &core.data,
                        &mut core.engine.impossible_evolution_updates,
                        evo,
                        &partner_name,
                    );
                }
            }
            self.add_extra_evolutions(pokemon, &extra_evolutions);
        }
        Ok(())
    }

    pub(crate) fn make_evolutions_easier_impl(&mut self, settings: &Settings) -> Result<()> {
        let wilds_randomized = settings.wild_pokemon_mod != WildPokemonMod::Unchanged;
        let offset = find(&self.arm9, gen5::FRIENDSHIP_VALUE_FOR_EVO_LOCATOR);
        if offset > 0 {
            for delta in [0, 20, 38] {
                let index = (offset + delta) as usize;
                if self.arm9[index] == HAPPINESS_THRESHOLD {
                    self.arm9[index] = EASIER_HAPPINESS_THRESHOLD;
                }
            }
        }
        if wilds_randomized {
            for pokemon in self.all_pokemon_ids() {
                for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                    if self.core.data.evo(evo).kind == EvolutionType::LevelWithOther {
                        let evolution = self.core.data.evo_mut(evo);
                        evolution.kind = EvolutionType::Level;
                        evolution.extra_info = 35;
                        let core = &mut self.core;
                        add_evo_update_condensed(
                            &core.data,
                            &mut core.engine.easier_evolution_updates,
                            evo,
                            false,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn has_item_evolution(&self, pokemon: PokemonId, kind: EvolutionType, item: i32) -> bool {
        let data = &self.core.data;
        data.evolutions_from(pokemon)
            .iter()
            .any(|&evo| data.evo(evo).kind == kind && data.evo(evo).extra_info == item)
    }

    fn eevee_stone_evolution(&mut self, evo: EvolutionId, stone: i32) -> Result<()> {
        let evolution = self.core.data.evo_mut(evo);
        evolution.kind = EvolutionType::Stone;
        evolution.extra_info = stone;
        let name = self.item_name(stone)?;
        let core = &mut self.core;
        add_evo_update_stone(
            &core.data,
            &mut core.engine.time_based_evolution_updates,
            evo,
            &name,
        );
        Ok(())
    }

    fn happiness_twin(
        &mut self,
        evo: EvolutionId,
        kind: EvolutionType,
        extra: &mut JavaHashSet<ExtraEvolution>,
    ) {
        let core = &mut self.core;
        add_evo_update_happiness(
            &core.data,
            &mut core.engine.time_based_evolution_updates,
            evo,
        );
        let evolution = core.data.evo(evo);
        extra.add(ExtraEvolution(Evolution::new(
            evolution.from,
            evolution.to,
            true,
            kind,
            0,
        )));
    }

    fn held_item_twin(
        &mut self,
        evo: EvolutionId,
        existing_kind: EvolutionType,
        extra: &mut JavaHashSet<ExtraEvolution>,
    ) -> Result<()> {
        let item = self.core.data.evo(evo).extra_info;
        let from = self.core.data.evo(evo).from;
        if self.has_item_evolution(from, existing_kind, item) {
            return Ok(());
        }
        let name = self.item_name(item)?;
        let core = &mut self.core;
        add_evo_update_held_item(
            &core.data,
            &mut core.engine.time_based_evolution_updates,
            evo,
            &name,
        );
        let evolution = core.data.evo(evo);
        extra.add(ExtraEvolution(Evolution::new(
            evolution.from,
            evolution.to,
            true,
            existing_kind,
            item,
        )));
        Ok(())
    }

    pub(crate) fn remove_time_based_evolutions_impl(&mut self) -> Result<()> {
        let mut extra_evolutions: JavaHashSet<ExtraEvolution> = JavaHashSet::new();
        for pokemon in self.all_pokemon_ids() {
            extra_evolutions.clear();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                let kind = self.core.data.evo(evo).kind;
                let from_number = self.core.data.pk(self.core.data.evo(evo).from).number;
                match kind {
                    EvolutionType::HappinessDay if from_number == species::EEVEE => {
                        self.eevee_stone_evolution(evo, items::SUN_STONE)?
                    }
                    EvolutionType::HappinessDay => self.happiness_twin(
                        evo,
                        EvolutionType::HappinessNight,
                        &mut extra_evolutions,
                    ),
                    EvolutionType::HappinessNight if from_number == species::EEVEE => {
                        self.eevee_stone_evolution(evo, items::MOON_STONE)?
                    }
                    EvolutionType::HappinessNight => {
                        self.happiness_twin(evo, EvolutionType::HappinessDay, &mut extra_evolutions)
                    }
                    EvolutionType::LevelItemDay => self.held_item_twin(
                        evo,
                        EvolutionType::LevelItemNight,
                        &mut extra_evolutions,
                    )?,
                    EvolutionType::LevelItemNight => self.held_item_twin(
                        evo,
                        EvolutionType::LevelItemDay,
                        &mut extra_evolutions,
                    )?,
                    _ => {}
                }
            }
            self.add_extra_evolutions(pokemon, &extra_evolutions);
        }
        Ok(())
    }

    pub(crate) fn remove_evos_for_pokemon_pool_impl(&mut self) -> Result<()> {
        let included = self.core.engine.pool.main.clone();
        for pokemon in self.all_pokemon_ids() {
            let mut keep: Vec<EvolutionId> = Vec::new();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                let evolution = self.core.data.evo(evo);
                if included.contains(&evolution.from) && included.contains(&evolution.to) {
                    keep.push(evo);
                } else {
                    let target = evolution.to;
                    self.core.data.remove_first_equal_to(target, evo);
                }
            }
            let data = &mut self.core.data;
            let kept: Vec<EvolutionId> = data
                .evolutions_from(pokemon)
                .iter()
                .copied()
                .filter(|&evo| {
                    keep.iter()
                        .any(|&kept| data.evo(kept).java_equals(data.evo(evo)))
                })
                .collect();
            *data.evolutions_from_mut(pokemon) = kept;
        }
        let mut babies = self.read_narc("BabyPokemon")?;
        for number in 1..=gen5::POKEMON_COUNT {
            let data = &self.core.data;
            let mut baby = self.pk(number);
            while let Some(&first) = data.evolutions_to(baby).first() {
                baby = data.evo(first).from;
            }
            let file = babies
                .files
                .get_mut(number as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("babyNARC.files"))?;
            write_word(file, 0, data.pk(baby).number);
        }
        self.write_narc("BabyPokemon", &babies)
    }
}
