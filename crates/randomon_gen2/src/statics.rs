use std::collections::VecDeque;

use randomon_core::{RandomizationError, Result};
use randomon_data::StaticEncounter;
use randomon_data::constants::moves;

use crate::constants::ODD_EGG_POKEMON_COUNT;
use crate::handler::{Gen2RomHandler, pokemon};
use crate::rom_entry::StaticPokemonEntry;

const ODD_EGG_MOVES_OFFSET: i32 = 2;
const ODD_EGG_PP_OFFSET: i32 = 23;
const ODD_EGG_LEVEL: i32 = 5;

impl Gen2RomHandler {
    fn odd_egg_offset(&self) -> i32 {
        self.value("StaticPokemonOddEggOffset")
    }

    fn odd_egg_slot(&self, index: i32) -> i32 {
        self.odd_egg_offset() + index * self.value("StaticPokemonOddEggDataSize")
    }

    fn static_level(&self, entry: &StaticPokemonEntry) -> i32 {
        entry
            .level_offsets
            .first()
            .map_or(1, |&offset| self.signed_byte(offset))
    }

    pub(crate) fn read_static_pokemon(&self) -> Vec<StaticEncounter> {
        let egg_indexes = self.array("StaticEggPokemonOffsets");
        let mut statics = Vec::new();
        if self.value("StaticPokemonSupport") > 0 {
            for (index, entry) in self.entry.static_pokemon.iter().enumerate() {
                let mut encounter = StaticEncounter::new(self.pokemon_at(entry.species_offsets[0]));
                encounter.level = self.static_level(entry);
                encounter.is_egg = egg_indexes.contains(&(index as i32));
                statics.push(encounter);
            }
        }
        if self.odd_egg_offset() > 0 {
            for index in 0..ODD_EGG_POKEMON_COUNT {
                let mut encounter = StaticEncounter::new(self.pokemon_at(self.odd_egg_slot(index)));
                encounter.is_egg = true;
                statics.push(encounter);
            }
        }
        statics
    }

    fn write_padded_pokemon_name(&mut self, name: &str, length: i32, offset: i32) -> Result<()> {
        let width = length.max(0) as usize;
        let name_units = name.encode_utf16().count();
        let padded = format!("{name}{}", " ".repeat(width.saturating_sub(name_units)));
        let raw = self.translate(&padded);
        let bytes = raw
            .get(..width)
            .ok_or(RandomizationError::IndexOutOfBounds(
                "writePaddedPokemonName",
            ))?
            .to_vec();
        self.write_bytes(offset, &bytes);
        Ok(())
    }

    fn write_static_species(
        &mut self,
        entry: &StaticPokemonEntry,
        encounter: &StaticEncounter,
    ) -> Result<()> {
        let number = self.number_of(encounter.pkmn);
        if !entry.is_game_corner {
            for &offset in &entry.species_offsets {
                self.set_byte(offset, number);
            }
            return Ok(());
        }
        let (name_offset, species_offsets) = entry
            .species_offsets
            .split_last()
            .ok_or(RandomizationError::IndexOutOfBounds("speciesOffsets"))?;
        for &offset in species_offsets {
            self.set_byte(offset, number);
        }
        let name = self.core.data.pk(encounter.pkmn).name.clone();
        let length = self.value("GameCornerPokemonNameLength");
        self.write_padded_pokemon_name(&name, length, *name_offset)
    }

    pub(crate) fn write_static_pokemon(&mut self, statics: &[StaticEncounter]) -> Result<bool> {
        if self.value("StaticPokemonSupport") == 0 {
            return Ok(false);
        }
        self.ensure_fleeing_patched();
        let entries = &self.entry.static_pokemon;
        let mut desired = entries.len();
        if self.odd_egg_offset() > 0 {
            desired += ODD_EGG_POKEMON_COUNT as usize;
        }
        if statics.len() != desired {
            return Ok(false);
        }
        for (entry, encounter) in entries.iter().zip(statics) {
            self.write_static_species(entry, encounter)?;
            if let Some(&level_offset) = entry.level_offsets.first() {
                self.set_byte(level_offset, encounter.level as u8);
            }
        }
        if self.odd_egg_offset() > 0 {
            for (index, encounter) in statics[entries.len()..].iter().enumerate() {
                let slot = self.odd_egg_slot(index as i32);
                let number = self.number_of(encounter.pkmn);
                self.set_byte(slot, number);
                self.set_odd_egg_moves(i32::from(number), slot)?;
            }
        }
        Ok(true)
    }

    fn set_odd_egg_moves(&mut self, number: i32, slot: i32) -> Result<()> {
        let movesets = self.read_moves_learnt();
        let moveset = movesets
            .get(&number)
            .ok_or(RandomizationError::NullPointer("movesets.get"))?;
        let mut level_five: VecDeque<i32> = VecDeque::new();
        for learnt in moveset
            .iter()
            .take_while(|learnt| learnt.level <= ODD_EGG_LEVEL)
        {
            if level_five.len() == 4 {
                level_five.pop_front();
            }
            level_five.push_back(learnt.move_number);
        }
        if level_five.len() == 4 {
            level_five.pop_front();
        }
        level_five.push_back(moves::DIZZY_PUNCH);
        for index in 0..4 {
            let (move_number, pp) = match level_five.pop_front() {
                Some(move_number) => {
                    let pp = self
                        .core
                        .data
                        .move_by_number(move_number)
                        .ok_or(RandomizationError::NullPointer("moves.get(move)"))?
                        .pp;
                    (move_number, pp)
                }
                None => (0, 0),
            };
            self.set_byte(slot + ODD_EGG_MOVES_OFFSET + index, move_number as u8);
            self.set_byte(slot + ODD_EGG_PP_OFFSET + index, pp as u8);
        }
        Ok(())
    }

    pub(crate) fn unown(&self) -> randomon_data::PokemonId {
        pokemon(randomon_data::constants::species::UNOWN)
    }
}
