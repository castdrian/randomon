use std::collections::BTreeSet;

use randomon_core::{RandomizationError, Result};
use randomon_data::constants::global;
use randomon_data::{Encounter, EncounterSet, PokemonId};

use crate::constants as c;
use crate::handler::Gen3RomHandler;

const WILD_TABLE_END: u8 = 0xFF;

#[derive(Clone, Copy)]
struct WildArea {
    pointer_slot: i32,
    slots: i32,
    suffix: &'static str,
}

const AREAS: [WildArea; 4] = [
    WildArea {
        pointer_slot: 4,
        slots: c::GRASS_SLOTS,
        suffix: " Grass/Cave",
    },
    WildArea {
        pointer_slot: 8,
        slots: c::SURFING_SLOTS,
        suffix: " Surfing",
    },
    WildArea {
        pointer_slot: 12,
        slots: c::ROCK_SMASH_SLOTS,
        suffix: " Rock Smash",
    },
    WildArea {
        pointer_slot: 16,
        slots: c::FISHING_SLOTS,
        suffix: " Fishing",
    },
];

struct WildHeaderVisit {
    header: i32,
    area: WildArea,
    area_offset: i32,
}

impl Gen3RomHandler {
    fn wild_header_visits(&self) -> Vec<WildHeaderVisit> {
        let mut visits = Vec::new();
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        let mut header = self.value("WildPokemon");
        while self.rom.contains_range(header, c::WILD_HEADER_SIZE) {
            if self.rom.byte(header) == WILD_TABLE_END
                && self.rom.byte(header + 1) == WILD_TABLE_END
            {
                break;
            }
            for area in AREAS {
                let area_offset = self.rom.pointer(header + area.pointer_slot);
                if !self.rom.contains(area_offset) || self.rom.byte(area_offset) == 0 {
                    continue;
                }
                let data = self.rom.pointer(area_offset + 4);
                if seen.insert(data) {
                    visits.push(WildHeaderVisit {
                        header,
                        area,
                        area_offset,
                    });
                }
            }
            header += c::WILD_HEADER_SIZE;
        }
        visits
    }

    fn read_wild_area(&self, visit: &WildHeaderVisit) -> EncounterSet {
        let bank = self.rom.int(visit.header);
        let map = self.rom.int(visit.header + 1);
        let data = self.rom.pointer(visit.area_offset + 4);
        let encounters = (0..visit.area.slots)
            .map(|slot| {
                let entry = data + slot * 4;
                Encounter {
                    level: self.rom.signed(entry),
                    max_level: self.rom.signed(entry + 1),
                    pokemon: self.pokemon_or_fallback(self.rom.word(entry + 2)),
                    ..Encounter::default()
                }
            })
            .collect();
        EncounterSet {
            rate: self.rom.signed(visit.area_offset),
            encounters,
            display_name: Some(format!("{}{}", self.map_name(bank, map), visit.area.suffix)),
            ..EncounterSet::default()
        }
    }

    fn battle_trappers(&self) -> Vec<PokemonId> {
        self.pokemon_list
            .iter()
            .flatten()
            .copied()
            .filter(|&id| {
                let pk = self.core.data.pk(id);
                global::BATTLE_TRAPPING_ABILITIES.contains(&pk.ability1)
                    || global::BATTLE_TRAPPING_ABILITIES.contains(&pk.ability2)
            })
            .collect()
    }

    pub(crate) fn read_encounters(&self) -> Vec<EncounterSet> {
        let mut areas: Vec<EncounterSet> = self
            .wild_header_visits()
            .iter()
            .map(|visit| self.read_wild_area(visit))
            .collect();
        if self.entry.has_array("BattleTrappersBanned") {
            let trappers = self.battle_trappers();
            for index in self.array("BattleTrappersBanned") {
                let Some(area) = usize::try_from(index)
                    .ok()
                    .and_then(|index| areas.get_mut(index))
                else {
                    continue;
                };
                for &trapper in &trappers {
                    area.ban(trapper);
                }
            }
        }
        areas
    }

    fn write_wild_area(&mut self, visit: &WildHeaderVisit, set: &EncounterSet) {
        let data = self.rom.pointer(visit.area_offset + 4);
        for (slot, encounter) in set
            .encounters
            .iter()
            .take(visit.area.slots as usize)
            .enumerate()
        {
            let entry = data + slot as i32 * 4;
            let species = self.internal_of(encounter.pokemon);
            self.rom.set_byte(entry, encounter.level as u8);
            self.rom.set_byte(entry + 1, encounter.max_level as u8);
            self.rom.set_word(entry + 2, species);
        }
    }

    pub(crate) fn write_encounters(&mut self, encounters: &[EncounterSet]) -> Result<()> {
        self.attempt_obedience_evolution_patches();
        let visits = self.wild_header_visits();
        if visits.len() > encounters.len() {
            return Err(RandomizationError::IndexOutOfBounds("encounterAreas"));
        }
        for (visit, set) in visits.iter().zip(encounters) {
            self.write_wild_area(visit, set);
        }
        Ok(())
    }
}
