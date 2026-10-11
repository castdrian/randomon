use randomon_core::{EggMoves, Movesets, RandomizationError, Result};
use randomon_data::constants::gb;
use randomon_data::{EvolutionType, MoveLearnt};
use randomon_gb::{bank_of, calculate_offset, make_gb_pointer, write_word};

use crate::constants::POKEMON_COUNT;
use crate::handler::{Gen2RomHandler, pokemon};

const VIET_CRYSTAL_DATA_END: i32 = 0x43E00;

struct DataBlock {
    bytes: Vec<u8>,
    used: usize,
}

impl DataBlock {
    fn new(size: i32) -> Self {
        Self {
            bytes: vec![0; size.max(0) as usize],
            used: 0,
        }
    }

    fn push(&mut self, value: u8) -> Result<()> {
        let slot = self
            .bytes
            .get_mut(self.used)
            .ok_or(RandomizationError::IndexOutOfBounds("dataBlock"))?;
        *slot = value;
        self.used += 1;
        Ok(())
    }
}

fn evolution_condition_bytes(kind: EvolutionType, extra_info: i32) -> Vec<u8> {
    match kind {
        EvolutionType::Level | EvolutionType::Stone | EvolutionType::TradeItem => {
            vec![extra_info as u8]
        }
        EvolutionType::Trade => vec![0xFF],
        EvolutionType::Happiness => vec![0x01],
        EvolutionType::HappinessDay => vec![0x02],
        EvolutionType::HappinessNight => vec![0x03],
        EvolutionType::LevelAttackHigher => vec![extra_info as u8, 0x01],
        EvolutionType::LevelDefenseHigher => vec![extra_info as u8, 0x02],
        EvolutionType::LevelAtkDefSame => vec![extra_info as u8, 0x03],
        _ => Vec::new(),
    }
}

impl Gen2RomHandler {
    fn evolution_entry_length(&self, offset: i32) -> i32 {
        if self.byte(offset) == 5 { 4 } else { 3 }
    }

    fn skip_evolutions(&self, mut offset: i32) -> i32 {
        while self.byte(offset) != 0 {
            offset += self.evolution_entry_length(offset);
        }
        offset + 1
    }

    fn copy_old_evolutions(&self, old_offset: i32, block: &mut DataBlock) -> Result<bool> {
        let mut offset = old_offset;
        let mut written = false;
        while self.byte(offset) != 0 {
            for _ in 0..self.evolution_entry_length(offset) {
                block.push(self.byte(offset))?;
                offset += 1;
            }
            written = true;
        }
        Ok(written)
    }

    fn write_new_evolutions(&self, number: i32, block: &mut DataBlock) -> Result<bool> {
        let data = &self.core.data;
        let mut written = false;
        for &evo in data.evolutions_from(pokemon(number)) {
            let evolution = data.evo(evo);
            block.push(evolution.kind.to_index(2) as u8)?;
            for byte in evolution_condition_bytes(evolution.kind, evolution.extra_info) {
                block.push(byte)?;
            }
            block.push(data.pk(evolution.to).number as u8)?;
            written = true;
        }
        Ok(written)
    }

    fn copy_old_moves(&self, old_offset: i32, block: &mut DataBlock) -> Result<()> {
        let mut offset = self.skip_evolutions(old_offset);
        while self.byte(offset) != 0 {
            block.push(self.byte(offset))?;
            block.push(self.byte(offset + 1))?;
            offset += 2;
        }
        Ok(())
    }

    fn write_new_moves(moves: &[MoveLearnt], block: &mut DataBlock) -> Result<()> {
        for learnt in moves {
            block.push(learnt.level as u8)?;
            block.push(learnt.move_number as u8)?;
        }
        Ok(())
    }

    pub(crate) fn write_evos_and_moves_learnt(
        &mut self,
        write_evos: bool,
        movesets: Option<&Movesets>,
    ) -> Result<()> {
        let start = self.value("PokemonMovesetsTableOffset");
        let bank = bank_of(start);
        let mut pointer_table = vec![0u8; (POKEMON_COUNT * 2) as usize];
        let start_of_next_bank = if self.is_viet_crystal {
            VIET_CRYSTAL_DATA_END
        } else {
            ((start / gb::BANK_SIZE) + 1) * gb::BANK_SIZE
        };
        let block_offset = start + pointer_table.len() as i32;
        let mut block = DataBlock::new(start_of_next_bank - block_offset);
        for number in 1..=POKEMON_COUNT {
            let old_offset = calculate_offset(bank, self.word(start + (number - 1) * 2));
            let mut entry_start = block_offset + block.used as i32;
            let evolutions_written = if write_evos {
                self.write_new_evolutions(number, &mut block)?
            } else {
                self.copy_old_evolutions(old_offset, &mut block)?
            };
            if !evolutions_written && entry_start != block_offset {
                entry_start -= 1;
            } else {
                block.push(0)?;
            }
            write_word(
                &mut pointer_table,
                (number - 1) * 2,
                make_gb_pointer(entry_start),
            );
            match movesets {
                None => self.copy_old_moves(old_offset, &mut block)?,
                Some(movesets) => {
                    let moves = movesets
                        .get(&number)
                        .ok_or(RandomizationError::NullPointer("movesets.get"))?;
                    Self::write_new_moves(moves, &mut block)?;
                }
            }
            block.push(0)?;
        }
        self.write_bytes(start, &pointer_table);
        self.write_bytes(block_offset, &block.bytes);
        Ok(())
    }

    pub(crate) fn read_moves_learnt(&self) -> Movesets {
        let mut movesets = Movesets::new();
        for number in 1..=POKEMON_COUNT {
            let mut offset = self.skip_evolutions(self.moveset_pointer(number));
            let mut moves = Vec::new();
            while self.byte(offset) != 0 {
                moves.push(MoveLearnt {
                    level: self.int(offset),
                    move_number: self.int(offset + 1),
                });
                offset += 2;
            }
            movesets.insert(number, moves);
        }
        movesets
    }

    fn egg_move_offset(&self, number: i32) -> i32 {
        let table = self.value("EggMovesTableOffset");
        let base = (table / 0x1000) * 0x1000;
        base + (self.word(table + (number - 1) * 2) % 0x1000)
    }

    pub(crate) fn read_egg_moves(&self) -> EggMoves {
        let mut egg_moves = EggMoves::new();
        for number in 1..=POKEMON_COUNT {
            let mut offset = self.egg_move_offset(number);
            let mut moves = Vec::new();
            while self.byte(offset) != 0xFF {
                moves.push(self.int(offset));
                offset += 1;
            }
            if !moves.is_empty() {
                egg_moves.insert(number, moves);
            }
        }
        egg_moves
    }

    pub(crate) fn write_egg_moves(&mut self, egg_moves: &EggMoves) {
        for number in 1..=POKEMON_COUNT {
            let mut offset = self.egg_move_offset(number);
            if let Some(moves) = egg_moves.get(&number) {
                for &move_number in moves {
                    self.set_byte(offset, move_number as u8);
                    offset += 1;
                }
            }
        }
    }
}
