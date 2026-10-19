use randomon_core::Movesets;
use randomon_data::{Trainer, TrainerPokemon, moves_at_level};

use crate::TextRole;
use crate::constants;
use crate::formats::common::Stats;
use crate::formats::trainers::{Trainer as TrainerRecord, TrainerPokemon as TrainerSlot};
use crate::handler::SwshRomHandler;

const SHINY_BIT: i32 = 1 << 30;
const IV_BITS: i32 = 5;
const IV_MASK: i32 = 0x1F;
const DOUBLE_BATTLE: u8 = 1;
const HAS_ITEMS_AND_MOVES: i32 = 3;
const RANDOM_GENDER: u8 = 0;

fn pack_ivs(ivs: &Stats<u8>) -> i32 {
    ivs.to_hp_atk_def_spe_spa_spd()
        .iter()
        .enumerate()
        .fold(0, |packed, (slot, &value)| {
            packed | ((i32::from(value) & IV_MASK) << (slot as i32 * IV_BITS))
        })
}

fn unpack_ivs(packed: i32) -> Stats<u8> {
    let iv = |slot: i32| ((packed >> (slot * IV_BITS)) & IV_MASK) as u8;
    Stats::from_hp_atk_def_spe_spa_spd([iv(0), iv(1), iv(2), iv(3), iv(4), iv(5)])
}

fn ev(value: i8) -> u8 {
    value as u8
}

impl SwshRomHandler {
    pub(crate) fn class_names(&self) -> Vec<String> {
        self.text_lines(TextRole::TrainerClasses)
    }

    pub(crate) fn set_class_names(&mut self, names: &[String]) {
        let count = self.class_names().len();
        for (index, name) in names.iter().enumerate().take(count) {
            self.set_text_line(TextRole::TrainerClasses, index, name);
        }
    }

    fn trainer_indices(&self) -> std::ops::Range<usize> {
        1..self.files.data.trainers.len()
    }

    pub(crate) fn trainer_name_list(&self) -> Vec<String> {
        let names = self.text_lines(TextRole::TrainerNames);
        self.trainer_indices()
            .map(|index| names.get(index).cloned().unwrap_or_default())
            .collect()
    }

    pub(crate) fn set_trainer_name_list(&mut self, names: &[String]) {
        let available = self.text_lines(TextRole::TrainerNames).len();
        for (index, name) in self.trainer_indices().zip(names) {
            if index < available {
                self.set_text_line(TextRole::TrainerNames, index, name);
            }
        }
    }

    pub(crate) fn ability_for_trainer_pokemon(&self, pokemon: &TrainerPokemon) -> i32 {
        let entry = self.core.data.pk(pokemon.pokemon);
        match pokemon.ability_slot {
            2 if entry.ability2 != 0 => entry.ability2,
            3 if entry.ability3 != 0 => entry.ability3,
            _ => entry.ability1,
        }
    }

    fn read_trainer_pokemon(&self, slot: &TrainerSlot) -> Option<TrainerPokemon> {
        let species_number = i32::from(slot.species);
        let forme = i32::from(slot.form);
        let shown = self.pokemon_with_forme(species_number, forme)?;
        let base = self.existing(species_number)?;
        let mut pokemon = TrainerPokemon::new(base, i32::from(slot.level));
        pokemon.forme = forme;
        pokemon.forme_suffix = self.core.data.pk(shown).forme_suffix.clone();
        pokemon.held_item = i32::from(slot.held_item);
        pokemon.nature = slot.nature as i8;
        pokemon.ability_slot = i32::from(slot.ability_slot);
        pokemon.ivs = pack_ivs(&slot.ivs);
        if slot.shiny {
            pokemon.ivs |= SHINY_BIT;
        }
        pokemon.hp_evs = slot.evs.hp as i8;
        pokemon.atk_evs = slot.evs.attack as i8;
        pokemon.def_evs = slot.evs.defense as i8;
        pokemon.spatk_evs = slot.evs.special_attack as i8;
        pokemon.spdef_evs = slot.evs.special_defense as i8;
        pokemon.speed_evs = slot.evs.speed as i8;
        pokemon.moves = slot.moves.map(i32::from);
        Some(pokemon)
    }

    fn readable_team(&self, team: &[TrainerSlot]) -> Option<Vec<TrainerPokemon>> {
        team.iter()
            .map(|slot| self.read_trainer_pokemon(slot))
            .collect()
    }

    fn build_trainers(&self, sources: &[TrainerRecord]) -> Vec<Trainer> {
        let class_names = self.class_names();
        let names = self.text_lines(TextRole::TrainerNames);
        let mut trainers = Vec::new();
        for index in self.trainer_indices() {
            let Some(source) = sources.get(index) else {
                continue;
            };
            let Some(team) = self.readable_team(&source.team) else {
                continue;
            };
            let mut trainer = Trainer::new();
            trainer.index = index as i32;
            trainer.offset = index as i32;
            trainer.poketype = HAS_ITEMS_AND_MOVES;
            trainer.trainerclass = i32::from(source.data.class);
            let class_name = class_names
                .get(usize::from(source.data.class))
                .cloned()
                .unwrap_or_default();
            let name = names.get(index).cloned().unwrap_or_default();
            trainer.full_display_name = Some(format!("{class_name} {name}").trim().to_owned());
            trainer.name = Some(name);
            trainer.pokemon = team;
            trainers.push(trainer);
        }
        trainers
    }

    pub(crate) fn read_trainers(&self) -> Vec<Trainer> {
        let mut originals = self.build_trainers(&self.files.original().trainers);
        constants::tag_trainers(&mut originals);
        let mut trainers = self.build_trainers(&self.files.data.trainers);
        for trainer in trainers.iter_mut() {
            if let Some(original) = originals
                .iter()
                .find(|original| original.index == trainer.index)
            {
                trainer.tag = original.tag.clone();
                trainer.force_starter_position = original.force_starter_position;
            }
        }
        trainers
    }

    fn trainer_moves(&self, pokemon: &TrainerPokemon, movesets: &Movesets) -> [i32; 4] {
        let shown = self.alt_forme_of(pokemon.pokemon, pokemon.forme);
        let valid = |move_number: i32| {
            move_number == 0 || self.core.data.move_by_number(move_number).is_some()
        };
        if !pokemon.reset_moves && pokemon.moves.iter().all(|&move_number| valid(move_number)) {
            return pokemon.moves;
        }
        movesets.get(&shown.number()).map_or([0; 4], |movepool| {
            moves_at_level(movepool, pokemon.level, 0)
        })
    }

    fn write_trainer_pokemon(
        &self,
        template: &TrainerSlot,
        pokemon: &TrainerPokemon,
        movesets: &Movesets,
    ) -> TrainerSlot {
        let (species_number, form) = self.placement(
            pokemon.pokemon,
            pokemon.forme,
            (i32::from(template.species), i32::from(template.form)),
        );
        let changed_species =
            i32::from(template.species) != species_number || i32::from(template.form) != form;
        let mut slot = *template;
        slot.species = species_number as u16;
        slot.form = form as u16;
        slot.level = pokemon.level.clamp(1, 100) as u16;
        slot.held_item = pokemon.held_item.clamp(0, i32::from(u16::MAX)) as u16;
        slot.nature = pokemon.nature as u8;
        slot.ability_slot = pokemon.ability_slot.clamp(0, 3) as u8;
        slot.ivs = unpack_ivs(pokemon.ivs);
        slot.shiny = pokemon.ivs & SHINY_BIT != 0;
        slot.evs = Stats {
            hp: ev(pokemon.hp_evs),
            attack: ev(pokemon.atk_evs),
            defense: ev(pokemon.def_evs),
            speed: ev(pokemon.speed_evs),
            special_attack: ev(pokemon.spatk_evs),
            special_defense: ev(pokemon.spdef_evs),
        };
        slot.moves = self
            .trainer_moves(pokemon, movesets)
            .map(|move_number| move_number.clamp(0, i32::from(u16::MAX)) as u16);
        if changed_species {
            slot.gender = RANDOM_GENDER;
            slot.can_gigantamax = false;
        }
        if self.cannot_dynamax(species_number, form) {
            slot.can_dynamax = false;
            slot.can_gigantamax = false;
        }
        slot
    }

    pub(crate) fn write_trainers(&mut self, trainers: &[Trainer], double_battle_mode: bool) {
        let movesets = self.read_movesets();
        for trainer in trainers {
            let Ok(index) = usize::try_from(trainer.index) else {
                continue;
            };
            let Some(source) = self.files.data.trainers.get(index) else {
                continue;
            };
            if index == 0 || self.readable_team(&source.team).is_none() {
                continue;
            }
            let fallback = source.team.last().copied().unwrap_or_default();
            let team: Vec<TrainerSlot> = trainer
                .pokemon
                .iter()
                .take(constants::PARTY_SIZE)
                .enumerate()
                .map(|(position, pokemon)| {
                    let template = source.team.get(position).copied().unwrap_or(fallback);
                    self.write_trainer_pokemon(&template, pokemon, &movesets)
                })
                .collect();
            if team.is_empty() {
                continue;
            }
            let mut data = source.data;
            if double_battle_mode && !trainer.skip_important() && team.len() >= 2 {
                data.battle_mode = DOUBLE_BATTLE;
            }
            let target = &mut self.files.data.trainers[index];
            if target.team != team {
                target.team = team;
            }
            if target.data != data {
                target.data = data;
            }
        }
    }
}
