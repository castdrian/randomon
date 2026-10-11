use randomon_core::{RandomizationError, Result};
use randomon_ctr::bytes::{find_after_prefix, read_long, read_word, write_word};
use randomon_data::{Encounter, EncounterSet, PokemonId, SosType};
use randomon_formats::ctr::{Decompression, mini};
use randomon_java::JavaHashSet;

use crate::constants;
use crate::handler::{AreaData, Gen7RomHandler, io};
use crate::pokemon::garc_file;

const TABLE_LENGTH: usize = 0x164;
const TRIMMED_TABLE_LENGTH: usize = 0x2C8;
const ZONE_SIZE: usize = 0x54;
const AREA_FILE_STRIDE: usize = 11;

struct ZoneData {
    area_index: i32,
    location_name: String,
}

fn signed(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset] as i8)
}

fn unpack(data: &[u8], identifier: &str) -> Result<Option<Vec<Vec<u8>>>> {
    mini::unpack(data, identifier).map_err(io)
}

fn unpack_required(data: &[u8], identifier: &str) -> Result<Vec<Vec<u8>>> {
    unpack(data, identifier)?.ok_or(RandomizationError::NullPointer("Mini.UnpackMini"))
}

fn sos_type_for_index(index: usize) -> SosType {
    match index / 2 {
        0 => SosType::Rain,
        1 => SosType::Hail,
        _ => SosType::Sand,
    }
}

fn no_such_element() -> RandomizationError {
    RandomizationError::randomization("java.util.NoSuchElementException")
}

fn location_list(locations: &[String]) -> Result<Vec<String>> {
    let mut good = locations.to_vec();
    for index in (0..locations.len()).step_by(2) {
        let subarea = locations
            .get(index + 1)
            .ok_or(RandomizationError::IndexOutOfBounds("locationList"))?;
        if !subarea.is_empty() && !subarea.starts_with('[') {
            good[index] = format!("{} ({subarea})", good[index]);
        }
        if index > 0 {
            let earlier = &good[..index - 1];
            let current = &good[index];
            if !current.is_empty() && earlier.contains(current) {
                let usages = earlier.iter().filter(|name| *name == current).count();
                good[index] = format!("{current} ({})", usages + 1);
            }
        }
    }
    Ok(good)
}

fn area_name(zones: &[&ZoneData]) -> String {
    let mut unique: JavaHashSet<String> = JavaHashSet::new();
    for zone in zones {
        unique.add(zone.location_name.clone());
    }
    unique.iter().cloned().collect::<Vec<_>>().join(" / ")
}

impl Gen7RomHandler {
    fn read_worlds(&self) -> Result<Vec<Vec<u8>>> {
        let garc = self.read_garc(self.file("WorldData"), Decompression::All)?;
        (0..garc.files.len())
            .map(|index| {
                let packed = garc_file(&garc, index)?;
                unpack_required(packed, "WD")?
                    .into_iter()
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("worlds"))
            })
            .collect()
    }

    fn read_zones(&self, worlds: &[Vec<u8>]) -> Result<Vec<ZoneData>> {
        let garc = self.read_garc(self.file("ZoneData"), Decompression::All)?;
        let zone_bytes = garc_file(&garc, 0)?;
        let world_data = garc_file(&garc, 1)?;
        let locations = location_list(&self.strings(false, self.entry.int("MapNamesTextOffset"))?)?;
        let mut zones = Vec::new();
        for index in 0..zone_bytes.len() / ZONE_SIZE {
            let parent_map = read_long(zone_bytes, index * ZONE_SIZE + 0x1C);
            let world_index = read_word(world_data, index * 2) as usize;
            let location_name = locations
                .get(parent_map as usize)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("locationList"))?;
            let world = worlds
                .get(world_index)
                .ok_or(RandomizationError::IndexOutOfBounds("worlds"))?;
            zones.push(ZoneData {
                area_index: Self::zone_area_index(world, index as i32),
                location_name,
            });
        }
        Ok(zones)
    }

    fn zone_area_index(world: &[u8], zone: i32) -> i32 {
        let mapping_offset = read_long(world, 0x8) as usize;
        let mut offset = mapping_offset;
        while offset < world.len() {
            if read_word(world, offset) == zone {
                return read_word(world, offset + 2);
            }
            offset += 4;
        }
        0
    }

    pub(crate) fn read_area_data(&mut self) -> Result<Vec<AreaData>> {
        let worlds = self.read_worlds()?;
        let zones = self.read_zones(&worlds)?;
        self.encounter_garc = self.read_garc(
            self.file("WildPokemon"),
            Decompression::Selected(constants::relevant_encounter_files(self.rom_type)),
        )?;
        let area_count = self.encounter_garc.files.len() / AREA_FILE_STRIDE;
        let mut areas = Vec::with_capacity(area_count);
        for index in 0..area_count {
            let area_zones: Vec<&ZoneData> = zones
                .iter()
                .filter(|zone| zone.area_index == index as i32)
                .collect();
            let mut area = AreaData {
                file_number: 9 + AREA_FILE_STRIDE * index,
                name: area_name(&area_zones),
                ..AreaData::default()
            };
            let data = garc_file(&self.encounter_garc, area.file_number)?;
            if !data.is_empty() {
                let tables = unpack_required(data, "EA")?;
                area.has_tables = tables.iter().any(|table| !table.is_empty());
                if area.has_tables {
                    for table in tables {
                        let trimmed = table
                            .get(4..4 + TRIMMED_TABLE_LENGTH)
                            .ok_or(RandomizationError::IndexOutOfBounds("encounterTable"))?;
                        area.encounter_tables.push(trimmed.to_vec());
                    }
                }
            }
            areas.push(area);
        }
        Ok(areas)
    }

    fn encounter_pokemon(&self, species: i32, forme: i32) -> Result<PokemonId> {
        if !self.core.data.exists(self.pk(species)) {
            return Err(RandomizationError::NullPointer("pokes[species]"));
        }
        Ok(self.pokemon_with_forme(species, forme))
    }

    fn read_encounter_table(&self, table: &[u8]) -> Result<EncounterSet> {
        let min_level = signed(table, 0);
        let max_level = signed(table, 1);
        let mut set = EncounterSet {
            rate: 1,
            ..EncounterSet::default()
        };
        let encounter = |species: i32, forme: i32| -> Result<Encounter> {
            Ok(Encounter {
                pokemon: self.encounter_pokemon(species, forme)?,
                forme_number: forme,
                level: min_level,
                max_level,
                ..Encounter::default()
            })
        };
        for slot in 0..10 {
            let offset = 0xC + slot * 4;
            let value = read_word(table, offset);
            if value & 0x7FF == 0 {
                continue;
            }
            set.encounters.push(encounter(value & 0x7FF, value >> 11)?);
            for ally in 1..8 {
                let value = read_word(table, offset + 40 * ally);
                let mut sos = encounter(value & 0x7FF, value >> 11)?;
                sos.is_sos = true;
                sos.sos_type = Some(SosType::Generic);
                set.encounters.push(sos);
            }
        }
        for weather in 0..6 {
            let value = read_word(table, 0x14C + weather * 4);
            if value & 0x7FF == 0 {
                continue;
            }
            let mut sos = encounter(value & 0x7FF, value >> 11)?;
            sos.is_sos = true;
            sos.sos_type = Some(sos_type_for_index(weather));
            set.encounters.push(sos);
        }
        Ok(set)
    }

    pub(crate) fn read_encounters(&self, use_time_of_day: bool) -> Result<Vec<EncounterSet>> {
        let mut encounters = Vec::new();
        for area in self.area_data.iter().filter(|area| area.has_tables) {
            for (index, table) in area.encounter_tables.iter().enumerate() {
                let mut day = self.read_encounter_table(&table[..TABLE_LENGTH])?;
                let label = format!("{}, Table {}", area.name, index + 1);
                if !use_time_of_day {
                    day.display_name = Some(label);
                    encounters.push(day);
                    continue;
                }
                day.display_name = Some(format!("{label} (Day)"));
                encounters.push(day);
                let mut night =
                    self.read_encounter_table(&table[TABLE_LENGTH..TRIMMED_TABLE_LENGTH])?;
                night.display_name = Some(format!("{label} (Night)"));
                encounters.push(night);
            }
        }
        Ok(encounters)
    }

    fn species_and_forme(&self, encounter: &Encounter) -> i32 {
        (encounter.forme_number << 11).wrapping_add(self.core.data.base_number(encounter.pokemon))
    }

    fn write_encounter_table(
        &self,
        table: &mut [u8],
        offset: usize,
        encounters: &[Encounter],
    ) -> Result<()> {
        let first = encounters
            .first()
            .ok_or(RandomizationError::IndexOutOfBounds("encounters.get(0)"))?;
        table[offset] = first.level as u8;
        table[offset + 1] = first.max_level as u8;
        let slots = encounters.len() / 8;
        let mut iterator = encounters.iter();
        for slot in 0..slots {
            let current = offset + 0xC + slot * 4;
            let encounter = iterator.next().ok_or_else(no_such_element)?;
            write_word(table, current, self.species_and_forme(encounter));
            for ally in 1..8 {
                let sos = iterator.next().ok_or_else(no_such_element)?;
                write_word(table, current + 40 * ally, self.species_and_forme(sos));
            }
        }
        if encounters.len() != slots * 8 {
            for weather in 0..6 {
                let sos = iterator.next().ok_or_else(no_such_element)?;
                write_word(
                    table,
                    offset + 0x14C + weather * 4,
                    self.species_and_forme(sos),
                );
            }
        }
        Ok(())
    }

    pub(crate) fn write_encounters(
        &mut self,
        use_time_of_day: bool,
        sets: &[EncounterSet],
    ) -> Result<()> {
        let mut iterator = sets.iter();
        let mut areas = std::mem::take(&mut self.area_data);
        let result = (|| {
            for area in areas.iter_mut().filter(|area| area.has_tables) {
                for table in area.encounter_tables.iter_mut() {
                    let day = iterator.next().ok_or_else(no_such_element)?;
                    let night = if use_time_of_day {
                        iterator.next().ok_or_else(no_such_element)?
                    } else {
                        day
                    };
                    self.write_encounter_table(table, 0, &day.encounters)?;
                    self.write_encounter_table(table, TABLE_LENGTH, &night.encounters)?;
                }
            }
            Ok(())
        })();
        self.area_data = areas;
        result?;
        self.save_area_data()?;
        self.patch_minior_encounter_code();
        Ok(())
    }

    fn save_area_data(&mut self) -> Result<()> {
        for area in self.area_data.iter().filter(|area| area.has_tables) {
            let data = garc_file(&self.encounter_garc, area.file_number)?;
            let mut tables = unpack_required(data, "EA")?;
            for (table, updated) in tables.iter_mut().zip(&area.encounter_tables) {
                table
                    .get_mut(4..4 + updated.len())
                    .ok_or(RandomizationError::IndexOutOfBounds("encounterTable"))?
                    .copy_from_slice(updated);
            }
            let packed = mini::pack(&tables, "EA").map_err(io)?;
            self.encounter_garc
                .set_file(area.file_number, packed)
                .map_err(io)?;
        }
        Ok(())
    }

    fn patch_minior_encounter_code(&mut self) {
        if let Some(offset) =
            find_after_prefix(&self.code, constants::MINIOR_WILD_ENCOUNTER_PATCH_PREFIX)
        {
            self.code[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]);
        }
    }

    fn environment(&self, area: usize) -> Result<Option<Vec<Vec<u8>>>> {
        unpack(
            garc_file(&self.encounter_garc, area * AREA_FILE_STRIDE)?,
            "ED",
        )
    }

    pub(crate) fn field_items(&self) -> Result<Vec<i32>> {
        let mut items = Vec::new();
        for area in 0..self.encounter_garc.files.len() / AREA_FILE_STRIDE {
            let Some(environment) = self.environment(area)? else {
                continue;
            };
            let item_data = unpack_required(&environment[10], "EI")?;
            let berry_piles = unpack_required(&environment[11], "EB")?;
            for data in item_data.iter().filter(|data| !data.is_empty()) {
                for item in 0..signed(data, 0).max(0) as usize {
                    items.push(read_word(data, item * 64 + 52));
                }
            }
            for data in berry_piles.iter().filter(|data| !data.is_empty()) {
                for pile in 0..signed(data, 0).max(0) as usize {
                    for berry in 0..7 {
                        items.push(read_word(data, 4 + pile * 68 + 54 + berry * 2));
                    }
                }
            }
        }
        Ok(items)
    }

    pub(crate) fn write_field_items(&mut self, field_items: &[i32]) -> Result<()> {
        let mut iterator = field_items.iter();
        for area in 0..self.encounter_garc.files.len() / AREA_FILE_STRIDE {
            let Some(mut environment) = self.environment(area)? else {
                continue;
            };
            let mut item_data = unpack_required(&environment[10], "EI")?;
            let mut berry_piles = unpack_required(&environment[11], "EB")?;
            for data in item_data.iter_mut().filter(|data| !data.is_empty()) {
                for item in 0..signed(data, 0).max(0) as usize {
                    let value = *iterator.next().ok_or_else(no_such_element)?;
                    write_word(data, item * 64 + 52, value);
                }
            }
            environment[10] = mini::pack(&item_data, "EI").map_err(io)?;
            for data in berry_piles.iter_mut().filter(|data| !data.is_empty()) {
                for pile in 0..signed(data, 0).max(0) as usize {
                    for berry in 0..7 {
                        let value = *iterator.next().ok_or_else(no_such_element)?;
                        write_word(data, 4 + pile * 68 + 54 + berry * 2, value);
                    }
                }
            }
            environment[11] = mini::pack(&berry_piles, "EB").map_err(io)?;
            let packed = mini::pack(&environment, "ED").map_err(io)?;
            self.encounter_garc
                .set_file(area * AREA_FILE_STRIDE, packed)
                .map_err(io)?;
        }
        Ok(())
    }
}
