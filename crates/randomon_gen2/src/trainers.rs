use randomon_core::{RandomizationError, Result};
use randomon_data::{PokemonId, Trainer, TrainerPokemon};
use randomon_gb::{bank_of, calculate_offset};
use randomon_settings::java_text::java_trim;

use crate::constants::tag_trainers;
use crate::handler::Gen2RomHandler;

const POKEMON_TERMINATOR: u8 = 0xFF;

fn has_items(data_type: i32) -> bool {
    data_type == 2 || data_type == 3
}

fn has_moves(data_type: i32) -> bool {
    data_type % 2 == 1
}

impl Gen2RomHandler {
    fn trainer_class_amount(&self) -> i32 {
        self.value("TrainerClassAmount")
    }

    fn trainer_class_limits(&self) -> &'static [i32] {
        self.array("TrainerDataClassCounts")
    }

    fn trainer_pointers(&self) -> Vec<i32> {
        let table = self.value("TrainerDataTableOffset");
        (0..self.trainer_class_amount())
            .map(|class| calculate_offset(bank_of(table), self.word(table + class * 2)))
            .collect()
    }

    fn class_limit(&self, class: i32) -> Result<i32> {
        self.trainer_class_limits()
            .get(class as usize)
            .copied()
            .ok_or(RandomizationError::IndexOutOfBounds("trainerclasslimits"))
    }

    fn loaded_pokemon_at(&self, offset: i32) -> Result<PokemonId> {
        let id = self.pokemon_at(offset);
        if self.core.data.exists(id) {
            Ok(id)
        } else {
            Err(RandomizationError::IndexOutOfBounds("pokes[rom[offs + 1]]"))
        }
    }

    fn read_trainer_pokemon(&self, offset: &mut i32, data_type: i32) -> Result<TrainerPokemon> {
        let mut tp = TrainerPokemon::new(self.loaded_pokemon_at(*offset + 1)?, self.int(*offset));
        *offset += 2;
        if (data_type & 2) == 2 {
            tp.held_item = self.int(*offset);
            *offset += 1;
        }
        if (data_type & 1) == 1 {
            for (slot, mv) in tp.moves.iter_mut().enumerate() {
                *mv = self.int(*offset + slot as i32);
            }
            *offset += 4;
        }
        Ok(tp)
    }

    pub(crate) fn read_trainers(&self) -> Result<Vec<Trainer>> {
        let pointers = self.trainer_pointers();
        let class_names = self.read_trainer_class_names();
        let mut trainers = Vec::new();
        let mut index = 0;
        for class in 0..self.trainer_class_amount() {
            let mut offset = pointers[class as usize];
            for _ in 0..self.class_limit(class)? {
                index += 1;
                let mut trainer = Trainer::new();
                trainer.offset = offset;
                trainer.index = index;
                trainer.trainerclass = class;
                let name = self.read_variable_string(offset);
                let class_name = class_names
                    .get(class as usize)
                    .ok_or(RandomizationError::IndexOutOfBounds("tcnames"))?;
                trainer.full_display_name = Some(format!("{class_name} {name}"));
                trainer.name = Some(name);
                offset += self.string_length_at(offset) + 1;
                let data_type = self.int(offset);
                trainer.poketype = data_type;
                offset += 1;
                while self.byte(offset) != POKEMON_TERMINATOR {
                    let tp = self.read_trainer_pokemon(&mut offset, data_type)?;
                    trainer.pokemon.push(tp);
                }
                trainers.push(trainer);
                offset += 1;
            }
        }
        tag_trainers(&mut trainers);
        Ok(trainers)
    }

    pub(crate) fn write_trainers(&mut self, trainers: &[Trainer]) -> Result<()> {
        self.store_trainers(trainers)
    }

    fn skip_trainer_pokemon(&self, mut offset: i32, data_type: i32) -> i32 {
        while self.byte(offset) != POKEMON_TERMINATOR {
            offset += 2;
            if has_items(data_type) {
                offset += 1;
            }
            if has_moves(data_type) {
                offset += 4;
            }
        }
        offset + 1
    }

    pub(crate) fn read_trainer_names(&self) -> Result<Vec<String>> {
        let pointers = self.trainer_pointers();
        let mut names = Vec::new();
        for class in 0..self.trainer_class_amount() {
            let mut offset = pointers[class as usize];
            for _ in 0..self.class_limit(class)? {
                names.push(self.read_variable_string(offset));
                offset += self.string_length_at(offset) + 1;
                let data_type = self.int(offset);
                offset = self.skip_trainer_pokemon(offset + 1, data_type);
            }
        }
        Ok(names)
    }

    pub(crate) fn write_trainer_names(&mut self, names: &[String]) -> Result<()> {
        if self.value("CanChangeTrainerText") == 0 {
            return Ok(());
        }
        let mut trainers = self.read_trainers()?;
        for (trainer, name) in trainers.iter_mut().zip(names) {
            let stripped = name.replace('9', "");
            trainer.name = Some(java_trim(&stripped).to_owned());
        }
        self.store_trainers(&trainers)
    }

    pub(crate) fn read_trainer_class_names(&self) -> Vec<String> {
        let mut offset = self.value("TrainerClassNamesOffset");
        let mut names = Vec::new();
        for _ in 0..self.trainer_class_amount() {
            names.push(self.read_variable_string(offset));
            offset += self.string_length_at(offset) + 1;
        }
        names
    }

    pub(crate) fn write_trainer_class_names(&mut self, names: &[String]) {
        if self.value("CanChangeTrainerText") == 0 {
            return;
        }
        let mut offset = self.value("TrainerClassNamesOffset");
        let mut names = names.iter();
        for _ in 0..self.trainer_class_amount() {
            let length = self.string_length_at(offset) + 1;
            let Some(name) = names.next() else {
                return;
            };
            self.write_fixed_string(name, offset, length);
            offset += length;
        }
    }

    pub(crate) fn class_name_lengths_by_trainer(&self) -> Vec<i32> {
        let class_names = self.read_trainer_class_names();
        let mut lengths = Vec::new();
        for (class, name) in class_names.iter().enumerate() {
            let length = self.translate(name).len() as i32;
            let count = self.trainer_class_limits().get(class).copied().unwrap_or(0);
            lengths.extend(std::iter::repeat_n(length, count.max(0) as usize));
        }
        lengths
    }
}
