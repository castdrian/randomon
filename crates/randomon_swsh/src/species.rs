use std::collections::BTreeMap;

use randomon_core::{Compatibility, EggMoves, Movesets};
use randomon_data::constants::species;
use randomon_data::{ExpCurve, MoveLearnt, PokemonId, StatLayout};
use randomon_gen7::constants::{type_from_byte, type_to_byte};

use crate::constants;
use crate::formats::common::BitFlags;
use crate::formats::learnset::{LevelUpMove, MAX_MOVES};
use crate::formats::personal::PersonalInfo;
use crate::handler::SwshRomHandler;

const MAX_LEVEL: i32 = 100;

fn byte(value: i32) -> u8 {
    value.clamp(0, i32::from(u8::MAX)) as u8
}

fn word(value: i32) -> u16 {
    value.clamp(0, i32::from(u16::MAX)) as u16
}

fn read_flags<const N: usize>(flags: &BitFlags<N>, count: i32) -> Vec<bool> {
    std::iter::once(false)
        .chain((0..count as usize).map(|index| flags.get(index)))
        .collect()
}

fn write_flags<const N: usize>(flags: &mut BitFlags<N>, values: &[bool], count: i32) {
    for (index, &value) in values.iter().skip(1).take(count as usize).enumerate() {
        flags.set(index, value);
    }
}

#[derive(Clone, Copy)]
enum FormKind {
    Real,
    Cosmetic,
}

fn form_kind(species_number: i32, forme: i32) -> Option<FormKind> {
    if constants::BATTLE_ONLY_FORMS.contains(&(species_number, forme))
        || species_number == species::PIKACHU
        || species_number == species::MAGEARNA
        || species_number == species::ZARUDE
    {
        return None;
    }
    Some(
        if constants::COSMETIC_FORM_SPECIES.contains(&species_number) {
            FormKind::Cosmetic
        } else {
            FormKind::Real
        },
    )
}

fn is_populated(entry: &PersonalInfo) -> bool {
    entry.is_present_in_game && entry.base_stats.total() > 0
}

impl SwshRomHandler {
    pub(crate) fn load_pokemon(&mut self, names: &[String]) {
        let personal = self.files.data.personal.clone();
        self.pokes = vec![None; personal.len()];
        for number in 1..=self.species_count {
            let Some(entry) = personal.get(number as usize) else {
                continue;
            };
            if !is_populated(entry) {
                continue;
            }
            let id = self.core.data.create_pokemon(StatLayout::Standard, number);
            self.pokes[number as usize] = Some(id);
            self.read_stats(id, entry);
            self.core.data.pk_mut(id).name =
                names.get(number as usize).cloned().unwrap_or_default();
        }
        for species_number in 1..=self.species_count {
            let Some(base) = self.existing(species_number) else {
                continue;
            };
            let entry = &personal.entries[species_number as usize];
            for forme in 1..i32::from(entry.form_count) {
                let Some(index) = entry.alternate_form_index(forme as u16) else {
                    continue;
                };
                let Some(form_entry) = personal.get(index) else {
                    continue;
                };
                let Some(kind) = form_kind(species_number, forme) else {
                    continue;
                };
                if index <= self.species_count as usize || !is_populated(form_entry) {
                    continue;
                }
                self.link_forme(base, species_number, forme, index as i32, kind, form_entry);
            }
            let base_pokemon = self.core.data.pk_mut(base);
            if !base_pokemon.real_cosmetic_form_numbers.is_empty() {
                base_pokemon.real_cosmetic_form_numbers.push(0);
                base_pokemon.cosmetic_forms += 1;
            }
        }
        self.load_evolutions();
    }

    fn link_forme(
        &mut self,
        base: PokemonId,
        species_number: i32,
        forme: i32,
        number: i32,
        kind: FormKind,
        entry: &PersonalInfo,
    ) {
        let id = self.core.data.create_pokemon(StatLayout::Standard, number);
        self.pokes[number as usize] = Some(id);
        self.read_stats(id, entry);
        let name = self.core.data.pk(base).name.clone();
        let cosmetic = matches!(kind, FormKind::Cosmetic);
        let pokemon = self.core.data.pk_mut(id);
        pokemon.name = name;
        pokemon.base_forme = Some(base);
        pokemon.forme_number = forme;
        pokemon.actually_cosmetic = cosmetic;
        pokemon.forme_suffix = if cosmetic {
            String::new()
        } else {
            constants::forme_suffix(species_number, forme)
        };
        if cosmetic {
            let base_pokemon = self.core.data.pk_mut(base);
            base_pokemon.cosmetic_forms += 1;
            base_pokemon.real_cosmetic_form_numbers.push(forme);
        }
    }

    fn read_stats(&mut self, id: PokemonId, entry: &PersonalInfo) {
        let pokemon = self.core.data.pk_mut(id);
        let stats = &entry.base_stats;
        pokemon.hp = i32::from(stats.hp);
        pokemon.attack = i32::from(stats.attack);
        pokemon.defense = i32::from(stats.defense);
        pokemon.speed = i32::from(stats.speed);
        pokemon.spatk = i32::from(stats.special_attack);
        pokemon.spdef = i32::from(stats.special_defense);
        pokemon.primary_type = type_from_byte(entry.types[0]);
        pokemon.secondary_type = type_from_byte(entry.types[1]);
        if pokemon.secondary_type == pokemon.primary_type {
            pokemon.secondary_type = None;
        }
        pokemon.catch_rate = i32::from(entry.catch_rate);
        pokemon.exp_yield = i32::from(entry.base_experience);
        pokemon.gender_ratio = i32::from(entry.gender_ratio);
        pokemon.growth_curve = ExpCurve::from_byte(entry.growth_rate as i8);
        let [first, second, hidden] = entry.abilities.map(i32::from);
        pokemon.ability1 = first;
        pokemon.ability2 = if second == first { 0 } else { second };
        pokemon.ability3 = hidden;
        let [common, rare, very_rare] = entry.held_items.map(i32::from);
        if common == rare && common != 0 {
            pokemon.guaranteed_held_item = common;
        } else {
            pokemon.common_held_item = common;
            pokemon.rare_held_item = rare;
        }
        pokemon.dark_grass_held_item = very_rare;
    }

    fn keeps_own_types(&self, id: PokemonId) -> bool {
        let pokemon = self.core.data.pk(id);
        pokemon.actually_cosmetic
            && pokemon
                .base_forme
                .is_some_and(|base| base.number() == species::SILVALLY)
    }

    fn write_stats(&self, id: PokemonId, entry: &mut PersonalInfo) {
        let pokemon = self.core.data.pk(id);
        let stats = &mut entry.base_stats;
        stats.hp = byte(pokemon.hp);
        stats.attack = byte(pokemon.attack);
        stats.defense = byte(pokemon.defense);
        stats.speed = byte(pokemon.speed);
        stats.special_attack = byte(pokemon.spatk);
        stats.special_defense = byte(pokemon.spdef);
        if !self.keeps_own_types(id) {
            let primary = type_to_byte(pokemon.primary_type);
            let secondary = pokemon
                .secondary_type
                .map_or(primary, |kind| type_to_byte(Some(kind)));
            entry.types = [primary, secondary];
        }
        entry.catch_rate = byte(pokemon.catch_rate);
        entry.base_experience = word(pokemon.exp_yield);
        if let Some(curve) = pokemon.growth_curve {
            entry.growth_rate = curve.to_byte() as u8;
        }
        let second = if pokemon.ability2 == 0 {
            pokemon.ability1
        } else {
            pokemon.ability2
        };
        entry.abilities = [word(pokemon.ability1), word(second), word(pokemon.ability3)];
        let (common, rare) = if pokemon.guaranteed_held_item > 0 {
            (pokemon.guaranteed_held_item, pokemon.guaranteed_held_item)
        } else {
            (pokemon.common_held_item, pokemon.rare_held_item)
        };
        entry.held_items[0] = word(common);
        entry.held_items[1] = word(rare);
        if pokemon.dark_grass_held_item >= 0 {
            entry.held_items[2] = word(pokemon.dark_grass_held_item);
        }
    }

    pub(crate) fn save_pokemon(&mut self) {
        for (number, slot) in self.pokes.clone().into_iter().enumerate() {
            let Some(id) = slot else {
                continue;
            };
            let mut entry = self.files.data.personal.entries[number].clone();
            self.write_stats(id, &mut entry);
            if entry != self.files.data.personal.entries[number] {
                self.files.data.personal.entries[number] = entry;
            }
        }
        self.save_evolutions();
    }

    pub(crate) fn all_pokemon(&self) -> Vec<PokemonId> {
        self.pokes.iter().flatten().copied().collect()
    }

    fn present_numbers(&self) -> Vec<i32> {
        self.all_pokemon()
            .into_iter()
            .map(|pokemon| pokemon.number())
            .collect()
    }

    pub(crate) fn read_movesets(&self) -> Movesets {
        let mut movesets = BTreeMap::new();
        for number in self.present_numbers() {
            let Some(learnset) = self.files.data.learnsets.get(number as usize) else {
                continue;
            };
            let learnt = learnset
                .moves
                .iter()
                .map(|entry| MoveLearnt {
                    level: i32::from(entry.level),
                    move_number: i32::from(entry.move_id),
                })
                .collect();
            movesets.insert(number, learnt);
        }
        movesets
    }

    pub(crate) fn write_movesets(&mut self, movesets: &Movesets) {
        for (&number, learnt) in movesets {
            if self.existing(number).is_none() {
                continue;
            }
            let Some(learnset) = self.files.data.learnsets.get_mut(number as usize) else {
                continue;
            };
            let moves: Vec<LevelUpMove> = learnt
                .iter()
                .filter(|entry| entry.move_number > 0)
                .take(MAX_MOVES)
                .map(|entry| LevelUpMove {
                    move_id: word(entry.move_number),
                    level: word(entry.level.clamp(0, MAX_LEVEL)),
                })
                .collect();
            if learnset.moves != moves {
                learnset.moves = moves;
            }
        }
    }

    fn egg_move_file(&self, number: i32) -> Option<usize> {
        let pokemon = self.existing(number)?;
        let entry = self.core.data.pk(pokemon);
        let index = match entry.base_forme {
            None => number as usize,
            Some(base) => self
                .files
                .data
                .egg_moves
                .get(base.number() as usize)?
                .alternate_form_entry(entry.forme_number as u16)?,
        };
        (index < self.files.data.egg_moves.len()).then_some(index)
    }

    pub(crate) fn read_egg_moves(&self) -> EggMoves {
        let mut egg_moves = BTreeMap::new();
        for number in self.present_numbers() {
            let Some(index) = self.egg_move_file(number) else {
                continue;
            };
            let moves = self.files.data.egg_moves[index]
                .moves
                .iter()
                .map(|&move_id| i32::from(move_id))
                .collect();
            egg_moves.insert(number, moves);
        }
        egg_moves
    }

    pub(crate) fn write_egg_moves(&mut self, egg_moves: &EggMoves) {
        for (&number, moves) in egg_moves {
            let Some(index) = self.egg_move_file(number) else {
                continue;
            };
            let encoded: Vec<u16> = moves.iter().map(|&move_number| word(move_number)).collect();
            let file = &mut self.files.data.egg_moves[index];
            if file.moves != encoded {
                file.moves = encoded;
            }
        }
    }

    fn compatibility(&self, flags_of: impl Fn(&PersonalInfo) -> Vec<bool>) -> Compatibility {
        self.all_pokemon()
            .into_iter()
            .filter_map(|pokemon| {
                let entry = self.files.data.personal.get(pokemon.number() as usize)?;
                Some((pokemon, flags_of(entry)))
            })
            .collect()
    }

    pub(crate) fn read_tm_compatibility(&self) -> Compatibility {
        self.compatibility(|entry| read_flags(&entry.tm, constants::TM_COUNT))
    }

    pub(crate) fn read_tr_compatibility(&self) -> Compatibility {
        self.compatibility(|entry| read_flags(&entry.tr, constants::TR_COUNT))
    }

    pub(crate) fn write_tm_compatibility(&mut self, compatibility: &Compatibility) {
        for (pokemon, flags) in compatibility {
            if let Some(entry) = self
                .files
                .data
                .personal
                .entries
                .get_mut(pokemon.number() as usize)
            {
                write_flags(&mut entry.tm, flags, constants::TM_COUNT);
            }
        }
    }

    pub(crate) fn write_tr_compatibility(&mut self, compatibility: &Compatibility) {
        for (pokemon, flags) in compatibility {
            if let Some(entry) = self
                .files
                .data
                .personal
                .entries
                .get_mut(pokemon.number() as usize)
            {
                write_flags(&mut entry.tr, flags, constants::TR_COUNT);
            }
        }
    }
}
