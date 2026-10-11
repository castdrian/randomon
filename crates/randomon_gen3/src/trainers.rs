use randomon_core::{Movesets, RandomizationError, Result};
use randomon_data::{Trainer, TrainerPokemon, moves_at_level};

use crate::constants as c;
use crate::handler::Gen3RomHandler;
use crate::rom_entry::RomType;

const TRAINER_NAME_OFFSET: i32 = 4;
const GENDER_FLAG: i32 = 0x80;
const MAX_TRAINER_IV_BYTE: i32 = 255;
const MAX_IV: i32 = 31;
const DOUBLE_BATTLE: u8 = 0x01;
const SMALL_POKEMON_BLOCK: i32 = 8;
const LARGE_POKEMON_BLOCK: i32 = 16;

fn block_size(poketype: i32) -> i32 {
    if poketype & 1 == 1 {
        LARGE_POKEMON_BLOCK
    } else {
        SMALL_POKEMON_BLOCK
    }
}

fn iv_byte(ivs: i32) -> i32 {
    MAX_TRAINER_IV_BYTE.min(1 + (ivs * MAX_TRAINER_IV_BYTE) / MAX_IV)
}

struct TrainerLayout {
    base: i32,
    count: i32,
    entry_size: i32,
}

impl TrainerLayout {
    fn offset(&self, index: i32) -> i32 {
        self.base + index * self.entry_size
    }

    fn type_offset(&self, index: i32) -> i32 {
        self.offset(index)
    }

    fn battle_type_offset(&self, index: i32) -> i32 {
        self.offset(index) + self.entry_size - 16
    }

    fn team_size_offset(&self, index: i32) -> i32 {
        self.offset(index) + self.entry_size - 8
    }

    fn team_pointer_offset(&self, index: i32) -> i32 {
        self.offset(index) + self.entry_size - 4
    }
}

impl Gen3RomHandler {
    fn trainer_layout(&self) -> TrainerLayout {
        TrainerLayout {
            base: self.value("TrainerData"),
            count: self.value("TrainerCount"),
            entry_size: self.value("TrainerEntrySize"),
        }
    }

    fn read_trainer_pokemon(&self, pointer: i32, poketype: i32, slot: i32) -> TrainerPokemon {
        let block = pointer + slot * block_size(poketype);
        let species = self.pokemon_or_fallback(self.rom.word(block + 4));
        let mut tp = TrainerPokemon::new(species, self.rom.word(block + 2));
        tp.ivs = ((self.rom.word(block) & 0xFF) * MAX_IV) / MAX_TRAINER_IV_BYTE;
        let moves_start = match poketype {
            2 => {
                tp.held_item = self.rom.word(block + 6);
                None
            }
            1 => Some(6),
            3 => {
                tp.held_item = self.rom.word(block + 6);
                Some(8)
            }
            _ => None,
        };
        if let Some(start) = moves_start {
            for (index, mv) in tp.moves.iter_mut().enumerate() {
                *mv = self.rom.word(block + start + index as i32 * 2);
            }
        }
        tp
    }

    fn read_trainer(&self, layout: &TrainerLayout, index: i32, class_names: &[String]) -> Trainer {
        let offset = layout.offset(index);
        let mut trainer = Trainer::new();
        trainer.offset = offset;
        trainer.index = index;
        let class = self.rom.int(offset + 1);
        trainer.trainerclass = i32::from(self.rom.int(offset + 2) & GENDER_FLAG > 0);
        let poketype = self.rom.int(layout.type_offset(index));
        let team_size = self.rom.int(layout.team_size_offset(index));
        let pointer = self.rom.pointer(layout.team_pointer_offset(index));
        trainer.poketype = poketype;
        let name = self.read_variable_string(offset + TRAINER_NAME_OFFSET);
        let class_name = class_names.get(class as usize).cloned().unwrap_or_default();
        trainer.full_display_name = Some(format!("{class_name} {name}"));
        trainer.name = Some(name);
        if (0..=3).contains(&poketype) {
            trainer.pokemon = (0..team_size)
                .map(|slot| self.read_trainer_pokemon(pointer, poketype, slot))
                .collect();
        }
        trainer
    }

    fn read_mossdeep_steven(&self, trainers: &[Trainer], index: i32) -> Trainer {
        let offset = self.value("MossdeepStevenTeamOffset");
        let mut steven = Trainer::new();
        steven.offset = offset;
        steven.index = index;
        steven.poketype = 1;
        if let Some(meteor_falls) = trainers.get(c::EM_METEOR_FALLS_STEVEN_INDEX as usize - 1) {
            steven.trainerclass = meteor_falls.trainerclass;
            steven.name = meteor_falls.name.clone();
            steven.full_display_name = meteor_falls.full_display_name.clone();
        }
        for slot in 0..c::MOSSDEEP_STEVEN_TEAM_SIZE as i32 {
            let entry = offset + slot * c::MOSSDEEP_STEVEN_POKEMON_SIZE;
            let species = self.pokemon_or_fallback(self.rom.word(entry));
            let mut tp = TrainerPokemon::new(species, self.rom.signed(entry + 3));
            tp.ivs = self.rom.signed(entry + 2);
            for (index, mv) in tp.moves.iter_mut().enumerate() {
                *mv = self.rom.word(entry + 12 + index as i32 * 2);
            }
            steven.pokemon.push(tp);
        }
        steven
    }

    fn has_mossdeep_steven(&self) -> bool {
        self.rom_type().is_emerald() && self.value("MossdeepStevenTeamOffset") > 0
    }

    pub(crate) fn read_trainers(&self) -> Vec<Trainer> {
        let layout = self.trainer_layout();
        let class_names = self.read_trainer_class_names();
        let mut trainers: Vec<Trainer> = (1..layout.count)
            .map(|index| self.read_trainer(&layout, index, &class_names))
            .collect();
        if self.has_mossdeep_steven() {
            let steven = self.read_mossdeep_steven(&trainers, layout.count);
            trainers.push(steven);
        }
        match self.rom_type() {
            RomType::Ruby | RomType::Sapphire => c::trainer_tags_rs(&mut trainers, self.rom_type()),
            RomType::Emerald => {
                c::trainer_tags_e(&mut trainers);
                c::set_multi_battle_status_em(&mut trainers);
            }
            RomType::FireRedLeafGreen => c::trainer_tags_frlg(&mut trainers),
        }
        trainers
    }

    fn team_moves(&self, tp: &TrainerPokemon, movesets: &Movesets) -> [i32; 4] {
        if !tp.reset_moves {
            return tp.moves;
        }
        let number = self.core.data.pk(tp.pokemon).number;
        movesets
            .get(&number)
            .map_or([0; 4], |moveset| moves_at_level(moveset, tp.level, 0))
    }

    fn write_trainer_pokemon(
        &mut self,
        trainer: &Trainer,
        tp: &TrainerPokemon,
        block: i32,
        movesets: &Movesets,
    ) {
        self.rom.set_word(block, iv_byte(tp.ivs));
        self.rom.set_word(block + 2, tp.level);
        let species = self.internal_of(tp.pokemon);
        self.rom.set_word(block + 4, species);
        if !trainer.pokemon_have_custom_moves() {
            let item = if trainer.pokemon_have_items() {
                tp.held_item
            } else {
                0
            };
            self.rom.set_word(block + 6, item);
            return;
        }
        let moves_start = if trainer.pokemon_have_items() {
            self.rom.set_word(block + 6, tp.held_item);
            8
        } else {
            self.rom.set_word(block + 14, 0);
            6
        };
        let moves = self.team_moves(tp, movesets);
        for (index, mv) in moves.into_iter().enumerate() {
            self.rom
                .set_word(block + moves_start + index as i32 * 2, mv);
        }
    }

    fn team_pointer_for(
        &mut self,
        layout: &TrainerLayout,
        index: i32,
        old_size: i32,
        new_size: i32,
    ) -> Result<i32> {
        if new_size <= old_size {
            return Ok(self.rom.pointer(layout.team_pointer_offset(index)));
        }
        let free_space = self.value("FreeSpace");
        let space = self
            .rom
            .allocate(new_size, free_space, true)
            .ok_or_else(|| RandomizationError::Io("ROM is full".to_owned()))?;
        self.rom
            .set_pointer(layout.team_pointer_offset(index), space);
        Ok(space)
    }

    fn write_trainer(
        &mut self,
        layout: &TrainerLayout,
        index: i32,
        trainer: &Trainer,
        double_battle_mode: bool,
        movesets: &Movesets,
    ) -> Result<()> {
        let old_type = self.rom.int(layout.type_offset(index));
        let old_count = self.rom.int(layout.team_size_offset(index));
        let new_count = trainer.pokemon.len() as i32;
        let new_size = new_count * block_size(trainer.poketype);
        let old_size = old_count * block_size(old_type);
        self.rom
            .set_byte(layout.type_offset(index), trainer.poketype as u8);
        self.rom
            .set_byte(layout.team_size_offset(index), new_count as u8);
        if double_battle_mode && !trainer.skip_important() {
            self.rom
                .set_byte(layout.battle_type_offset(index), DOUBLE_BATTLE);
        }
        let pointer = self.team_pointer_for(layout, index, old_size, new_size)?;
        let block = block_size(trainer.poketype);
        for (slot, tp) in trainer.pokemon.iter().enumerate() {
            self.write_trainer_pokemon(trainer, tp, pointer + slot as i32 * block, movesets);
        }
        Ok(())
    }

    fn write_mossdeep_steven(&mut self, steven: &Trainer) {
        let offset = self.value("MossdeepStevenTeamOffset");
        for (slot, tp) in steven
            .pokemon
            .iter()
            .take(c::MOSSDEEP_STEVEN_TEAM_SIZE)
            .enumerate()
        {
            let entry = offset + slot as i32 * c::MOSSDEEP_STEVEN_POKEMON_SIZE;
            let species = self.internal_of(tp.pokemon);
            self.rom.set_word(entry, species);
            self.rom.set_byte(entry + 2, tp.ivs as u8);
            self.rom.set_byte(entry + 3, tp.level as u8);
            for (index, &mv) in tp.moves.iter().enumerate() {
                self.rom.set_word(entry + 12 + index as i32 * 2, mv);
            }
        }
    }

    pub(crate) fn write_trainers(
        &mut self,
        trainers: &[Trainer],
        double_battle_mode: bool,
    ) -> Result<()> {
        let layout = self.trainer_layout();
        let movesets = self.read_moves_learnt();
        let regular = (layout.count - 1).max(0) as usize;
        if trainers.len() < regular {
            return Err(RandomizationError::IndexOutOfBounds("trainerData"));
        }
        for (position, trainer) in trainers.iter().take(regular).enumerate() {
            self.write_trainer(
                &layout,
                position as i32 + 1,
                trainer,
                double_battle_mode,
                &movesets,
            )?;
        }
        if self.has_mossdeep_steven()
            && let Some(steven) = trainers.get(regular)
        {
            self.write_mossdeep_steven(steven);
        }
        Ok(())
    }

    pub(crate) fn read_trainer_names(&self) -> Vec<String> {
        let layout = self.trainer_layout();
        (1..layout.count)
            .map(|index| self.read_variable_string(layout.offset(index) + TRAINER_NAME_OFFSET))
            .collect()
    }

    pub(crate) fn write_trainer_names(&mut self, names: &[String]) {
        let layout = self.trainer_layout();
        let length = self.value("TrainerNameLength");
        for (position, name) in names
            .iter()
            .take((layout.count - 1).max(0) as usize)
            .enumerate()
        {
            let offset = layout.offset(position as i32 + 1) + TRAINER_NAME_OFFSET;
            self.write_fixed_string(name, offset, length);
        }
    }

    pub(crate) fn read_trainer_class_names(&self) -> Vec<String> {
        let base = self.value("TrainerClassNames");
        let length = self.value("TrainerClassNameLength");
        (0..self.value("TrainerClassCount"))
            .map(|index| self.read_variable_string(base + index * length))
            .collect()
    }

    pub(crate) fn write_trainer_class_names(&mut self, names: &[String]) {
        let base = self.value("TrainerClassNames");
        let length = self.value("TrainerClassNameLength");
        let count = self.value("TrainerClassCount").max(0) as usize;
        for (index, name) in names.iter().take(count).enumerate() {
            self.write_fixed_string(name, base + index as i32 * length, length);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iv_bytes_round_trip_through_the_upstream_scale() {
        for ivs in 0..=31 {
            let byte = iv_byte(ivs);
            assert!(byte <= 255);
            assert_eq!(((byte & 0xFF) * MAX_IV) / MAX_TRAINER_IV_BYTE, ivs);
        }
        assert_eq!(iv_byte(0), 1);
        assert_eq!(iv_byte(31), 255);
    }

    #[test]
    fn block_sizes_follow_the_custom_moves_bit() {
        assert_eq!(block_size(0), 8);
        assert_eq!(block_size(1), 16);
        assert_eq!(block_size(2), 8);
        assert_eq!(block_size(3), 16);
    }
}
