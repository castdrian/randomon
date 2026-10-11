use std::collections::BTreeMap;

use randomon_core::{RandomizationError, Result, RomHandler};
use randomon_data::constants::{gen5, species};
use randomon_data::{PokemonId, StaticEncounter};
use randomon_formats::nds::NarcArchive;
use randomon_java::{JavaHashMap, JavaRng};
use randomon_nds::{find, read_word, signed_byte, write_full_int, write_word};

use crate::handler::Gen5RomHandler;
use crate::rom_entry::{FileEntry, RoamingPokemonEntry, StaticPokemonEntry};

const HIDDEN_HOLLOW_VERSION_STRIDE: i32 = 78;
const HIDDEN_HOLLOW_RARITY_STRIDE: i32 = 26;
const HIDDEN_HOLLOW_VERSIONS: i32 = 2;
const HIDDEN_HOLLOW_RARITIES: i32 = 3;
const HIDDEN_HOLLOW_GROUPS: i32 = 4;
const LAST_HIDDEN_HOLLOW_NATIONAL_NUMBER: i32 = 493;
const KYUREM_MUSIC_PATCH_WORD: i32 = 0x4290;

fn entry_file<'a>(narc: &'a NarcArchive, entry: &FileEntry) -> Result<&'a Vec<u8>> {
    narc.files
        .get(entry.entry as usize)
        .ok_or(RandomizationError::IndexOutOfBounds("NARC.files"))
}

fn entry_file_mut<'a>(narc: &'a mut NarcArchive, entry: &FileEntry) -> Result<&'a mut Vec<u8>> {
    narc.files
        .get_mut(entry.entry as usize)
        .ok_or(RandomizationError::IndexOutOfBounds("NARC.files"))
}

fn static_species(narc: &NarcArchive, entry: &StaticPokemonEntry) -> Result<i32> {
    let first = entry
        .species_entries
        .first()
        .ok_or(RandomizationError::IndexOutOfBounds("speciesEntries"))?;
    Ok(read_word(entry_file(narc, first)?, first.offset))
}

fn static_forme(narc: &NarcArchive, entry: &StaticPokemonEntry) -> Result<i32> {
    match entry.forme_entries.first() {
        None => Ok(0),
        Some(first) => Ok(signed_byte(entry_file(narc, first)?, first.offset)),
    }
}

fn static_level(narc: &NarcArchive, entry: &StaticPokemonEntry, index: usize) -> Result<i32> {
    match entry.level_entries.get(index) {
        None => Ok(1),
        Some(level) => Ok(signed_byte(entry_file(narc, level)?, level.offset)),
    }
}

fn set_static_species(
    narc: &mut NarcArchive,
    entry: &StaticPokemonEntry,
    number: i32,
) -> Result<()> {
    for species_entry in &entry.species_entries {
        write_word(
            entry_file_mut(narc, species_entry)?,
            species_entry.offset,
            number,
        );
    }
    Ok(())
}

fn set_static_forme(narc: &mut NarcArchive, entry: &StaticPokemonEntry, forme: i32) -> Result<()> {
    for forme_entry in &entry.forme_entries {
        entry_file_mut(narc, forme_entry)?[forme_entry.offset as usize] = forme as u8;
    }
    Ok(())
}

fn set_static_level(
    narc: &mut NarcArchive,
    entry: &StaticPokemonEntry,
    level: i32,
    index: usize,
) -> Result<()> {
    if let Some(level_entry) = entry.level_entries.get(index) {
        entry_file_mut(narc, level_entry)?[level_entry.offset as usize] = level as u8;
    }
    Ok(())
}

fn hidden_hollow_base(version: i32, rarity: i32) -> i32 {
    version * HIDDEN_HOLLOW_VERSION_STRIDE + rarity * HIDDEN_HOLLOW_RARITY_STRIDE
}

fn first_offset(offsets: &[i32]) -> Result<i32> {
    offsets
        .first()
        .copied()
        .ok_or(RandomizationError::IndexOutOfBounds("roamer offsets"))
}

impl Gen5RomHandler {
    pub(crate) fn alt_forme_of(&self, pokemon: PokemonId, forme: i32) -> PokemonId {
        let number = self.core.data.pk(pokemon).number;
        match gen5::absolute_poke_num_by_base_forme(number, forme) {
            0 => pokemon,
            with_forme => self.pk(with_forme),
        }
    }

    fn static_with_linked_levels(
        &self,
        pokemon: PokemonId,
        narc: &NarcArchive,
        entry: &StaticPokemonEntry,
    ) -> Result<StaticEncounter> {
        let mut encounter = StaticEncounter::new(pokemon);
        encounter.level = static_level(narc, entry, 0)?;
        for index in 1..entry.level_entries.len() {
            let mut linked = StaticEncounter::new(pokemon);
            linked.level = static_level(narc, entry, index)?;
            encounter.linked_encounters.push(linked);
        }
        Ok(encounter)
    }

    fn hidden_hollow_pool(&self) -> Vec<PokemonId> {
        (1..=LAST_HIDDEN_HOLLOW_NATIONAL_NUMBER)
            .chain(gen5::BW2_HIDDEN_HOLLOW_UNOVA_POKEMON.iter().copied())
            .map(|number| self.pk(number))
            .collect()
    }

    fn hidden_hollow_statics(&self, statics: &mut Vec<StaticEncounter>) -> Result<()> {
        let allowed = self.hidden_hollow_pool();
        let narc = self.read_narc("HiddenHollows")?;
        let wanted_version = self.entry.int("HiddenHollowIndex");
        for hollow in &narc.files {
            for version in 0..HIDDEN_HOLLOW_VERSIONS {
                if version != wanted_version {
                    continue;
                }
                for rarity in 0..HIDDEN_HOLLOW_RARITIES {
                    let base = hidden_hollow_base(version, rarity);
                    let mut group_indexes: Vec<usize> = Vec::new();
                    for group in 0..HIDDEN_HOLLOW_GROUPS {
                        let pokemon = self.pk(read_word(hollow, base + group * 2));
                        let pokemon =
                            self.alt_forme_of(pokemon, signed_byte(hollow, base + 20 + group));
                        let mut encounter = StaticEncounter::new(pokemon);
                        encounter.level = signed_byte(hollow, base + 12 + group);
                        encounter.max_level = signed_byte(hollow, base + 8 + group);
                        encounter.restricted_pool = true;
                        encounter.restricted_list = allowed.clone();
                        match group_indexes
                            .iter()
                            .find(|&&index| statics[index].pkmn == encounter.pkmn)
                        {
                            Some(&index) => statics[index].linked_encounters.push(encounter),
                            None => {
                                group_indexes.push(statics.len());
                                statics.push(encounter);
                                if !self.hidden_hollow_counted.get() {
                                    self.hidden_hollow_count
                                        .set(self.hidden_hollow_count.get() + 1);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn roamer_species(&self, roamer: &RoamingPokemonEntry) -> Result<PokemonId> {
        let overlay = self.read_overlay(self.entry.int("RoamerOvlNumber"))?;
        Ok(self.pk(read_word(
            &overlay,
            first_offset(&roamer.species_overlay_offsets)?,
        )))
    }

    fn roamer_level(&self, roamer: &RoamingPokemonEntry) -> Result<i32> {
        match roamer.level_overlay_offsets.first() {
            None => Ok(1),
            Some(&offset) => {
                let overlay = self.read_overlay(self.entry.int("RoamerOvlNumber"))?;
                Ok(signed_byte(&overlay, offset))
            }
        }
    }

    fn apply_black_white_roamer_patch(&self) -> Result<()> {
        let offset = self.entry.int("GetRoamerFlagOffsetStartOffset");
        let number = self.entry.int("RoamerOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        overlay[(offset + 10) as usize] = 0x00;
        write_full_int(&mut overlay, offset + 20, species::THUNDURUS);
        overlay[(offset + 6) as usize] = 0x03;
        overlay[(offset + 7) as usize] = 0x49;
        self.write_overlay(number, &overlay)
    }

    fn roaming_statics(&self, statics: &mut Vec<StaticEncounter>) -> Result<()> {
        let Some(first) = self.entry.roaming_pokemon.first() else {
            return Ok(());
        };
        let first_species_offset = first_offset(&first.species_overlay_offsets)?;
        let overlay = self.read_overlay(self.entry.int("RoamerOvlNumber"))?;
        if read_word(&overlay, first_species_offset) > self.pokemon_list_incl_formes.len() as i32 {
            self.apply_black_white_roamer_patch()?;
        }
        for roamer in &self.entry.roaming_pokemon {
            let mut encounter = StaticEncounter::new(self.roamer_species(roamer)?);
            encounter.level = self.roamer_level(roamer)?;
            statics.push(encounter);
        }
        Ok(())
    }

    pub(crate) fn static_pokemon(&self) -> Result<Vec<StaticEncounter>> {
        let mut statics = Vec::new();
        if !self.entry.static_pokemon_support {
            return Ok(statics);
        }
        let egg_offsets = self.entry.array("StaticEggPokemonOffsets").unwrap_or(&[]);
        for (index, entry) in self.entry.static_pokemon.iter().enumerate() {
            let pokemon = self.pk(static_species(&self.script_narc, entry)?);
            let pokemon = self.alt_forme_of(pokemon, static_forme(&self.script_narc, entry)?);
            let mut encounter =
                self.static_with_linked_levels(pokemon, &self.script_narc, entry)?;
            encounter.is_egg = egg_offsets.contains(&(index as i32));
            statics.push(encounter);
        }
        let map_narc = self.read_narc("MapFiles")?;
        for entry in &self.entry.static_pokemon_fake_ball {
            let pokemon = self.pk(static_species(&self.script_narc, entry)?);
            statics.push(self.static_with_linked_levels(pokemon, &map_narc, entry)?);
        }
        if self.is_bw2() {
            self.hidden_hollow_statics(&mut statics)?;
        }
        self.hidden_hollow_counted.set(true);
        self.roaming_statics(&mut statics)?;
        Ok(statics)
    }

    fn expected_static_count(&self) -> usize {
        self.entry.static_pokemon.len()
            + self.entry.static_pokemon_fake_ball.len()
            + self.hidden_hollow_count.get() as usize
            + self.entry.roaming_pokemon.len()
    }

    fn write_hidden_hollow_slot(
        &self,
        hollow: &mut [u8],
        base: i32,
        group: i32,
        encounter: &StaticEncounter,
        gender_ratio: i32,
    ) {
        write_word(
            hollow,
            base + group * 2,
            self.core.data.pk(encounter.pkmn).number,
        );
        hollow[(base + 16 + group) as usize] = gender_ratio as u8;
        hollow[(base + 20 + group) as usize] = encounter.forme as u8;
        hollow[(base + 12 + group) as usize] = encounter.level as u8;
        hollow[(base + 8 + group) as usize] = encounter.max_level as u8;
    }

    fn set_hidden_hollow_statics<'a>(
        &mut self,
        statics: &mut impl Iterator<Item = &'a StaticEncounter>,
    ) -> Result<()> {
        let mut narc = self.read_narc("HiddenHollows")?;
        let wanted_version = self.entry.int("HiddenHollowIndex");
        let mut random = self.core.engine.random.clone();
        for hollow in &mut narc.files {
            for version in 0..HIDDEN_HOLLOW_VERSIONS {
                if version != wanted_version {
                    continue;
                }
                for rarity in 0..HIDDEN_HOLLOW_RARITIES {
                    let base = hidden_hollow_base(version, rarity);
                    let mut group = 0;
                    while group < HIDDEN_HOLLOW_GROUPS {
                        let encounter = statics
                            .next()
                            .ok_or(RandomizationError::IndexOutOfBounds("statics iterator"))?;
                        let gender_ratio = random.next_int_bounded(101);
                        self.write_hidden_hollow_slot(hollow, base, group, encounter, gender_ratio);
                        for linked in &encounter.linked_encounters {
                            group += 1;
                            self.write_hidden_hollow_slot(
                                hollow,
                                base,
                                group,
                                linked,
                                gender_ratio,
                            );
                        }
                        group += 1;
                    }
                }
            }
        }
        self.write_narc("HiddenHollows", &narc)
    }

    fn set_roamer(
        &mut self,
        roamer: &RoamingPokemonEntry,
        encounter: &StaticEncounter,
    ) -> Result<()> {
        let number = self.entry.int("RoamerOvlNumber");
        let species_number = self.core.data.pk(encounter.pkmn).number;
        let mut overlay = self.read_overlay(number)?;
        for &offset in &roamer.species_overlay_offsets {
            write_word(&mut overlay, offset, species_number);
        }
        self.write_overlay(number, &overlay)?;
        for script in &roamer.species_script_offsets {
            write_word(
                entry_file_mut(&mut self.script_narc, script)?,
                script.offset,
                species_number,
            );
        }
        let mut overlay = self.read_overlay(number)?;
        for &offset in &roamer.level_overlay_offsets {
            overlay[offset as usize] = encounter.level as u8;
        }
        self.write_overlay(number, &overlay)
    }

    pub(crate) fn set_static_pokemon_impl(&mut self, statics: &[StaticEncounter]) -> Result<bool> {
        if !self.entry.static_pokemon_support || statics.len() != self.expected_static_count() {
            return Ok(false);
        }
        let entry = self.entry;
        let mut iterator = statics.iter();
        let mut next = || {
            iterator
                .next()
                .ok_or(RandomizationError::IndexOutOfBounds("statics iterator"))
        };
        for static_entry in &entry.static_pokemon {
            let encounter = next()?;
            let pk = self.core.data.pk(encounter.pkmn);
            let (number, forme) = (pk.number, pk.forme_number);
            set_static_species(&mut self.script_narc, static_entry, number)?;
            set_static_forme(&mut self.script_narc, static_entry, forme)?;
            set_static_level(&mut self.script_narc, static_entry, encounter.level, 0)?;
            for (index, linked) in encounter.linked_encounters.iter().enumerate() {
                set_static_level(&mut self.script_narc, static_entry, linked.level, index + 1)?;
            }
        }
        let mut map_narc = self.read_narc("MapFiles")?;
        for static_entry in &entry.static_pokemon_fake_ball {
            let encounter = next()?;
            let number = self.core.data.pk(encounter.pkmn).number;
            set_static_species(&mut self.script_narc, static_entry, number)?;
            set_static_level(&mut map_narc, static_entry, encounter.level, 0)?;
            for (index, linked) in encounter.linked_encounters.iter().enumerate() {
                set_static_level(&mut map_narc, static_entry, linked.level, index + 1)?;
            }
        }
        self.write_narc("MapFiles", &map_narc)?;
        let consumed = entry.static_pokemon.len() + entry.static_pokemon_fake_ball.len();
        let mut remaining = statics[consumed..].iter();
        if self.is_bw2() {
            self.set_hidden_hollow_statics(&mut remaining)?;
        }
        for roamer in &entry.roaming_pokemon {
            let encounter = remaining
                .next()
                .ok_or(RandomizationError::IndexOutOfBounds("statics iterator"))?;
            self.set_roamer(roamer, encounter)?;
        }
        if !self.is_bw2() {
            let box_legendary = statics
                .get(entry.int("BoxLegendaryOffset") as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("staticPokemon"))?;
            let species_number = self.core.data.pk(box_legendary.pkmn).number;
            self.fix_box_legendary_bw1(species_number)?;
        }
        Ok(true)
    }

    fn fix_box_legendary_bw1(&mut self, species_number: i32) -> Result<()> {
        let number = self.entry.int("FieldOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        if self.entry.is_black {
            for prefix in [
                gen5::BLACK_BOX_LEGENDARY_CHECK_PREFIX1,
                gen5::BLACK_BOX_LEGENDARY_CHECK_PREFIX2,
            ] {
                let offset = find(&overlay, prefix);
                if offset > 0 {
                    write_full_int(
                        &mut overlay,
                        offset + prefix.len() as i32 / 2,
                        species_number,
                    );
                }
            }
        } else {
            let prefix = gen5::WHITE_BOX_LEGENDARY_CHECK_PREFIX1;
            let offset = find(&overlay, prefix);
            if offset > 0 {
                let start = offset + prefix.len() as i32 / 2;
                let mut set = |delta: i32, value: u8| overlay[(start + delta) as usize] = value;
                set(66, 0x00);
                set(67, 0x00);
                write_full_int(&mut overlay, start + 320, species_number);
                let mut set = |delta: i32, value: u8| overlay[(start + delta) as usize] = value;
                set(18, 0x00);
                set(19, 0x00);
                set(26, 0x49);
                set(27, 0x49);
            }
            let prefix = gen5::WHITE_BOX_LEGENDARY_CHECK_PREFIX2;
            let offset = find(&overlay, prefix);
            if offset > 0 {
                let start = offset + prefix.len() as i32 / 2;
                let mut set = |delta: i32, value: u8| overlay[(start + delta) as usize] = value;
                set(502, 0x00);
                set(503, 0x24);
                set(504, 0x06);
                set(505, 0x1C);
                write_full_int(&mut overlay, start + 556, species_number);
                let mut set = |delta: i32, value: u8| overlay[(start + delta) as usize] = value;
                set(78, 0x77);
                set(79, 0x49);
                set(80, 0x00);
                set(81, 0x00);
            }
        }
        self.write_overlay(number, &overlay)
    }

    fn music_change_order(&self, changes: &BTreeMap<i32, i32>) -> Vec<i32> {
        let mut map: JavaHashMap<i32, i32> = JavaHashMap::new();
        let statics = self.static_pokemon().unwrap_or_default();
        let change_music = self.entry.array("SpecialMusicStatics").unwrap_or(&[]);
        for encounter in &statics {
            let old = self.core.data.pk(encounter.pkmn).number;
            if change_music.contains(&old)
                && let Some(&new) = changes.get(&old)
            {
                map.put(old, new);
            }
        }
        for (&old, &new) in changes {
            if !map.contains_key(&old) {
                map.put(old, new);
            }
        }
        map.keys().copied().collect()
    }

    fn pool_slot(&self, pool_offset: i32, old_static: i32, replaced: &[i32]) -> Result<i32> {
        let mut position = pool_offset;
        loop {
            if position < 0 || position as usize + 2 > self.arm9.len() {
                return Err(RandomizationError::IndexOutOfBounds("arm9 music pool"));
            }
            let index = read_word(&self.arm9, position);
            if index == old_static && !replaced.contains(&position) {
                return Ok(position);
            }
            position += 4;
        }
    }

    pub(crate) fn apply_correct_static_music_impl(
        &mut self,
        changes: &BTreeMap<i32, i32>,
    ) -> Result<()> {
        let field_number = self.entry.int("FieldOvlNumber");
        let mut field_overlay = self.read_overlay(field_number)?;
        self.generic_ips_patch(&mut field_overlay, "NewIndexToMusicOvlTweak")?;
        self.write_overlay(field_number, &field_overlay)?;
        let mut arm9 = std::mem::take(&mut self.arm9);
        let patched = self.generic_ips_patch(&mut arm9, "NewIndexToMusicTweak");
        self.arm9 = arm9;
        patched?;
        let prefix = self.entry.string("NewIndexToMusicPrefix");
        let pool_offset = find(&self.arm9, prefix) + prefix.len() as i32 / 2;
        let order = self.music_change_order(changes);
        let mut replaced: Vec<i32> = Vec::new();
        let mut highest = -1;
        for old_static in order {
            let position = self.pool_slot(pool_offset, old_static, &replaced)?;
            if self.is_bw2() && old_static > gen5::POKEMON_COUNT {
                write_word(&mut self.arm9, position - 0xFE, 0);
                write_word(&mut self.arm9, position - 0xFC, 0);
                write_word(&mut self.arm9, position - 0xFA, 0);
                write_word(&mut self.arm9, position - 0xF8, KYUREM_MUSIC_PATCH_WORD);
            }
            write_word(&mut self.arm9, position, changes[&old_static]);
            replaced.push(position);
            highest = highest.max(position);
        }
        let special_music_statics = self.get_special_music_statics();
        let mut position = pool_offset;
        while position <= highest {
            if !replaced.contains(&position) {
                let pokemon = read_word(&self.arm9, position);
                if !special_music_statics.contains(&pokemon) {
                    write_word(&mut self.arm9, position, 0);
                }
            }
            position += 4;
        }
        Ok(())
    }
}
