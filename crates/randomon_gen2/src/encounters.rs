use randomon_core::{RandomizationError, Result};
use randomon_data::constants::species;
use randomon_data::{Encounter, EncounterSet};

use crate::constants::{
    FISHING_GROUP_COUNT, FISHING_GROUP_ENTRY_LENGTH, LAND_ENCOUNTER_SLOTS, POKES_PER_FISHING_GROUP,
    POKES_PER_TS_FISHING_GROUP, SEA_ENCOUNTER_SLOTS, TIME_SPECIFIC_FISHING_GROUP_COUNT,
};
use crate::handler::{Gen2RomHandler, pokemon};

const TIMES_OF_DAY: [&str; 3] = ["Morning", "Day", "Night"];
const LAND_AREA_SIZE: i32 = 5 + 6 * LAND_ENCOUNTER_SLOTS;
const SEA_AREA_SIZE: i32 = 3 + SEA_ENCOUNTER_SLOTS * 2;
const ENCOUNTER_REGIONS: usize = 3;

fn named_set(name: String) -> EncounterSet {
    EncounterSet {
        display_name: Some(name),
        ..EncounterSet::default()
    }
}

fn land_slot_offset(area: i32, time: i32, slot: i32) -> i32 {
    area + 5 + time * LAND_ENCOUNTER_SLOTS * 2 + slot * 2
}

struct SetCursor<'a> {
    sets: std::slice::Iter<'a, EncounterSet>,
}

impl<'a> SetCursor<'a> {
    fn next_set(&mut self) -> Result<&'a EncounterSet> {
        self.sets
            .next()
            .ok_or(RandomizationError::IndexOutOfBounds("encounter sets"))
    }
}

fn next_encounter<'a>(encounters: &mut std::slice::Iter<'a, Encounter>) -> Result<&'a Encounter> {
    encounters
        .next()
        .ok_or(RandomizationError::IndexOutOfBounds("encounters"))
}

impl Gen2RomHandler {
    fn encounter_at(&self, offset: i32) -> Encounter {
        Encounter {
            level: self.int(offset),
            pokemon: self.pokemon_at(offset + 1),
            ..Encounter::default()
        }
    }

    fn area_map_name(&self, offset: i32) -> String {
        self.maps
            .name(self.byte(offset) as usize, self.byte(offset + 1) as usize)
    }

    fn land_set(&self, area: i32, time: i32, name: String, rate_offset: i32) -> EncounterSet {
        let mut set = named_set(name);
        set.rate = self.int(rate_offset);
        set.encounters = (0..LAND_ENCOUNTER_SLOTS)
            .map(|slot| self.encounter_at(land_slot_offset(area, time, slot)))
            .collect();
        set
    }

    fn read_land_encounters(
        &self,
        mut offset: i32,
        areas: &mut Vec<EncounterSet>,
        use_time_of_day: bool,
    ) -> i32 {
        while self.byte(offset) != 0xFF {
            let map_name = self.area_map_name(offset);
            if use_time_of_day {
                for (time, label) in TIMES_OF_DAY.iter().enumerate() {
                    let time = time as i32;
                    areas.push(self.land_set(
                        offset,
                        time,
                        format!("{map_name} Grass/Cave ({label})"),
                        offset + 2 + time,
                    ));
                }
            } else {
                areas.push(self.land_set(offset, 1, format!("{map_name} Grass/Cave"), offset + 3));
            }
            offset += LAND_AREA_SIZE;
        }
        offset + 1
    }

    fn read_sea_encounters(&self, mut offset: i32, areas: &mut Vec<EncounterSet>) -> i32 {
        while self.byte(offset) != 0xFF {
            let mut set = named_set(format!("{} Surfing", self.area_map_name(offset)));
            set.rate = self.int(offset + 2);
            set.encounters = (0..SEA_ENCOUNTER_SLOTS)
                .map(|slot| self.encounter_at(offset + 3 + slot * 2))
                .collect();
            areas.push(set);
            offset += SEA_AREA_SIZE;
        }
        offset + 1
    }

    fn read_fishing_encounters(&self, areas: &mut Vec<EncounterSet>, use_time_of_day: bool) {
        let root = self.value("FishingWildsOffset");
        let mut offset = root;
        for group in 0..FISHING_GROUP_COUNT {
            let mut set = named_set(format!("Fishing Group {}", group + 1));
            for _ in 0..POKES_PER_FISHING_GROUP {
                offset += 1;
                let number = self.int(offset);
                let level = self.int(offset + 1);
                offset += 2;
                if number != 0 {
                    set.encounters.push(Encounter {
                        level,
                        pokemon: pokemon(number),
                        ..Encounter::default()
                    });
                } else if !use_time_of_day {
                    let special = root
                        + FISHING_GROUP_ENTRY_LENGTH
                            * POKES_PER_FISHING_GROUP
                            * FISHING_GROUP_COUNT
                        + level * 4
                        + 2;
                    set.encounters.push(self.special_fishing_encounter(special));
                }
            }
            areas.push(set);
        }
        if use_time_of_day {
            for group in 0..TIME_SPECIFIC_FISHING_GROUP_COUNT {
                let mut set = named_set(format!("Time-Specific Fishing {}", group + 1));
                for _ in 0..POKES_PER_TS_FISHING_GROUP {
                    set.encounters.push(self.special_fishing_encounter(offset));
                    offset += 2;
                }
                areas.push(set);
            }
        }
    }

    fn special_fishing_encounter(&self, offset: i32) -> Encounter {
        Encounter {
            pokemon: self.pokemon_at(offset),
            level: self.int(offset + 1),
            ..Encounter::default()
        }
    }

    fn read_headbutt_encounters(&self, areas: &mut Vec<EncounterSet>) {
        let mut offset = self.value("HeadbuttWildsOffset");
        for tree in 0..self.value("HeadbuttTableSize") {
            let mut set = named_set(format!("Headbutt Trees Set {}", tree + 1));
            while self.byte(offset) != 0xFF {
                set.encounters
                    .push(self.special_fishing_encounter(offset + 1));
                offset += 3;
            }
            offset += 1;
            areas.push(set);
        }
    }

    fn read_bug_catching_contest(&self, areas: &mut Vec<EncounterSet>) {
        let mut offset = self.value("BCCWildsOffset");
        let mut set = named_set("Bug Catching Contest".to_owned());
        while self.byte(offset) != 0xFF {
            set.encounters.push(Encounter {
                pokemon: self.pokemon_at(offset + 1),
                level: self.int(offset + 2),
                max_level: self.int(offset + 3),
                ..Encounter::default()
            });
            offset += 4;
        }
        set.banned_pokemon.push(pokemon(species::UNOWN));
        areas.push(set);
    }

    pub(crate) fn read_encounters(&self, use_time_of_day: bool) -> Vec<EncounterSet> {
        let mut areas = Vec::new();
        let mut offset = self.value("WildPokemonOffset");
        for _ in 0..ENCOUNTER_REGIONS {
            offset = self.read_land_encounters(offset, &mut areas, use_time_of_day);
            offset = self.read_sea_encounters(offset, &mut areas);
        }
        self.read_fishing_encounters(&mut areas, use_time_of_day);
        self.read_headbutt_encounters(&mut areas);
        self.read_bug_catching_contest(&mut areas);
        areas
    }

    fn write_land_time(&mut self, area: i32, time: i32, set: &EncounterSet) -> Result<()> {
        let mut encounters = set.encounters.iter();
        for slot in 0..LAND_ENCOUNTER_SLOTS {
            let encounter = next_encounter(&mut encounters)?;
            let target = land_slot_offset(area, time, slot);
            self.set_byte(target, encounter.level as u8);
            let number = self.number_of(encounter.pokemon);
            self.set_byte(target + 1, number);
        }
        Ok(())
    }

    fn write_land_encounters(
        &mut self,
        mut offset: i32,
        cursor: &mut SetCursor<'_>,
        use_time_of_day: bool,
    ) -> Result<i32> {
        while self.byte(offset) != 0xFF {
            if use_time_of_day {
                for time in 0..3 {
                    let set = cursor.next_set()?;
                    self.write_land_time(offset, time, set)?;
                }
            } else {
                let set = cursor.next_set()?;
                for time in 0..3 {
                    self.write_land_time(offset, time, set)?;
                }
            }
            offset += LAND_AREA_SIZE;
        }
        Ok(offset + 1)
    }

    fn write_sea_encounters(&mut self, mut offset: i32, cursor: &mut SetCursor<'_>) -> Result<i32> {
        while self.byte(offset) != 0xFF {
            let set = cursor.next_set()?;
            let mut encounters = set.encounters.iter();
            for slot in 0..SEA_ENCOUNTER_SLOTS {
                let encounter = next_encounter(&mut encounters)?;
                self.write_encounter(offset + 3 + slot * 2, encounter, true);
            }
            offset += SEA_AREA_SIZE;
        }
        Ok(offset + 1)
    }

    fn write_encounter(&mut self, offset: i32, encounter: &Encounter, level_first: bool) {
        let number = self.number_of(encounter.pokemon);
        let level = encounter.level as u8;
        if level_first {
            self.set_byte(offset, level);
            self.set_byte(offset + 1, number);
        } else {
            self.set_byte(offset, number);
            self.set_byte(offset + 1, level);
        }
    }

    fn write_fishing_encounters(
        &mut self,
        cursor: &mut SetCursor<'_>,
        use_time_of_day: bool,
    ) -> Result<()> {
        let mut offset = self.value("FishingWildsOffset");
        for _ in 0..FISHING_GROUP_COUNT {
            let set = cursor.next_set()?;
            let mut encounters = set.encounters.iter();
            for _ in 0..POKES_PER_FISHING_GROUP {
                offset += 1;
                if self.byte(offset) != 0 || !use_time_of_day {
                    let encounter = next_encounter(&mut encounters)?;
                    self.write_encounter(offset, encounter, false);
                }
                offset += 2;
            }
        }
        if use_time_of_day {
            for _ in 0..TIME_SPECIFIC_FISHING_GROUP_COUNT {
                let set = cursor.next_set()?;
                let mut encounters = set.encounters.iter();
                for _ in 0..POKES_PER_TS_FISHING_GROUP {
                    let encounter = next_encounter(&mut encounters)?;
                    self.write_encounter(offset, encounter, false);
                    offset += 2;
                }
            }
        }
        Ok(())
    }

    fn write_headbutt_encounters(&mut self, cursor: &mut SetCursor<'_>) -> Result<()> {
        let mut offset = self.value("HeadbuttWildsOffset");
        for _ in 0..self.value("HeadbuttTableSize") {
            let set = cursor.next_set()?;
            let mut encounters = set.encounters.iter();
            while self.byte(offset) != 0xFF {
                let encounter = next_encounter(&mut encounters)?;
                self.write_encounter(offset + 1, encounter, false);
                offset += 3;
            }
            offset += 1;
        }
        Ok(())
    }

    fn write_bug_catching_contest(&mut self, cursor: &mut SetCursor<'_>) -> Result<()> {
        let mut offset = self.value("BCCWildsOffset");
        let set = cursor.next_set()?;
        let mut encounters = set.encounters.iter();
        while self.byte(offset) != 0xFF {
            let encounter = next_encounter(&mut encounters)?;
            self.write_encounter(offset + 1, encounter, false);
            self.set_byte(offset + 3, encounter.max_level as u8);
            offset += 4;
        }
        Ok(())
    }

    pub(crate) fn write_encounters(
        &mut self,
        use_time_of_day: bool,
        encounters: &[EncounterSet],
    ) -> Result<()> {
        self.ensure_fleeing_patched();
        let mut cursor = SetCursor {
            sets: encounters.iter(),
        };
        let mut offset = self.value("WildPokemonOffset");
        for _ in 0..ENCOUNTER_REGIONS {
            offset = self.write_land_encounters(offset, &mut cursor, use_time_of_day)?;
            offset = self.write_sea_encounters(offset, &mut cursor)?;
        }
        self.write_fishing_encounters(&mut cursor, use_time_of_day)?;
        self.write_headbutt_encounters(&mut cursor)?;
        self.write_bug_catching_contest(&mut cursor)
    }
}
