use std::cmp::Ordering;

use randomon_core::engine::evolutions::{
    add_evo_update_condensed, add_evo_update_happiness, add_evo_update_held_item,
    add_evo_update_level, add_evo_update_stone,
};
use randomon_core::{RandomizationError, Result};
use randomon_data::constants::{gen4, items, species};
use randomon_data::{Evolution, EvolutionId, EvolutionType, PokemonId};
use randomon_java::{JavaHash, JavaHashSet};
use randomon_settings::{MovesetsMod, Settings, WildPokemonMod};

use crate::bytes::{file, file_mut, found_at, set_bytes, set_u16, u8_at, u16_at};
use crate::handler::Gen4RomHandler;

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
const HAPPINESS_THRESHOLD: i32 = 220;
const EASIER_HAPPINESS_THRESHOLD: i32 = 160;
const HAPPINESS_OFFSETS: [i32; 3] = [0, 22, 44];
const MAX_BABY_SEARCH_STEPS: usize = 1000;

impl Gen4RomHandler {
    pub(crate) fn all_pokemon_ids(&self) -> Vec<PokemonId> {
        self.pokemon_list_incl_formes
            .iter()
            .flatten()
            .copied()
            .collect()
    }

    pub(crate) fn populate_evolutions(&mut self) -> Result<()> {
        let evolutions = self.read_narc("PokemonEvolutions")?;
        for number in 1..=gen4::POKEMON_COUNT {
            let pokemon = self.pk(number);
            let entry = file(&evolutions, number)?;
            for slot in 0..EVOLUTION_SLOTS {
                let method = u16_at(entry, slot * 6)?;
                let target = u16_at(entry, slot * 6 + 4)?;
                if !(1..=gen4::EVOLUTION_METHOD_COUNT).contains(&method) || target < 1 {
                    continue;
                }
                let kind = EvolutionType::from_index(4, method)
                    .ok_or(RandomizationError::NullPointer("EvolutionType.fromIndex"))?;
                let extra_info = u16_at(entry, slot * 6 + 2)?;
                let target = self.pokemon_by_number(target)?;
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
        for number in 1..=gen4::POKEMON_COUNT {
            if number == species::NINCADA {
                self.write_shedinja_evolution()?;
            }
            let pokemon = self.pk(number);
            let entry = file_mut(&mut evolutions, number)?;
            let data = &self.core.data;
            let mut written = 0;
            for &evo in data.evolutions_from(pokemon) {
                if written == EVOLUTION_SLOTS {
                    break;
                }
                let evolution = data.evo(evo);
                set_u16(entry, written * 6, evolution.kind.to_index(4))?;
                set_u16(entry, written * 6 + 2, evolution.extra_info)?;
                set_u16(entry, written * 6 + 4, data.pk(evolution.to).number)?;
                written += 1;
            }
            while written < EVOLUTION_SLOTS {
                set_bytes(entry, written * 6, &[0; 6])?;
                written += 1;
            }
        }
        self.write_narc("PokemonEvolutions", &evolutions)
    }

    fn write_shedinja_evolution(&mut self) -> Result<()> {
        let nincada = self.pk(species::NINCADA);
        let data = &self.core.data;
        let Some(&extra) = data.evolutions_from(nincada).get(1) else {
            return Ok(());
        };
        let extra_evolution = data.pk(data.evo(extra).to).number;
        let Some(offset) = found_at(&self.arm9, gen4::SHEDINJA_SPECIES_LOCATOR) else {
            return Ok(());
        };
        let (low_byte, high_byte) = if extra_evolution < 256 {
            (extra_evolution, 0)
        } else {
            (255, extra_evolution - 255)
        };
        set_bytes(
            &mut self.arm9,
            offset,
            &[low_byte as u8, 0x20, high_byte as u8, 0x30],
        )
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

    pub(crate) fn item_name(&self, item: i32) -> Result<String> {
        usize::try_from(item)
            .ok()
            .and_then(|item| self.item_names.get(item))
            .cloned()
            .ok_or(RandomizationError::IndexOutOfBounds("itemNames"))
    }

    fn convert_to_level(&mut self, evo: EvolutionId, level: i32) {
        let evolution = self.core.data.evo_mut(evo);
        evolution.kind = EvolutionType::Level;
        evolution.extra_info = level;
        let core = &mut self.core;
        add_evo_update_level(
            &core.data,
            &mut core.engine.impossible_evolution_updates,
            evo,
        );
    }

    fn convert_to_stone(&mut self, evo: EvolutionId, stone: i32) -> Result<()> {
        let evolution = self.core.data.evo_mut(evo);
        evolution.kind = EvolutionType::Stone;
        evolution.extra_info = stone;
        let name = self.item_name(stone)?;
        let core = &mut self.core;
        add_evo_update_stone(
            &core.data,
            &mut core.engine.impossible_evolution_updates,
            evo,
            &name,
        );
        Ok(())
    }

    fn fix_hgss_location_evolution(&mut self, evo: EvolutionId) -> Result<()> {
        match self.core.data.evo(evo).kind {
            EvolutionType::LevelHighBeauty => self.convert_to_level(evo, 35),
            EvolutionType::LevelElectrifiedArea => self.convert_to_level(evo, 40),
            EvolutionType::LevelMossRock => self.convert_to_stone(evo, items::LEAF_STONE)?,
            EvolutionType::LevelIcyRock => self.convert_to_stone(evo, items::DAWN_STONE)?,
            _ => {}
        }
        Ok(())
    }

    fn level_learnt_at(movesets: &randomon_core::Movesets, from: i32, move_number: i32) -> i32 {
        let level = movesets
            .get(&from)
            .and_then(|learnt| learnt.iter().find(|entry| entry.move_number == move_number))
            .map_or(1, |entry| entry.level);
        if level == 1 { 45 } else { level }
    }

    fn fix_trade_item_evolution(
        &mut self,
        evo: EvolutionId,
        extra: &mut JavaHashSet<ExtraEvolution>,
    ) -> Result<()> {
        let item = self.core.data.evo(evo).extra_info;
        let from = self.core.data.evo(evo).from;
        if self.number_of(from) == species::SLOWPOKE {
            return self.convert_to_stone(evo, items::WATER_STONE);
        }
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
        extra.add(ExtraEvolution(Evolution::new(
            evolution.from,
            evolution.to,
            true,
            EvolutionType::LevelItemNight,
            item,
        )));
        Ok(())
    }

    pub(crate) fn remove_impossible_evolutions_impl(&mut self, settings: &Settings) -> Result<()> {
        let change_move_evos = settings.movesets_mod != MovesetsMod::Unchanged;
        let movesets = self.moves_learnt()?;
        let mut extra_evolutions: JavaHashSet<ExtraEvolution> = JavaHashSet::new();
        for pokemon in self.all_pokemon_ids() {
            extra_evolutions.clear();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                if self.entry.is_hgss() {
                    self.fix_hgss_location_evolution(evo)?;
                }
                if change_move_evos && self.core.data.evo(evo).kind == EvolutionType::LevelWithMove
                {
                    let from = self.number_of(self.core.data.evo(evo).from);
                    let move_number = self.core.data.evo(evo).extra_info;
                    let level = Self::level_learnt_at(&movesets, from, move_number);
                    self.convert_to_level(evo, level);
                }
                if self.core.data.evo(evo).kind == EvolutionType::Trade {
                    self.convert_to_level(evo, 37);
                }
                if self.core.data.evo(evo).kind == EvolutionType::TradeItem {
                    self.fix_trade_item_evolution(evo, &mut extra_evolutions)?;
                }
            }
            self.add_extra_evolutions(pokemon, &extra_evolutions);
        }
        Ok(())
    }

    pub(crate) fn make_evolutions_easier_impl(&mut self, settings: &Settings) -> Result<()> {
        let wilds_randomized = settings.wild_pokemon_mod != WildPokemonMod::Unchanged;
        if let Some(offset) = found_at(&self.arm9, gen4::FRIENDSHIP_VALUE_FOR_EVO_LOCATOR) {
            for delta in HAPPINESS_OFFSETS {
                if u8_at(&self.arm9, offset + delta).ok() == Some(HAPPINESS_THRESHOLD) {
                    crate::bytes::set_u8(
                        &mut self.arm9,
                        offset + delta,
                        EASIER_HAPPINESS_THRESHOLD,
                    )?;
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
                let from_number = self.number_of(self.core.data.evo(evo).from);
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

    fn baby_of(&self, pokemon: PokemonId) -> PokemonId {
        let data = &self.core.data;
        let mut baby = pokemon;
        for _ in 0..MAX_BABY_SEARCH_STEPS {
            match data.evolutions_to(baby).first() {
                Some(&first) => baby = data.evo(first).from,
                None => break,
            }
        }
        baby
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
        let mut babies = self.read_file("BabyPokemon")?;
        for number in 1..=gen4::POKEMON_COUNT {
            let baby = self.baby_of(self.pk(number));
            set_u16(&mut babies, number * 2, self.number_of(baby))?;
        }
        self.write_file("BabyPokemon", &babies)
    }
}
