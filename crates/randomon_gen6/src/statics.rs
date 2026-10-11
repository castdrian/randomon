use std::collections::HashMap;

use randomon_core::{RandomizationError, Result};
use randomon_ctr::bytes::{find, find_after_prefix, read_long, read_word, write_long, write_word};
use randomon_data::{PokemonId, StaticEncounter};
use randomon_formats::ctr::{Amx, Decompression};
use randomon_java::JavaRng;

use crate::constants;
use crate::handler::{Gen6RomHandler, io};
use crate::pokemon::{garc_file, garc_file_mut};

const ORAS_EGG_MET_LOCATION: i32 = 0xEA64;

fn signed(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset] as i8)
}

fn no_such_element() -> RandomizationError {
    RandomizationError::randomization("java.util.NoSuchElementException")
}

fn missing_script() -> RandomizationError {
    RandomizationError::NullPointer("AMX.decData")
}

fn write_species_comparison(data: &mut [u8], offset: usize, species: i32) {
    let upper = (species & 0x00FF) as u8;
    let lower = ((species & 0xFF00) >> 8) as u8;
    data[offset..offset + 8].copy_from_slice(&[lower, 0x0C, 0x40, 0xE2, upper, 0x00, 0x50, 0xE2]);
}

impl Gen6RomHandler {
    fn starter_indices(&self) -> &'static [i32] {
        self.entry.array("StarterIndices")
    }

    fn gift_layout(&self) -> (usize, usize, usize) {
        (
            constants::gift_pokemon_count(self.rom_type),
            constants::gift_pokemon_size(self.rom_type),
            self.entry.int("GiftPokemonOffset") as usize,
        )
    }

    pub(crate) fn read_starters(&self) -> Result<Vec<PokemonId>> {
        let static_cro = self.read_file(self.file("StaticPokemon"))?;
        let (count, size, offset) = self.gift_layout();
        let mut starters = Vec::new();
        for index in 0..count {
            if !self.starter_indices().contains(&(index as i32)) {
                continue;
            }
            let base = offset + index * size;
            let species = read_word(&static_cro, base);
            let forme = signed(&static_cro, base + 4);
            starters.push(self.pokemon_with_forme(species, forme));
        }
        Ok(starters)
    }

    pub(crate) fn write_starters(&mut self, starters: &[PokemonId]) -> Result<()> {
        let static_path = self.file("StaticPokemon");
        let display_path = self.file("StarterDisplay");
        let mut static_cro = self.read_file(static_path)?;
        let mut display_cro = self.read_file(display_path)?;
        let (count, size, offset) = self.gift_layout();
        let display_offset =
            (read_word(&display_cro, self.entry.int("StarterOffsetOffset") as usize)
                + self.entry.int("StarterExtraOffset")) as usize;
        let mut iterator = starters.iter();
        let mut display_index = 0usize;
        let mut starter_text = self.strings(false, self.entry.int("StarterTextOffset"))?;
        let text_indices = self.entry.array("SpecificStarterTextOffsets");
        for index in 0..count {
            if !self.starter_indices().contains(&(index as i32)) {
                continue;
            }
            let mut starter = *iterator.next().ok_or_else(no_such_element)?;
            let mut forme = 0;
            let pokemon = self.core.data.pk(starter);
            if pokemon.forme_number > 0 {
                forme = pokemon.forme_number;
                starter = pokemon
                    .base_forme
                    .ok_or(RandomizationError::NullPointer("baseForme"))?;
            }
            let cosmetic_forms = self.core.data.pk(starter).cosmetic_forms;
            if cosmetic_forms > 0 {
                let mut random = self.core.engine.random.clone();
                forme = random.next_int_bounded(cosmetic_forms);
            }
            let base = offset + index * size;
            write_word(&mut static_cro, base, starter.number());
            static_cro[base + 4] = forme as u8;
            let display = display_offset + display_index * 0x54;
            write_word(&mut display_cro, display, starter.number());
            display_cro[display + 2] = forme as u8;
            if display_index < 3 {
                let text_index = text_indices
                    .get(display_index)
                    .copied()
                    .ok_or(RandomizationError::IndexOutOfBounds("starterTextIndices"))?;
                *starter_text
                    .get_mut(text_index as usize)
                    .ok_or(RandomizationError::IndexOutOfBounds("starterText"))? =
                    "[VAR PKNAME(0000)]".to_owned();
            }
            display_index += 1;
        }
        self.write_file(static_path, &static_cro)?;
        self.write_file(display_path, &display_cro)?;
        self.set_strings(false, self.entry.int("StarterTextOffset"), &starter_text)
    }

    pub(crate) fn read_starter_held_items(&self) -> Result<Vec<i32>> {
        let static_cro = self.read_file(self.file("StaticPokemon"))?;
        let (count, size, offset) = self.gift_layout();
        Ok((0..count)
            .filter(|index| self.starter_indices().contains(&(*index as i32)))
            .map(|index| read_long(&static_cro, offset + index * size + 12).max(0))
            .collect())
    }

    pub(crate) fn write_starter_held_items(&self, items: &[i32]) -> Result<()> {
        let static_path = self.file("StaticPokemon");
        let mut static_cro = self.read_file(static_path)?;
        let (count, size, offset) = self.gift_layout();
        let mut iterator = items.iter();
        for index in 0..count {
            if !self.starter_indices().contains(&(index as i32)) {
                continue;
            }
            let item = *iterator.next().ok_or_else(no_such_element)?;
            let value = if item == 0 { -1 } else { item };
            write_long(&mut static_cro, offset + index * size + 12, value);
        }
        self.write_file(static_path, &static_cro)
    }

    pub(crate) fn read_static_pokemon(&self) -> Result<Vec<StaticEncounter>> {
        let static_cro = self.read_file(self.file("StaticPokemon"))?;
        let mut statics = Vec::new();
        let size = constants::STATIC_POKEMON_SIZE;
        let offset = self.entry.int("StaticPokemonOffset") as usize;
        for index in 0..constants::static_pokemon_count(self.rom_type) {
            let base = offset + index * size;
            let species = read_word(&static_cro, base);
            let forme = signed(&static_cro, base + 2);
            let mut encounter = StaticEncounter::new(self.pokemon_with_forme(species, forme));
            encounter.forme = forme;
            encounter.level = signed(&static_cro, base + 3);
            encounter.held_item = i32::from(read_word(&static_cro, base + 4) as i16).max(0);
            statics.push(encounter);
        }
        let (count, gift_size, gift_offset) = self.gift_layout();
        for index in 0..count {
            if self.starter_indices().contains(&(index as i32)) {
                continue;
            }
            let base = gift_offset + index * gift_size;
            let species = read_word(&static_cro, base);
            let forme = signed(&static_cro, base + 4);
            let mut encounter = StaticEncounter::new(self.pokemon_with_forme(species, forme));
            encounter.forme = forme;
            encounter.level = signed(&static_cro, base + 5);
            encounter.held_item = read_long(&static_cro, base + 12).max(0);
            if self.is_oras() && read_word(&static_cro, base + 18) == ORAS_EGG_MET_LOCATION {
                encounter.is_egg = true;
            }
            statics.push(encounter);
        }
        if self.is_xy() {
            self.read_trash_can_encounters(&mut statics);
        }
        self.consolidate_linked_encounters(&mut statics)?;
        Ok(statics)
    }

    fn read_trash_can_encounters(&self, statics: &mut Vec<StaticEncounter>) {
        let Some(table) =
            find_after_prefix(&self.code, constants::XY_TRASH_ENCOUNTERS_TABLE_PREFIX)
        else {
            return;
        };
        for (start, count, consolidate) in constants::XY_TRASH_CAN_SETS {
            statics.extend(self.read_trash_can_set(table, start, count, consolidate));
        }
    }

    fn read_trash_can_set(
        &self,
        table: usize,
        start: usize,
        count: usize,
        consolidate: bool,
    ) -> Vec<StaticEncounter> {
        let mut statics: Vec<StaticEncounter> = Vec::new();
        let mut by_species: HashMap<PokemonId, usize> = HashMap::new();
        let mut offset = table + start * constants::XY_TRASH_ENCOUNTER_DATA_LENGTH;
        for _ in 0..count {
            let encounter = self.read_trash_can_encounter(offset);
            match by_species.get(&encounter.pkmn) {
                Some(&main) if consolidate => statics[main].linked_encounters.push(encounter),
                _ => {
                    by_species.insert(encounter.pkmn, statics.len());
                    statics.push(encounter);
                }
            }
            offset += constants::XY_TRASH_ENCOUNTER_DATA_LENGTH;
        }
        statics
    }

    fn read_trash_can_encounter(&self, offset: usize) -> StaticEncounter {
        let species = read_long(&self.code, offset);
        let forme = read_long(&self.code, offset + 4);
        let level = read_long(&self.code, offset + 8);
        let mut encounter = StaticEncounter::new(self.pokemon_with_forme(species, forme));
        encounter.forme = forme;
        encounter.level = level;
        encounter
    }

    fn consolidate_linked_encounters(&self, statics: &mut Vec<StaticEncounter>) -> Result<()> {
        let mut removed = Vec::new();
        for (&base, &linked) in &self.entry.linked_static_offsets {
            let linked_encounter = statics
                .get(linked as usize)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("statics"))?;
            statics
                .get_mut(base as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("statics"))?
                .linked_encounters
                .push(linked_encounter);
            removed.push(linked as usize);
        }
        removed.sort_unstable();
        for index in removed.into_iter().rev() {
            statics.remove(index);
        }
        Ok(())
    }

    fn unlink_static_encounters(&self, statics: &mut Vec<StaticEncounter>) -> Result<()> {
        let mut insert_at: Vec<i32> = self.entry.linked_static_offsets.values().copied().collect();
        insert_at.sort_unstable();
        for index in insert_at {
            if index as usize > statics.len() {
                return Err(RandomizationError::IndexOutOfBounds("statics.add"));
            }
            statics.insert(index as usize, StaticEncounter::default());
        }
        for (&base, &linked) in &self.entry.linked_static_offsets {
            let first_linked = statics
                .get(base as usize)
                .and_then(|encounter| encounter.linked_encounters.first())
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("linkedEncounters"))?;
            statics[linked as usize] = first_linked;
        }
        Ok(())
    }

    pub(crate) fn write_static_pokemon(
        &mut self,
        static_pokemon: &[StaticEncounter],
    ) -> Result<()> {
        let static_path = self.file("StaticPokemon");
        let mut static_cro = self.read_file(static_path)?;
        let mut statics = static_pokemon.to_vec();
        self.unlink_static_encounters(&mut statics)?;
        let mut iterator = statics.iter();
        let size = constants::STATIC_POKEMON_SIZE;
        let offset = self.entry.int("StaticPokemonOffset") as usize;
        for index in 0..constants::static_pokemon_count(self.rom_type) {
            let encounter = iterator.next().ok_or_else(no_such_element)?;
            let base = offset + index * size;
            write_word(&mut static_cro, base, encounter.pkmn.number());
            static_cro[base + 2] = encounter.forme as u8;
            static_cro[base + 3] = encounter.level as u8;
            let item = if encounter.held_item == 0 {
                -1
            } else {
                encounter.held_item
            };
            write_word(&mut static_cro, base + 4, item);
        }
        let (count, gift_size, gift_offset) = self.gift_layout();
        for index in 0..count {
            if self.starter_indices().contains(&(index as i32)) {
                continue;
            }
            let encounter = iterator.next().ok_or_else(no_such_element)?;
            let base = gift_offset + index * gift_size;
            write_word(&mut static_cro, base, encounter.pkmn.number());
            static_cro[base + 4] = encounter.forme as u8;
            static_cro[base + 5] = encounter.level as u8;
            let item = if encounter.held_item == 0 {
                -1
            } else {
                encounter.held_item
            };
            write_long(&mut static_cro, base + 12, item);
        }
        self.write_file(static_path, &static_cro)?;
        if self.is_xy() {
            self.write_trash_can_encounters(&mut iterator)?;
            let box_offsets = self.entry.array("BoxLegendaryOffsets");
            let box_index = *box_offsets
                .first()
                .ok_or(RandomizationError::IndexOutOfBounds("BoxLegendaryOffsets"))?;
            let box_species = statics
                .get(box_index as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("staticPokemon"))?
                .pkmn
                .number();
            self.fix_box_legendaries_xy(box_species)?;
            self.set_roamers_xy(&statics)?;
        } else {
            let rayquaza_index = self.entry.int("RayquazaEncounterNumber") as usize;
            let rayquaza = statics
                .get(rayquaza_index)
                .ok_or(RandomizationError::IndexOutOfBounds("staticPokemon"))?
                .pkmn
                .number();
            self.fix_rayquaza_oras(rayquaza)?;
        }
        Ok(())
    }

    fn write_trash_can_entry(&mut self, offset: usize, encounter: &StaticEncounter) {
        let base = self.core.data.base_number(encounter.pkmn);
        write_long(&mut self.code, offset, base);
        write_long(&mut self.code, offset + 4, encounter.forme);
        write_long(&mut self.code, offset + 8, encounter.level);
    }

    fn write_trash_can_encounters<'a>(
        &mut self,
        iterator: &mut impl Iterator<Item = &'a StaticEncounter>,
    ) -> Result<()> {
        let Some(mut offset) =
            find_after_prefix(&self.code, constants::XY_TRASH_ENCOUNTERS_TABLE_PREFIX)
        else {
            return Ok(());
        };
        let mut current = 0;
        while current != constants::XY_TRASH_CAN_ENCOUNTER_COUNT {
            let encounter = iterator.next().ok_or_else(no_such_element)?;
            self.write_trash_can_entry(offset, encounter);
            offset += constants::XY_TRASH_ENCOUNTER_DATA_LENGTH;
            current += 1;
            for linked in &encounter.linked_encounters {
                self.write_trash_can_entry(offset, linked);
                offset += constants::XY_TRASH_ENCOUNTER_DATA_LENGTH;
                current += 1;
            }
        }
        Ok(())
    }

    fn fix_box_legendaries_xy(&mut self, species: i32) -> Result<()> {
        let wild_path = self.file("WildPokemon");
        let mut encounter_garc = self.read_garc(wild_path, Decompression::All)?;
        let file_index = constants::BOX_LEGENDARY_ENCOUNTER_FILE_XY;
        let mut room_data = garc_file(&encounter_garc, file_index)?.to_vec();
        let mut script = Amx::find_script(&room_data, 1)
            .map_err(io)?
            .ok_or_else(missing_script)?;
        for &script_offset in self.entry.array("BoxLegendaryScriptOffsets") {
            write_word(&mut script.dec_data, script_offset as usize, species);
        }
        let modified = script.to_bytes().map_err(io)?;
        let start = constants::BOX_LEGENDARY_LOCAL_SCRIPT_OFFSET_XY;
        room_data
            .get_mut(start..start + modified.len())
            .ok_or(RandomizationError::IndexOutOfBounds("boxLegendaryRoomData"))?
            .copy_from_slice(&modified);
        encounter_garc.set_file(file_index, room_data).map_err(io)?;
        self.write_garc(wild_path, &encounter_garc)?;
        let static_path = self.file("StaticPokemon");
        let mut static_cro = self.read_file(static_path)?;
        if let Some(function) =
            find_after_prefix(&static_cro, constants::BOX_LEGENDARY_FUNCTION_PREFIX_XY)
        {
            for code_offset in constants::BOX_LEGENDARY_CODE_OFFSETS_XY {
                write_species_comparison(&mut static_cro, function + code_offset, species);
            }
        }
        self.write_file(static_path, &static_cro)
    }

    fn set_roamers_xy(&mut self, statics: &[StaticEncounter]) -> Result<()> {
        let roamers: Vec<StaticEncounter> = self
            .entry
            .array("RoamingLegendaryOffsets")
            .iter()
            .map(|&offset| {
                statics
                    .get(offset as usize)
                    .cloned()
                    .ok_or(RandomizationError::IndexOutOfBounds("staticPokemon"))
            })
            .collect::<Result<_>>()?;
        self.patch_roamer_species(&roamers);
        let field_path = self.file("Field");
        let mut field_cro = self.read_file(field_path)?;
        if let Some(level_offset) = find_after_prefix(&field_cro, constants::XY_ROAMER_LEVEL_PREFIX)
        {
            field_cro[level_offset] = roamers
                .first()
                .ok_or(RandomizationError::IndexOutOfBounds("roamers"))?
                .level as u8;
        }
        self.write_file(field_path, &field_cro)?;
        self.patch_sea_spirits_den(&roamers)
    }

    fn patch_roamer_species(&mut self, roamers: &[StaticEncounter]) {
        let species_offset = find(&self.code, constants::XY_ROAMER_SPECIES_LOCATOR);
        let free_space = find(&self.code, constants::XY_ROAMER_FREE_SPACE_POSTFIX);
        if species_offset <= 0 || free_space <= 0 {
            return;
        }
        let free_space = (free_space - 20) as usize;
        let species_offset = species_offset as usize;
        let code = &mut self.code;
        code[free_space] = code[free_space + 4].wrapping_add(1);
        code[free_space + 1] = code[free_space + 5];
        code[free_space + 2] = code[free_space + 6];
        code[free_space + 3] = code[free_space + 7];
        code[free_space + 4] = code[free_space + 16].wrapping_add(3);
        code[free_space + 5] = code[free_space + 17];
        code[free_space + 6] = code[free_space + 18];
        code[free_space + 7] = code[free_space + 19];
        for (index, roamer) in roamers.iter().enumerate() {
            let base = self.core.data.base_number(roamer.pkmn);
            write_long(&mut self.code, free_space + 8 + index * 4, base);
        }
        for index in 0..roamers.len() {
            let offset = species_offset + index * 12;
            self.code[offset..offset + 4].copy_from_slice(&[
                (0xAC - 8 * index as i32) as u8,
                0x41,
                0x9F,
                0xE5,
            ]);
        }
    }

    fn patch_sea_spirits_den(&self, roamers: &[StaticEncounter]) -> Result<()> {
        let wild_path = self.file("WildPokemon");
        let mut encounter_garc = self.read_garc(wild_path, Decompression::All)?;
        let file_index = constants::SEA_SPIRITS_DEN_ENCOUNTER_FILE_XY;
        let mut area_file = garc_file(&encounter_garc, file_index)?.to_vec();
        let mut script = Amx::find_script(&area_file, 1)
            .map_err(io)?
            .ok_or_else(missing_script)?;
        for (index, roamer) in roamers.iter().enumerate() {
            let offset = constants::SEA_SPIRITS_DEN_SCRIPT_OFFSETS_XY[index];
            write_word(
                &mut script.dec_data,
                offset,
                self.core.data.base_number(roamer.pkmn),
            );
        }
        let modified = script.to_bytes().map_err(io)?;
        let start = constants::SEA_SPIRITS_DEN_LOCAL_SCRIPT_OFFSET_XY;
        area_file
            .get_mut(start..start + modified.len())
            .ok_or(RandomizationError::IndexOutOfBounds(
                "seaSpiritsDenAreaFile",
            ))?
            .copy_from_slice(&modified);
        encounter_garc.set_file(file_index, area_file).map_err(io)?;
        self.write_garc(wild_path, &encounter_garc)
    }

    fn fix_rayquaza_oras(&self, species: i32) -> Result<()> {
        let script_file = self.entry.int("RayquazaEncounterScriptNumber") as usize;
        let scripts_path = self.file("Scripts");
        let mut script_garc = self.read_garc(scripts_path, Decompression::Skip)?;
        let mut script = Amx::parse(garc_file(&script_garc, script_file)?).map_err(io)?;
        for offset in constants::RAYQUAZA_SCRIPT_OFFSETS_ORAS {
            write_word(&mut script.dec_data, offset, species);
        }
        let encoded = script.to_bytes().map_err(io)?;
        *garc_file_mut(&mut script_garc, script_file)? = encoded;
        self.write_garc(scripts_path, &script_garc)?;
        let static_path = self.file("StaticPokemon");
        let mut static_cro = self.read_file(static_path)?;
        if let Some(function) =
            find_after_prefix(&static_cro, constants::RAYQUAZA_FUNCTION_PREFIX_ORAS)
        {
            for code_offset in constants::RAYQUAZA_CODE_OFFSETS_ORAS {
                write_species_comparison(&mut static_cro, function + code_offset, species);
            }
        }
        self.write_file(static_path, &static_cro)
    }
}
