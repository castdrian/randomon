use randomon_core::{RandomizationError, Result};
use randomon_data::constants::gen4;
use randomon_data::{Encounter, EncounterSet, PokemonId};
use randomon_formats::nds::NarcArchive;

use crate::bytes::{
    after_prefix, file, file_mut, i8_at, i32_at, set_bytes, set_i32, set_u8, set_u16, u8_at, u16_at,
};
use crate::handler::{Gen4RomHandler, MapNames};

const UNKNOWN_MAP_NAME: &str = "? Unknown ?";
const MAP_HEADER_FILE_STRIDE: usize = 16;
const MAP_HEADER_ARM9_STRIDE: i32 = 24;
const DPPT_GRASS_SLOTS: usize = 12;
const DPPT_ALTERNATE_SLOT_COUNT: i32 = 20;
const DPPT_TIME_OF_DAY_OFFSET: i32 = 108;
const DPPT_SEA_START: i32 = 204;
const DPPT_SEA_SETS: i32 = 5;
const DPPT_SEA_SLOTS: i32 = 5;
const DPPT_FILLER_SET: i32 = 1;
const FEEBAS_FILE: i32 = 0;
const TROPHY_GARDEN_FILE: i32 = 8;
const GREAT_MARSH_FILES: [i32; 2] = [9, 10];
const TROPHY_GARDEN_SLOTS: i32 = 16;
const GREAT_MARSH_SLOTS: i32 = 32;
const HONEY_TREE_SLOTS: i32 = 6;
const TROPHY_GARDEN_RETRIES: usize = 1000;
const HGSS_AMOUNTS: [i32; 6] = [0, 5, 2, 5, 5, 5];
const HGSS_GRASS_SLOTS: i32 = 12;
const HGSS_GRASS_LEVELS: i32 = 8;
const HGSS_GRASS_POKEMON: i32 = 20;
const HGSS_TIME_STRIDE: i32 = 24;
const HGSS_RADIO_OFFSET: i32 = 92;
const HGSS_SEA_START: i32 = 100;
const HEADBUTT_EMPTY_LENGTH: usize = 4;
const HEADBUTT_UNUSED_MAP: usize = 24;
const HEADBUTT_SLOTS: i32 = 18;
const BCC_OFFSETS: [i32; 4] = [0, 80, 160, 240];
const BCC_SLOTS: i32 = 10;
const BCC_NAMES: [&str; 4] = [
    "Bug Catching Contest (Pre-National Dex)",
    "Bug Catching Contest (Post-National Dex, Tuesdays)",
    "Bug Catching Contest (Post-National Dex, Thursdays)",
    "Bug Catching Contest (Post-National Dex, Saturdays)",
];

fn ran_out() -> RandomizationError {
    RandomizationError::IndexOutOfBounds("encounter set iterator")
}

struct SetCursor<'a> {
    sets: std::slice::Iter<'a, EncounterSet>,
}

impl<'a> SetCursor<'a> {
    fn new(sets: &'a [EncounterSet]) -> Self {
        Self { sets: sets.iter() }
    }

    fn next(&mut self) -> Result<&'a EncounterSet> {
        self.sets.next().ok_or_else(ran_out)
    }
}

fn encounter_at(encounters: &[Encounter], index: usize) -> Result<&Encounter> {
    encounters.get(index).ok_or_else(ran_out)
}

fn is_regular_species(number: i32) -> bool {
    (1..=gen4::POKEMON_COUNT).contains(&number)
}

fn dppt_alternate_offset(slot: i32) -> i32 {
    100 + slot * 4 + if slot >= 10 { 24 } else { 0 }
}

fn is_time_of_day_slot(slot: i32) -> bool {
    (2..=5).contains(&slot)
}

fn set_named(name: String, rate: i32, offset: i32, encounters: Vec<Encounter>) -> EncounterSet {
    EncounterSet {
        rate,
        encounters,
        display_name: Some(name),
        offset,
        ..EncounterSet::default()
    }
}

fn level_range(encounters: &[Encounter]) -> Option<(i32, i32)> {
    encounters
        .first()
        .map(|first| (first.level, first.max_level))
}

impl Gen4RomHandler {
    fn load_map_names(&self) -> Result<MapNames> {
        let internal_names = self.read_file("MapTableFile")?;
        let header_count = internal_names.len() / MAP_HEADER_FILE_STRIDE;
        let base = self.entry.int("MapTableARM9Offset");
        let all_map_names = self.get_strings(self.entry.int("MapNamesTextOffset"))?;
        let two_byte_index = self.entry.int("MapTableNameIndexSize") == 2;
        let mut names = MapNames::default();
        for map in 0..header_count as i32 {
            let header = base + map * MAP_HEADER_ARM9_STRIDE;
            let name_index = if two_byte_index {
                u16_at(&self.arm9, header + 18)?
            } else {
                u8_at(&self.arm9, header + 18)?
            };
            let name = all_map_names
                .get(name_index as usize)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("allMapNames"))?;
            if self.entry.is_hgss() {
                let wild_set = u8_at(&self.arm9, header)?;
                if wild_set != 255 {
                    names.wild.insert(wild_set, name.clone());
                }
                names.headbutt.insert(map, name);
            } else {
                let wild_set = u16_at(&self.arm9, header + 14)?;
                if wild_set != 65535 {
                    names.wild.insert(wild_set, name);
                }
            }
        }
        Ok(names)
    }

    fn ensure_map_names(&self) -> Result<()> {
        if self.map_names.borrow().is_none() {
            let names = self.load_map_names()?;
            *self.map_names.borrow_mut() = Some(names);
        }
        Ok(())
    }

    fn wild_map_name(&self, index: i32) -> String {
        let mut names = self.map_names.borrow_mut();
        names
            .get_or_insert_with(MapNames::default)
            .wild
            .entry(index)
            .or_insert_with(|| UNKNOWN_MAP_NAME.to_owned())
            .clone()
    }

    fn headbutt_map_name(&self, index: i32) -> String {
        self.map_names
            .borrow()
            .as_ref()
            .and_then(|names| names.headbutt.get(&index).cloned())
            .unwrap_or_else(|| "null".to_owned())
    }

    pub(crate) fn encounters(&self, use_time_of_day: bool) -> Result<Vec<EncounterSet>> {
        self.ensure_map_names()?;
        if self.entry.is_hgss() {
            self.encounters_hgss(use_time_of_day)
        } else {
            self.encounters_dppt(use_time_of_day)
        }
    }

    pub(crate) fn set_encounters_impl(
        &mut self,
        use_time_of_day: bool,
        encounters: &[EncounterSet],
    ) -> Result<()> {
        if encounters.is_empty() {
            return Ok(());
        }
        if self.entry.is_hgss() {
            self.set_encounters_hgss(use_time_of_day, encounters)?;
            self.update_pokedex_area_data_hgss(encounters)
        } else {
            self.set_encounters_dppt(use_time_of_day, encounters)?;
            self.update_pokedex_area_data_dppt(encounters)
        }
    }

    fn encounter(&self, species: i32, level: i32, max_level: i32) -> Result<Encounter> {
        Ok(Encounter {
            pokemon: self.pokemon_by_number(species)?,
            level,
            max_level,
            ..Encounter::default()
        })
    }

    fn read_grass_dppt(&self, data: &[u8]) -> Result<Vec<Encounter>> {
        (0..DPPT_GRASS_SLOTS as i32)
            .map(|slot| {
                let level = i32_at(data, 4 + slot * 8)?;
                let species = i32_at(data, 8 + slot * 8)?;
                self.encounter(species, level, 0)
            })
            .collect()
    }

    fn read_sea_dppt(&self, data: &[u8], offset: i32) -> Result<Vec<Encounter>> {
        (0..DPPT_SEA_SLOTS)
            .map(|slot| {
                let level = i32_at(data, offset + slot * 8)?;
                let species = i32_at(data, offset + 4 + slot * 8)?;
                self.encounter(species, level >> 8, level & 0xFF)
            })
            .collect()
    }

    fn read_extra_dppt(&self, data: &[u8], amount: i32) -> Result<Vec<Encounter>> {
        (0..amount)
            .map(|slot| self.encounter(i32_at(data, slot * 4)?, 1, 0))
            .collect()
    }

    fn dppt_area_sets(
        &self,
        data: &[u8],
        index: i32,
        use_time_of_day: bool,
        encounters: &mut Vec<EncounterSet>,
    ) -> Result<()> {
        let map_name = self.wild_map_name(index);
        let grass_rate = i32_at(data, 0)?;
        if grass_rate != 0 {
            let mut grass = self.read_grass_dppt(data)?;
            let base_grass = grass.clone();
            if use_time_of_day {
                for slot in 0..4 {
                    let species = i32_at(data, DPPT_TIME_OF_DAY_OFFSET + 4 * slot)?;
                    if is_regular_species(species) {
                        let source = gen4::DPPT_ALTERNATE_SLOTS[slot as usize + 2];
                        let level = encounter_at(&base_grass, source)?.level;
                        grass.push(self.encounter(species, level, 0)?);
                    }
                }
            }
            encounters.push(set_named(
                format!("{map_name} Grass/Cave"),
                grass_rate,
                index,
                grass,
            ));
            let mut conditions = Vec::new();
            for slot in 0..DPPT_ALTERNATE_SLOT_COUNT {
                if is_time_of_day_slot(slot) {
                    continue;
                }
                let species = i32_at(data, dppt_alternate_offset(slot))?;
                if is_regular_species(species) {
                    let source = gen4::DPPT_ALTERNATE_SLOTS[slot as usize];
                    let level = encounter_at(&base_grass, source)?.level;
                    conditions.push(self.encounter(species, level, 0)?);
                }
            }
            if !conditions.is_empty() {
                encounters.push(set_named(
                    format!("{map_name} Swarm/Radar/GBA"),
                    grass_rate,
                    index,
                    conditions,
                ));
            }
        }
        let mut offset = DPPT_SEA_START;
        for set in 0..DPPT_SEA_SETS {
            let rate = i32_at(data, offset)?;
            offset += 4;
            if rate != 0 && set != DPPT_FILLER_SET {
                encounters.push(set_named(
                    format!(
                        "{map_name} {}",
                        gen4::DPPT_WATER_SLOT_SET_NAMES[set as usize]
                    ),
                    rate,
                    index,
                    self.read_sea_dppt(data, offset)?,
                ));
            }
            offset += 40;
        }
        Ok(())
    }

    fn extra_set(&self, data: &[u8], amount: i32, name: String) -> Result<EncounterSet> {
        Ok(set_named(name, 1, 0, self.read_extra_dppt(data, amount)?))
    }

    fn feebas_set(&self, extra: &NarcArchive, overlay: &[u8]) -> Result<EncounterSet> {
        let mut set = self.extra_set(
            file(extra, FEEBAS_FILE)?,
            1,
            "Mt. Coronet Feebas Tiles".to_owned(),
        )?;
        if let Some(offset) = after_prefix(overlay, gen4::FEEBAS_LEVEL_PREFIX_DPPT) {
            let max_level = i8_at(overlay, offset)?;
            let level = i8_at(overlay, offset + 4)?;
            for encounter in &mut set.encounters {
                encounter.max_level = max_level;
                encounter.level = level;
            }
        }
        Ok(set)
    }

    fn honey_tree_set(&self, data: &[u8], overlay: &[u8], group: usize) -> Result<EncounterSet> {
        let mut set = self.extra_set(
            data,
            HONEY_TREE_SLOTS,
            format!("Honey Tree Group {}", group + 1),
        )?;
        if let Some(offset) = after_prefix(overlay, gen4::HONEY_TREE_LEVEL_PREFIX_DPPT) {
            let unpatched =
                u8_at(overlay, offset + 46)? == 0x0B && u8_at(overlay, offset + 47)? == 0x2E;
            let level = if unpatched {
                5
            } else {
                i8_at(overlay, offset + 46)?
            };
            let max_level = i8_at(overlay, offset + 102)?;
            for encounter in &mut set.encounters {
                encounter.max_level = max_level;
                encounter.level = level;
            }
        }
        Ok(set)
    }

    fn grass_level_of(encounters: &[EncounterSet], set: usize, slot: usize) -> Option<i32> {
        encounters
            .get(set)
            .and_then(|set| set.encounters.get(slot))
            .map(|encounter| encounter.level)
    }

    fn trophy_garden_set(
        &self,
        extra: &NarcArchive,
        encounters: &[EncounterSet],
    ) -> Result<EncounterSet> {
        let mut set = self.extra_set(
            file(extra, TROPHY_GARDEN_FILE)?,
            TROPHY_GARDEN_SLOTS,
            "Trophy Garden Rotating Pokemon (via Mr. Backlot)".to_owned(),
        )?;
        let index = gen4::trophy_garden_grass_encounter_index(self.rom_type());
        if let (Some(first), Some(second)) = (
            Self::grass_level_of(encounters, index, 6),
            Self::grass_level_of(encounters, index, 7),
        ) {
            for encounter in &mut set.encounters {
                encounter.level = first.min(second);
                if first != second {
                    encounter.max_level = first.max(second);
                }
            }
        }
        Ok(set)
    }

    fn great_marsh_levels(&self, encounters: &[EncounterSet]) -> (i32, i32) {
        let mut level = 100;
        let mut max_level = 0;
        for &set in gen4::marsh_grass_encounter_indices(self.rom_type()) {
            for slot in [6, 7] {
                if let Some(current) = Self::grass_level_of(encounters, set, slot) {
                    level = level.min(current);
                    max_level = max_level.max(current);
                }
            }
        }
        (level, max_level)
    }

    fn encounters_dppt(&self, use_time_of_day: bool) -> Result<Vec<EncounterSet>> {
        let data = self.read_narc("WildPokemon")?;
        let mut encounters = Vec::new();
        for (index, area) in data.files.iter().enumerate() {
            self.dppt_area_sets(area, index as i32, use_time_of_day, &mut encounters)?;
        }
        let extra = self.read_narc("ExtraEncounters")?;
        let overlay = self.read_overlay(self.entry.int("EncounterOvlNumber"))?;
        encounters.push(self.feebas_set(&extra, &overlay)?);
        for (group, &honey_file) in self
            .entry
            .array_or_empty("HoneyTreeOffsets")
            .iter()
            .enumerate()
        {
            encounters.push(self.honey_tree_set(file(&extra, honey_file)?, &overlay, group)?);
        }
        let trophy = self.trophy_garden_set(&extra, &encounters)?;
        encounters.push(trophy);
        let (level, max_level) = self.great_marsh_levels(&encounters);
        for (position, &marsh_file) in GREAT_MARSH_FILES.iter().enumerate() {
            let status = if position == 0 {
                "(Post-National Dex)"
            } else {
                "(Pre-National Dex)"
            };
            let mut set = self.extra_set(
                file(&extra, marsh_file)?,
                GREAT_MARSH_SLOTS,
                format!("Great Marsh Rotating Pokemon {status}"),
            )?;
            for encounter in &mut set.encounters {
                encounter.level = level;
                encounter.max_level = max_level;
            }
            encounters.push(set);
        }
        Ok(encounters)
    }

    fn write_dppt_area(
        &self,
        data: &mut [u8],
        use_time_of_day: bool,
        cursor: &mut SetCursor<'_>,
    ) -> Result<()> {
        let grass_rate = i32_at(data, 0)?;
        if grass_rate != 0 {
            let grass = &cursor.next()?.encounters;
            for slot in 0..DPPT_GRASS_SLOTS {
                let encounter = encounter_at(grass, slot)?;
                set_i32(data, 4 + slot as i32 * 8, encounter.level)?;
                set_i32(data, 8 + slot as i32 * 8, self.number_of(encounter.pokemon))?;
            }
            let mut time_of_day_slot = DPPT_GRASS_SLOTS;
            for slot in 0..4 {
                let offset = DPPT_TIME_OF_DAY_OFFSET + 4 * slot;
                if !is_regular_species(i32_at(data, offset)?) {
                    continue;
                }
                let source = if use_time_of_day {
                    time_of_day_slot += 1;
                    time_of_day_slot - 1
                } else {
                    gen4::DPPT_ALTERNATE_SLOTS[slot as usize + 2]
                };
                let pokemon = encounter_at(grass, source)?.pokemon;
                set_i32(data, offset, self.number_of(pokemon))?;
            }
            let mut conditions: Option<std::slice::Iter<'_, Encounter>> = None;
            for slot in 0..DPPT_ALTERNATE_SLOT_COUNT {
                if is_time_of_day_slot(slot) {
                    continue;
                }
                let offset = dppt_alternate_offset(slot);
                if !is_regular_species(i32_at(data, offset)?) {
                    continue;
                }
                if conditions.is_none() {
                    conditions = Some(cursor.next()?.encounters.iter());
                }
                let encounter = conditions
                    .as_mut()
                    .and_then(Iterator::next)
                    .ok_or_else(ran_out)?;
                set_i32(data, offset, self.number_of(encounter.pokemon))?;
            }
        }
        let mut offset = DPPT_SEA_START;
        for set in 0..DPPT_SEA_SETS {
            let rate = i32_at(data, offset)?;
            offset += 4;
            if rate != 0 && set != DPPT_FILLER_SET {
                for (slot, encounter) in cursor.next()?.encounters.iter().enumerate() {
                    let slot_offset = offset + slot as i32 * 8;
                    set_i32(
                        data,
                        slot_offset,
                        (encounter.level << 8) + encounter.max_level,
                    )?;
                    set_i32(data, slot_offset + 4, self.number_of(encounter.pokemon))?;
                }
            }
            offset += 40;
        }
        Ok(())
    }

    fn write_extra_dppt(&self, data: &mut [u8], encounters: &[Encounter]) -> Result<()> {
        for (slot, encounter) in encounters.iter().enumerate() {
            set_i32(data, slot as i32 * 4, self.number_of(encounter.pokemon))?;
        }
        Ok(())
    }

    fn patch_honey_tree_levels(overlay: &mut [u8], level: i32, max_level: i32) -> Result<()> {
        let Some(offset) = after_prefix(overlay, gen4::HONEY_TREE_LEVEL_PREFIX_DPPT) else {
            return Ok(());
        };
        set_u8(overlay, offset + 46, level)?;
        set_bytes(
            overlay,
            offset + 47,
            &[0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x19],
        )?;
        set_u8(overlay, offset + 102, max_level)?;
        let range = max_level - level;
        let divisor = 0xFFFF / (range.max(0) + 1) + 1;
        set_i32(overlay, offset + 148, divisor)
    }

    fn distinct_species(&self, encounters: &[Encounter]) -> usize {
        let mut seen: Vec<PokemonId> = Vec::new();
        for encounter in encounters {
            if !seen.contains(&encounter.pokemon) {
                seen.push(encounter.pokemon);
            }
        }
        seen.len()
    }

    fn avoid_trophy_garden_softlock(&mut self, encounters: &mut [Encounter]) -> Result<()> {
        for _ in 0..TROPHY_GARDEN_RETRIES {
            if encounters.len() < 2 || self.distinct_species(encounters) != 1 {
                break;
            }
            encounters[0].pokemon = randomon_core::engine::pool::random_pokemon(self)?;
        }
        Ok(())
    }

    fn set_encounters_dppt(
        &mut self,
        use_time_of_day: bool,
        encounters: &[EncounterSet],
    ) -> Result<()> {
        let mut cursor = SetCursor::new(encounters);
        let mut wild = self.read_narc("WildPokemon")?;
        for area in &mut wild.files {
            self.write_dppt_area(area, use_time_of_day, &mut cursor)?;
        }
        self.write_narc("WildPokemon", &wild)?;
        let mut extra = self.read_narc("ExtraEncounters")?;
        let overlay_number = self.entry.int("EncounterOvlNumber");
        let mut overlay = self.read_overlay(overlay_number)?;
        let feebas = cursor.next()?;
        if let (Some(offset), Some((level, max_level))) = (
            after_prefix(&overlay, gen4::FEEBAS_LEVEL_PREFIX_DPPT),
            level_range(&feebas.encounters),
        ) {
            set_u8(&mut overlay, offset, max_level)?;
            set_u8(&mut overlay, offset + 4, level)?;
        }
        self.write_extra_dppt(file_mut(&mut extra, FEEBAS_FILE)?, &feebas.encounters)?;
        for &honey_file in self.entry.array_or_empty("HoneyTreeOffsets") {
            let honey = cursor.next()?;
            if let Some((level, max_level)) = level_range(&honey.encounters) {
                Self::patch_honey_tree_levels(&mut overlay, level, max_level)?;
            }
            self.write_extra_dppt(file_mut(&mut extra, honey_file)?, &honey.encounters)?;
        }
        let mut trophy = cursor.next()?.encounters.clone();
        self.avoid_trophy_garden_softlock(&mut trophy)?;
        self.write_extra_dppt(file_mut(&mut extra, TROPHY_GARDEN_FILE)?, &trophy)?;
        for marsh_file in GREAT_MARSH_FILES {
            let marsh = cursor.next()?;
            self.write_extra_dppt(file_mut(&mut extra, marsh_file)?, &marsh.encounters)?;
        }
        self.write_overlay(overlay_number, &overlay)?;
        self.write_narc("ExtraEncounters", &extra)
    }

    fn read_optional_hgss(
        &self,
        data: &[u8],
        offset: i32,
        amount: i32,
        name: String,
    ) -> Result<EncounterSet> {
        let mut encounters = Vec::new();
        for slot in 0..amount {
            let species = u16_at(data, offset + slot * 2)?;
            if species != 0 {
                encounters.push(self.encounter(species, 1, 0)?);
            }
        }
        Ok(set_named(name, 1, 0, encounters))
    }

    fn read_grass_hgss(&self, data: &[u8], time: i32) -> Result<Vec<Encounter>> {
        (0..HGSS_GRASS_SLOTS)
            .map(|slot| {
                let level = u8_at(data, HGSS_GRASS_LEVELS + slot)?;
                let species = u16_at(
                    data,
                    HGSS_GRASS_POKEMON + time * HGSS_TIME_STRIDE + slot * 2,
                )?;
                self.encounter(species, level, 0)
            })
            .collect()
    }

    fn read_sea_hgss(&self, data: &[u8], offset: i32, amount: i32) -> Result<Vec<Encounter>> {
        (0..amount)
            .map(|slot| {
                let level = u16_at(data, offset + slot * 4)?;
                let species = u16_at(data, offset + 2 + slot * 4)?;
                self.encounter(species, level & 0xFF, level >> 8)
            })
            .collect()
    }

    fn push_if_any(encounters: &mut Vec<EncounterSet>, set: EncounterSet) {
        if !set.encounters.is_empty() {
            encounters.push(set);
        }
    }

    fn hgss_area_sets(
        &self,
        data: &[u8],
        index: i32,
        use_time_of_day: bool,
        encounters: &mut Vec<EncounterSet>,
    ) -> Result<()> {
        let map_name = self.wild_map_name(index);
        let rates: Vec<i32> = (0..6)
            .map(|rate| u8_at(data, rate))
            .collect::<Result<_>>()?;
        if rates[0] != 0 {
            if use_time_of_day {
                for (time, time_name) in gen4::HGSS_TIME_OF_DAY_NAMES.iter().enumerate() {
                    encounters.push(set_named(
                        format!("{map_name} {time_name} Grass/Cave"),
                        rates[0],
                        0,
                        self.read_grass_hgss(data, time as i32)?,
                    ));
                }
            } else {
                encounters.push(set_named(
                    format!("{map_name} Grass/Cave"),
                    rates[0],
                    0,
                    self.read_grass_hgss(data, 1)?,
                ));
            }
        }
        let radio = self.read_optional_hgss(
            data,
            HGSS_RADIO_OFFSET,
            4,
            format!("{map_name} Hoenn/Sinnoh Radio"),
        )?;
        Self::push_if_any(encounters, radio);
        let mut offset = HGSS_SEA_START;
        for set in 1..6 {
            if rates[set] != 0 {
                encounters.push(set_named(
                    format!("{map_name} {}", gen4::HGSS_NON_GRASS_SET_NAMES[set]),
                    rates[set],
                    0,
                    self.read_sea_hgss(data, offset, HGSS_AMOUNTS[set])?,
                ));
            }
            offset += 4 * HGSS_AMOUNTS[set];
        }
        for (delta, amount, label) in [
            (0, 2, "Swarms"),
            (4, 1, "Night Fishing Replacement"),
            (6, 1, "Fishing Swarm"),
        ] {
            let set = self.read_optional_hgss(
                data,
                offset + delta,
                amount,
                format!("{map_name} {label}"),
            )?;
            Self::push_if_any(encounters, set);
        }
        Ok(())
    }

    fn headbutt_set(&self, data: &[u8], index: usize) -> Result<Option<EncounterSet>> {
        if data.len() == HEADBUTT_EMPTY_LENGTH || index == HEADBUTT_UNUSED_MAP {
            return Ok(None);
        }
        let mut encounters = Vec::new();
        for slot in 0..HEADBUTT_SLOTS {
            let species = u16_at(data, 4 + slot * 4)?;
            if species != 0 {
                let level = i8_at(data, 6 + slot * 4)?;
                let max_level = i8_at(data, 7 + slot * 4)?;
                encounters.push(self.encounter(species, level, max_level)?);
            }
        }
        if encounters.is_empty() {
            return Ok(None);
        }
        let name = format!("{} Headbutt", self.headbutt_map_name(index as i32));
        Ok(Some(set_named(name, 1, 0, encounters)))
    }

    fn bcc_set(&self, data: &[u8], offset: i32, name: &str) -> Result<Option<EncounterSet>> {
        let mut encounters = Vec::new();
        for slot in 0..BCC_SLOTS {
            let species = u16_at(data, offset + slot * 8)?;
            if species != 0 {
                let level = i8_at(data, offset + 2 + slot * 8)?;
                let max_level = i8_at(data, offset + 3 + slot * 8)?;
                encounters.push(self.encounter(species, level, max_level)?);
            }
        }
        Ok((!encounters.is_empty()).then(|| set_named(name.to_owned(), 1, 0, encounters)))
    }

    fn encounters_hgss(&self, use_time_of_day: bool) -> Result<Vec<EncounterSet>> {
        let data = self.read_narc("WildPokemon")?;
        let mut encounters = Vec::new();
        for (index, area) in data.files.iter().enumerate() {
            self.hgss_area_sets(area, index as i32, use_time_of_day, &mut encounters)?;
        }
        let headbutt = self.read_narc("HeadbuttPokemon")?;
        for (index, area) in headbutt.files.iter().enumerate() {
            encounters.extend(self.headbutt_set(area, index)?);
        }
        let bcc = self.read_file("BCCWilds")?;
        for (offset, name) in BCC_OFFSETS.iter().zip(BCC_NAMES) {
            encounters.extend(self.bcc_set(&bcc, *offset, name)?);
        }
        Ok(encounters)
    }

    fn write_grass_hgss(
        &self,
        data: &mut [u8],
        encounters: &[Encounter],
        times: &[i32],
        with_levels: bool,
    ) -> Result<()> {
        for (slot, encounter) in encounters.iter().enumerate() {
            if with_levels {
                set_u8(data, HGSS_GRASS_LEVELS + slot as i32, encounter.level)?;
            }
            for &time in times {
                set_u16(
                    data,
                    HGSS_GRASS_POKEMON + time * HGSS_TIME_STRIDE + slot as i32 * 2,
                    self.number_of(encounter.pokemon),
                )?;
            }
        }
        Ok(())
    }

    fn write_optional_hgss(
        &self,
        data: &mut [u8],
        offset: i32,
        amount: i32,
        cursor: &mut SetCursor<'_>,
    ) -> Result<()> {
        let mut replacements: Option<std::slice::Iter<'_, Encounter>> = None;
        for slot in 0..amount {
            if u16_at(data, offset + slot * 2)? == 0 {
                continue;
            }
            if replacements.is_none() {
                replacements = Some(cursor.next()?.encounters.iter());
            }
            let encounter = replacements
                .as_mut()
                .and_then(Iterator::next)
                .ok_or_else(ran_out)?;
            set_u16(data, offset + slot * 2, self.number_of(encounter.pokemon))?;
        }
        Ok(())
    }

    fn write_hgss_area(
        &self,
        data: &mut [u8],
        use_time_of_day: bool,
        cursor: &mut SetCursor<'_>,
    ) -> Result<()> {
        let rates: Vec<i32> = (0..6)
            .map(|rate| u8_at(data, rate))
            .collect::<Result<_>>()?;
        if rates[0] != 0 {
            let grass = &cursor.next()?.encounters;
            if use_time_of_day {
                self.write_grass_hgss(data, grass, &[0], true)?;
                for time in 1..3 {
                    let grass = &cursor.next()?.encounters;
                    self.write_grass_hgss(data, grass, &[time], false)?;
                }
            } else {
                self.write_grass_hgss(data, grass, &[0, 1, 2], true)?;
            }
        }
        self.write_optional_hgss(data, HGSS_RADIO_OFFSET, 4, cursor)?;
        let mut offset = HGSS_SEA_START;
        for set in 1..6 {
            if rates[set] != 0 {
                for (slot, encounter) in cursor.next()?.encounters.iter().enumerate() {
                    let slot_offset = offset + slot as i32 * 4;
                    set_u8(data, slot_offset, encounter.level)?;
                    set_u8(data, slot_offset + 1, encounter.max_level)?;
                    set_u16(data, slot_offset + 2, self.number_of(encounter.pokemon))?;
                }
            }
            offset += 4 * HGSS_AMOUNTS[set];
        }
        self.write_optional_hgss(data, offset, 2, cursor)?;
        self.write_optional_hgss(data, offset + 4, 1, cursor)?;
        self.write_optional_hgss(data, offset + 6, 1, cursor)
    }

    fn write_encounter_slots(
        &self,
        data: &mut [u8],
        offset: i32,
        stride: i32,
        slots: i32,
        encounters: &[Encounter],
    ) -> Result<()> {
        let mut replacements = encounters.iter();
        for slot in 0..slots {
            let slot_offset = offset + slot * stride;
            if u16_at(data, slot_offset)? == 0 {
                continue;
            }
            let Some(encounter) = replacements.next() else {
                break;
            };
            set_u16(data, slot_offset, self.number_of(encounter.pokemon))?;
            set_u8(data, slot_offset + 2, encounter.level)?;
            set_u8(data, slot_offset + 3, encounter.max_level)?;
        }
        Ok(())
    }

    fn set_encounters_hgss(
        &mut self,
        use_time_of_day: bool,
        encounters: &[EncounterSet],
    ) -> Result<()> {
        let mut cursor = SetCursor::new(encounters);
        let mut wild = self.read_narc("WildPokemon")?;
        for area in &mut wild.files {
            self.write_hgss_area(area, use_time_of_day, &mut cursor)?;
        }
        self.write_narc("WildPokemon", &wild)?;
        let mut headbutt = self.read_narc("HeadbuttPokemon")?;
        for index in 0..headbutt.files.len() {
            if self.headbutt_set(&headbutt.files[index], index)?.is_none() {
                continue;
            }
            let set = cursor.next()?;
            self.write_encounter_slots(
                &mut headbutt.files[index],
                4,
                4,
                HEADBUTT_SLOTS,
                &set.encounters,
            )?;
        }
        self.write_narc("HeadbuttPokemon", &headbutt)?;
        let mut bcc = self.read_file("BCCWilds")?;
        for (offset, name) in BCC_OFFSETS.iter().zip(BCC_NAMES) {
            if self.bcc_set(&bcc, *offset, name)?.is_none() {
                continue;
            }
            let set = cursor.next()?;
            self.write_encounter_slots(&mut bcc, *offset, 8, BCC_SLOTS, &set.encounters)?;
        }
        self.write_file("BCCWilds", &bcc)
    }
}
