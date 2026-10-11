use std::collections::HashMap;

use randomon_core::{RandomizationError, Result};
use randomon_ctr::bytes::{read_long, read_word, write_long, write_word};
use randomon_data::{Encounter, EncounterSet};
use randomon_formats::ctr::{Decompression, GarcArchive};

use crate::constants;
use crate::handler::{Gen6RomHandler, io};
use crate::pokemon::{garc_file, garc_file_mut};

const XY_ENCOUNTER_DATA_LENGTH: usize = 0x178;
const ORAS_ENCOUNTER_DATA_LENGTH: usize = 0xF6;
const ORAS_DEC_STORAGE_LENGTH: usize = 0xF4;
const UNKNOWN_MAP_NAME: &str = "? Unknown ?";

struct Slot {
    offset: usize,
    amount: usize,
    suffix: &'static str,
    area_type: i32,
}

const fn slot(offset: usize, amount: usize, suffix: &'static str, area_type: i32) -> Slot {
    Slot {
        offset,
        amount,
        suffix,
        area_type,
    }
}

const XY_SLOTS: [Slot; 13] = [
    slot(0, 12, "Grass/Cave", 0x1),
    slot(48, 12, "Yellow Flowers", 0x2),
    slot(96, 12, "Purple Flowers", 0x4),
    slot(144, 12, "Red Flowers", 0x8),
    slot(192, 12, "Rough Terrain/Tall Grass", 0x10),
    slot(240, 5, "Surf", 0x20),
    slot(260, 5, "Rock Smash", 0x40),
    slot(280, 3, "Old Rod", 0x80),
    slot(292, 3, "Good Rod", 0x100),
    slot(304, 3, "Super Rod", 0x200),
    slot(316, 5, "Common Horde", 0x400),
    slot(336, 5, "Uncommon Horde", 0x400),
    slot(356, 5, "Rare Horde", 0x400),
];

const ORAS_SLOTS: [Slot; 11] = [
    slot(0, 12, "Grass/Cave", 0x1),
    slot(48, 12, "Long Grass", 0x2),
    slot(96, 3, "DexNav Foreign Encounter", 0),
    slot(108, 5, "Surf", 0x10),
    slot(128, 5, "Rock Smash", 0x20),
    slot(148, 3, "Old Rod", 0x40),
    slot(160, 3, "Good Rod", 0x80),
    slot(172, 3, "Super Rod", 0x100),
    slot(184, 5, "Common Horde", 0x200),
    slot(204, 5, "Uncommon Horde", 0x200),
    slot(224, 5, "Rare Horde", 0x200),
];

fn signed(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset] as i8)
}

fn copy_range(data: &[u8], offset: i64, length: usize) -> Result<Vec<u8>> {
    let start =
        usize::try_from(offset).map_err(|_| RandomizationError::IndexOutOfBounds("arraycopy"))?;
    data.get(start..start + length)
        .map(<[u8]>::to_vec)
        .ok_or(RandomizationError::IndexOutOfBounds("arraycopy"))
}

fn xy_encounter_offset(file: &[u8]) -> Option<i64> {
    let offset = read_long(file, 0x10) as i64 + 0x10;
    let length = file.len() as i64 - offset;
    (length >= XY_ENCOUNTER_DATA_LENGTH as i64).then_some(offset)
}

fn oras_encounter_offset(file: &[u8]) -> Option<i64> {
    let offset = read_long(file, 0x10) as i64 + 0xE;
    let end = read_long(file, 0x14) as i64;
    (end - offset >= ORAS_ENCOUNTER_DATA_LENGTH as i64).then_some(offset)
}

impl Gen6RomHandler {
    fn read_encounter(&self, data: &[u8], offset: usize, amount: usize) -> EncounterSet {
        let mut set = EncounterSet {
            rate: 1,
            ..EncounterSet::default()
        };
        for index in 0..amount {
            let value = read_word(data, offset + index * 4);
            let species = value & 0x7FF;
            let forme = value >> 11;
            if species == 0 {
                continue;
            }
            set.encounters.push(Encounter {
                pokemon: self.pokemon_with_forme(species, forme),
                forme_number: forme,
                level: signed(data, offset + 2 + index * 4),
                max_level: signed(data, offset + 3 + index * 4),
                ..Encounter::default()
            });
        }
        set
    }

    fn read_field_encounter(&self, data: &[u8], offset: usize) -> EncounterSet {
        let mut set = EncounterSet::default();
        for index in 0..7 {
            let species = read_word(data, offset + 4 + index * 8);
            let level = signed(data, offset + 8 + index * 8);
            if species != 0 {
                set.encounters.push(Encounter {
                    pokemon: self.pk(species),
                    forme_number: 0,
                    level,
                    max_level: level,
                    ..Encounter::default()
                });
            }
        }
        set
    }

    fn write_encounter(&self, data: &mut [u8], offset: usize, encounters: &[Encounter]) {
        for (index, encounter) in encounters.iter().enumerate() {
            let base = self.core.data.base_number(encounter.pokemon);
            let value = (encounter.forme_number << 11).wrapping_add(base);
            write_word(data, offset + index * 4, value);
            data[offset + 2 + index * 4] = encounter.level as u8;
            data[offset + 3 + index * 4] = encounter.max_level as u8;
        }
    }

    fn write_field_encounter(&self, data: &mut [u8], offset: usize, encounters: &[Encounter]) {
        for (index, encounter) in encounters.iter().enumerate() {
            let base = self.core.data.base_number(encounter.pokemon);
            write_word(data, offset + 4 + index * 8, base);
            data[offset + 8 + index * 8] = encounter.level as u8;
        }
    }

    fn map_names(&self) -> Result<HashMap<usize, String>> {
        if let Some(names) = self.wild_map_names.borrow().as_ref() {
            return Ok(names.clone());
        }
        let garc = self.read_garc(self.file("WildPokemon"), Decompression::All)?;
        let zone_data_offset = self.entry.int("MapTableFileOffset") as usize;
        let zone_data = garc_file(&garc, zone_data_offset)?;
        let all_names = self.strings(false, self.entry.int("MapNamesTextOffset"))?;
        let mut names = HashMap::new();
        for map in 0..zone_data_offset {
            let index = map * 56 + 0x1C;
            let name_index =
                usize::from(zone_data[index]) + 0x100 * usize::from(zone_data[index + 1] & 1);
            let name = all_names
                .get(name_index)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("map names"))?;
            names.insert(map, name);
        }
        *self.wild_map_names.borrow_mut() = Some(names.clone());
        Ok(names)
    }

    fn map_name(&self, names: &mut HashMap<usize, String>, map: usize) -> String {
        names
            .entry(map)
            .or_insert_with(|| UNKNOWN_MAP_NAME.to_owned())
            .clone()
    }

    pub(crate) fn read_encounters(&self) -> Result<Vec<EncounterSet>> {
        let mut names = self.map_names()?;
        let garc = self.read_garc(self.file("WildPokemon"), Decompression::All)?;
        let result = if self.is_oras() {
            self.read_encounters_oras(&garc, &mut names)
        } else {
            self.read_encounters_xy(&garc, &mut names)
        };
        *self.wild_map_names.borrow_mut() = Some(names);
        result
    }

    fn push_slot_sets(
        &self,
        encounters: &mut Vec<EncounterSet>,
        data: &[u8],
        slots: &[Slot],
        map_name: &str,
        offset: Option<i32>,
    ) {
        for slot in slots {
            let mut set = self.read_encounter(data, slot.offset, slot.amount);
            if set.encounters.is_empty() {
                continue;
            }
            set.display_name = Some(format!("{map_name} {}", slot.suffix));
            if let Some(offset) = offset {
                set.offset = offset;
            }
            encounters.push(set);
        }
    }

    fn read_encounters_xy(
        &self,
        garc: &GarcArchive,
        names: &mut HashMap<usize, String>,
    ) -> Result<Vec<EncounterSet>> {
        let mut encounters = Vec::new();
        for index in 0..garc.files.len().saturating_sub(1) {
            let file = garc_file(garc, index)?;
            let map_name = self.map_name(names, index);
            let Some(offset) = xy_encounter_offset(file) else {
                continue;
            };
            let data = copy_range(file, offset, XY_ENCOUNTER_DATA_LENGTH)?;
            self.push_slot_sets(&mut encounters, &data, &XY_SLOTS, &map_name, None);
        }
        let field_cro = self.read_file(self.file("Field"))?;
        self.push_field_sets(
            &mut encounters,
            &field_cro,
            constants::FALLING_ENCOUNTER_OFFSET,
            constants::FALLING_ENCOUNTER_COUNT,
            constants::falling_encounter_name,
        );
        self.push_field_sets(
            &mut encounters,
            &field_cro,
            constants::RUSTLING_BUSH_ENCOUNTER_OFFSET,
            constants::RUSTLING_BUSH_ENCOUNTER_COUNT,
            constants::rustling_bush_encounter_name,
        );
        Ok(encounters)
    }

    fn push_field_sets(
        &self,
        encounters: &mut Vec<EncounterSet>,
        field_cro: &[u8],
        start: usize,
        count: usize,
        name_at: fn(usize) -> Option<&'static str>,
    ) {
        let mut current_name = name_at(0).unwrap_or("");
        let mut start_of_name = 0;
        for index in 0..count {
            let offset = start + index * constants::FIELD_ENCOUNTER_SIZE;
            let mut set = self.read_field_encounter(field_cro, offset);
            if let Some(name) = name_at(index) {
                current_name = name;
                start_of_name = index;
            }
            set.display_name = Some(format!("{current_name} #{}", index - start_of_name + 1));
            encounters.push(set);
        }
    }

    fn read_encounters_oras(
        &self,
        garc: &GarcArchive,
        names: &mut HashMap<usize, String>,
    ) -> Result<Vec<EncounterSet>> {
        let mut encounters = Vec::new();
        for index in 0..garc.files.len().saturating_sub(2) {
            let file = garc_file(garc, index)?;
            let map_name = self.map_name(names, index);
            let Some(offset) = oras_encounter_offset(file) else {
                continue;
            };
            let data = copy_range(file, offset, ORAS_ENCOUNTER_DATA_LENGTH)?;
            self.push_slot_sets(
                &mut encounters,
                &data,
                &ORAS_SLOTS,
                &map_name,
                Some(index as i32),
            );
        }
        Ok(encounters)
    }

    fn write_slot_sets<'a>(
        &self,
        data: &mut [u8],
        slots: &[Slot],
        encounters: &mut impl Iterator<Item = &'a EncounterSet>,
    ) -> Result<()> {
        for slot in slots {
            if self
                .read_encounter(data, slot.offset, slot.amount)
                .encounters
                .is_empty()
            {
                continue;
            }
            let set = encounters.next().ok_or(RandomizationError::randomization(
                "java.util.NoSuchElementException",
            ))?;
            self.write_encounter(data, slot.offset, &set.encounters);
        }
        Ok(())
    }

    pub(crate) fn write_encounters(&mut self, sets: &[EncounterSet]) -> Result<()> {
        if self.is_oras() {
            self.write_encounters_oras(sets)
        } else {
            self.write_encounters_xy(sets)
        }
    }

    fn write_encounters_xy(&mut self, sets: &[EncounterSet]) -> Result<()> {
        let path = self.file("WildPokemon");
        let mut garc = self.read_garc(path, Decompression::All)?;
        let mut encounters = sets.iter();
        for index in 0..garc.files.len().saturating_sub(1) {
            let file = garc_file_mut(&mut garc, index)?;
            let Some(offset) = xy_encounter_offset(file) else {
                continue;
            };
            let mut data = copy_range(file, offset, XY_ENCOUNTER_DATA_LENGTH)?;
            self.write_slot_sets(&mut data, &XY_SLOTS, &mut encounters)?;
            let start = offset as usize;
            file[start..start + XY_ENCOUNTER_DATA_LENGTH].copy_from_slice(&data);
        }
        self.write_garc(path, &garc)?;
        let field_path = self.file("Field");
        let mut field_cro = self.read_file(field_path)?;
        for (start, count) in [
            (
                constants::FALLING_ENCOUNTER_OFFSET,
                constants::FALLING_ENCOUNTER_COUNT,
            ),
            (
                constants::RUSTLING_BUSH_ENCOUNTER_OFFSET,
                constants::RUSTLING_BUSH_ENCOUNTER_COUNT,
            ),
        ] {
            for index in 0..count {
                let offset = start + index * constants::FIELD_ENCOUNTER_SIZE;
                let set = encounters.next().ok_or(RandomizationError::randomization(
                    "java.util.NoSuchElementException",
                ))?;
                self.write_field_encounter(&mut field_cro, offset, &set.encounters);
            }
        }
        self.write_file(field_path, &field_cro)?;
        self.update_pokedex_area_data_xy(&garc, &field_cro)
    }

    fn write_encounters_oras(&mut self, sets: &[EncounterSet]) -> Result<()> {
        let path = self.file("WildPokemon");
        let mut garc = self.read_garc(path, Decompression::All)?;
        let mut encounters = sets.iter();
        let storage_index = garc.files.len() - 1;
        let mut dec_storage = garc_file(&garc, storage_index)?.to_vec();
        for index in 0..garc.files.len().saturating_sub(2) {
            let file = garc_file_mut(&mut garc, index)?;
            let Some(offset) = oras_encounter_offset(file) else {
                continue;
            };
            let mut data = copy_range(file, offset, ORAS_ENCOUNTER_DATA_LENGTH)?;
            self.write_slot_sets(&mut data, &ORAS_SLOTS, &mut encounters)?;
            let start = offset as usize;
            file[start..start + ORAS_ENCOUNTER_DATA_LENGTH].copy_from_slice(&data);
            let storage_offset = read_long(&dec_storage, (index + 1) * 4) as i64 + 0xE;
            let storage_start = usize::try_from(storage_offset)
                .map_err(|_| RandomizationError::IndexOutOfBounds("decStorage"))?;
            dec_storage
                .get_mut(storage_start..storage_start + ORAS_DEC_STORAGE_LENGTH)
                .ok_or(RandomizationError::IndexOutOfBounds("decStorage"))?
                .copy_from_slice(&data[..ORAS_DEC_STORAGE_LENGTH]);
        }
        garc.set_file(storage_index, dec_storage).map_err(io)?;
        self.write_garc(path, &garc)?;
        self.update_pokedex_area_data_oras(&garc)
    }

    fn mark_area(
        &self,
        set: &EncounterSet,
        area_data: &mut [u8],
        area_index: i32,
        encounter_type: i32,
    ) {
        let length = if self.is_xy() {
            constants::PER_POKEMON_AREA_DATA_LENGTH_XY
        } else {
            constants::PER_POKEMON_AREA_DATA_LENGTH_ORAS
        };
        for encounter in &set.encounters {
            let base = self.core.data.base_number(encounter.pokemon) as usize;
            let offset = base * length + area_index as usize * 4;
            let value = read_long(area_data, offset) | encounter_type;
            write_long(area_data, offset, value);
        }
    }

    fn update_pokedex_area_data_xy(&self, garc: &GarcArchive, field_cro: &[u8]) -> Result<()> {
        let length = constants::PER_POKEMON_AREA_DATA_LENGTH_XY;
        let mut area_data = vec![0u8; (constants::POKEMON_COUNT as usize + 1) * length];
        for start in (0..area_data.len()).step_by(length) {
            area_data[start + 133] = 0x10;
        }
        let indexes = constants::map_num_to_pokedex_index(self.rom_type);
        let mut current_map = 0usize;
        for index in 0..garc.files.len().saturating_sub(1) {
            let file = garc_file(garc, index)?;
            let Some(offset) = xy_encounter_offset(file) else {
                continue;
            };
            let area_index = indexes[current_map];
            let data = copy_range(file, offset, XY_ENCOUNTER_DATA_LENGTH)?;
            for slot in &XY_SLOTS {
                let set = self.read_encounter(&data, slot.offset, slot.amount);
                self.mark_area(&set, &mut area_data, area_index, slot.area_type);
            }
            current_map += 1;
        }
        for (start, count) in [
            (
                constants::FALLING_ENCOUNTER_OFFSET,
                constants::FALLING_ENCOUNTER_COUNT,
            ),
            (
                constants::RUSTLING_BUSH_ENCOUNTER_OFFSET,
                constants::RUSTLING_BUSH_ENCOUNTER_COUNT,
            ),
        ] {
            for index in 0..count {
                let area_index = indexes[current_map];
                let offset = start + index * constants::FIELD_ENCOUNTER_SIZE;
                let set = self.read_field_encounter(field_cro, offset);
                self.mark_area(&set, &mut area_data, area_index, 0x800);
                current_map += 1;
            }
        }
        self.write_area_data(area_data)
    }

    fn update_pokedex_area_data_oras(&self, garc: &GarcArchive) -> Result<()> {
        let length = constants::PER_POKEMON_AREA_DATA_LENGTH_ORAS;
        let mut area_data = vec![0u8; (constants::POKEMON_COUNT as usize + 1) * length];
        let indexes = constants::map_num_to_pokedex_index(self.rom_type);
        let mut current_map = 0usize;
        for index in 0..garc.files.len().saturating_sub(2) {
            let file = garc_file(garc, index)?;
            let Some(offset) = oras_encounter_offset(file) else {
                continue;
            };
            let area_index = indexes[current_map];
            if area_index == -1 {
                current_map += 1;
                continue;
            }
            let data = copy_range(file, offset, ORAS_ENCOUNTER_DATA_LENGTH)?;
            let grass = self.read_encounter(&data, 0, 12);
            let foreign_type = if grass.encounters.is_empty() {
                0x08
            } else {
                0x04
            };
            for slot in &ORAS_SLOTS {
                let set = self.read_encounter(&data, slot.offset, slot.amount);
                let encounter_type = if slot.offset == 96 {
                    foreign_type
                } else {
                    slot.area_type
                };
                self.mark_area(&set, &mut area_data, area_index, encounter_type);
            }
            current_map += 1;
        }
        self.write_area_data(area_data)
    }

    fn write_area_data(&self, area_data: Vec<u8>) -> Result<()> {
        let path = self.file("PokedexAreaData");
        let mut area_garc = self.read_garc(path, Decompression::Skip)?;
        area_garc.set_file(0, area_data).map_err(io)?;
        self.write_garc(path, &area_garc)
    }
}
