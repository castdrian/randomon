use std::collections::{BTreeMap, HashMap};

use randomon_core::engine::evolutions::{
    add_evo_update_condensed, add_evo_update_happiness, add_evo_update_held_item,
    add_evo_update_level, add_evo_update_party, add_evo_update_stone,
};
use randomon_core::{Movesets, RandomizationError, Result};
use randomon_ctr::bytes::{find_after_prefix, read_word, write_long, write_word};
use randomon_data::constants::{items, species};
use randomon_data::{
    Evolution, EvolutionId, EvolutionType, ExpCurve, MegaEvolution, PokemonId, StatLayout,
};
use randomon_formats::ctr::{Decompression, GarcArchive};
use randomon_java::{JavaHash, JavaHashSet};
use randomon_settings::{MovesetsMod, Settings, WildPokemonMod};

use crate::constants::{self, POKEMON_COUNT};
use crate::handler::{FormeInfo, Gen6RomHandler};

pub(crate) fn garc_file(garc: &GarcArchive, index: usize) -> Result<&[u8]> {
    garc.file(index)
        .ok_or(RandomizationError::IndexOutOfBounds("garc.files"))
}

pub(crate) fn garc_file_mut(garc: &mut GarcArchive, index: usize) -> Result<&mut Vec<u8>> {
    garc.directory_mut(index)
        .and_then(|directory| directory.get_mut(&0))
        .and_then(Option::as_mut)
        .ok_or(RandomizationError::IndexOutOfBounds("garc.files"))
}

fn read_u16(data: &[u8], offset: usize) -> i32 {
    read_word(data, offset)
}

#[derive(Clone, Debug)]
pub(crate) struct EvolutionKey {
    pub evolution: Evolution,
}

impl PartialEq for EvolutionKey {
    fn eq(&self, other: &Self) -> bool {
        self.evolution.java_equals(&other.evolution)
    }
}

impl Eq for EvolutionKey {}

impl JavaHash for EvolutionKey {
    fn java_hash_code(&self) -> i32 {
        self.evolution.java_hash_code()
    }
}

impl Gen6RomHandler {
    pub(crate) fn load_pokemon_stats(&mut self) -> Result<()> {
        self.poke_garc = self.read_garc(self.file("PokemonStats"), Decompression::Skip)?;
        let names = self.read_pokemon_names()?;
        let total = self.total_pokemon();
        self.pokes = vec![None; (total + 1) as usize];
        for number in 1..=POKEMON_COUNT {
            let id = self.core.data.create_pokemon(StatLayout::Standard, number);
            self.pokes[number as usize] = Some(id);
            let stats = garc_file(&self.poke_garc, number as usize)?.to_vec();
            self.load_basic_poke_stats(id, &stats);
            self.core.data.pk_mut(id).name = names[number as usize].clone();
        }
        self.absolute_poke_num_by_base_forme = HashMap::new();
        self.load_alternate_formes(&names)?;
        self.populate_evolutions()?;
        self.populate_mega_evolutions()
    }

    fn load_alternate_formes(&mut self, names: &[String]) -> Result<()> {
        let mut form_number = 1;
        let mut previous_species = 0;
        let mut current: HashMap<i32, i32> = HashMap::new();
        let keys: Vec<i32> = self.forme_mappings.keys().copied().collect();
        for (number, key) in (POKEMON_COUNT + 1..).zip(keys) {
            let id = self.core.data.create_pokemon(StatLayout::Standard, number);
            if let Some(slot) = self.pokes.get_mut(number as usize) {
                *slot = Some(id);
            } else {
                return Err(RandomizationError::IndexOutOfBounds("pokes"));
            }
            let stats = garc_file(&self.poke_garc, key as usize)?.to_vec();
            self.load_basic_poke_stats(id, &stats);
            let info = self.forme_mappings[&key];
            let base = self.pk(info.base_forme);
            let pokemon = self.core.data.pk_mut(id);
            pokemon.name = names[info.base_forme as usize].clone();
            pokemon.base_forme = Some(base);
            pokemon.forme_number = info.forme_number;
            pokemon.forme_suffix = constants::forme_suffix(key).to_owned();
            if info.base_forme == previous_species {
                form_number += 1;
                current.insert(form_number, number);
            } else {
                if previous_species != 0 {
                    self.absolute_poke_num_by_base_forme
                        .insert(previous_species, std::mem::take(&mut current));
                }
                previous_species = info.base_forme;
                form_number = 1;
                current = HashMap::from([(form_number, number)]);
            }
        }
        if previous_species != 0 {
            self.absolute_poke_num_by_base_forme
                .insert(previous_species, current);
        }
        Ok(())
    }

    fn load_basic_poke_stats(&mut self, id: PokemonId, stats: &[u8]) {
        let pokemon = self.core.data.pk_mut(id);
        pokemon.hp = i32::from(stats[constants::BS_HP_OFFSET]);
        pokemon.attack = i32::from(stats[constants::BS_ATTACK_OFFSET]);
        pokemon.defense = i32::from(stats[constants::BS_DEFENSE_OFFSET]);
        pokemon.speed = i32::from(stats[constants::BS_SPEED_OFFSET]);
        pokemon.spatk = i32::from(stats[constants::BS_SP_ATK_OFFSET]);
        pokemon.spdef = i32::from(stats[constants::BS_SP_DEF_OFFSET]);
        pokemon.primary_type = constants::type_from_byte(stats[constants::BS_PRIMARY_TYPE_OFFSET]);
        pokemon.secondary_type =
            constants::type_from_byte(stats[constants::BS_SECONDARY_TYPE_OFFSET]);
        if pokemon.secondary_type == pokemon.primary_type {
            pokemon.secondary_type = None;
        }
        pokemon.catch_rate = i32::from(stats[constants::BS_CATCH_RATE_OFFSET]);
        pokemon.growth_curve = ExpCurve::from_byte(stats[constants::BS_GROWTH_CURVE_OFFSET] as i8);
        pokemon.ability1 = i32::from(stats[constants::BS_ABILITY1_OFFSET]);
        pokemon.ability2 = i32::from(stats[constants::BS_ABILITY2_OFFSET]);
        pokemon.ability3 = i32::from(stats[constants::BS_ABILITY3_OFFSET]);
        if pokemon.ability1 == pokemon.ability2 {
            pokemon.ability2 = 0;
        }
        let item1 = read_u16(stats, constants::BS_COMMON_HELD_ITEM_OFFSET);
        let item2 = read_u16(stats, constants::BS_RARE_HELD_ITEM_OFFSET);
        if item1 == item2 {
            pokemon.guaranteed_held_item = item1;
            pokemon.common_held_item = 0;
            pokemon.rare_held_item = 0;
        } else {
            pokemon.guaranteed_held_item = 0;
            pokemon.common_held_item = item1;
            pokemon.rare_held_item = item2;
        }
        pokemon.dark_grass_held_item = -1;
        let forme_count = i32::from(stats[constants::BS_FORME_COUNT_OFFSET]);
        if forme_count > 1 {
            let number = pokemon.number;
            self.register_formes(id, number, forme_count, stats);
        }
    }

    fn register_formes(&mut self, id: PokemonId, number: i32, forme_count: i32, stats: &[u8]) {
        if self.forme_mappings.contains_key(&number) {
            if constants::is_actually_cosmetic(number) {
                self.core.data.pk_mut(id).actually_cosmetic = true;
            }
            return;
        }
        let first_forme_offset = read_u16(stats, constants::BS_FORME_OFFSET);
        if first_forme_offset != 0 {
            for forme in 1..forme_count {
                let forme_index = first_forme_offset + forme - 1;
                self.forme_mappings.insert(
                    forme_index,
                    FormeInfo {
                        base_forme: number,
                        forme_number: forme,
                    },
                );
                if constants::is_actually_cosmetic(forme_index)
                    && number != species::PIKACHU
                    && number != species::CHERRIM
                {
                    self.core.data.pk_mut(id).cosmetic_forms += 1;
                }
            }
        } else if number != species::ARCEUS
            && number != species::GENESECT
            && number != species::XERNEAS
        {
            self.core.data.pk_mut(id).cosmetic_forms = forme_count;
        }
    }

    fn read_pokemon_names(&self) -> Result<Vec<String>> {
        let list = self.strings(false, self.entry.int("PokemonNamesTextOffset"))?;
        let mut names = vec![String::new(); (POKEMON_COUNT + 1) as usize];
        for (number, name) in names.iter_mut().enumerate().skip(1) {
            *name = list
                .get(number)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("pokemon names"))?;
        }
        Ok(names)
    }

    fn populate_evolutions(&mut self) -> Result<()> {
        let evo_garc = self.read_garc(self.file("PokemonEvolutions"), Decompression::Skip)?;
        for number in 1..=self.total_pokemon() {
            let pokemon = self.pk(number);
            let entry = garc_file(&evo_garc, number as usize)?.to_vec();
            for slot in 0..8 {
                let method = read_word(&entry, slot * 6);
                let target = read_word(&entry, slot * 6 + 4);
                if !(1..=constants::EVOLUTION_METHOD_COUNT).contains(&method) || target < 1 {
                    continue;
                }
                let Some(kind) = EvolutionType::from_index(6, method) else {
                    return Err(RandomizationError::NullPointer("EvolutionType.fromIndex"));
                };
                if kind == EvolutionType::LevelHighBeauty {
                    continue;
                }
                let extra_info = read_word(&entry, slot * 6 + 2);
                self.add_evolution_if_new(pokemon, self.pk(target), kind, extra_info);
            }
            if number == species::NINCADA {
                let shedinja = self.pk(species::SHEDINJA);
                let data = &mut self.core.data;
                let id = data.new_evolution(Evolution::new(
                    pokemon,
                    shedinja,
                    false,
                    EvolutionType::LevelIsExtra,
                    20,
                ));
                data.evolutions_from_mut(pokemon).push(id);
                data.evolutions_to_mut(shedinja).push(id);
            }
            let data = &mut self.core.data;
            if data.evolutions_from(pokemon).len() > 1 {
                for evo in data.evolutions_from(pokemon).to_vec() {
                    if data.evo(evo).kind != EvolutionType::LevelCreateExtra {
                        data.evo_mut(evo).carry_stats = false;
                    }
                }
            }
        }
        Ok(())
    }

    fn add_evolution_if_new(
        &mut self,
        from: PokemonId,
        to: PokemonId,
        kind: EvolutionType,
        extra_info: i32,
    ) {
        let data = &mut self.core.data;
        let evolution = Evolution::new(from, to, true, kind, extra_info);
        if data.evolution_list_contains(data.evolutions_from(from), &evolution) {
            return;
        }
        let id = data.new_evolution(evolution);
        data.evolutions_from_mut(from).push(id);
        if !data.pk(from).actually_cosmetic {
            data.evolutions_to_mut(to).push(id);
        }
    }

    fn populate_mega_evolutions(&mut self) -> Result<()> {
        self.mega_evolutions = Vec::new();
        let mega_garc = self.read_garc(self.file("MegaEvolutions"), Decompression::Skip)?;
        for number in 1..=POKEMON_COUNT {
            let pokemon = self.pk(number);
            let entry = garc_file(&mega_garc, number as usize)?.to_vec();
            for slot in 0..3 {
                let form_number = read_word(&entry, slot * 8);
                let method = read_word(&entry, slot * 8 + 2);
                if method < 1 {
                    continue;
                }
                let argument = read_word(&entry, slot * 8 + 4);
                let mega_number = self.absolute_poke_num(number, form_number);
                let mega_pokemon = self.pk(mega_number);
                let id = self.core.data.new_mega_evolution(MegaEvolution::new(
                    pokemon,
                    mega_pokemon,
                    method,
                    argument,
                ));
                self.core.data.pk_mut(pokemon).mega_evolutions_from.push(id);
                if self.core.data.exists(mega_pokemon) {
                    self.core
                        .data
                        .pk_mut(mega_pokemon)
                        .mega_evolutions_to
                        .push(id);
                }
                self.mega_evolutions.push(id);
            }
            let megas = self.core.data.pk(pokemon).mega_evolutions_from.clone();
            if megas.len() > 1 {
                for mega in megas {
                    self.core.data.mega_mut(mega).carry_stats = false;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn save_pokemon_stats(&mut self) -> Result<()> {
        let mut offset = constants::bs_size(self.rom_type);
        let total = self.total_pokemon();
        let duplicate_index = (total + 1) as usize;
        for number in 1..=total {
            let id = self.pk(number);
            let mut data = garc_file(&self.poke_garc, number as usize)?.to_vec();
            self.save_basic_poke_stats(id, &mut data)?;
            *garc_file_mut(&mut self.poke_garc, number as usize)? = data.clone();
            let duplicate = garc_file_mut(&mut self.poke_garc, duplicate_index)?;
            for byte in data {
                *duplicate
                    .get_mut(offset)
                    .ok_or(RandomizationError::IndexOutOfBounds("duplicateData"))? = byte;
                offset += 1;
            }
        }
        self.write_garc(self.file("PokemonStats"), &self.poke_garc)?;
        self.write_evolutions()
    }

    fn save_basic_poke_stats(&self, id: PokemonId, stats: &mut [u8]) -> Result<()> {
        let pokemon = self.core.data.pk(id);
        stats[constants::BS_HP_OFFSET] = pokemon.hp as u8;
        stats[constants::BS_ATTACK_OFFSET] = pokemon.attack as u8;
        stats[constants::BS_DEFENSE_OFFSET] = pokemon.defense as u8;
        stats[constants::BS_SPEED_OFFSET] = pokemon.speed as u8;
        stats[constants::BS_SP_ATK_OFFSET] = pokemon.spatk as u8;
        stats[constants::BS_SP_DEF_OFFSET] = pokemon.spdef as u8;
        stats[constants::BS_PRIMARY_TYPE_OFFSET] = constants::type_to_byte(pokemon.primary_type);
        stats[constants::BS_SECONDARY_TYPE_OFFSET] = match pokemon.secondary_type {
            None => stats[constants::BS_PRIMARY_TYPE_OFFSET],
            secondary => constants::type_to_byte(secondary),
        };
        stats[constants::BS_CATCH_RATE_OFFSET] = pokemon.catch_rate as u8;
        stats[constants::BS_GROWTH_CURVE_OFFSET] = pokemon
            .growth_curve
            .ok_or(RandomizationError::NullPointer("growthCurve"))?
            .to_byte() as u8;
        stats[constants::BS_ABILITY1_OFFSET] = pokemon.ability1 as u8;
        stats[constants::BS_ABILITY2_OFFSET] = if pokemon.ability2 != 0 {
            pokemon.ability2 as u8
        } else {
            pokemon.ability1 as u8
        };
        stats[constants::BS_ABILITY3_OFFSET] = pokemon.ability3 as u8;
        let (common, rare) = if pokemon.guaranteed_held_item > 0 {
            (pokemon.guaranteed_held_item, pokemon.guaranteed_held_item)
        } else {
            (pokemon.common_held_item, pokemon.rare_held_item)
        };
        write_word(stats, constants::BS_COMMON_HELD_ITEM_OFFSET, common);
        write_word(stats, constants::BS_RARE_HELD_ITEM_OFFSET, rare);
        write_word(stats, constants::BS_DARK_GRASS_HELD_ITEM_OFFSET, 0);
        match pokemon.full_name().as_str() {
            "Meowstic" => stats[constants::BS_GENDER_OFFSET] = 0,
            "Meowstic-F" => stats[constants::BS_GENDER_OFFSET] = 0xFE,
            _ => {}
        }
        Ok(())
    }

    fn write_evolutions(&mut self) -> Result<()> {
        let path = self.file("PokemonEvolutions");
        let mut evo_garc = self.read_garc(path, Decompression::Skip)?;
        for number in 1..=self.total_pokemon() {
            let pokemon = self.pk(number);
            if number == species::NINCADA {
                self.write_shedinja_evolution()?;
            } else if number == species::FEEBAS && self.is_oras() {
                self.recreate_feebas_beauty_evolution();
            }
            let evolutions: Vec<EvolutionId> = self.core.data.evolutions_from(pokemon).to_vec();
            let entry = garc_file_mut(&mut evo_garc, number as usize)?;
            let mut written = 0;
            for evo in evolutions {
                let evolution = self.core.data.evo(evo);
                write_word(entry, written * 6, evolution.kind.to_index(6));
                write_word(entry, written * 6 + 2, evolution.extra_info);
                write_word(entry, written * 6 + 4, evolution.to.number());
                written += 1;
                if written == 8 {
                    break;
                }
            }
            while written < 8 {
                write_word(entry, written * 6, 0);
                write_word(entry, written * 6 + 2, 0);
                write_word(entry, written * 6 + 4, 0);
                written += 1;
            }
        }
        self.write_garc(path, &evo_garc)
    }

    fn write_shedinja_evolution(&mut self) -> Result<()> {
        let nincada = self.pk(species::NINCADA);
        let evolutions = self.core.data.evolutions_from(nincada).to_vec();
        if evolutions.len() < 2 {
            return Ok(());
        }
        let primary = self.core.data.evo(evolutions[0]).to;
        let extra = self.core.data.evo(evolutions[1]).to;
        let evolution_path = self.file("Evolution");
        let mut evolution_cro = self.read_file(evolution_path)?;
        if let Some(offset) = find_after_prefix(&evolution_cro, constants::NINJASK_SPECIES_PREFIX) {
            let number = primary.number();
            evolution_cro[offset] = ((number & 0xFF00) >> 8) as u8;
            evolution_cro[offset + 4] = (number & 0xFF) as u8;
        }
        if let Some(offset) = find_after_prefix(&self.code, constants::SHEDINJA_SPECIES_PREFIX) {
            self.relocate_shedinja_code(offset, extra.number());
        }
        self.core.data.evolutions_from_mut(nincada).remove(1);
        if !self.core.data.evolutions_to(extra).is_empty() {
            self.core.data.evolutions_to_mut(extra).remove(0);
        } else {
            return Err(RandomizationError::IndexOutOfBounds(
                "extraEvolution.evolutionsTo",
            ));
        }
        self.write_file(evolution_path, &evolution_cro)
    }

    fn relocate_shedinja_code(&mut self, offset: usize, extra_number: i32) {
        let code = &mut self.code;
        for index in 80..188 {
            code[offset + index] = code[offset + index + 4];
        }
        for bl_offset in [80, 92, 104, 116, 128, 140, 152, 164, 176] {
            code[offset + bl_offset] = code[offset + bl_offset].wrapping_add(1);
        }
        write_long(code, offset + 188, extra_number);
        code[offset..offset + 4].copy_from_slice(&[0xB4, 0x10, 0x9F, 0xE5]);
    }

    fn recreate_feebas_beauty_evolution(&mut self) {
        let feebas = self.pk(species::FEEBAS);
        let data = &mut self.core.data;
        let Some(&prism_scale) = data.evolutions_from(feebas).first() else {
            return;
        };
        let target = data.evo(prism_scale).to;
        let id = data.new_evolution(Evolution::new(
            feebas,
            target,
            true,
            EvolutionType::LevelHighBeauty,
            170,
        ));
        data.evolutions_from_mut(feebas).push(id);
        data.evolutions_to_mut(target).push(id);
    }

    pub(crate) fn remove_evos_for_pool(&mut self) -> Result<()> {
        let included = self.core.engine.pool.main_incl_formes.clone();
        for pokemon in self.pokes.clone().into_iter().flatten() {
            let mut keep: Vec<EvolutionId> = Vec::new();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                let evolution = self.core.data.evo(evo).clone();
                if included.contains(&evolution.from) && included.contains(&evolution.to) {
                    keep.push(evo);
                } else {
                    self.core.data.remove_first_equal_to(evolution.to, evo);
                }
            }
            let keep_evolutions: Vec<Evolution> = keep
                .iter()
                .map(|&evo| self.core.data.evo(evo).clone())
                .collect();
            let data = &mut self.core.data;
            let retained: Vec<EvolutionId> = data
                .evolutions_from(pokemon)
                .iter()
                .copied()
                .filter(|&evo| {
                    keep_evolutions
                        .iter()
                        .any(|kept| kept.java_equals(data.evo(evo)))
                })
                .collect();
            *data.evolutions_from_mut(pokemon) = retained;
        }
        self.rewrite_baby_pokemon()
    }

    fn rewrite_baby_pokemon(&mut self) -> Result<()> {
        let path = self.file("BabyPokemon");
        let mut baby_garc = self.read_garc(path, Decompression::Skip)?;
        let master_index = (POKEMON_COUNT + 1) as usize;
        for number in 1..=POKEMON_COUNT {
            let mut baby = self.pk(number);
            while let Some(&evo) = self.core.data.evolutions_to(baby).first() {
                baby = self.core.data.evo(evo).from;
            }
            write_word(
                garc_file_mut(&mut baby_garc, number as usize)?,
                0,
                baby.number(),
            );
            write_word(
                garc_file_mut(&mut baby_garc, master_index)?,
                number as usize * 2,
                baby.number(),
            );
        }
        self.write_garc(path, &baby_garc)
    }

    pub(crate) fn remove_impossible_evos(&mut self, settings: &Settings) -> Result<()> {
        let change_move_evos = settings.movesets_mod != MovesetsMod::Unchanged;
        let movesets = self.read_movesets()?;
        let mut extra_evolutions: JavaHashSet<EvolutionKey> = JavaHashSet::new();
        for pokemon in self.pokes.clone().into_iter().flatten() {
            extra_evolutions.clear();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                self.fix_impossible_evolution(
                    evo,
                    change_move_evos,
                    &movesets,
                    &mut extra_evolutions,
                )?;
            }
            self.add_extra_evolutions(pokemon, &extra_evolutions);
        }
        Ok(())
    }

    fn fix_impossible_evolution(
        &mut self,
        evo: EvolutionId,
        change_move_evos: bool,
        movesets: &Movesets,
        extra_evolutions: &mut JavaHashSet<EvolutionKey>,
    ) -> Result<()> {
        let data = &mut self.core.data;
        if change_move_evos && data.evo(evo).kind == EvolutionType::LevelWithMove {
            let move_number = data.evo(evo).extra_info;
            let from_number = data.evo(evo).from.number();
            let learnt = movesets
                .get(&from_number)
                .ok_or(RandomizationError::NullPointer("movesets.get"))?;
            let mut level_learnt_at = learnt
                .iter()
                .find(|learnt| learnt.move_number == move_number)
                .map_or(1, |learnt| learnt.level);
            if level_learnt_at == 1 {
                level_learnt_at = 45;
            }
            let evolution = data.evo_mut(evo);
            evolution.kind = EvolutionType::Level;
            evolution.extra_info = level_learnt_at;
            add_evo_update_level(
                data,
                &mut self.core.engine.impossible_evolution_updates,
                evo,
            );
        }
        if data.evo(evo).kind == EvolutionType::Trade {
            let evolution = data.evo_mut(evo);
            evolution.kind = EvolutionType::Level;
            evolution.extra_info = 37;
            add_evo_update_level(
                data,
                &mut self.core.engine.impossible_evolution_updates,
                evo,
            );
        }
        if data.evo(evo).kind == EvolutionType::TradeItem {
            let item = data.evo(evo).extra_info;
            if data.evo(evo).from.number() == species::SLOWPOKE {
                let evolution = data.evo_mut(evo);
                evolution.kind = EvolutionType::Stone;
                evolution.extra_info = items::WATER_STONE;
                let name = self.item_names[items::WATER_STONE as usize].clone();
                add_evo_update_stone(
                    data,
                    &mut self.core.engine.impossible_evolution_updates,
                    evo,
                    &name,
                );
            } else {
                let name = self.item_names[item as usize].clone();
                add_evo_update_held_item(
                    data,
                    &mut self.core.engine.impossible_evolution_updates,
                    evo,
                    &name,
                );
                data.evo_mut(evo).kind = EvolutionType::LevelItemDay;
                let evolution = data.evo(evo);
                extra_evolutions.add(EvolutionKey {
                    evolution: Evolution::new(
                        evolution.from,
                        evolution.to,
                        true,
                        EvolutionType::LevelItemNight,
                        item,
                    ),
                });
            }
        }
        if data.evo(evo).kind == EvolutionType::TradeSpecial {
            let partner = if data.evo(evo).from.number() == species::KARRABLAST {
                species::SHELMET
            } else {
                species::KARRABLAST
            };
            let evolution = data.evo_mut(evo);
            evolution.kind = EvolutionType::LevelWithOther;
            evolution.extra_info = partner;
            let partner_name = data.full_name(PokemonId::from_index(partner as usize));
            add_evo_update_party(
                data,
                &mut self.core.engine.impossible_evolution_updates,
                evo,
                &partner_name,
            );
        }
        Ok(())
    }

    fn add_extra_evolutions(&mut self, pokemon: PokemonId, extras: &JavaHashSet<EvolutionKey>) {
        let data = &mut self.core.data;
        let mut created = Vec::new();
        for extra in extras.iter() {
            let id = data.new_evolution(extra.evolution.clone());
            data.evolutions_from_mut(pokemon).push(id);
            created.push(id);
        }
        for id in created {
            let to = data.evo(id).to;
            data.evolutions_to_mut(to).push(id);
        }
    }

    pub(crate) fn make_evos_easier(&mut self, settings: &Settings) -> Result<()> {
        let wilds_randomized = settings.wild_pokemon_mod != WildPokemonMod::Unchanged;
        let offset =
            randomon_ctr::bytes::find(&self.code, constants::FRIENDSHIP_VALUE_FOR_EVO_LOCATOR);
        if offset > 0 {
            let offset = offset as usize;
            for relative in [0, 12, 36] {
                if self.code[offset + relative] == 220 {
                    self.code[offset + relative] = 160;
                }
            }
        }
        if wilds_randomized {
            for pokemon in self.pokes.clone().into_iter().flatten() {
                for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                    let data = &mut self.core.data;
                    if data.evo(evo).kind == EvolutionType::LevelWithOther {
                        let evolution = data.evo_mut(evo);
                        evolution.kind = EvolutionType::Level;
                        evolution.extra_info = 35;
                        add_evo_update_condensed(
                            data,
                            &mut self.core.engine.easier_evolution_updates,
                            evo,
                            false,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn remove_time_evos(&mut self) -> Result<()> {
        let mut extra_evolutions: JavaHashSet<EvolutionKey> = JavaHashSet::new();
        for pokemon in self.pokes.clone().into_iter().flatten() {
            extra_evolutions.clear();
            for evo in self.core.data.evolutions_from(pokemon).to_vec() {
                self.fix_time_based_evolution(evo, &mut extra_evolutions);
            }
            self.add_extra_evolutions(pokemon, &extra_evolutions);
        }
        Ok(())
    }

    fn fix_time_based_evolution(
        &mut self,
        evo: EvolutionId,
        extra_evolutions: &mut JavaHashSet<EvolutionKey>,
    ) {
        let data = &mut self.core.data;
        let updates = &mut self.core.engine.time_based_evolution_updates;
        let evolution = data.evo(evo).clone();
        let is_eevee = evolution.from.number() == species::EEVEE;
        match evolution.kind {
            EvolutionType::HappinessDay | EvolutionType::HappinessNight if is_eevee => {
                let stone = if evolution.kind == EvolutionType::HappinessDay {
                    items::SUN_STONE
                } else {
                    items::MOON_STONE
                };
                let target = data.evo_mut(evo);
                target.kind = EvolutionType::Stone;
                target.extra_info = stone;
                let name = self.item_names[stone as usize].clone();
                add_evo_update_stone(data, updates, evo, &name);
            }
            EvolutionType::HappinessDay | EvolutionType::HappinessNight => {
                add_evo_update_happiness(data, updates, evo);
                let other = if evolution.kind == EvolutionType::HappinessDay {
                    EvolutionType::HappinessNight
                } else {
                    EvolutionType::HappinessDay
                };
                extra_evolutions.add(EvolutionKey {
                    evolution: Evolution::new(evolution.from, evolution.to, true, other, 0),
                });
            }
            EvolutionType::LevelItemDay | EvolutionType::LevelItemNight => {
                let other = if evolution.kind == EvolutionType::LevelItemDay {
                    EvolutionType::LevelItemNight
                } else {
                    EvolutionType::LevelItemDay
                };
                let item = evolution.extra_info;
                let already_present =
                    data.evolutions_from(evolution.from)
                        .iter()
                        .any(|&other_evo| {
                            data.evo(other_evo).kind == other
                                && data.evo(other_evo).extra_info == item
                        });
                if !already_present {
                    let name = self.item_names[item as usize].clone();
                    add_evo_update_held_item(data, updates, evo, &name);
                    extra_evolutions.add(EvolutionKey {
                        evolution: Evolution::new(evolution.from, evolution.to, true, other, item),
                    });
                }
            }
            EvolutionType::LevelDay | EvolutionType::LevelNight => {
                add_evo_update_level(data, updates, evo);
                data.evo_mut(evo).kind = EvolutionType::Level;
            }
            _ => {}
        }
    }

    pub(crate) fn read_movesets(&self) -> Result<Movesets> {
        let garc = self.read_garc(self.file("PokemonMovesets"), Decompression::Skip)?;
        let mut movesets = BTreeMap::new();
        for number in 1..=self.total_pokemon() {
            let data = garc_file(&garc, number as usize)?;
            let mut learnt = Vec::new();
            let mut offset = 0;
            while read_word(data, offset) != 0xFFFF || read_word(data, offset + 2) != 0xFFFF {
                learnt.push(randomon_data::MoveLearnt {
                    move_number: read_word(data, offset),
                    level: read_word(data, offset + 2),
                });
                offset += 4;
            }
            movesets.insert(number, learnt);
        }
        Ok(movesets)
    }

    pub(crate) fn write_movesets(&self, movesets: &Movesets) -> Result<()> {
        let path = self.file("PokemonMovesets");
        let mut garc = self.read_garc(path, Decompression::Skip)?;
        for number in 1..=self.total_pokemon() {
            let learnt = movesets
                .get(&number)
                .ok_or(RandomizationError::NullPointer("movesets.get"))?;
            let mut moveset = vec![0u8; learnt.len() * 4 + 4];
            for (index, entry) in learnt.iter().enumerate() {
                write_word(&mut moveset, index * 4, entry.move_number);
                write_word(&mut moveset, index * 4 + 2, entry.level);
            }
            write_word(&mut moveset, learnt.len() * 4, 0xFFFF);
            write_word(&mut moveset, learnt.len() * 4 + 2, 0xFFFF);
            garc.set_file(number as usize, moveset)
                .map_err(crate::handler::io)?;
        }
        self.write_garc(path, &garc)
    }

    pub(crate) fn read_egg_moves(&self) -> Result<BTreeMap<i32, Vec<i32>>> {
        let garc = self.read_garc(self.file("EggMoves"), Decompression::Skip)?;
        let mut egg_moves = BTreeMap::new();
        for number in 1..=POKEMON_COUNT {
            let data = garc_file(&garc, number as usize)?;
            let count = read_word(data, 0) as usize;
            let moves = (0..count)
                .map(|index| read_word(data, 2 + index * 2))
                .collect();
            egg_moves.insert(number, moves);
        }
        Ok(egg_moves)
    }

    pub(crate) fn write_egg_moves(&self, egg_moves: &BTreeMap<i32, Vec<i32>>) -> Result<()> {
        let path = self.file("EggMoves");
        let mut garc = self.read_garc(path, Decompression::Skip)?;
        for number in 1..=POKEMON_COUNT {
            let moves = egg_moves
                .get(&number)
                .ok_or(RandomizationError::NullPointer("eggMoves.get"))?;
            let data = garc_file_mut(&mut garc, number as usize)?;
            for (index, &move_number) in moves.iter().enumerate() {
                write_word(data, 2 + index * 2, move_number);
            }
        }
        self.write_garc(path, &garc)
    }
}
