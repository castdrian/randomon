use std::collections::BTreeMap;

use randomon_core::{Movesets, RandomizationError, Result};
use randomon_ctr::bytes::{find_after_prefix, read_long, read_word, write_long, write_word};
use randomon_data::{PokemonId, Trainer, TrainerPokemon, moves_at_level};
use randomon_formats::ctr::Decompression;
use randomon_java::JavaRng;

use crate::constants;
use crate::handler::{Gen7RomHandler, io};
use crate::pokemon::{garc_file, garc_file_mut};

const TRAINER_POKEMON_SIZE: usize = 32;

fn signed(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset] as i8)
}

fn no_such_element() -> RandomizationError {
    RandomizationError::randomization("java.util.NoSuchElementException")
}

impl Gen7RomHandler {
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
            let mut trainer = Trainer::new();
            trainer.poketype = i32::from(trainer_data[13]);
            trainer.index = index as i32;
            trainer.trainerclass = i32::from(trainer_data[0]);
            let class_name = class_names
                .get(trainer.trainerclass as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("trainer classes"))?;
            let name = names.get(&(index - 1)).map_or("UNKNOWN", String::as_str);
            trainer.full_display_name = Some(format!("{class_name} {name}"));
            let count = trainer_data[3] as usize;
            for slot in 0..count {
                let pokemon = self.parse_trainer_pokemon(pokemon_data, slot, index, &trainer)?;
                trainer.pokemon.push(pokemon);
            }
            trainers.push(trainer);
        }
        if self.is_sm() {
            constants::tag_trainers_sm(&mut trainers);
            constants::set_multi_battle_status_sm(&mut trainers);
        } else {
            constants::tag_trainers_usum(&mut trainers);
            constants::set_multi_battle_status_usum(&mut trainers);
            constants::set_forced_rival_starter_positions_usum(&mut trainers);
        }
        Ok(trainers)
    }

    fn parse_trainer_pokemon(
        &self,
        data: &[u8],
        slot: usize,
        trainer_index: usize,
        trainer: &Trainer,
    ) -> Result<TrainerPokemon> {
        let offset = slot * TRAINER_POKEMON_SIZE;
        let ability_and_flag = signed(data, offset);
        let species = read_word(data, offset + 16);
        let forme = read_word(data, offset + 18);
        let mut pokemon = TrainerPokemon::new(self.pk(species), read_word(data, offset + 14));
        pokemon.ability_slot = ((ability_and_flag as u32) >> 4) as i32 & 0xF;
        pokemon.forced_gender_flag = ability_and_flag & 0xF;
        pokemon.nature = data[offset + 1] as i8;
        pokemon.hp_evs = data[offset + 2] as i8;
        pokemon.atk_evs = data[offset + 3] as i8;
        pokemon.def_evs = data[offset + 4] as i8;
        pokemon.spatk_evs = data[offset + 5] as i8;
        pokemon.spdef_evs = data[offset + 6] as i8;
        pokemon.speed_evs = data[offset + 7] as i8;
        pokemon.ivs = read_long(data, offset + 8);
        if self.is_usum()
            && trainer_index == 78
            && slot == 3
            && pokemon.level == 16
            && trainer
                .pokemon
                .first()
                .ok_or(RandomizationError::IndexOutOfBounds("tr.pokemon"))?
                .level
                == 16
        {
            pokemon.level = 14;
        }
        pokemon.forme = forme;
        pokemon.forme_suffix = constants::forme_suffix_by_base_forme(species, forme).to_owned();
        pokemon.held_item = read_word(data, offset + 20);
        pokemon.has_mega_stone = constants::is_mega_stone(pokemon.held_item);
        pokemon.has_z_crystal = constants::is_z_crystal(pokemon.held_item);
        for move_slot in 0..4 {
            pokemon.moves[move_slot] = read_word(data, offset + 24 + move_slot * 2);
        }
        Ok(pokemon)
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
            let trainer = iterator.next().ok_or_else(no_such_element)?;
            let header = garc_file_mut(&mut trainers_garc, index)?;
            header[13] = trainer.poketype as u8;
            header[3] = trainer.pokemon.len() as u8;
            if double_battle_mode && !trainer.skip_important() && header[2] == 0 {
                header[2] = 1;
                header[12] |= 0x8;
            }
            let encoded = self.encode_trainer_pokemon(trainer, &movesets)?;
            pokemon_garc.set_file(index, encoded).map_err(io)?;
        }
        self.write_garc(data_path, &trainers_garc)?;
        self.write_garc(pokemon_path, &pokemon_garc)?;
        if self.is_sm() {
            let lusamine = trainers
                .get(constants::BEAST_LUSAMINE_TRAINER_INDEX)
                .ok_or(RandomizationError::IndexOutOfBounds("trainerData"))?;
            self.set_beast_lusamine_pokemon_buffs(lusamine)?;
        }
        Ok(())
    }

    fn encode_trainer_pokemon(&self, trainer: &Trainer, movesets: &Movesets) -> Result<Vec<u8>> {
        let mut data = vec![0u8; TRAINER_POKEMON_SIZE * trainer.pokemon.len()];
        for (slot, pokemon) in trainer.pokemon.iter().enumerate() {
            let offset = slot * TRAINER_POKEMON_SIZE;
            data[offset] = ((pokemon.ability_slot << 4) | pokemon.forced_gender_flag) as u8;
            data[offset + 1] = pokemon.nature as u8;
            data[offset + 2] = pokemon.hp_evs as u8;
            data[offset + 3] = pokemon.atk_evs as u8;
            data[offset + 4] = pokemon.def_evs as u8;
            data[offset + 5] = pokemon.spatk_evs as u8;
            data[offset + 6] = pokemon.spdef_evs as u8;
            data[offset + 7] = pokemon.speed_evs as u8;
            write_long(&mut data, offset + 8, pokemon.ivs);
            write_word(&mut data, offset + 14, pokemon.level);
            write_word(&mut data, offset + 16, pokemon.pokemon.number());
            write_word(&mut data, offset + 18, pokemon.forme);
            write_word(&mut data, offset + 20, pokemon.held_item);
            let moves = if pokemon.reset_moves {
                let alternate = self.alt_forme_of(pokemon.pokemon, pokemon.forme);
                let movepool = movesets
                    .get(&alternate.number())
                    .ok_or(RandomizationError::NullPointer("movesets.get"))?;
                moves_at_level(movepool, pokemon.level, 0)
            } else {
                pokemon.moves
            };
            for (move_slot, &move_number) in moves.iter().enumerate() {
                write_word(&mut data, offset + 24 + move_slot * 2, move_number);
            }
            if constants::HELD_Z_CRYSTALS.contains(&pokemon.held_item) {
                let crystal = self.random_z_crystal(&moves)?;
                write_word(&mut data, offset + 20, crystal);
            }
        }
        Ok(data)
    }

    fn random_z_crystal(&self, moves: &[i32; 4]) -> Result<i32> {
        let known = moves
            .iter()
            .filter(|&&move_number| move_number != 0)
            .count() as i32;
        if known <= 0 {
            return Err(RandomizationError::randomization(
                "java.lang.IllegalArgumentException: bound must be positive",
            ));
        }
        let mut random = self.core.engine.random.clone();
        let chosen = random.next_int_bounded(known) as usize;
        let move_number = moves[chosen];
        let mv = self
            .core
            .data
            .move_by_number(move_number)
            .filter(|_| move_number != 0)
            .ok_or(RandomizationError::NullPointer(
                "moves[pokeMoves[chosenMove]]",
            ))?;
        let type_index = constants::type_to_byte(mv.kind) as usize;
        constants::HELD_Z_CRYSTALS
            .get(type_index)
            .copied()
            .ok_or(RandomizationError::IndexOutOfBounds("heldZCrystals"))
    }

    fn set_beast_lusamine_pokemon_buffs(&self, lusamine: &Trainer) -> Result<()> {
        let battle_path = self.file("Battle");
        let mut battle_cro = self.read_file(battle_path)?;
        let Some(offset) =
            find_after_prefix(&battle_cro, constants::BEAST_LUSAMINE_POKEMON_BOOSTS_PREFIX)
        else {
            return Ok(());
        };
        let mut unique: Vec<PokemonId> = Vec::new();
        for pokemon in &lusamine.pokemon {
            if !unique.contains(&pokemon.pokemon) {
                unique.push(pokemon.pokemon);
            }
        }
        for (index, &boosted) in unique.iter().take(5).enumerate() {
            let aura = self.aura_number_for_highest_stat(boosted);
            let entry = offset + index * 0x10;
            write_word(&mut battle_cro, entry, self.core.data.base_number(boosted));
            battle_cro[entry + 2] = aura as u8;
        }
        self.write_file(battle_path, &battle_cro)
    }

    fn aura_number_for_highest_stat(&self, pokemon: PokemonId) -> i32 {
        let stats = self.core.data.pk(pokemon);
        let mut random = self.core.engine.random.clone();
        let mut best = stats.attack;
        let mut aura = 1;
        for (value, number) in [
            (stats.defense, 2),
            (stats.spatk, 3),
            (stats.spdef, 4),
            (stats.speed, 5),
        ] {
            if value > best || (value == best && random.next_boolean()) {
                best = value;
                aura = number;
            }
        }
        aura
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
}
