use std::collections::BTreeMap;

use randomon_core::{EggMoves, Movesets, RandomizationError, Result};
use randomon_data::{MoveLearnt, PokemonId};

use crate::constants as c;
use crate::handler::Gen3RomHandler;

const STANDARD_ENTRY_SIZE: i32 = 2;
const JAMBO_ENTRY_SIZE: i32 = 3;
const HIGH_MOVE_FLAG: i32 = 0x01;
const LEVEL_MASK: i32 = 0xFE;
const HIGH_MOVE_THRESHOLD: i32 = 255;

impl Gen3RomHandler {
    fn moveset_pointer_location(&self, id: PokemonId) -> i32 {
        self.value("PokemonMovesets") + self.internal_of(id) * 4
    }

    fn entry_size(&self) -> i32 {
        if self.jambo_moveset_hack {
            JAMBO_ENTRY_SIZE
        } else {
            STANDARD_ENTRY_SIZE
        }
    }

    fn is_moveset_terminator(&self, offset: i32) -> bool {
        if !self.rom.contains_range(offset, self.entry_size()) {
            return true;
        }
        if self.jambo_moveset_hack {
            self.rom.byte(offset) == 0x00
                && self.rom.byte(offset + 1) == 0x00
                && self.rom.byte(offset + 2) == 0xFF
        } else {
            self.rom.byte(offset) == 0xFF && self.rom.byte(offset + 1) == 0xFF
        }
    }

    fn read_move_learnt(&self, offset: i32) -> MoveLearnt {
        if self.jambo_moveset_hack {
            return MoveLearnt {
                move_number: self.rom.word(offset),
                level: self.rom.int(offset + 2),
            };
        }
        let mut move_number = self.rom.int(offset);
        let flags = self.rom.int(offset + 1);
        if flags & HIGH_MOVE_FLAG == HIGH_MOVE_FLAG {
            move_number += 0x100;
        }
        MoveLearnt {
            move_number,
            level: (flags & LEVEL_MASK) >> 1,
        }
    }

    fn read_moveset_at(&self, mut offset: i32) -> Vec<MoveLearnt> {
        let mut moves = Vec::new();
        while !self.is_moveset_terminator(offset) {
            moves.push(self.read_move_learnt(offset));
            offset += self.entry_size();
        }
        moves
    }

    pub(crate) fn read_moves_learnt(&self) -> Movesets {
        let mut movesets = Movesets::new();
        for id in self.real_pokemon() {
            let pointer = self.rom.pointer(self.moveset_pointer_location(id));
            let moves = if self.rom.contains(pointer) {
                self.read_moveset_at(pointer)
            } else {
                Vec::new()
            };
            movesets.insert(self.core.data.pk(id).number, moves);
        }
        movesets
    }

    fn write_move_learnt(&mut self, offset: i32, learnt: &MoveLearnt) -> i32 {
        if self.jambo_moveset_hack {
            self.rom.set_word(offset, learnt.move_number);
            self.rom.set_byte(offset + 2, learnt.level as u8);
            return JAMBO_ENTRY_SIZE;
        }
        self.rom.set_byte(offset, (learnt.move_number & 0xFF) as u8);
        let mut level_part = (learnt.level << 1) & LEVEL_MASK;
        if learnt.move_number > HIGH_MOVE_THRESHOLD {
            level_part += 1;
        }
        self.rom.set_byte(offset + 1, level_part as u8);
        STANDARD_ENTRY_SIZE
    }

    fn write_moveset_terminator(&mut self, offset: i32) {
        let terminator: &[u8] = if self.jambo_moveset_hack {
            &[0x00, 0x00, 0xFF, 0x00, 0x00, 0x00]
        } else {
            &[0xFF, 0xFF, 0x00, 0x00]
        };
        self.rom.write_bytes(offset, terminator);
    }

    fn current_move_count(&self, start: i32) -> i32 {
        let mut offset = start;
        while !self.is_moveset_terminator(offset) {
            offset += self.entry_size();
        }
        (offset - start) / self.entry_size()
    }

    fn write_moveset(&mut self, id: PokemonId, moves: &[MoveLearnt]) -> Result<()> {
        let pointer_location = self.moveset_pointer_location(id);
        let mut location = self.rom.pointer(pointer_location);
        let entry_size = self.entry_size();
        let current_count = if self.rom.contains(location) {
            self.current_move_count(location)
        } else {
            0
        };
        let new_count = moves.len() as i32;
        if new_count > current_count || !self.rom.contains(location) {
            let needed = new_count * entry_size + entry_size * 2;
            let free_space = self.value("FreeSpace");
            let space = self
                .rom
                .allocate(needed, free_space, true)
                .ok_or_else(|| RandomizationError::Io("ROM is full".to_owned()))?;
            self.rom.set_pointer(pointer_location, space);
            location = space;
        }
        for learnt in moves {
            location += self.write_move_learnt(location, learnt);
        }
        if new_count != current_count {
            self.write_moveset_terminator(location);
        }
        Ok(())
    }

    pub(crate) fn write_moves_learnt(&mut self, movesets: &Movesets) -> Result<()> {
        for id in self.real_pokemon() {
            let number = self.core.data.pk(id).number;
            let Some(moves) = movesets.get(&number) else {
                continue;
            };
            self.write_moveset(id, moves)?;
        }
        Ok(())
    }

    fn egg_move_words(&self) -> Vec<i32> {
        let mut offset = self.value("EggMoves");
        let mut words = Vec::new();
        while self.rom.contains_range(offset, 2) {
            let value = self.rom.word(offset);
            if value == c::EGG_MOVES_TERMINATOR {
                break;
            }
            words.push(value);
            offset += 2;
        }
        words
    }

    pub(crate) fn read_egg_moves(&self) -> EggMoves {
        let mut egg_moves: EggMoves = BTreeMap::new();
        let mut species = 0;
        let mut moves: Vec<i32> = Vec::new();
        for value in self.egg_move_words() {
            if value > c::EGG_MOVES_SPECIES_MARKER {
                if !moves.is_empty() {
                    egg_moves.insert(self.dex_of_internal(species), std::mem::take(&mut moves));
                }
                species = value - c::EGG_MOVES_SPECIES_MARKER;
            } else {
                moves.push(value);
            }
        }
        if !moves.is_empty() {
            egg_moves.insert(self.dex_of_internal(species), moves);
        }
        egg_moves
    }

    pub(crate) fn write_egg_moves(&mut self, egg_moves: &EggMoves) {
        let capacity = self.egg_move_words().len();
        let mut words: Vec<i32> = Vec::with_capacity(capacity);
        for (&species, moves) in egg_moves {
            let block_length = moves.len() + 1;
            if words.len() + block_length > capacity {
                break;
            }
            words.push(self.internal_of_number(species) + c::EGG_MOVES_SPECIES_MARKER);
            words.extend(moves.iter().copied());
        }
        let mut offset = self.value("EggMoves");
        for &word in &words {
            self.rom.set_word(offset, word);
            offset += 2;
        }
        if words.len() < capacity {
            self.rom.set_word(offset, c::EGG_MOVES_TERMINATOR);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_entries_pack_the_high_move_bit_with_the_level() {
        let learnt = MoveLearnt {
            move_number: 0x150,
            level: 42,
        };
        let level_part = ((learnt.level << 1) & LEVEL_MASK) + 1;
        assert_eq!(level_part & HIGH_MOVE_FLAG, 1);
        assert_eq!((level_part & LEVEL_MASK) >> 1, 42);
        assert_eq!(0x150 & 0xFF, 0x50);
    }
}
