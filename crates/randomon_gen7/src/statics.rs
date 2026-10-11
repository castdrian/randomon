use std::collections::BTreeMap;

use randomon_core::{RandomizationError, Result};
use randomon_ctr::bytes::{find, find_after_prefix, read_long, read_word, write_long, write_word};
use randomon_data::{Aura, IngameTrade, PokemonId, StaticEncounter, TotemPokemon};
use randomon_formats::ctr::{Amx, Decompression, GarcArchive};
use randomon_java::JavaRng;

use crate::constants;
use crate::handler::{Gen7RomHandler, io};
use crate::pokemon::{garc_file, garc_file_mut};

const GIFT_SIZE: usize = 0x14;
const STATIC_SIZE: usize = 0x38;
const TRADE_SIZE: usize = 0x34;
const UNMODIFIED_ZYGARDE_FORME_LOAD: i32 = 0xE59D0040_u32 as i32;

fn signed(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset] as i8)
}

fn no_such_element() -> RandomizationError {
    RandomizationError::randomization("java.util.NoSuchElementException")
}

fn java_substring(text: &str, begin: usize) -> Result<String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    units
        .get(begin..)
        .map(String::from_utf16_lossy)
        .ok_or(RandomizationError::IndexOutOfBounds("String.substring"))
}

fn java_substring_from(text: &str, needle: &str) -> Result<String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let needle: Vec<u16> = needle.encode_utf16().collect();
    let begin = units
        .windows(needle.len())
        .position(|window| window == needle.as_slice())
        .ok_or(RandomizationError::IndexOutOfBounds("String.substring(-1)"))?;
    Ok(String::from_utf16_lossy(&units[begin..]))
}

fn text_at(strings: &[String], index: usize) -> Result<&String> {
    strings
        .get(index)
        .ok_or(RandomizationError::IndexOutOfBounds("starterText"))
}

impl Gen7RomHandler {
    fn static_garc(&self) -> Result<GarcArchive> {
        self.read_garc(self.file("StaticPokemon"), Decompression::Skip)
    }

    pub(crate) fn read_starters(&self) -> Result<Vec<PokemonId>> {
        let garc = self.static_garc()?;
        let gifts = garc_file(&garc, 0)?;
        Ok((0..3)
            .map(|index| {
                let offset = index * GIFT_SIZE;
                self.pokemon_with_forme(read_word(gifts, offset), signed(gifts, offset + 2))
            })
            .collect())
    }

    fn starter_forme(&self, starter: PokemonId) -> Result<(PokemonId, i32)> {
        let mut starter = starter;
        let mut forme = 0;
        let mut check_cosmetics = true;
        let pokemon = self.core.data.pk(starter);
        if pokemon.forme_number > 0 {
            forme = pokemon.forme_number;
            starter = pokemon
                .base_forme
                .ok_or(RandomizationError::NullPointer("baseForme"))?;
            check_cosmetics = false;
        }
        let base = self.core.data.pk(starter);
        if base.cosmetic_forms > 0 {
            let mut random = self.core.engine.random.clone();
            let cosmetic = base.cosmetic_form_number(random.next_int_bounded(base.cosmetic_forms));
            if check_cosmetics {
                forme = cosmetic;
            } else {
                forme += cosmetic;
            }
        }
        Ok((starter, forme))
    }

    pub(crate) fn write_starters(&mut self, starters: &[PokemonId]) -> Result<()> {
        let path = self.file("StaticPokemon");
        let mut garc = self.static_garc()?;
        for index in 0..3 {
            let starter = *starters
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds("newStarters"))?;
            let (base, forme) = self.starter_forme(starter)?;
            let gifts = garc_file_mut(&mut garc, 0)?;
            let offset = index * GIFT_SIZE;
            write_word(gifts, offset, base.number());
            gifts[offset + 2] = forme as u8;
        }
        self.write_garc(path, &garc)?;
        self.set_starter_text(starters)
    }

    fn starter_name(&self, starter: PokemonId) -> String {
        self.core.data.pk(starter).name.clone()
    }

    fn starter_type(&self, starter: PokemonId) -> Result<String> {
        self.core
            .data
            .pk(starter)
            .primary_type
            .map(|kind| kind.camel_case())
            .ok_or(RandomizationError::NullPointer("primaryType"))
    }

    fn set_starter_text(&mut self, starters: &[PokemonId]) -> Result<()> {
        let index = self.entry.int("StarterTextOffset");
        let mut text = self.strings(true, index)?;
        let descriptors = if self.is_usum() {
            [1, 2, 3]
        } else {
            [11, 12, 13]
        };
        for (slot, line) in descriptors.into_iter().enumerate() {
            let cut = if slot == 2 { 7 } else { 6 };
            let rest = java_substring(text_at(&text, line)?, cut)?;
            text[line] = format!("{}{rest}", self.starter_name(starters[slot]));
        }
        for slot in 0..3 {
            let starter = starters[slot];
            let name = self.starter_name(starter);
            let kind = self.starter_type(starter)?;
            if self.is_usum() {
                text[slot + 7] = format!(
                    "So, you wanna go with the {kind}-type Pokémon\\n{name}?[VAR 0114(0005)]"
                );
                text[slot + 14] = name;
            } else {
                text[slot + 1] = format!("The {kind}-type {name}");
                text[slot + 4] =
                    format!("Will you choose the {kind}-type Pokémon\\n{name}?[VAR 0114(0008)]");
                let flavor = java_substring_from(text_at(&text, slot + 35)?, "\\n")?;
                text[slot + 35] = format!("The {kind}-type {name}{flavor}");
            }
        }
        self.set_strings(true, index, &text)
    }

    pub(crate) fn read_starter_held_items(&self) -> Result<Vec<i32>> {
        let garc = self.static_garc()?;
        let gifts = garc_file(&garc, 0)?;
        Ok((0..3)
            .map(|index| read_word(gifts, index * GIFT_SIZE + 8))
            .collect())
    }

    pub(crate) fn write_starter_held_items(&self, items: &[i32]) -> Result<()> {
        let path = self.file("StaticPokemon");
        let mut garc = self.static_garc()?;
        let gifts = garc_file_mut(&mut garc, 0)?;
        for index in 0..3 {
            let item = *items
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds("items"))?;
            write_word(gifts, index * GIFT_SIZE + 8, item);
        }
        self.write_garc(path, &garc)
    }

    fn read_static_encounter(&self, data: &[u8], offset: usize) -> StaticEncounter {
        let species = read_word(data, offset);
        let forme = signed(data, offset + 2);
        let mut encounter = StaticEncounter::new(self.pokemon_with_forme(species, forme));
        encounter.forme = forme;
        encounter.level = signed(data, offset + 3);
        let item = read_word(data, offset + 4);
        encounter.held_item = if item == 0xFFFF { 0 } else { item };
        encounter
    }

    fn skipped_static_indices(&self) -> Vec<i32> {
        let mut skipped = self.entry.array("TotemPokemonIndices").to_vec();
        skipped.extend_from_slice(self.entry.array("AllyPokemonIndices"));
        skipped
    }

    pub(crate) fn read_static_pokemon(&self) -> Result<Vec<StaticEncounter>> {
        let garc = self.static_garc()?;
        let skipped = self.skipped_static_indices();
        let gifts = garc_file(&garc, 0)?;
        let mut statics = Vec::new();
        for index in 3..gifts.len() / GIFT_SIZE {
            let offset = index * GIFT_SIZE;
            let species = read_word(gifts, offset);
            let forme = signed(gifts, offset + 2);
            let mut encounter = StaticEncounter::new(self.pokemon_with_forme(species, forme));
            encounter.forme = forme;
            encounter.level = signed(gifts, offset + 3);
            encounter.held_item = read_word(gifts, offset + 8);
            encounter.is_egg = gifts[offset + 10] == 1;
            statics.push(encounter);
        }
        let encounters = garc_file(&garc, 1)?;
        for index in 0..encounters.len() / STATIC_SIZE {
            if skipped.contains(&(index as i32)) {
                continue;
            }
            statics.push(self.read_static_encounter(encounters, index * STATIC_SIZE));
        }
        self.read_assembly_zygarde(&mut statics)?;
        self.consolidate_linked_encounters(&mut statics)?;
        Ok(statics)
    }

    fn zygarde_script(&self) -> Result<(GarcArchive, Amx)> {
        let garc = self.read_garc(self.file("Scripts"), Decompression::Skip)?;
        let script =
            Amx::parse(garc_file(&garc, constants::ZYGARDE_ASSEMBLY_SCRIPT_FILE)?).map_err(io)?;
        Ok((garc, script))
    }

    fn zygarde_offsets(&self) -> Option<(usize, usize)> {
        let species = find_after_prefix(&self.code, constants::ZYGARDE_ASSEMBLY_SPECIES_PREFIX)?;
        let forme = find_after_prefix(&self.code, constants::ZYGARDE_ASSEMBLY_FORME_PREFIX)?;
        Some((species, forme))
    }

    fn read_assembly_zygarde(&self, statics: &mut Vec<StaticEncounter>) -> Result<()> {
        let (_, script) = self.zygarde_script()?;
        let levels: Vec<i32> = self
            .entry
            .array("ZygardeScriptLevelOffsets")
            .iter()
            .map(|&offset| signed(&script.dec_data, offset as usize))
            .collect();
        let species_raw = find(&self.code, constants::ZYGARDE_ASSEMBLY_SPECIES_PREFIX);
        let forme_raw = find(&self.code, constants::ZYGARDE_ASSEMBLY_FORME_PREFIX);
        if species_raw <= 0 || forme_raw <= 0 {
            return Ok(());
        }
        let Some((species_offset, forme_offset)) = self.zygarde_offsets() else {
            return Ok(());
        };
        let species = read_word(&self.code, species_offset);
        let forme = if read_long(&self.code, forme_offset) == UNMODIFIED_ZYGARDE_FORME_LOAD {
            i32::from(self.is_sm())
        } else {
            signed(&self.code, forme_offset)
        };
        let pokemon = self.pokemon_with_forme(species, forme);
        let mut lowest = StaticEncounter::new(pokemon);
        lowest.forme = forme;
        lowest.level = *levels
            .first()
            .ok_or(RandomizationError::IndexOutOfBounds("levels"))?;
        for &level in &levels[1..] {
            let mut higher = StaticEncounter::new(pokemon);
            higher.forme = forme;
            higher.level = level;
            lowest.linked_encounters.push(higher);
        }
        statics.push(lowest);
        Ok(())
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

    fn write_static_slot(data: &mut [u8], offset: usize, encounter: &StaticEncounter) {
        write_word(data, offset, encounter.pkmn.number());
        data[offset + 2] = encounter.forme as u8;
        data[offset + 3] = encounter.level as u8;
        let item = if encounter.held_item == 0 {
            -1
        } else {
            encounter.held_item
        };
        write_word(data, offset + 4, item);
        if encounter.reset_moves {
            for field in [12, 14, 16, 18] {
                write_word(data, offset + field, 0);
            }
        }
    }

    pub(crate) fn write_static_pokemon(
        &mut self,
        static_pokemon: &[StaticEncounter],
    ) -> Result<()> {
        let mut statics = static_pokemon.to_vec();
        self.unlink_static_encounters(&mut statics)?;
        let path = self.file("StaticPokemon");
        let mut garc = self.static_garc()?;
        let skipped = self.skipped_static_indices();
        let mut iterator = statics.iter();
        let gifts = garc_file_mut(&mut garc, 0)?;
        for index in 3..gifts.len() / GIFT_SIZE {
            let encounter = iterator.next().ok_or_else(no_such_element)?;
            let offset = index * GIFT_SIZE;
            write_word(gifts, offset, encounter.pkmn.number());
            gifts[offset + 2] = encounter.forme as u8;
            gifts[offset + 3] = encounter.level as u8;
            write_word(gifts, offset + 8, encounter.held_item);
        }
        let encounters = garc_file_mut(&mut garc, 1)?;
        for index in 0..encounters.len() / STATIC_SIZE {
            if skipped.contains(&(index as i32)) {
                continue;
            }
            let encounter = iterator.next().ok_or_else(no_such_element)?;
            Self::write_static_slot(encounters, index * STATIC_SIZE, encounter);
        }
        let zygarde = iterator.next().ok_or_else(no_such_element)?.clone();
        self.write_assembly_zygarde(&zygarde)?;
        self.write_garc(path, &garc)
    }

    fn write_assembly_zygarde(&mut self, encounter: &StaticEncounter) -> Result<()> {
        let mut levels = vec![encounter.level];
        levels.extend(
            encounter
                .linked_encounters
                .iter()
                .map(|linked| linked.level),
        );
        let (mut garc, mut script) = self.zygarde_script()?;
        for (index, &offset) in self
            .entry
            .array("ZygardeScriptLevelOffsets")
            .iter()
            .enumerate()
        {
            let level = *levels
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds("levels"))?;
            script.dec_data[offset as usize] = level as u8;
        }
        garc.set_file(
            constants::ZYGARDE_ASSEMBLY_SCRIPT_FILE,
            script.to_bytes().map_err(io)?,
        )
        .map_err(io)?;
        self.write_garc(self.file("Scripts"), &garc)?;
        if let Some((species_offset, forme_offset)) = self.zygarde_offsets() {
            let base = self.core.data.base_number(encounter.pkmn);
            write_word(&mut self.code, species_offset, base);
            self.code[forme_offset..forme_offset + 4].copy_from_slice(&[
                encounter.forme as u8,
                0x00,
                0xA0,
                0xE3,
            ]);
        }
        Ok(())
    }

    pub(crate) fn read_totem_pokemon(&self) -> Result<Vec<TotemPokemon>> {
        let garc = self.static_garc()?;
        let data = garc_file(&garc, 1)?;
        let mut totems = Vec::new();
        for &index in self.entry.array("TotemPokemonIndices") {
            let offset = index as usize * STATIC_SIZE;
            let mut encounter = self.read_static_encounter(data, offset);
            let item = read_word(data, offset + 4);
            encounter.held_item = if item == 0xFFFF { 0 } else { item };
            let mut allies = BTreeMap::new();
            let ally_count = signed(data, offset + 0x27);
            for ally in 0..ally_count.max(0) as usize {
                let ally_index = (signed(data, offset + 0x28 + 4 * ally) - 1) & 0xFF;
                allies.insert(
                    ally_index,
                    self.read_static_encounter(data, ally_index as usize * STATIC_SIZE),
                );
            }
            totems.push(TotemPokemon {
                encounter,
                aura: Aura::from_byte(data[offset + 0x25] as i8),
                ally1_offset: 0,
                ally2_offset: 0,
                allies,
                unused: false,
            });
        }
        Ok(totems)
    }

    fn base_and_forme(&self, encounter: &mut StaticEncounter) -> Result<()> {
        let pokemon = self.core.data.pk(encounter.pkmn);
        if pokemon.forme_number > 0 {
            encounter.forme = pokemon.forme_number;
            encounter.pkmn = pokemon
                .base_forme
                .ok_or(RandomizationError::NullPointer("baseForme"))?;
        }
        Ok(())
    }

    pub(crate) fn write_totem_pokemon(&mut self, totems: &[TotemPokemon]) -> Result<()> {
        let path = self.file("StaticPokemon");
        let mut garc = self.static_garc()?;
        let mut iterator = totems.iter();
        for &index in self.entry.array("TotemPokemonIndices") {
            let mut totem = iterator.next().ok_or_else(no_such_element)?.clone();
            self.base_and_forme(&mut totem.encounter)?;
            let data = garc_file_mut(&mut garc, 1)?;
            let offset = index as usize * STATIC_SIZE;
            Self::write_static_slot(data, offset, &totem.encounter);
            data[offset + 0x25] = totem.aura.to_byte() as u8;
            for (&ally_index, ally) in &totem.allies {
                let mut ally = ally.clone();
                self.base_and_forme(&mut ally)?;
                let data = garc_file_mut(&mut garc, 1)?;
                Self::write_static_slot(data, ally_index as usize * STATIC_SIZE, &ally);
            }
        }
        self.write_garc(path, &garc)
    }

    pub(crate) fn read_ingame_trades(&self) -> Result<Vec<IngameTrade>> {
        let garc = self.static_garc()?;
        let strings = self.strings(true, self.entry.int("IngameTradesTextOffset"))?;
        let data = garc_file(&garc, 4)?;
        let text = |index: i32| {
            strings
                .get(index as usize)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("tradeStrings"))
        };
        let mut trades = Vec::new();
        for index in 0..data.len() / TRADE_SIZE {
            let offset = index * TRADE_SIZE;
            let given_species = read_word(data, offset);
            let forme = signed(data, offset + 4);
            let requested = self.pk(read_word(data, offset + 0x2C));
            trades.push(IngameTrade {
                id: 0,
                given_pokemon: self.pokemon_with_forme(given_species, forme),
                requested_pokemon: self.core.data.exists(requested).then_some(requested),
                nickname: text(read_word(data, offset + 2))?,
                ot_name: text(read_word(data, offset + 0x18))?,
                ot_id: read_long(data, offset + 0x10),
                ivs: (0..6).map(|iv| signed(data, offset + 6 + iv)).collect(),
                item: read_word(data, offset + 0x14),
            });
        }
        Ok(trades)
    }

    pub(crate) fn write_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        let old_trades = self.read_ingame_trades()?;
        let path = self.file("StaticPokemon");
        let mut garc = self.static_garc()?;
        let text_index = self.entry.int("IngameTradesTextOffset");
        let mut strings = self.strings(true, text_index)?;
        let count = garc_file(&garc, 4)?.len() / TRADE_SIZE;
        for index in 0..count {
            let trade = trades
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds("trades"))?;
            let offset = index * TRADE_SIZE;
            let mut given = trade.given_pokemon;
            let mut forme = 0;
            let given_pokemon = self.core.data.pk(given);
            if given_pokemon.forme_number > 0 {
                forme = given_pokemon.forme_number;
                given = given_pokemon
                    .base_forme
                    .ok_or(RandomizationError::NullPointer("baseForme"))?;
            }
            let requested = trade
                .requested_pokemon
                .ok_or(RandomizationError::NullPointer("requestedPokemon"))?;
            let data = garc_file_mut(&mut garc, 4)?;
            write_word(data, offset, given.number());
            data[offset + 4] = forme as u8;
            write_word(data, offset + 0x2C, requested.number());
            let nickname_index = read_word(data, offset + 2) as usize;
            let ot_index = read_word(data, offset + 0x18) as usize;
            *strings
                .get_mut(nickname_index)
                .ok_or(RandomizationError::IndexOutOfBounds("tradeStrings"))? =
                trade.nickname.clone();
            *strings
                .get_mut(ot_index)
                .ok_or(RandomizationError::IndexOutOfBounds("tradeStrings"))? =
                trade.ot_name.clone();
            write_long(data, offset + 0x10, trade.ot_id);
            for iv in 0..6 {
                data[offset + 6 + iv] = trade.ivs[iv] as u8;
            }
            write_word(data, offset + 0x14, trade.item);
            if let Some(offsets) = constants::hardcoded_trade_text_offsets(self.rom_type, index) {
                self.update_hardcoded_trade_text(
                    old_trades
                        .get(index)
                        .ok_or(RandomizationError::IndexOutOfBounds("oldTrades"))?,
                    trade,
                    &mut strings,
                    offsets,
                )?;
            }
        }
        self.write_garc(path, &garc)?;
        self.set_strings(true, text_index, &strings)
    }

    fn update_hardcoded_trade_text(
        &self,
        old_trade: &IngameTrade,
        new_trade: &IngameTrade,
        strings: &mut [String],
        offsets: &[usize],
    ) -> Result<()> {
        let name = |pokemon: Option<PokemonId>| {
            pokemon
                .map(|pokemon| self.core.data.pk(pokemon).name.clone())
                .ok_or(RandomizationError::NullPointer("requestedPokemon"))
        };
        for &offset in offsets {
            let old_requested = name(old_trade.requested_pokemon)?;
            let new_requested = name(new_trade.requested_pokemon)?;
            let old_given = self.core.data.pk(old_trade.given_pokemon).name.clone();
            let new_given = self.core.data.pk(new_trade.given_pokemon).name.clone();
            let text = strings
                .get_mut(offset)
                .ok_or(RandomizationError::IndexOutOfBounds("tradeStrings"))?;
            *text = text
                .replace(old_requested.as_str(), &new_requested)
                .replace(old_given.as_str(), &new_given);
        }
        Ok(())
    }
}
