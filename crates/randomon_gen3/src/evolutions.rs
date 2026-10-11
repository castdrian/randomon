use randomon_core::Result;
use randomon_core::engine::evolutions::{add_evo_update_level, add_evo_update_stone};
use randomon_data::constants::species;
use randomon_data::{Evolution, EvolutionId, EvolutionType, PokemonId};

use crate::constants as c;
use crate::handler::Gen3RomHandler;
use crate::items;

enum EvolutionChange {
    Stone(i32),
    Level(i32),
}

#[derive(Clone, Copy)]
enum UpdateLog {
    Impossible,
    TimeBased,
}

impl Gen3RomHandler {
    fn evolution_offset(&self, id: PokemonId) -> i32 {
        self.value("PokemonEvolutions") + self.internal_of(id) * c::EVOLUTION_ENTRY_SIZE
    }

    pub(crate) fn populate_evolutions(&mut self) {
        let count = self.pokemon_count();
        for from in self.real_pokemon() {
            let base = self.evolution_offset(from);
            for slot in 0..c::EVOLUTIONS_PER_POKEMON {
                let offset = base + slot * c::EVOLUTION_SLOT_SIZE;
                let method = self.rom.word(offset);
                let target_internal = self.rom.word(offset + 4);
                let in_range = (1..=c::EVOLUTION_METHOD_COUNT).contains(&method)
                    && (1..=count).contains(&target_internal);
                if !in_range {
                    continue;
                }
                let (Some(kind), Some(to)) = (
                    EvolutionType::from_index(3, method),
                    self.pokemon_from_internal(target_internal),
                ) else {
                    continue;
                };
                let extra = self.rom.word(offset + 2);
                let evolution = Evolution::new(from, to, true, kind, extra);
                let data = &mut self.core.data;
                if !data.evolution_list_contains(data.evolutions_from(from), &evolution) {
                    let id = data.new_evolution(evolution);
                    data.evolutions_from_mut(from).push(id);
                    data.evolutions_to_mut(to).push(id);
                }
            }
            let data = &mut self.core.data;
            if data.evolutions_from(from).len() > 1 {
                for evo in data.evolutions_from(from).to_vec() {
                    if data.evo(evo).kind != EvolutionType::LevelCreateExtra {
                        data.evo_mut(evo).carry_stats = false;
                    }
                }
            }
        }
    }

    pub(crate) fn write_evolutions(&mut self) {
        for from in self.real_pokemon() {
            let mut offset = self.evolution_offset(from);
            let evolutions: Vec<Evolution> = self
                .core
                .data
                .evolutions_from(from)
                .iter()
                .take(c::EVOLUTIONS_PER_POKEMON as usize)
                .map(|&evo| self.core.data.evo(evo).clone())
                .collect();
            let written = evolutions.len() as i32;
            for evolution in evolutions {
                let target = self.internal_of(evolution.to);
                self.rom.set_word(offset, evolution.kind.to_index(3));
                self.rom.set_word(offset + 2, evolution.extra_info);
                self.rom.set_word(offset + 4, target);
                self.rom.set_word(offset + 6, 0);
                offset += c::EVOLUTION_SLOT_SIZE;
            }
            for _ in written..c::EVOLUTIONS_PER_POKEMON {
                for field in 0..4 {
                    self.rom.set_word(offset + field * 2, 0);
                }
                offset += c::EVOLUTION_SLOT_SIZE;
            }
        }
    }

    fn all_loaded_evolutions(&self) -> Vec<EvolutionId> {
        self.loaded_pokemon()
            .into_iter()
            .flat_map(|id| self.core.data.evolutions_from(id).to_vec())
            .collect()
    }

    fn apply_evolution_change(
        &mut self,
        evo: EvolutionId,
        change: EvolutionChange,
        log: UpdateLog,
    ) {
        let item_names = &self.item_names;
        let core = &mut self.core;
        let updates = match log {
            UpdateLog::Impossible => &mut core.engine.impossible_evolution_updates,
            UpdateLog::TimeBased => &mut core.engine.time_based_evolution_updates,
        };
        let evolution = core.data.evo_mut(evo);
        match change {
            EvolutionChange::Stone(item) => {
                evolution.kind = EvolutionType::Stone;
                evolution.extra_info = item;
                let name = item_names
                    .get(item as usize)
                    .map(String::as_str)
                    .unwrap_or_default();
                add_evo_update_stone(&core.data, updates, evo, name);
            }
            EvolutionChange::Level(level) => {
                evolution.kind = EvolutionType::Level;
                evolution.extra_info = level;
                add_evo_update_level(&core.data, updates, evo);
            }
        }
    }

    fn frlg_impossible_change(&self, kind: EvolutionType) -> Option<EvolutionChange> {
        if !self.rom_type().is_frlg() {
            return None;
        }
        match kind {
            EvolutionType::HappinessDay => Some(EvolutionChange::Stone(items::SUN_STONE)),
            EvolutionType::HappinessNight => Some(EvolutionChange::Stone(items::MOON_STONE)),
            EvolutionType::LevelHighBeauty => Some(EvolutionChange::Level(35)),
            _ => None,
        }
    }

    fn trade_item_change(&self, evo: EvolutionId) -> EvolutionChange {
        let evolution = self.core.data.evo(evo);
        let from = self.core.data.pk(evolution.from).number;
        match from {
            species::POLIWHIRL => EvolutionChange::Level(37),
            species::SLOWPOKE => EvolutionChange::Stone(items::WATER_STONE),
            species::SEADRA => EvolutionChange::Level(40),
            species::CLAMPERL if evolution.extra_info == items::DEEP_SEA_TOOTH => {
                EvolutionChange::Level(30)
            }
            species::CLAMPERL if evolution.extra_info == items::DEEP_SEA_SCALE => {
                EvolutionChange::Stone(items::WATER_STONE)
            }
            _ => EvolutionChange::Level(30),
        }
    }

    fn trade_change(&self, evo: EvolutionId) -> Option<EvolutionChange> {
        match self.core.data.evo(evo).kind {
            EvolutionType::Trade => Some(EvolutionChange::Level(37)),
            EvolutionType::TradeItem => Some(self.trade_item_change(evo)),
            _ => None,
        }
    }

    pub(crate) fn remove_trade_and_frlg_impossible_evolutions(&mut self) {
        self.attempt_obedience_evolution_patches();
        for evo in self.all_loaded_evolutions() {
            let kind = self.core.data.evo(evo).kind;
            if let Some(change) = self.frlg_impossible_change(kind) {
                self.apply_evolution_change(evo, change, UpdateLog::Impossible);
            }
            if let Some(change) = self.trade_change(evo) {
                self.apply_evolution_change(evo, change, UpdateLog::Impossible);
            }
        }
    }

    pub(crate) fn lower_happiness_requirements(&mut self) -> Result<()> {
        let offset = self.rom.find(c::FRIENDSHIP_VALUE_FOR_EVO_LOCATOR);
        if offset <= 0 {
            return Ok(());
        }
        let mut distances = vec![0];
        if !self.rom_type().is_frlg() {
            distances.push(c::HAPPINESS_DAY_LOCATOR_DISTANCE);
            distances.push(c::HAPPINESS_NIGHT_LOCATOR_DISTANCE);
        }
        for distance in distances {
            if self.rom.byte(offset + distance) == c::HAPPINESS_FOR_EVOLUTION {
                self.rom
                    .set_byte(offset + distance, c::EASIER_HAPPINESS_FOR_EVOLUTION);
            }
        }
        Ok(())
    }

    pub(crate) fn remove_time_evolutions(&mut self) {
        for evo in self.all_loaded_evolutions() {
            let change = match self.core.data.evo(evo).kind {
                EvolutionType::HappinessDay => EvolutionChange::Stone(items::SUN_STONE),
                EvolutionType::HappinessNight => EvolutionChange::Stone(items::MOON_STONE),
                _ => continue,
            };
            self.apply_evolution_change(evo, change, UpdateLog::TimeBased);
        }
    }

    pub(crate) fn remove_evolutions_outside_pool(&mut self) {
        let included: Vec<PokemonId> = self.core.engine.pool.main.clone();
        for id in self.loaded_pokemon() {
            let mut keep = Vec::new();
            for evo in self.core.data.evolutions_from(id).to_vec() {
                let evolution = self.core.data.evo(evo).clone();
                if included.contains(&evolution.from) && included.contains(&evolution.to) {
                    keep.push(evolution);
                } else {
                    self.core.data.remove_first_equal_to(evolution.to, evo);
                }
            }
            let data = &mut self.core.data;
            let retained: Vec<EvolutionId> = data
                .evolutions_from(id)
                .iter()
                .copied()
                .filter(|&evo| keep.iter().any(|kept| data.evo(evo).java_equals(kept)))
                .collect();
            *data.evolutions_from_mut(id) = retained;
        }
    }
}
