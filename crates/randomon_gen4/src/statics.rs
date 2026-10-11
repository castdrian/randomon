use std::collections::BTreeMap;

use randomon_core::engine::util::random_index;
use randomon_core::{RandomizationError, Result};
use randomon_data::constants::{gen4, species};
use randomon_data::{PokemonId, StaticEncounter};
use randomon_formats::nds::NarcArchive;

use crate::bytes::{
    after_prefix, file, file_mut, i8_at, i32_at, set_bytes, set_i32, set_u8, set_u16, u8_at, u16_at,
};
use crate::handler::Gen4RomHandler;
use crate::rom_entry::{RoamingPokemonEntry, ScriptEntry, StaticPokemonEntry};

const FEMALE_ONLY_GENDER_RATIO: i32 = 0xFE;
const DISTORTION_WORLD_GROUND_TYPE: i32 = 23;
const MAX_MUSIC_TABLE_SCAN: usize = 0x1000;

fn script_file<'a>(narc: &'a NarcArchive, entry: &ScriptEntry) -> Result<&'a Vec<u8>> {
    file(narc, entry.file)
}

fn script_file_mut<'a>(narc: &'a mut NarcArchive, entry: &ScriptEntry) -> Result<&'a mut Vec<u8>> {
    file_mut(narc, entry.file)
}

fn static_species(narc: &NarcArchive, entry: &StaticPokemonEntry) -> Result<i32> {
    let first = entry
        .species_entries
        .first()
        .ok_or(RandomizationError::IndexOutOfBounds("speciesEntries"))?;
    u16_at(script_file(narc, first)?, first.offset)
}

fn static_forme(narc: &NarcArchive, entry: &StaticPokemonEntry) -> Result<i32> {
    match entry.forme_entries.first() {
        None => Ok(0),
        Some(first) => i8_at(script_file(narc, first)?, first.offset),
    }
}

fn static_level(narc: &NarcArchive, entry: &StaticPokemonEntry, index: usize) -> Result<i32> {
    match entry.level_entries.get(index) {
        None => Ok(1),
        Some(level) => i8_at(script_file(narc, level)?, level.offset),
    }
}

fn write_script_words(narc: &mut NarcArchive, entries: &[ScriptEntry], value: i32) -> Result<()> {
    for entry in entries {
        set_u16(script_file_mut(narc, entry)?, entry.offset, value)?;
    }
    Ok(())
}

fn write_script_bytes(narc: &mut NarcArchive, entries: &[ScriptEntry], value: i32) -> Result<()> {
    for entry in entries {
        set_u8(script_file_mut(narc, entry)?, entry.offset, value)?;
    }
    Ok(())
}

impl Gen4RomHandler {
    fn trade_static_count(&self) -> usize {
        self.entry.array_or_empty("StaticPokemonTrades").len()
    }

    fn has_mystery_egg(&self) -> bool {
        self.entry.int("MysteryEggOffset") > 0
    }

    fn has_fossils(&self) -> bool {
        self.entry.int("FossilTableOffset") > 0
    }

    fn roamer_count(&self) -> usize {
        if self.roamer_randomization_enabled {
            self.entry.roaming_pokemon.len()
        } else {
            0
        }
    }

    fn expected_static_count(&self) -> usize {
        self.entry.static_pokemon.len()
            + self.trade_static_count()
            + usize::from(self.has_mystery_egg())
            + if self.has_fossils() {
                gen4::FOSSIL_COUNT as usize
            } else {
                0
            }
            + self.roamer_count()
    }

    fn script_statics(&self, statics: &mut Vec<StaticEncounter>) -> Result<()> {
        let egg_offsets = self.entry.array_or_empty("StaticEggPokemonOffsets");
        for (index, entry) in self.entry.static_pokemon.iter().enumerate() {
            let base = self.pokemon_by_number(static_species(&self.script_narc, entry)?)?;
            let pokemon = self.alt_forme_of(base, static_forme(&self.script_narc, entry)?);
            let mut encounter = StaticEncounter::new(pokemon);
            encounter.level = static_level(&self.script_narc, entry, 0)?;
            encounter.is_egg = egg_offsets.contains(&(index as i32));
            for level_entry in 1..entry.level_entries.len() {
                let mut linked = StaticEncounter::new(pokemon);
                linked.level = static_level(&self.script_narc, entry, level_entry)?;
                encounter.linked_encounters.push(linked);
            }
            statics.push(encounter);
        }
        Ok(())
    }

    fn trade_statics(&self, statics: &mut Vec<StaticEncounter>) -> Result<()> {
        let trades = self.entry.array_or_empty("StaticPokemonTrades");
        if trades.is_empty() {
            return Ok(());
        }
        let trade_narc = self.read_narc("InGameTrades")?;
        let scripts = self.entry.array_or_empty("StaticPokemonTradeScripts");
        let level_offsets = self.entry.array_or_empty("StaticPokemonTradeLevelOffsets");
        for (index, &trade) in trades.iter().enumerate() {
            let script = *scripts
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds("trade scripts"))?;
            let level_offset = *level_offsets
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds("trade level offsets"))?;
            let level = i8_at(file(&self.script_narc, script)?, level_offset)?;
            let species = i32_at(file(&trade_narc, trade)?, 0)?;
            let mut encounter = StaticEncounter::new(self.pokemon_by_number(species)?);
            encounter.level = level;
            statics.push(encounter);
        }
        Ok(())
    }

    fn mystery_egg_static(&self, statics: &mut Vec<StaticEncounter>) -> Result<()> {
        if !self.has_mystery_egg() {
            return Ok(());
        }
        let overlay = self.read_overlay(self.entry.int("FieldOvlNumber"))?;
        let species = u8_at(&overlay, self.entry.int("MysteryEggOffset"))?;
        let mut encounter = StaticEncounter::new(self.pokemon_by_number(species)?);
        encounter.is_egg = true;
        statics.push(encounter);
        Ok(())
    }

    fn fossil_table(&self) -> Result<Vec<u8>> {
        if self.entry.is_hgss() {
            self.read_overlay(self.entry.int("FossilTableOvlNumber"))
        } else {
            Ok(self.arm9.clone())
        }
    }

    fn fossil_statics(&self, statics: &mut Vec<StaticEncounter>) -> Result<()> {
        if !self.has_fossils() {
            return Ok(());
        }
        let base = self.entry.int("FossilTableOffset");
        let level_script = file(&self.script_narc, self.entry.int("FossilLevelScriptNumber"))?;
        let level = i8_at(level_script, self.entry.int("FossilLevelOffset"))?;
        let table = self.fossil_table()?;
        for fossil in 0..gen4::FOSSIL_COUNT {
            let species = u16_at(&table, base + 2 + fossil * 4)?;
            let mut encounter = StaticEncounter::new(self.pokemon_by_number(species)?);
            encounter.level = level;
            statics.push(encounter);
        }
        Ok(())
    }

    fn roamer_static(&self, roamer: &RoamingPokemonEntry) -> Result<StaticEncounter> {
        let species_offset = *roamer
            .species_code_offsets
            .first()
            .ok_or(RandomizationError::IndexOutOfBounds("speciesCodeOffsets"))?;
        let mut encounter =
            StaticEncounter::new(self.pokemon_by_number(u16_at(&self.arm9, species_offset)?)?);
        encounter.level = match roamer.level_code_offsets.first() {
            Some(&offset) => i8_at(&self.arm9, offset)?,
            None => 1,
        };
        Ok(encounter)
    }

    pub(crate) fn static_pokemon(&self) -> Result<Vec<StaticEncounter>> {
        let mut statics = Vec::new();
        if !self.entry.static_pokemon_support {
            return Ok(statics);
        }
        self.script_statics(&mut statics)?;
        self.trade_statics(&mut statics)?;
        self.mystery_egg_static(&mut statics)?;
        self.fossil_statics(&mut statics)?;
        if self.roamer_randomization_enabled {
            for roamer in &self.entry.roaming_pokemon {
                statics.push(self.roamer_static(roamer)?);
            }
        }
        Ok(statics)
    }

    pub(crate) fn prepare_roamer_code(&mut self) {
        let prepared = if self.entry.is_dp() {
            self.prepare_diamond_pearl_roamers()
        } else {
            self.prepare_roamer_subroutine()
        };
        if let Err(error) = prepared {
            log::warn!("roamer randomization disabled: {error}");
            self.roamer_randomization_enabled = false;
        }
    }

    fn prepare_diamond_pearl_roamers(&mut self) -> Result<()> {
        let offset = self.entry.int("RoamingPokemonFunctionStartOffset");
        if u16_at(&self.arm9, offset + 44)? == 0 {
            return Ok(());
        }
        let mut arm9 = self.arm9.clone();
        set_i32(&mut arm9, offset + 244, species::CRESSELIA)?;
        set_bytes(&mut arm9, offset + 42, &[0x32, 0x4F, 0x00, 0x00])?;
        self.arm9 = arm9;
        Ok(())
    }

    fn prepare_roamer_subroutine(&mut self) -> Result<()> {
        let first_species = self
            .entry
            .roaming_pokemon
            .first()
            .and_then(|roamer| roamer.species_code_offsets.first().copied())
            .ok_or(RandomizationError::IndexOutOfBounds("roamingPokemon"))?;
        let already_patched = u16_at(&self.arm9, first_species).is_ok_and(|species| species != 0);
        if already_patched {
            return Ok(());
        }
        self.extend_arm9()?;
        if !self.patch_arm9("NewRoamerSubroutineTweak")? {
            return Err(RandomizationError::randomization("roamer patch missing"));
        }
        Ok(())
    }

    pub(crate) fn set_static_pokemon_impl(&mut self, statics: &[StaticEncounter]) -> Result<bool> {
        if !self.entry.static_pokemon_support || statics.len() != self.expected_static_count() {
            return Ok(false);
        }
        let mut statics = statics.iter();
        let mut next = || {
            statics
                .next()
                .ok_or(RandomizationError::IndexOutOfBounds("staticPokemon"))
        };
        let script_statics: Vec<&StaticEncounter> = (0..self.entry.static_pokemon.len())
            .map(|_| next())
            .collect::<Result<_>>()?;
        let trade_statics: Vec<&StaticEncounter> = (0..self.trade_static_count())
            .map(|_| next())
            .collect::<Result<_>>()?;
        let mystery_egg = if self.has_mystery_egg() {
            Some(next()?)
        } else {
            None
        };
        let fossils: Vec<&StaticEncounter> = if self.has_fossils() {
            (0..gen4::FOSSIL_COUNT)
                .map(|_| next())
                .collect::<Result<_>>()?
        } else {
            Vec::new()
        };
        let roamers: Vec<&StaticEncounter> = (0..self.roamer_count())
            .map(|_| next())
            .collect::<Result<_>>()?;
        self.write_script_statics(&script_statics)?;
        self.write_trade_statics(&trade_statics)?;
        if let Some(egg) = mystery_egg {
            self.write_mystery_egg(egg)?;
        }
        self.write_fossils(&fossils)?;
        self.write_roamers(&roamers)?;
        if self.entry.is_plat() {
            self.patch_distortion_world_ground_check()?;
        }
        Ok(true)
    }

    fn write_script_statics(&mut self, statics: &[&StaticEncounter]) -> Result<()> {
        for (entry, encounter) in self.entry.static_pokemon.iter().zip(statics) {
            let number = self.number_of(encounter.pkmn);
            let forme = self.core.data.pk(encounter.pkmn).forme_number;
            write_script_words(&mut self.script_narc, &entry.species_entries, number)?;
            write_script_bytes(&mut self.script_narc, &entry.forme_entries, forme)?;
            if let Some(level) = entry.level_entries.first() {
                set_u8(
                    script_file_mut(&mut self.script_narc, level)?,
                    level.offset,
                    encounter.level,
                )?;
            }
            for (index, linked) in encounter.linked_encounters.iter().enumerate() {
                if let Some(level) = entry.level_entries.get(index + 1) {
                    set_u8(
                        script_file_mut(&mut self.script_narc, level)?,
                        level.offset,
                        linked.level,
                    )?;
                }
            }
            if let Some(texts) = &entry.game_corner_texts {
                let name = self.core.data.pk(encounter.pkmn).name.to_uppercase();
                for text in texts {
                    let mut strings = self.get_strings(text.text_index)?;
                    if let Some(original) = usize::try_from(text.string_number)
                        .ok()
                        .and_then(|index| strings.get_mut(index))
                        && let Some(post_name) = original.find('\\')
                    {
                        *original = format!("{name}{}", &original[post_name..]);
                    }
                    self.set_strings(text.text_index, &strings)?;
                }
            }
        }
        Ok(())
    }

    fn possible_abilities(&self, pokemon: PokemonId) -> Vec<i32> {
        let pk = self.core.data.pk(pokemon);
        let mut abilities = vec![pk.ability1];
        if pk.ability2 > 0 {
            abilities.push(pk.ability2);
        }
        if pk.ability3 > 0 {
            abilities.push(pk.ability3);
        }
        abilities
    }

    fn write_trade_statics(&mut self, statics: &[&StaticEncounter]) -> Result<()> {
        if statics.is_empty() {
            return Ok(());
        }
        let mut trade_narc = self.read_narc("InGameTrades")?;
        let trades = self.entry.array_or_empty("StaticPokemonTrades");
        let scripts = self.entry.array_or_empty("StaticPokemonTradeScripts");
        let level_offsets = self.entry.array_or_empty("StaticPokemonTradeLevelOffsets");
        for (index, encounter) in statics.iter().enumerate() {
            let abilities = self.possible_abilities(encounter.pkmn);
            let ability = abilities[random_index(&mut self.core.engine.random, abilities.len())?];
            let trade_file = file_mut(&mut trade_narc, trades[index])?;
            set_i32(trade_file, 0, self.number_of(encounter.pkmn))?;
            set_i32(trade_file, 0x1C, ability)?;
            if let (Some(&script), Some(&offset)) = (scripts.get(index), level_offsets.get(index)) {
                set_u8(
                    file_mut(&mut self.script_narc, script)?,
                    offset,
                    encounter.level,
                )?;
            }
            if index == 1 {
                let original = self
                    .core
                    .data
                    .pk(self.pk(species::SPEAROW))
                    .name
                    .to_uppercase();
                let replacement = self.core.data.pk(encounter.pkmn).name.clone();
                let replacements = BTreeMap::from([(original, replacement)]);
                self.replace_all_strings_in_entry(
                    self.entry.int("KenyaTextOffset"),
                    &replacements,
                )?;
            }
        }
        self.write_narc("InGameTrades", &trade_narc)
    }

    fn write_mystery_egg(&mut self, egg: &StaticEncounter) -> Result<()> {
        let mut number = self.number_of(egg.pkmn);
        if number > 255 {
            number = random_index(&mut self.core.engine.random, 255)? as i32 + 1;
        }
        let overlay_number = self.entry.int("FieldOvlNumber");
        let mut overlay = self.read_overlay(overlay_number)?;
        set_u8(&mut overlay, self.entry.int("MysteryEggOffset"), number)?;
        self.write_overlay(overlay_number, &overlay)
    }

    fn write_fossils(&mut self, fossils: &[&StaticEncounter]) -> Result<()> {
        if fossils.is_empty() {
            return Ok(());
        }
        let base = self.entry.int("FossilTableOffset");
        let level_script = self.entry.int("FossilLevelScriptNumber");
        let level_offset = self.entry.int("FossilLevelOffset");
        let mut table = self.fossil_table()?;
        for (index, fossil) in fossils.iter().enumerate() {
            set_u16(
                &mut table,
                base + 2 + index as i32 * 4,
                self.number_of(fossil.pkmn),
            )?;
            set_u8(
                file_mut(&mut self.script_narc, level_script)?,
                level_offset,
                fossil.level,
            )?;
        }
        if self.entry.is_hgss() {
            self.write_overlay(self.entry.int("FossilTableOvlNumber"), &table)
        } else {
            self.arm9 = table;
            Ok(())
        }
    }

    fn write_roamers(&mut self, roamers: &[&StaticEncounter]) -> Result<()> {
        for (roamer, encounter) in self.entry.roaming_pokemon.iter().zip(roamers) {
            let number = self.number_of(encounter.pkmn);
            for &offset in &roamer.species_code_offsets {
                set_u16(&mut self.arm9, offset, number)?;
            }
            write_script_words(
                &mut self.script_narc,
                &roamer.species_script_offsets,
                number,
            )?;
            let gender = i32::from(
                self.core.data.pk(encounter.pkmn).gender_ratio == FEMALE_ONLY_GENDER_RATIO,
            );
            write_script_words(&mut self.script_narc, &roamer.gender_offsets, gender)?;
            for &offset in &roamer.level_code_offsets {
                set_u8(&mut self.arm9, offset, encounter.level)?;
            }
        }
        Ok(())
    }

    fn patch_distortion_world_ground_check(&mut self) -> Result<()> {
        let overlay_number = self.entry.int("FieldOvlNumber");
        let mut overlay = self.read_overlay(overlay_number)?;
        if let Some(offset) = after_prefix(&overlay, gen4::DISTORTION_WORLD_GROUND_CHECK_PREFIX) {
            set_u8(
                &mut overlay,
                offset + 2 * DISTORTION_WORLD_GROUND_TYPE,
                0x30,
            )?;
            self.write_overlay(overlay_number, &overlay)?;
        }
        Ok(())
    }

    fn find_music_slot(
        &self,
        start: i32,
        stride: i32,
        mask: i32,
        wanted: i32,
        replaced: &[i32],
    ) -> Option<i32> {
        (0..MAX_MUSIC_TABLE_SCAN)
            .map(|step| start + step as i32 * stride)
            .take_while(|&offset| u16_at(&self.arm9, offset).is_ok())
            .find(|&offset| {
                u16_at(&self.arm9, offset).is_ok_and(|value| value & mask == wanted)
                    && !replaced.contains(&offset)
            })
    }

    pub(crate) fn apply_correct_static_music_impl(
        &mut self,
        changes: &BTreeMap<i32, i32>,
    ) -> Result<()> {
        let mut replaced = Vec::new();
        match self.rom_type() {
            gen4::TYPE_DP | gen4::TYPE_PLAT => {
                self.extend_arm9()?;
                self.patch_arm9("NewIndexToMusicTweak")?;
                let prefix = self.entry.string("NewIndexToMusicPrefix");
                let Some(start) = after_prefix(&self.arm9, prefix) else {
                    return Ok(());
                };
                for (&old, &new) in changes {
                    if let Some(offset) = self.find_music_slot(start, 4, 0xFFFF, old, &replaced) {
                        set_u16(&mut self.arm9, offset, new)?;
                        replaced.push(offset);
                    }
                }
            }
            _ => {
                let prefix = self.entry.string("IndexToMusicPrefix");
                let Some(start) = after_prefix(&self.arm9, prefix) else {
                    return Ok(());
                };
                for (&old, &new) in changes {
                    if let Some(offset) = self.find_music_slot(start, 2, 0x3FF, old, &replaced) {
                        let existing = u16_at(&self.arm9, offset)?;
                        set_u16(&mut self.arm9, offset, new | (existing & 0xFC00))?;
                        replaced.push(offset);
                    }
                }
            }
        }
        Ok(())
    }
}
