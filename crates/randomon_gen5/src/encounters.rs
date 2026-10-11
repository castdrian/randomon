use std::collections::HashMap;

use randomon_core::{RandomizationError, Result};
use randomon_data::constants::gen5;
use randomon_data::{Encounter, EncounterSet, PokemonId};
use randomon_formats::nds::NarcArchive;
use randomon_nds::{read_byte, read_word, write_word};

use crate::handler::Gen5RomHandler;

const SEASONS: i32 = 4;
const MAP_HEADER_LENGTH: usize = 48;
const UNKNOWN_MAP_NAME: &str = "? Unknown ?";

fn season_length() -> i32 {
    gen5::PER_SEASON_ENCOUNTER_DATA_LENGTH
}

impl Gen5RomHandler {
    fn load_wild_map_names(&self) -> Result<HashMap<i32, String>> {
        let narc = self.read_narc("MapTableFile")?;
        let header_data = narc
            .files
            .first()
            .ok_or(RandomizationError::IndexOutOfBounds("mapTable.files"))?;
        let header_count = header_data.len() / MAP_HEADER_LENGTH;
        let all_map_names = self.get_strings(false, self.entry.int("MapNamesTextOffset"))?;
        let mut names = HashMap::new();
        for map in 0..header_count {
            let base = (map * MAP_HEADER_LENGTH) as i32;
            let name_index = read_byte(header_data, base + 26);
            let name = all_map_names
                .get(name_index as usize)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("allMapNames"))?;
            if self.is_bw2() {
                let wild_set = read_byte(header_data, base + 20);
                if wild_set != 255 {
                    names.insert(wild_set, name);
                }
            } else {
                let wild_set = read_word(header_data, base + 20);
                if wild_set != 65535 {
                    names.insert(wild_set, name);
                }
            }
        }
        Ok(names)
    }

    fn ensure_wild_map_names(&self) -> Result<()> {
        if self.wild_map_names.borrow().is_none() {
            let names = self.load_wild_map_names()?;
            *self.wild_map_names.borrow_mut() = Some(names);
        }
        Ok(())
    }

    fn map_name_for(&self, index: i32) -> String {
        let mut names = self.wild_map_names.borrow_mut();
        let names = names.get_or_insert_with(HashMap::new);
        names
            .entry(index)
            .or_insert_with(|| UNKNOWN_MAP_NAME.to_owned())
            .clone()
    }

    fn read_encounters(&self, data: &[u8], offset: i32, count: i32) -> Vec<Encounter> {
        (0..count)
            .map(|slot| {
                let word = read_word(data, offset + slot * 4);
                let species = word & 0x7FF;
                let forme = word >> 11;
                let base = self.pk(species);
                let cosmetic_forms = self.core.data.pk(base).cosmetic_forms;
                let pokemon = if forme <= cosmetic_forms || forme == 30 || forme == 31 {
                    base
                } else {
                    match gen5::absolute_poke_num_by_base_forme(species, forme) {
                        0 => base,
                        with_forme => self.pk(with_forme),
                    }
                };
                Encounter {
                    pokemon,
                    forme_number: forme,
                    level: read_byte(data, offset + 2 + slot * 4),
                    max_level: read_byte(data, offset + 3 + slot * 4),
                    ..Encounter::default()
                }
            })
            .collect()
    }

    fn process_encounter_entry(
        &self,
        encounters: &mut Vec<EncounterSet>,
        entry: &[u8],
        start: i32,
        index: i32,
    ) {
        let map_name = self.map_name_for(index);
        let mut offset = 8;
        for (kind, &amount) in gen5::ENCOUNTERS_OF_EACH_TYPE.iter().enumerate() {
            let rate = read_byte(entry, start + kind as i32);
            if rate != 0 {
                encounters.push(EncounterSet {
                    rate,
                    encounters: self.read_encounters(entry, start + offset, amount),
                    offset: index,
                    display_name: Some(format!("{map_name} {}", gen5::ENCOUNTER_TYPE_NAMES[kind])),
                    ..EncounterSet::default()
                });
            }
            offset += amount * 4;
        }
    }

    pub(crate) fn encounters(&self, use_time_of_day: bool) -> Result<Vec<EncounterSet>> {
        self.ensure_wild_map_names()?;
        let narc = self.read_narc("WildPokemon")?;
        let mut encounters = Vec::new();
        for (index, entry) in narc.files.iter().enumerate() {
            let index = index as i32;
            if entry.len() as i32 > season_length() && use_time_of_day {
                for season in 0..SEASONS {
                    self.process_encounter_entry(
                        &mut encounters,
                        entry,
                        season * season_length(),
                        index,
                    );
                }
            } else {
                self.process_encounter_entry(&mut encounters, entry, 0, index);
            }
        }
        Ok(encounters)
    }

    fn write_encounter_entry<'a>(
        &self,
        sets: &mut impl Iterator<Item = &'a EncounterSet>,
        entry: &mut [u8],
        start: i32,
    ) -> Result<()> {
        let mut offset = 8;
        for (kind, &amount) in gen5::ENCOUNTERS_OF_EACH_TYPE.iter().enumerate() {
            let rate = read_byte(entry, start + kind as i32);
            if rate != 0 {
                let area = sets
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("encounters iterator"))?;
                for slot in 0..amount {
                    let encounter = area
                        .encounters
                        .get(slot as usize)
                        .ok_or(RandomizationError::IndexOutOfBounds("area.encounters"))?;
                    let base_number = self.core.data.base_number(encounter.pokemon);
                    let position = start + offset + slot * 4;
                    write_word(
                        entry,
                        position,
                        (encounter.forme_number << 11) + base_number,
                    );
                    entry[(position + 2) as usize] = encounter.level as u8;
                    entry[(position + 3) as usize] = encounter.max_level as u8;
                }
            }
            offset += amount * 4;
        }
        Ok(())
    }

    pub(crate) fn set_encounters_impl(
        &mut self,
        use_time_of_day: bool,
        encounters: &[EncounterSet],
    ) -> Result<()> {
        let mut narc = self.read_narc("WildPokemon")?;
        let mut sets = encounters.iter();
        let length = season_length() as usize;
        for entry in &mut narc.files {
            self.write_encounter_entry(&mut sets, entry, 0)?;
            if entry.len() > length {
                if use_time_of_day {
                    for season in 1..SEASONS {
                        self.write_encounter_entry(&mut sets, entry, season * season_length())?;
                    }
                } else {
                    for season in 1..SEASONS as usize {
                        entry.copy_within(0..length, season * length);
                    }
                }
            }
        }
        self.write_narc("WildPokemon", &narc)?;
        self.update_pokedex_area_data(&narc)
    }

    fn wrong_version_area(&self, area_index: i32, file_number: i32) -> bool {
        if !self.is_bw2() {
            return false;
        }
        let is_white2 = self.entry.rom_code.as_bytes().get(2) == Some(&b'D');
        let is_black2_code = self.entry.rom_code.as_bytes().get(2) == Some(&b'E');
        if area_index == gen5::BW2_ROUTE4_AREA_INDEX
            && ((file_number == gen5::B2_ROUTE4_ENCOUNTER_FILE && is_white2)
                || (file_number == gen5::W2_ROUTE4_ENCOUNTER_FILE && is_black2_code))
        {
            return true;
        }
        if area_index == gen5::BW2_VICTORY_ROAD_AREA_INDEX {
            let exclusive = if is_white2 {
                [gen5::B2_VR_EXCLUSIVE_ROOM1, gen5::B2_VR_EXCLUSIVE_ROOM2]
            } else {
                [gen5::W2_VR_EXCLUSIVE_ROOM1, gen5::W2_VR_EXCLUSIVE_ROOM2]
            };
            if exclusive.contains(&file_number) {
                return true;
            }
        }
        if area_index == gen5::BW2_REVERSAL_MOUNTAIN_AREA_INDEX {
            let range = if is_white2 {
                gen5::B2_REVERSAL_MOUNTAIN_START..=gen5::B2_REVERSAL_MOUNTAIN_END
            } else {
                gen5::W2_REVERSAL_MOUNTAIN_START..=gen5::W2_REVERSAL_MOUNTAIN_END
            };
            if range.contains(&file_number) {
                return true;
            }
        }
        false
    }

    fn update_area_data_from_entry(
        &self,
        entry: &[u8],
        start: i32,
        area_data: &mut [Vec<u8>],
        season: i32,
        file_number: i32,
    ) -> Result<()> {
        let area_count = gen5::encounter_area_count(self.rom_type());
        let file_to_area = gen5::wild_file_to_area_map(self.rom_type());
        let mut offset = 8;
        for (kind, &amount) in gen5::ENCOUNTERS_OF_EACH_TYPE.iter().enumerate() {
            let rate = read_byte(entry, start + kind as i32);
            if rate != 0 {
                for slot in 0..amount {
                    let position = start + offset + slot * 4;
                    let species =
                        read_byte(entry, position) + ((read_byte(entry, position + 1) & 0x03) << 8);
                    let pokemon: PokemonId = self.pk(species);
                    if !self.core.data.exists(pokemon) {
                        return Err(RandomizationError::NullPointer("pokes[species]"));
                    }
                    let base_number = self.core.data.base_number(pokemon);
                    let area_index = *file_to_area
                        .get(file_number as usize)
                        .ok_or(RandomizationError::IndexOutOfBounds("wildFileToAreaMap"))?;
                    if area_index == -1 || self.wrong_version_area(area_index, file_number) {
                        continue;
                    }
                    let poke_file = area_data
                        .get_mut((base_number - 1) as usize)
                        .ok_or(RandomizationError::IndexOutOfBounds("areaData"))?;
                    let index = (season * (area_count + 1) + 2 + area_index) as usize;
                    poke_file[index] |= (1 << kind) as u8;
                }
            }
            offset += amount * 4;
        }
        Ok(())
    }

    fn update_pokedex_area_data(&self, encounter_narc: &NarcArchive) -> Result<()> {
        let mut area_narc = self.read_narc("PokedexAreaData")?;
        let entry_length = gen5::area_data_entry_length(self.rom_type()) as usize;
        let area_count = gen5::encounter_area_count(self.rom_type());
        let mut new_files = vec![vec![0u8; entry_length]; gen5::POKEMON_COUNT as usize];
        for (file_number, entry) in encounter_narc.files.iter().enumerate() {
            for season in 0..SEASONS {
                let start = if entry.len() as i32 > season_length() {
                    season * season_length()
                } else {
                    0
                };
                self.update_area_data_from_entry(
                    entry,
                    start,
                    &mut new_files,
                    season,
                    file_number as i32,
                )?;
            }
        }
        let stride = (area_count + 1) as usize;
        for (index, file) in new_files.into_iter().enumerate() {
            let mut file = file;
            for season in 0..SEASONS as usize {
                let unobtainable =
                    (0..area_count as usize).all(|area| file[season * stride + area + 2] == 0);
                if unobtainable {
                    file[season * stride + 1] = 1;
                }
            }
            let seasonal_dependent = (0..area_count as usize).any(|area| {
                let spring = file[area + 2];
                [1, 2, 3]
                    .iter()
                    .any(|&season| file[season * stride + area + 2] != spring)
            });
            if !seasonal_dependent {
                file[0] = 1;
            }
            let slot = area_narc
                .files
                .get_mut(index)
                .ok_or(RandomizationError::IndexOutOfBounds("areaNARC.files"))?;
            *slot = file;
        }
        self.write_narc("PokedexAreaData", &area_narc)
    }
}
