use randomon_core::engine::evolutions::{
    add_evo_update_condensed, add_evo_update_happiness, add_evo_update_held_item,
    add_evo_update_level, add_evo_update_party, add_evo_update_stone,
};
use randomon_core::{Movesets, RandomizationError, Result};
use randomon_data::constants::{items, species};
use randomon_data::{Evolution, EvolutionId, EvolutionType, PokemonId};
use randomon_java::{JavaHash, JavaHashSet};
use randomon_settings::{MovesetsMod, Settings, WildPokemonMod};

use crate::formats::evolution::{Evolution as EvolutionRecord, MAX_EVOLUTIONS, method};
use crate::handler::SwshRomHandler;

const GENERATION_TABLE: i32 = 7;
const SPECIAL_METHOD_LEVEL: i32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawEvolution {
    pub kind: EvolutionType,
    pub extra_info: i32,
    pub to: PokemonId,
    pub record: EvolutionRecord,
}

#[derive(Clone, Debug)]
struct EvolutionKey {
    evolution: Evolution,
}

impl PartialEq for EvolutionKey {
    fn eq(&self, other: &Self) -> bool {
        self.evolution.java_equals(&other.evolution)
    }
}

impl Eq for EvolutionKey {}

impl JavaHash for EvolutionKey {
    fn java_hash_code(&self) -> i32 {
        self.evolution.java_hash_code()
    }
}

fn evolution_with_forme(
    from: PokemonId,
    to: PokemonId,
    kind: EvolutionType,
    extra_info: i32,
    forme: i32,
) -> Evolution {
    let mut evolution = Evolution::new(from, to, true, kind, extra_info);
    evolution.forme = forme;
    evolution
}

fn engine_kind(record: &EvolutionRecord) -> Option<(EvolutionType, i32)> {
    let level = i32::from(record.level);
    let argument = i32::from(record.argument);
    if let Some(kind) = EvolutionType::from_index(GENERATION_TABLE, i32::from(record.method)) {
        let extra = if kind.uses_level() { level } else { argument };
        return Some((kind, extra));
    }
    match record.method {
        method::LEVEL_UP_NATURE_AMPED | method::LEVEL_UP_NATURE_LOW_KEY => {
            Some((EvolutionType::Level, level))
        }
        method::CRITICAL_HITS_IN_BATTLE
        | method::HIT_POINTS_LOST_IN_BATTLE
        | method::SPIN
        | method::TOWER_OF_DARKNESS
        | method::TOWER_OF_WATERS => Some((EvolutionType::Level, SPECIAL_METHOD_LEVEL)),
        _ => None,
    }
}

impl SwshRomHandler {
    fn evolution_source_forme(&self, from: PokemonId) -> i32 {
        let entry = self.core.data.pk(from);
        if entry.base_forme.is_some() {
            entry.forme_number
        } else {
            0
        }
    }

    pub(crate) fn load_evolutions(&mut self) {
        self.raw_evolutions = vec![Vec::new(); self.pokes.len()];
        for from in self.all_pokemon() {
            let Some(set) = self
                .files
                .data
                .evolutions
                .get(from.number() as usize)
                .cloned()
            else {
                continue;
            };
            for record in set.active() {
                let Some((kind, extra_info)) = engine_kind(record) else {
                    continue;
                };
                let forme = if record.form < 0 {
                    self.evolution_source_forme(from)
                } else {
                    i32::from(record.form)
                };
                let Some(to) = self.pokemon_with_forme(i32::from(record.species), forme) else {
                    self.dropped_evolutions += 1;
                    continue;
                };
                let mut evolution = Evolution::new(from, to, true, kind, extra_info);
                evolution.forme = forme;
                evolution.level = i32::from(record.level);
                let data = &self.core.data;
                if data.evolution_list_contains(data.evolutions_from(from), &evolution) {
                    continue;
                }
                self.raw_evolutions[from.number() as usize].push(RawEvolution {
                    kind,
                    extra_info,
                    to,
                    record: *record,
                });
                let evo = self.core.data.new_evolution(evolution);
                self.core.data.evolutions_from_mut(from).push(evo);
                if !self.core.data.pk(from).actually_cosmetic {
                    self.core.data.evolutions_to_mut(to).push(evo);
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

    fn fresh_record(&self, evolution: &Evolution) -> Option<EvolutionRecord> {
        let method = evolution.kind.to_index(GENERATION_TABLE);
        if method <= 0 {
            return None;
        }
        let (species_number, form) = self.species_and_form(evolution.to, evolution.forme);
        let uses_level = evolution.kind.uses_level();
        let (argument, level) = if uses_level {
            (0, evolution.extra_info)
        } else {
            (evolution.extra_info, evolution.level.max(0))
        };
        Some(EvolutionRecord {
            method: method as u16,
            argument: argument.clamp(0, i32::from(u16::MAX)) as u16,
            species: species_number as u16,
            form: form as i8,
            level: level.clamp(0, 100) as u8,
        })
    }

    fn record_for(
        &self,
        evolution: &Evolution,
        unused: &mut Vec<RawEvolution>,
    ) -> Option<EvolutionRecord> {
        let matching = unused
            .iter()
            .position(|raw| raw.kind == evolution.kind && raw.extra_info == evolution.extra_info);
        let Some(position) = matching else {
            return self.fresh_record(evolution);
        };
        let raw = unused.remove(position);
        if raw.to == evolution.to {
            return Some(raw.record);
        }
        let (species_number, form) = self.species_and_form(evolution.to, evolution.forme);
        Some(EvolutionRecord {
            species: species_number as u16,
            form: form as i8,
            ..raw.record
        })
    }

    pub(crate) fn save_evolutions(&mut self) {
        for from in self.all_pokemon() {
            let number = from.number() as usize;
            let Some(set) = self.files.data.evolutions.get(number) else {
                continue;
            };
            let mut unused = self.raw_evolutions[number].clone();
            let records: Vec<EvolutionRecord> = self
                .core
                .data
                .evolutions_from(from)
                .iter()
                .filter_map(|&evo| self.record_for(self.core.data.evo(evo), &mut unused))
                .take(MAX_EVOLUTIONS)
                .collect();
            let current: Vec<EvolutionRecord> = set.active().copied().collect();
            if current != records {
                self.files.data.evolutions[number].evolutions = records;
            }
        }
    }

    fn add_extra_evolutions(&mut self, pokemon: PokemonId, extras: &JavaHashSet<EvolutionKey>) {
        let data = &mut self.core.data;
        let mut created = Vec::new();
        for extra in extras.iter() {
            let id = data.new_evolution(extra.evolution.clone());
            data.evolutions_from_mut(pokemon).push(id);
            created.push(id);
        }
        for id in created {
            let to = data.evo(id).to;
            data.evolutions_to_mut(to).push(id);
        }
    }

    pub(crate) fn remove_evos_for_pool(&mut self) {
        let included = self.core.engine.pool.main_incl_formes.clone();
        for pokemon in self.all_pokemon() {
            let mut keep: Vec<Evolution> = Vec::new();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                let evolution = self.core.data.evo(evo).clone();
                if included.contains(&evolution.from) && included.contains(&evolution.to) {
                    keep.push(evolution);
                } else {
                    self.core.data.remove_first_equal_to(evolution.to, evo);
                }
            }
            let data = &mut self.core.data;
            let retained: Vec<EvolutionId> = data
                .evolutions_from(pokemon)
                .iter()
                .copied()
                .filter(|&evo| keep.iter().any(|kept| kept.java_equals(data.evo(evo))))
                .collect();
            *data.evolutions_from_mut(pokemon) = retained;
        }
    }

    pub(crate) fn remove_impossible_evos(&mut self, settings: &Settings) -> Result<()> {
        let change_move_evos = settings.movesets_mod != MovesetsMod::Unchanged;
        let movesets = self.read_movesets();
        let mut extra_evolutions: JavaHashSet<EvolutionKey> = JavaHashSet::new();
        for pokemon in self.all_pokemon() {
            extra_evolutions.clear();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                self.fix_impossible_evolution(
                    evo,
                    change_move_evos,
                    &movesets,
                    &mut extra_evolutions,
                )?;
            }
            self.add_extra_evolutions(pokemon, &extra_evolutions);
        }
        Ok(())
    }

    fn item_name(&self, item: i32) -> String {
        self.item_names
            .get(item as usize)
            .cloned()
            .unwrap_or_default()
    }

    fn fix_impossible_evolution(
        &mut self,
        evo: EvolutionId,
        change_move_evos: bool,
        movesets: &Movesets,
        extra_evolutions: &mut JavaHashSet<EvolutionKey>,
    ) -> Result<()> {
        let water_stone = self.item_name(items::WATER_STONE);
        let held_item_name = self.item_name(self.core.data.evo(evo).extra_info);
        let data = &mut self.core.data;
        let updates = &mut self.core.engine.impossible_evolution_updates;
        if change_move_evos && data.evo(evo).kind == EvolutionType::LevelWithMove {
            let move_number = data.evo(evo).extra_info;
            let from_number = data.evo(evo).from.number();
            let learnt = movesets
                .get(&from_number)
                .ok_or(RandomizationError::NullPointer("movesets.get"))?;
            let mut level = learnt
                .iter()
                .find(|learnt| learnt.move_number == move_number)
                .map_or(1, |learnt| learnt.level);
            if level == 1 {
                level = 45;
            }
            let evolution = data.evo_mut(evo);
            evolution.kind = EvolutionType::Level;
            evolution.extra_info = level;
            add_evo_update_level(data, updates, evo);
        }
        if data.evo(evo).kind == EvolutionType::Trade {
            let evolution = data.evo_mut(evo);
            evolution.kind = EvolutionType::Level;
            evolution.extra_info = 37;
            add_evo_update_level(data, updates, evo);
        }
        if data.evo(evo).kind == EvolutionType::TradeItem {
            let item = data.evo(evo).extra_info;
            if data.evo(evo).from.number() == species::SLOWPOKE {
                let evolution = data.evo_mut(evo);
                evolution.kind = EvolutionType::Stone;
                evolution.extra_info = items::WATER_STONE;
                add_evo_update_stone(data, updates, evo, &water_stone);
            } else {
                add_evo_update_held_item(data, updates, evo, &held_item_name);
                data.evo_mut(evo).kind = EvolutionType::LevelItemDay;
                let evolution = data.evo(evo);
                extra_evolutions.add(EvolutionKey {
                    evolution: evolution_with_forme(
                        evolution.from,
                        evolution.to,
                        EvolutionType::LevelItemNight,
                        item,
                        evolution.forme,
                    ),
                });
            }
        }
        if data.evo(evo).kind == EvolutionType::TradeSpecial {
            let partner = if data.evo(evo).from.number() == species::KARRABLAST {
                species::SHELMET
            } else {
                species::KARRABLAST
            };
            let evolution = data.evo_mut(evo);
            evolution.kind = EvolutionType::LevelWithOther;
            evolution.extra_info = partner;
            let partner_name = if data.exists(PokemonId::from_index(partner as usize)) {
                data.full_name(PokemonId::from_index(partner as usize))
            } else {
                String::new()
            };
            add_evo_update_party(data, updates, evo, &partner_name);
        }
        Ok(())
    }

    pub(crate) fn make_evos_easier(&mut self, settings: &Settings) {
        if settings.wild_pokemon_mod == WildPokemonMod::Unchanged {
            return;
        }
        for pokemon in self.all_pokemon() {
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                let data = &mut self.core.data;
                let updates = &mut self.core.engine.easier_evolution_updates;
                if data.evo(evo).kind == EvolutionType::LevelWithOther {
                    let evolution = data.evo_mut(evo);
                    evolution.kind = EvolutionType::Level;
                    evolution.extra_info = 35;
                    add_evo_update_condensed(data, updates, evo, false);
                }
            }
        }
    }

    pub(crate) fn remove_time_evos(&mut self) {
        let mut extra_evolutions: JavaHashSet<EvolutionKey> = JavaHashSet::new();
        for pokemon in self.all_pokemon() {
            extra_evolutions.clear();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                self.fix_time_based_evolution(evo, &mut extra_evolutions);
            }
            self.add_extra_evolutions(pokemon, &extra_evolutions);
        }
    }

    fn make_stone_evolution(&mut self, evo: EvolutionId, stone: i32) {
        let name = self.item_name(stone);
        let data = &mut self.core.data;
        let evolution = data.evo_mut(evo);
        evolution.kind = EvolutionType::Stone;
        evolution.extra_info = stone;
        add_evo_update_stone(
            data,
            &mut self.core.engine.time_based_evolution_updates,
            evo,
            &name,
        );
    }

    fn fix_time_based_evolution(
        &mut self,
        evo: EvolutionId,
        extra_evolutions: &mut JavaHashSet<EvolutionKey>,
    ) {
        let evolution = self.core.data.evo(evo).clone();
        let from_number = evolution.from.number();
        match evolution.kind {
            EvolutionType::HappinessDay if from_number == species::EEVEE => {
                self.make_stone_evolution(evo, items::SUN_STONE)
            }
            EvolutionType::HappinessNight if from_number == species::EEVEE => {
                self.make_stone_evolution(evo, items::MOON_STONE)
            }
            EvolutionType::HappinessDay | EvolutionType::HappinessNight => {
                add_evo_update_happiness(
                    &self.core.data,
                    &mut self.core.engine.time_based_evolution_updates,
                    evo,
                );
                let other = if evolution.kind == EvolutionType::HappinessDay {
                    EvolutionType::HappinessNight
                } else {
                    EvolutionType::HappinessDay
                };
                extra_evolutions.add(EvolutionKey {
                    evolution: evolution_with_forme(
                        evolution.from,
                        evolution.to,
                        other,
                        0,
                        evolution.forme,
                    ),
                });
            }
            EvolutionType::LevelItemDay | EvolutionType::LevelItemNight => {
                self.add_opposite_item_evolution(evo, &evolution, extra_evolutions)
            }
            EvolutionType::LevelDay | EvolutionType::LevelNight => {
                let data = &mut self.core.data;
                add_evo_update_level(
                    data,
                    &mut self.core.engine.time_based_evolution_updates,
                    evo,
                );
                data.evo_mut(evo).kind = EvolutionType::Level;
            }
            _ => {}
        }
    }

    fn add_opposite_item_evolution(
        &mut self,
        evo: EvolutionId,
        evolution: &Evolution,
        extra_evolutions: &mut JavaHashSet<EvolutionKey>,
    ) {
        let other = if evolution.kind == EvolutionType::LevelItemDay {
            EvolutionType::LevelItemNight
        } else {
            EvolutionType::LevelItemDay
        };
        let item = evolution.extra_info;
        let data = &self.core.data;
        let already_present = data
            .evolutions_from(evolution.from)
            .iter()
            .any(|&other_evo| {
                data.evo(other_evo).kind == other && data.evo(other_evo).extra_info == item
            });
        if already_present {
            return;
        }
        let name = self.item_name(item);
        add_evo_update_held_item(
            data,
            &mut self.core.engine.time_based_evolution_updates,
            evo,
            &name,
        );
        extra_evolutions.add(EvolutionKey {
            evolution: evolution_with_forme(
                evolution.from,
                evolution.to,
                other,
                item,
                evolution.forme,
            ),
        });
    }
}
