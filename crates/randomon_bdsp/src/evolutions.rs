use randomon_core::engine::evolutions::{
    add_evo_update_condensed, add_evo_update_happiness, add_evo_update_held_item,
    add_evo_update_level, add_evo_update_party, add_evo_update_stone,
};
use randomon_core::{Movesets, RandomizationError, Result};
use randomon_data::constants::{items, species};
use randomon_data::{Evolution, EvolutionId, EvolutionType, PokemonId};
use randomon_java::{JavaHash, JavaHashSet};
use randomon_settings::{MovesetsMod, Settings, WildPokemonMod};

use crate::handler::BdspRomHandler;

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

impl BdspRomHandler {
    fn all_pokemon(&self) -> Vec<PokemonId> {
        self.pokes.iter().flatten().copied().collect()
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
        let movesets = self.read_movesets()?;
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

    fn fix_impossible_evolution(
        &mut self,
        evo: EvolutionId,
        change_move_evos: bool,
        movesets: &Movesets,
        extra_evolutions: &mut JavaHashSet<EvolutionKey>,
    ) -> Result<()> {
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
                let name = self.item_names[items::WATER_STONE as usize].clone();
                add_evo_update_stone(data, updates, evo, &name);
            } else {
                let name = self
                    .item_names
                    .get(item as usize)
                    .cloned()
                    .unwrap_or_default();
                add_evo_update_held_item(data, updates, evo, &name);
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
            let partner_name = data.full_name(PokemonId::from_index(partner as usize));
            add_evo_update_party(data, updates, evo, &partner_name);
        }
        Ok(())
    }

    pub(crate) fn make_evos_easier(&mut self, settings: &Settings) {
        let wilds_randomized = settings.wild_pokemon_mod != WildPokemonMod::Unchanged;
        if !wilds_randomized {
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
        let data = &mut self.core.data;
        let evolution = data.evo_mut(evo);
        evolution.kind = EvolutionType::Stone;
        evolution.extra_info = stone;
        let name = self.item_names[stone as usize].clone();
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
        let name = self
            .item_names
            .get(item as usize)
            .cloned()
            .unwrap_or_default();
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
