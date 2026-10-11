use std::collections::BTreeMap;

use randomon_core::{RandomizationError, Result};
use randomon_data::constants::{gen5, species};
use randomon_data::{ExpCurve, GameData, PokemonId, StatLayout};
use randomon_nds::{read_word, write_word};

use crate::handler::{FormeInfo, Gen5RomHandler};

fn byte_of(stats: &[u8], offset: i32) -> i32 {
    i32::from(stats[offset as usize])
}

fn is_cosmetic_forme_candidate(number: i32) -> bool {
    number != species::CHERRIM
        && number != species::ARCEUS
        && number != species::DEERLING
        && number != species::SAWSBUCK
        && number < species::GENESECT
}

fn load_basic_poke_stats(
    data: &mut GameData,
    id: PokemonId,
    stats: &[u8],
    alt_formes: &mut BTreeMap<i32, FormeInfo>,
) {
    let primary = gen5::type_table(byte_of(stats, gen5::BS_PRIMARY_TYPE_OFFSET));
    let mut secondary = gen5::type_table(byte_of(stats, gen5::BS_SECONDARY_TYPE_OFFSET));
    if secondary == primary {
        secondary = None;
    }
    let common_item = read_word(stats, gen5::BS_COMMON_HELD_ITEM_OFFSET);
    let rare_item = read_word(stats, gen5::BS_RARE_HELD_ITEM_OFFSET);
    let pk = data.pk_mut(id);
    pk.hp = byte_of(stats, gen5::BS_HP_OFFSET);
    pk.attack = byte_of(stats, gen5::BS_ATTACK_OFFSET);
    pk.defense = byte_of(stats, gen5::BS_DEFENSE_OFFSET);
    pk.speed = byte_of(stats, gen5::BS_SPEED_OFFSET);
    pk.spatk = byte_of(stats, gen5::BS_SP_ATK_OFFSET);
    pk.spdef = byte_of(stats, gen5::BS_SP_DEF_OFFSET);
    pk.primary_type = primary;
    pk.secondary_type = secondary;
    pk.catch_rate = byte_of(stats, gen5::BS_CATCH_RATE_OFFSET);
    pk.growth_curve = ExpCurve::from_byte(stats[gen5::BS_GROWTH_CURVE_OFFSET as usize] as i8);
    pk.ability1 = byte_of(stats, gen5::BS_ABILITY1_OFFSET);
    pk.ability2 = byte_of(stats, gen5::BS_ABILITY2_OFFSET);
    pk.ability3 = byte_of(stats, gen5::BS_ABILITY3_OFFSET);
    if common_item == rare_item {
        pk.guaranteed_held_item = common_item;
        pk.common_held_item = 0;
        pk.rare_held_item = 0;
        pk.dark_grass_held_item = 0;
    } else {
        pk.guaranteed_held_item = 0;
        pk.common_held_item = common_item;
        pk.rare_held_item = rare_item;
        pk.dark_grass_held_item = read_word(stats, gen5::BS_DARK_GRASS_HELD_ITEM_OFFSET);
    }
    let forme_count = byte_of(stats, gen5::BS_FORME_COUNT_OFFSET);
    if forme_count <= 1 {
        return;
    }
    let number = pk.number;
    let first_forme_offset = read_word(stats, gen5::BS_FORME_OFFSET);
    if first_forme_offset != 0 {
        let sprite_offset = read_word(stats, gen5::BS_FORME_SPRITE_OFFSET);
        for forme in 1..forme_count {
            alt_formes.insert(
                first_forme_offset + forme - 1,
                FormeInfo {
                    base_forme: number,
                    forme_number: forme,
                    forme_sprite_offset: sprite_offset,
                },
            );
            if number == species::KELDEO {
                pk.cosmetic_forms = forme_count;
            }
        }
    } else {
        if is_cosmetic_forme_candidate(number) {
            pk.cosmetic_forms = forme_count;
        }
        if number == species::gen5_formes::KELDEO_COSMETIC1 {
            pk.actually_cosmetic = true;
        }
    }
}

fn save_basic_poke_stats(data: &GameData, id: PokemonId, stats: &mut [u8]) -> Result<()> {
    let pk = data.pk(id);
    let mut set = |offset: i32, value: i32| stats[offset as usize] = value as u8;
    set(gen5::BS_HP_OFFSET, pk.hp);
    set(gen5::BS_ATTACK_OFFSET, pk.attack);
    set(gen5::BS_DEFENSE_OFFSET, pk.defense);
    set(gen5::BS_SPEED_OFFSET, pk.speed);
    set(gen5::BS_SP_ATK_OFFSET, pk.spatk);
    set(gen5::BS_SP_DEF_OFFSET, pk.spdef);
    let primary = i32::from(gen5::type_to_byte(pk.primary_type));
    set(gen5::BS_PRIMARY_TYPE_OFFSET, primary);
    match pk.secondary_type {
        None => set(gen5::BS_SECONDARY_TYPE_OFFSET, primary),
        Some(kind) => set(
            gen5::BS_SECONDARY_TYPE_OFFSET,
            i32::from(gen5::type_to_byte(Some(kind))),
        ),
    }
    set(gen5::BS_CATCH_RATE_OFFSET, pk.catch_rate);
    let growth = pk
        .growth_curve
        .ok_or(RandomizationError::NullPointer("pkmn.growthCurve"))?;
    set(gen5::BS_GROWTH_CURVE_OFFSET, i32::from(growth.to_byte()));
    set(gen5::BS_ABILITY1_OFFSET, pk.ability1);
    set(gen5::BS_ABILITY2_OFFSET, pk.ability2);
    set(gen5::BS_ABILITY3_OFFSET, pk.ability3);
    let (common, rare, dark_grass) = if pk.guaranteed_held_item > 0 {
        (pk.guaranteed_held_item, pk.guaranteed_held_item, 0)
    } else {
        (
            pk.common_held_item,
            pk.rare_held_item,
            pk.dark_grass_held_item,
        )
    };
    write_word(stats, gen5::BS_COMMON_HELD_ITEM_OFFSET, common);
    write_word(stats, gen5::BS_RARE_HELD_ITEM_OFFSET, rare);
    write_word(stats, gen5::BS_DARK_GRASS_HELD_ITEM_OFFSET, dark_grass);
    Ok(())
}

fn narc_entry(files: &[Vec<u8>], index: i32) -> Result<&[u8]> {
    files
        .get(index as usize)
        .map(Vec::as_slice)
        .ok_or(RandomizationError::IndexOutOfBounds("pokeNarc.files"))
}

impl Gen5RomHandler {
    pub(crate) fn forme_count(&self) -> i32 {
        gen5::forme_count(self.rom_type())
    }

    pub(crate) fn forme_offset(&self) -> i32 {
        gen5::forme_offset(self.rom_type())
    }

    pub(crate) fn personal_index(&self, number: i32) -> i32 {
        if number > gen5::POKEMON_COUNT {
            number + self.forme_offset()
        } else {
            number
        }
    }

    fn read_pokemon_names(&self) -> Result<Vec<String>> {
        let names = self.get_strings(false, self.entry.int("PokemonNamesTextOffset"))?;
        (0..=gen5::POKEMON_COUNT as usize)
            .map(|index| {
                if index == 0 {
                    Ok(String::new())
                } else {
                    names
                        .get(index)
                        .cloned()
                        .ok_or(RandomizationError::IndexOutOfBounds("pokemon names"))
                }
            })
            .collect()
    }

    pub(crate) fn load_pokemon_stats(&mut self) -> Result<()> {
        let names = self.read_pokemon_names()?;
        let total = gen5::POKEMON_COUNT + self.forme_count();
        self.pokemon_list_incl_formes = vec![None];
        for number in 1..=gen5::POKEMON_COUNT {
            let id = self.core.data.create_pokemon(StatLayout::Standard, number);
            self.pokemon_list_incl_formes.push(Some(id));
            let stats = narc_entry(&self.poke_narc.files, number)?;
            load_basic_poke_stats(&mut self.core.data, id, stats, &mut self.forme_mappings);
            self.core.data.pk_mut(id).name = names[number as usize].clone();
        }
        let keys: Vec<i32> = self.forme_mappings.keys().copied().collect();
        for (number, key) in (gen5::POKEMON_COUNT + 1..).zip(keys) {
            if number > total {
                return Err(RandomizationError::IndexOutOfBounds("pokes"));
            }
            let id = self.core.data.create_pokemon(StatLayout::Standard, number);
            self.pokemon_list_incl_formes.push(Some(id));
            let stats = narc_entry(&self.poke_narc.files, key)?;
            let known_keys = self.forme_mappings.len();
            load_basic_poke_stats(&mut self.core.data, id, stats, &mut self.forme_mappings);
            if self.forme_mappings.len() != known_keys {
                return Err(RandomizationError::randomization(
                    "java.util.ConcurrentModificationException",
                ));
            }
            let info = self.forme_mappings[&key];
            let base_name = names
                .get(info.base_forme as usize)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("pokeNames"))?;
            let sprite_index = info.forme_sprite_offset
                + gen5::POKEMON_COUNT
                + gen5::non_pokemon_battle_sprite_count(self.rom_type());
            let suffix = gen5::forme_suffix(key, self.rom_type()).to_owned();
            let base = self.pk(info.base_forme);
            let pk = self.core.data.pk_mut(id);
            pk.name = base_name;
            pk.base_forme = Some(base);
            pk.forme_number = info.forme_number;
            pk.forme_sprite_index = sprite_index;
            pk.forme_suffix = suffix;
        }
        while self.pokemon_list_incl_formes.len() <= total as usize {
            self.pokemon_list_incl_formes.push(None);
        }
        self.pokemon_list = self.pokemon_list_incl_formes[..=gen5::POKEMON_COUNT as usize].to_vec();
        self.populate_evolutions()
    }

    pub(crate) fn save_pokemon_stats(&mut self) -> Result<()> {
        let names_index = self.entry.int("PokemonNamesTextOffset");
        let mut names = self.get_strings(false, names_index)?;
        let total = gen5::POKEMON_COUNT + self.forme_count();
        for number in 1..=total {
            let id = self.pk(number);
            let index = self.personal_index(number) as usize;
            let stats = self
                .poke_narc
                .files
                .get_mut(index)
                .ok_or(RandomizationError::IndexOutOfBounds("pokeNarc.files"))?;
            save_basic_poke_stats(&self.core.data, id, stats)?;
            if number <= gen5::POKEMON_COUNT {
                let name = self.core.data.pk(id).name.clone();
                let slot = names
                    .get_mut(number as usize)
                    .ok_or(RandomizationError::IndexOutOfBounds("nameList"))?;
                *slot = name;
            }
        }
        self.set_strings(false, names_index, &names)?;
        self.write_narc("PokemonStats", &self.poke_narc)?;
        self.write_evolutions()
    }
}
