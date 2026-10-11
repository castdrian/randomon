use randomon_core::{RandomizationError, Result};
use randomon_data::constants::gen4;
use randomon_data::{ExpCurve, GameData, PokemonId, StatLayout};

use crate::bytes::{file, file_mut, i8_at, set_u8, set_u16, u8_at, u16_at};
use crate::handler::Gen4RomHandler;

fn load_basic_poke_stats(
    data: &mut GameData,
    id: PokemonId,
    stats: &[u8],
    rom_type: i32,
) -> Result<()> {
    let primary = gen4::type_table(u8_at(stats, gen4::BS_PRIMARY_TYPE_OFFSET)?);
    let mut secondary = gen4::type_table(u8_at(stats, gen4::BS_SECONDARY_TYPE_OFFSET)?);
    if secondary == primary {
        secondary = None;
    }
    let common_item = u16_at(stats, gen4::BS_COMMON_HELD_ITEM_OFFSET)?;
    let rare_item = u16_at(stats, gen4::BS_RARE_HELD_ITEM_OFFSET)?;
    let growth = i8_at(stats, gen4::BS_GROWTH_CURVE_OFFSET)? as i8;
    let values = [
        u8_at(stats, gen4::BS_HP_OFFSET)?,
        u8_at(stats, gen4::BS_ATTACK_OFFSET)?,
        u8_at(stats, gen4::BS_DEFENSE_OFFSET)?,
        u8_at(stats, gen4::BS_SPEED_OFFSET)?,
        u8_at(stats, gen4::BS_SP_ATK_OFFSET)?,
        u8_at(stats, gen4::BS_SP_DEF_OFFSET)?,
        u8_at(stats, gen4::BS_CATCH_RATE_OFFSET)?,
        u8_at(stats, gen4::BS_ABILITY1_OFFSET)?,
        u8_at(stats, gen4::BS_ABILITY2_OFFSET)?,
        u8_at(stats, gen4::BS_GENDER_RATIO_OFFSET)?,
    ];
    let pk = data.pk_mut(id);
    [
        pk.hp,
        pk.attack,
        pk.defense,
        pk.speed,
        pk.spatk,
        pk.spdef,
        pk.catch_rate,
        pk.ability1,
        pk.ability2,
        pk.gender_ratio,
    ] = values;
    pk.primary_type = primary;
    pk.secondary_type = secondary;
    pk.growth_curve = ExpCurve::from_byte(growth);
    if common_item == rare_item {
        pk.guaranteed_held_item = common_item;
        pk.common_held_item = 0;
        pk.rare_held_item = 0;
    } else {
        pk.guaranteed_held_item = 0;
        pk.common_held_item = common_item;
        pk.rare_held_item = rare_item;
    }
    pk.dark_grass_held_item = -1;
    let cosmetic_forms = gen4::cosmetic_forms(pk.number);
    if cosmetic_forms > 0 && rom_type != gen4::TYPE_DP {
        pk.cosmetic_forms = cosmetic_forms;
    }
    Ok(())
}

fn save_basic_poke_stats(data: &GameData, id: PokemonId, stats: &mut [u8]) -> Result<()> {
    let pk = data.pk(id);
    let primary = i32::from(gen4::type_to_byte(pk.primary_type));
    let secondary = match pk.secondary_type {
        None => primary,
        Some(kind) => i32::from(gen4::type_to_byte(Some(kind))),
    };
    let growth = pk
        .growth_curve
        .ok_or(RandomizationError::NullPointer("pkmn.growthCurve"))?;
    for (offset, value) in [
        (gen4::BS_HP_OFFSET, pk.hp),
        (gen4::BS_ATTACK_OFFSET, pk.attack),
        (gen4::BS_DEFENSE_OFFSET, pk.defense),
        (gen4::BS_SPEED_OFFSET, pk.speed),
        (gen4::BS_SP_ATK_OFFSET, pk.spatk),
        (gen4::BS_SP_DEF_OFFSET, pk.spdef),
        (gen4::BS_PRIMARY_TYPE_OFFSET, primary),
        (gen4::BS_SECONDARY_TYPE_OFFSET, secondary),
        (gen4::BS_CATCH_RATE_OFFSET, pk.catch_rate),
        (gen4::BS_GROWTH_CURVE_OFFSET, i32::from(growth.to_byte())),
        (gen4::BS_ABILITY1_OFFSET, pk.ability1),
        (gen4::BS_ABILITY2_OFFSET, pk.ability2),
    ] {
        set_u8(stats, offset, value)?;
    }
    let (common, rare) = if pk.guaranteed_held_item > 0 {
        (pk.guaranteed_held_item, pk.guaranteed_held_item)
    } else {
        (pk.common_held_item, pk.rare_held_item)
    };
    set_u16(stats, gen4::BS_COMMON_HELD_ITEM_OFFSET, common)?;
    set_u16(stats, gen4::BS_RARE_HELD_ITEM_OFFSET, rare)
}

impl Gen4RomHandler {
    pub(crate) fn forme_count(&self) -> i32 {
        gen4::forme_count(self.rom_type())
    }

    pub(crate) fn total_pokemon(&self) -> i32 {
        gen4::POKEMON_COUNT + self.forme_count()
    }

    pub(crate) fn personal_index(number: i32) -> i32 {
        if number > gen4::POKEMON_COUNT {
            number + gen4::FORME_OFFSET
        } else {
            number
        }
    }

    fn read_pokemon_names(&self) -> Result<Vec<String>> {
        let names = self.get_strings(self.entry.int("PokemonNamesTextOffset"))?;
        (0..=gen4::POKEMON_COUNT as usize)
            .map(|index| match index {
                0 => Ok(String::new()),
                _ => names
                    .get(index)
                    .cloned()
                    .ok_or(RandomizationError::IndexOutOfBounds("pokemon names")),
            })
            .collect()
    }

    pub(crate) fn load_pokemon_stats(&mut self) -> Result<()> {
        let names = self.read_pokemon_names()?;
        let rom_type = self.rom_type();
        self.pokemon_list_incl_formes = vec![None];
        for number in 1..=gen4::POKEMON_COUNT {
            let id = self.core.data.create_pokemon(StatLayout::Standard, number);
            self.pokemon_list_incl_formes.push(Some(id));
            let stats = file(&self.poke_narc, number)?;
            load_basic_poke_stats(&mut self.core.data, id, stats, rom_type)?;
            self.core.data.pk_mut(id).name = names[number as usize].clone();
        }
        let total = self.total_pokemon();
        for (number, mapping) in (gen4::POKEMON_COUNT + 1..=total).zip(gen4::FORME_MAPPINGS) {
            let id = self.core.data.create_pokemon(StatLayout::Standard, number);
            self.pokemon_list_incl_formes.push(Some(id));
            let stats = file(&self.poke_narc, mapping.internal_index)?;
            load_basic_poke_stats(&mut self.core.data, id, stats, rom_type)?;
            let base = self.pk(mapping.base_forme);
            let pk = self.core.data.pk_mut(id);
            pk.name = names[mapping.base_forme as usize].clone();
            pk.base_forme = Some(base);
            pk.forme_number = mapping.forme_number;
            pk.forme_suffix = mapping.suffix.to_owned();
        }
        self.pokemon_list = self.pokemon_list_incl_formes[..=gen4::POKEMON_COUNT as usize].to_vec();
        self.populate_evolutions()
    }

    pub(crate) fn alt_forme_of(&self, pokemon: PokemonId, forme: i32) -> PokemonId {
        let number = self.number_of(pokemon);
        match gen4::absolute_poke_num_by_base_forme(number, forme) {
            0 => pokemon,
            absolute => self.pokemon_by_number(absolute).unwrap_or(pokemon),
        }
    }

    fn update_extra_names(&mut self, names: &[String]) -> Result<()> {
        if !self
            .entry
            .string("HasExtraPokemonNames")
            .eq_ignore_ascii_case("Yes")
        {
            return Ok(());
        }
        let index = self.entry.int("PokemonNamesTextOffset") + 1;
        let mut extra_names = self.get_strings(index)?;
        for number in 1..=gen4::POKEMON_COUNT as usize {
            let new_name = &self.core.data.pk(self.pk(number as i32)).name;
            if let (Some(extra), Some(old_name)) = (extra_names.get_mut(number), names.get(number))
            {
                *extra = extra.replace(old_name.as_str(), new_name);
            }
        }
        self.set_strings(index, &extra_names)
    }

    pub(crate) fn save_pokemon_stats(&mut self) -> Result<()> {
        let names_index = self.entry.int("PokemonNamesTextOffset");
        let original_names = self.get_strings(names_index)?;
        let mut names = original_names.clone();
        for number in 1..=self.total_pokemon() {
            let id = self.pk(number);
            let stats = file_mut(&mut self.poke_narc, Self::personal_index(number))?;
            save_basic_poke_stats(&self.core.data, id, stats)?;
            if number <= gen4::POKEMON_COUNT
                && let Some(slot) = names.get_mut(number as usize)
            {
                *slot = self.core.data.pk(id).name.clone();
            }
        }
        self.update_extra_names(&original_names)?;
        self.set_strings(names_index, &names)?;
        self.write_narc("PokemonStats", &self.poke_narc)?;
        self.write_evolutions()
    }
}
