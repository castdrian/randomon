use std::collections::BTreeMap;

use randomon_core::{RandomizationError, Result};
use randomon_ctr::bytes::{read_word, write_word};
use randomon_data::{Trainer, TrainerPokemon, moves_at_level};
use randomon_formats::ctr::Decompression;

use crate::constants;
use crate::handler::{Gen6RomHandler, io};
use crate::pokemon::{garc_file, garc_file_mut};

fn signed(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset] as i8)
}

impl Gen6RomHandler {
    pub(crate) fn read_trainers(&self) -> Result<Vec<Trainer>> {
        let trainers_garc = self.read_garc(self.file("TrainerData"), Decompression::Skip)?;
        let pokemon_garc = self.read_garc(self.file("TrainerPokemon"), Decompression::Skip)?;
        let class_names = self.class_names()?;
        let names: BTreeMap<usize, String> =
            self.trainer_name_list()?.into_iter().enumerate().collect();
        let mut trainers = Vec::new();
        for index in 1..trainers_garc.files.len() {
            let trainer_data = garc_file(&trainers_garc, index)?;
            let pokemon_data = garc_file(&pokemon_garc, index)?;
            let mut trainer = self.parse_trainer(index, trainer_data, &class_names, &names)?;
            self.parse_trainer_pokemon(&mut trainer, trainer_data, pokemon_data);
            trainers.push(trainer);
        }
        if self.is_xy() {
            constants::tag_trainers_xy(&mut trainers);
            constants::set_multi_battle_status_xy(&mut trainers);
        } else {
            constants::tag_trainers_oras(&mut trainers);
            constants::set_multi_battle_status_oras(&mut trainers);
        }
        Ok(trainers)
    }

    fn parse_trainer(
        &self,
        index: usize,
        data: &[u8],
        class_names: &[String],
        names: &BTreeMap<usize, String>,
    ) -> Result<Trainer> {
        let oras = self.is_oras();
        let mut trainer = Trainer::new();
        trainer.poketype = if oras {
            read_word(data, 0)
        } else {
            i32::from(data[0])
        };
        trainer.index = index as i32;
        trainer.trainerclass = if oras {
            read_word(data, 2)
        } else {
            i32::from(data[1])
        };
        let class_name = class_names
            .get(trainer.trainerclass as usize)
            .ok_or(RandomizationError::IndexOutOfBounds("trainer classes"))?;
        let name = names.get(&(index - 1)).map_or("UNKNOWN", String::as_str);
        trainer.full_display_name = Some(format!("{class_name} {name}"));
        Ok(trainer)
    }

    fn parse_trainer_pokemon(&self, trainer: &mut Trainer, trainer_data: &[u8], data: &[u8]) {
        let oras = self.is_oras();
        let offset = if oras { 6 } else { 2 };
        let count = trainer_data[offset + 1];
        let mut position = 0usize;
        for _ in 0..count {
            let level = read_word(data, position + 2);
            let species = read_word(data, position + 4);
            let forme = read_word(data, position + 6);
            let mut pokemon = TrainerPokemon::new(self.pk(species), level);
            pokemon.strength = signed(data, position);
            pokemon.ivs = if oras {
                pokemon.strength * 31 / 255
            } else {
                pokemon.strength & 0x1F
            };
            let ability_and_flag = signed(data, position + 1);
            pokemon.ability_slot = ((ability_and_flag as u32) >> 4) as i32 & 0xF;
            pokemon.forced_gender_flag = ability_and_flag & 0xF;
            pokemon.forme = forme;
            pokemon.forme_suffix = constants::forme_suffix_by_base_forme(species, forme).to_owned();
            position += 8;
            if trainer.pokemon_have_items() {
                pokemon.held_item = read_word(data, position);
                position += 2;
                pokemon.has_mega_stone = constants::is_mega_stone(pokemon.held_item);
            }
            if trainer.pokemon_have_custom_moves() {
                for slot in 0..4 {
                    pokemon.moves[slot] = read_word(data, position + slot * 2);
                }
                position += 8;
            }
            trainer.pokemon.push(pokemon);
        }
    }

    pub(crate) fn write_trainers(
        &mut self,
        trainers: &[Trainer],
        double_battle_mode: bool,
    ) -> Result<()> {
        let data_path = self.file("TrainerData");
        let pokemon_path = self.file("TrainerPokemon");
        let mut trainers_garc = self.read_garc(data_path, Decompression::Skip)?;
        let mut pokemon_garc = self.read_garc(pokemon_path, Decompression::Skip)?;
        let movesets = self.read_movesets()?;
        let mut iterator = trainers.iter();
        for index in 1..trainers_garc.files.len() {
            let trainer = iterator.next().ok_or(RandomizationError::randomization(
                "java.util.NoSuchElementException",
            ))?;
            let data = garc_file_mut(&mut trainers_garc, index)?;
            self.write_trainer_header(data, trainer, double_battle_mode);
            let encoded = self.encode_trainer_pokemon(trainer, &movesets)?;
            pokemon_garc.set_file(index, encoded).map_err(io)?;
        }
        self.write_garc(data_path, &trainers_garc)?;
        self.write_garc(pokemon_path, &pokemon_garc)
    }

    fn write_trainer_header(&self, data: &mut [u8], trainer: &Trainer, double_battle_mode: bool) {
        let offset = if self.is_oras() {
            write_word(data, 0, trainer.poketype);
            4
        } else {
            data[0] = trainer.poketype as u8;
            0
        };
        data[offset + 3] = trainer.pokemon.len() as u8;
        if double_battle_mode && !trainer.skip_important() && data[offset + 2] == 0 {
            data[offset + 2] = 1;
            data[offset + 12] |= 0x80;
        }
    }

    fn encode_trainer_pokemon(
        &self,
        trainer: &Trainer,
        movesets: &randomon_core::Movesets,
    ) -> Result<Vec<u8>> {
        let count = trainer.pokemon.len();
        let mut bytes_needed = 8 * count;
        if trainer.pokemon_have_custom_moves() {
            bytes_needed += 8 * count;
        }
        if trainer.pokemon_have_items() {
            bytes_needed += 2 * count;
        }
        let mut data = vec![0u8; bytes_needed];
        let mut position = 0usize;
        for pokemon in &trainer.pokemon {
            data[position] = pokemon.strength as u8;
            data[position + 1] = ((pokemon.ability_slot << 4) | pokemon.forced_gender_flag) as u8;
            write_word(&mut data, position + 2, pokemon.level);
            write_word(&mut data, position + 4, pokemon.pokemon.number());
            write_word(&mut data, position + 6, pokemon.forme);
            position += 8;
            if trainer.pokemon_have_items() {
                write_word(&mut data, position, pokemon.held_item);
                position += 2;
            }
            if trainer.pokemon_have_custom_moves() {
                let moves = if pokemon.reset_moves {
                    let alternate = self.alt_forme_of(pokemon.pokemon, pokemon.forme);
                    let movepool = movesets
                        .get(&alternate.number())
                        .ok_or(RandomizationError::NullPointer("movesets.get"))?;
                    moves_at_level(movepool, pokemon.level, 0)
                } else {
                    pokemon.moves
                };
                for (slot, &move_number) in moves.iter().enumerate() {
                    write_word(&mut data, position + slot * 2, move_number);
                }
                position += 8;
            }
        }
        Ok(data)
    }

    pub(crate) fn class_names(&self) -> Result<Vec<String>> {
        self.strings(false, self.entry.int("TrainerClassesTextOffset"))
    }

    pub(crate) fn trainer_name_list(&self) -> Result<Vec<String>> {
        let mut names = self.strings(false, self.entry.int("TrainerNamesTextOffset"))?;
        if names.is_empty() {
            return Err(RandomizationError::IndexOutOfBounds("trainer names"));
        }
        names.remove(0);
        Ok(names)
    }

    pub(crate) fn alt_forme_of(
        &self,
        pokemon: randomon_data::PokemonId,
        forme: i32,
    ) -> randomon_data::PokemonId {
        let number = self.absolute_poke_num(pokemon.number(), forme);
        if number != 0 {
            self.pk(number)
        } else {
            pokemon
        }
    }
}
