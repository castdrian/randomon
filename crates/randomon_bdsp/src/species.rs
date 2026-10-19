use std::collections::BTreeMap;

use randomon_core::{Compatibility, EggMoves, Movesets};
use randomon_data::constants::species;
use randomon_data::{Evolution, EvolutionType, ExpCurve, MoveLearnt, PokemonId, StatLayout};
use randomon_gen7::constants::{type_from_byte, type_to_byte};
use randomon_unity::Value;

use crate::assets::SPECIES_NAMES;
use crate::constants;
use crate::error::{Result, data_error};
use crate::handler::BdspRomHandler;
use crate::tables::{Table, int, ints, set_int, set_u16s};

const MACHINE_FIELDS: [&str; 4] = ["machine1", "machine2", "machine3", "machine4"];
const EVOLUTION_RECORD: usize = 5;

fn species_names() -> Vec<String> {
    let mut names: Vec<String> = SPECIES_NAMES
        .lines()
        .map(|line| line.trim().to_owned())
        .collect();
    if let Some(first) = names.first_mut() {
        first.clear();
    }
    names
}

struct PersonalRow {
    id: i32,
    monsno: i32,
    form_index: i32,
    form_max: i32,
}

impl PersonalRow {
    fn read(row: &Value) -> Result<Self> {
        Ok(Self {
            id: int(row, "id")?,
            monsno: int(row, "monsno")?,
            form_index: int(row, "form_index")?,
            form_max: int(row, "form_max")?,
        })
    }
}

impl BdspRomHandler {
    pub(crate) fn load_pokemon(&mut self) -> Result<()> {
        let names = species_names();
        let rows: Vec<Value> = self.tables.rows(Table::Personal, "Personal")?.to_vec();
        self.pokes = vec![None; constants::PERSONAL_COUNT as usize + 1];
        for row in &rows {
            let info = PersonalRow::read(row)?;
            if info.id == 0 || info.id > constants::PERSONAL_COUNT {
                continue;
            }
            let id = self.core.data.create_pokemon(StatLayout::Standard, info.id);
            self.pokes[info.id as usize] = Some(id);
            self.read_stats(id, row)?;
            let name = names
                .get(info.monsno as usize)
                .cloned()
                .unwrap_or_else(|| format!("#{}", info.monsno));
            self.core.data.pk_mut(id).name = name;
            if info.id == info.monsno && info.form_max > 1 && info.form_index > 0 {
                self.first_forme
                    .insert(info.monsno, (info.form_index, info.form_max));
            }
        }
        self.link_formes()?;
        self.load_evolutions()
    }

    fn link_formes(&mut self) -> Result<()> {
        for (&species_number, &(first, count)) in &self.first_forme.clone() {
            let base = self.pk(species_number);
            let cosmetic = constants::ACTUALLY_COSMETIC_SPECIES.contains(&species_number);
            let counted =
                cosmetic && species_number != species::ARCEUS && species_number != species::CHERRIM;
            for forme in 1..count {
                let number = first + forme - 1;
                let id = self.pk(number);
                if !self.core.data.exists(id) {
                    return Err(data_error(format!("missing personal entry {number}")));
                }
                let pokemon = self.core.data.pk_mut(id);
                pokemon.base_forme = Some(base);
                pokemon.forme_number = forme;
                pokemon.actually_cosmetic = cosmetic;
                pokemon.forme_suffix = if cosmetic {
                    String::new()
                } else {
                    constants::forme_suffix(species_number, forme).to_owned()
                };
                if counted {
                    let base_pokemon = self.core.data.pk_mut(base);
                    base_pokemon.cosmetic_forms += 1;
                    base_pokemon.real_cosmetic_form_numbers.push(forme);
                }
            }
            let base_pokemon = self.core.data.pk_mut(base);
            if !base_pokemon.real_cosmetic_form_numbers.is_empty() {
                base_pokemon.real_cosmetic_form_numbers.push(0);
                base_pokemon.cosmetic_forms += 1;
            }
        }
        Ok(())
    }

    fn read_stats(&mut self, id: PokemonId, row: &Value) -> Result<()> {
        let pokemon = self.core.data.pk_mut(id);
        pokemon.hp = int(row, "basic_hp")?;
        pokemon.attack = int(row, "basic_atk")?;
        pokemon.defense = int(row, "basic_def")?;
        pokemon.speed = int(row, "basic_agi")?;
        pokemon.spatk = int(row, "basic_spatk")?;
        pokemon.spdef = int(row, "basic_spdef")?;
        pokemon.primary_type = type_from_byte(int(row, "type1")? as u8);
        pokemon.secondary_type = type_from_byte(int(row, "type2")? as u8);
        if pokemon.secondary_type == pokemon.primary_type {
            pokemon.secondary_type = None;
        }
        pokemon.catch_rate = int(row, "get_rate")?;
        pokemon.exp_yield = int(row, "give_exp")?;
        pokemon.gender_ratio = int(row, "sex")?;
        pokemon.growth_curve = ExpCurve::from_byte(int(row, "grow")? as i8);
        pokemon.ability1 = int(row, "tokusei1")?;
        pokemon.ability2 = int(row, "tokusei2")?;
        pokemon.ability3 = int(row, "tokusei3")?;
        if pokemon.ability1 == pokemon.ability2 {
            pokemon.ability2 = 0;
        }
        let common = int(row, "item1")?;
        let rare = int(row, "item2")?;
        if common == rare && common != 0 {
            pokemon.guaranteed_held_item = common;
        } else {
            pokemon.common_held_item = common;
            pokemon.rare_held_item = rare;
        }
        pokemon.dark_grass_held_item = int(row, "item3")?;
        Ok(())
    }

    fn write_stats(&self, id: PokemonId, row: &mut Value) -> Result<()> {
        let pokemon = self.core.data.pk(id);
        let keeps_own_types = pokemon.actually_cosmetic
            && pokemon
                .base_forme
                .is_some_and(|base| base.number() == species::ARCEUS);
        set_int(row, "basic_hp", pokemon.hp)?;
        set_int(row, "basic_atk", pokemon.attack)?;
        set_int(row, "basic_def", pokemon.defense)?;
        set_int(row, "basic_agi", pokemon.speed)?;
        set_int(row, "basic_spatk", pokemon.spatk)?;
        set_int(row, "basic_spdef", pokemon.spdef)?;
        if !keeps_own_types {
            let primary = type_to_byte(pokemon.primary_type);
            let secondary = pokemon
                .secondary_type
                .map_or(primary, |kind| type_to_byte(Some(kind)));
            set_int(row, "type1", i32::from(primary))?;
            set_int(row, "type2", i32::from(secondary))?;
        }
        set_int(row, "get_rate", pokemon.catch_rate)?;
        set_int(row, "give_exp", pokemon.exp_yield)?;
        if let Some(curve) = pokemon.growth_curve {
            set_int(row, "grow", i32::from(curve.to_byte()))?;
        }
        set_int(row, "tokusei1", pokemon.ability1)?;
        let ability2 = if pokemon.ability2 == 0 {
            pokemon.ability1
        } else {
            pokemon.ability2
        };
        set_int(row, "tokusei2", ability2)?;
        set_int(row, "tokusei3", pokemon.ability3)?;
        let (common, rare) = if pokemon.guaranteed_held_item > 0 {
            (pokemon.guaranteed_held_item, pokemon.guaranteed_held_item)
        } else {
            (pokemon.common_held_item, pokemon.rare_held_item)
        };
        set_int(row, "item1", common)?;
        set_int(row, "item2", rare)?;
        if pokemon.dark_grass_held_item >= 0 {
            set_int(row, "item3", pokemon.dark_grass_held_item)?;
        }
        Ok(())
    }

    pub(crate) fn save_pokemon(&mut self) -> Result<()> {
        let mut rows = self.tables.rows(Table::Personal, "Personal")?.to_vec();
        for row in rows.iter_mut() {
            let number = int(row, "id")?;
            let Some(Some(id)) = self.pokes.get(number as usize).copied() else {
                continue;
            };
            self.write_stats(id, row)?;
        }
        *self.tables.rows_mut(Table::Personal, "Personal")? = rows;
        self.save_evolutions()
    }

    fn load_evolutions(&mut self) -> Result<()> {
        let rows: Vec<Value> = self.tables.rows(Table::Evolve, "Evolve")?.to_vec();
        for row in &rows {
            let number = int(row, "id")?;
            let Some(Some(from)) = self.pokes.get(number as usize).copied() else {
                continue;
            };
            let records = ints(row, "ar")?;
            for record in records.as_chunks::<EVOLUTION_RECORD>().0 {
                let [method, parameter, target, forme, level] =
                    [record[0], record[1], record[2], record[3], record[4]];
                let Some(kind) = EvolutionType::from_index(7, method) else {
                    continue;
                };
                if !(1..=constants::SPECIES_COUNT).contains(&target) {
                    continue;
                }
                let to = self.pokemon_with_forme(target, forme);
                let extra_info = if kind.uses_level() { level } else { parameter };
                let mut evolution = Evolution::new(from, to, true, kind, extra_info);
                evolution.forme = forme;
                evolution.level = level;
                let data = &self.core.data;
                if data.evolution_list_contains(data.evolutions_from(from), &evolution) {
                    continue;
                }
                let evo = self.core.data.new_evolution(evolution);
                self.core.data.evolutions_from_mut(from).push(evo);
                if !self.core.data.pk(from).actually_cosmetic {
                    self.core.data.evolutions_to_mut(to).push(evo);
                }
            }
            let data = &mut self.core.data;
            if data.evolutions_from(from).len() > 1 {
                for evo in data.evolutions_from(from).to_vec() {
                    if data.evo(evo).kind != EvolutionType::LevelCreateExtra {
                        data.evo_mut(evo).carry_stats = false;
                    }
                }
            }
        }
        Ok(())
    }

    fn save_evolutions(&mut self) -> Result<()> {
        let mut rows = self.tables.rows(Table::Evolve, "Evolve")?.to_vec();
        for row in rows.iter_mut() {
            let number = int(row, "id")?;
            let Some(Some(from)) = self.pokes.get(number as usize).copied() else {
                continue;
            };
            let data = &self.core.data;
            let mut records = Vec::new();
            for &evo in data
                .evolutions_from(from)
                .iter()
                .take(constants::EVOLUTIONS_PER_SPECIES)
            {
                let evolution = data.evo(evo);
                let uses_level = evolution.kind.uses_level();
                records.extend([
                    evolution.kind.to_index(7),
                    if uses_level { 0 } else { evolution.extra_info },
                    data.base_number(evolution.to),
                    evolution.forme.max(0),
                    if uses_level {
                        evolution.extra_info
                    } else {
                        evolution.level.max(0)
                    },
                ]);
            }
            set_u16s(row, "ar", &records)?;
        }
        *self.tables.rows_mut(Table::Evolve, "Evolve")? = rows;
        Ok(())
    }

    pub(crate) fn read_movesets(&self) -> Result<Movesets> {
        let mut movesets = BTreeMap::new();
        for row in self.tables.rows(Table::WazaOboe, "WazaOboe")? {
            let number = int(row, "id")?;
            if !(1..=constants::PERSONAL_COUNT).contains(&number) {
                continue;
            }
            let learnt = ints(row, "ar")?
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| MoveLearnt {
                    level: pair[0],
                    move_number: pair[1],
                })
                .collect();
            movesets.insert(number, learnt);
        }
        Ok(movesets)
    }

    pub(crate) fn write_movesets(&mut self, movesets: &Movesets) -> Result<()> {
        for row in self.tables.rows_mut(Table::WazaOboe, "WazaOboe")? {
            let number = int(row, "id")?;
            let Some(learnt) = movesets.get(&number) else {
                continue;
            };
            let flattened: Vec<i32> = learnt
                .iter()
                .flat_map(|entry| [entry.level, entry.move_number])
                .collect();
            set_u16s(row, "ar", &flattened)?;
        }
        Ok(())
    }

    fn egg_move_owner(&self, row: &Value) -> Result<Option<i32>> {
        let species_number = int(row, "no")?;
        let forme = int(row, "formNo")?;
        if !(1..=constants::SPECIES_COUNT).contains(&species_number) {
            return Ok(None);
        }
        if forme == 0 {
            return Ok(Some(species_number));
        }
        Ok(self.alt_forme_number(species_number, forme))
    }

    pub(crate) fn read_egg_moves(&self) -> Result<EggMoves> {
        let mut egg_moves = BTreeMap::new();
        for row in self.tables.rows(Table::TamagoWaza, "Data")? {
            if let Some(owner) = self.egg_move_owner(row)? {
                egg_moves.insert(owner, ints(row, "wazaNo")?);
            }
        }
        Ok(egg_moves)
    }

    pub(crate) fn write_egg_moves(&mut self, egg_moves: &EggMoves) -> Result<()> {
        let mut rows = self.tables.rows(Table::TamagoWaza, "Data")?.to_vec();
        for row in rows.iter_mut() {
            let Some(owner) = self.egg_move_owner(row)? else {
                continue;
            };
            if let Some(moves) = egg_moves.get(&owner) {
                set_u16s(row, "wazaNo", moves)?;
            }
        }
        *self.tables.rows_mut(Table::TamagoWaza, "Data")? = rows;
        Ok(())
    }

    pub(crate) fn read_tm_compatibility(&self) -> Result<Compatibility> {
        let mut compatibility = BTreeMap::new();
        for row in self.tables.rows(Table::Personal, "Personal")? {
            let number = int(row, "id")?;
            let Some(Some(id)) = self.pokes.get(number as usize).copied() else {
                continue;
            };
            let mut flags = vec![false; constants::TM_COUNT as usize + 1];
            for (word_index, field) in MACHINE_FIELDS.iter().enumerate() {
                let word = row.int_field(field)? as u32;
                for bit in 0..32 {
                    let tm = word_index * 32 + bit + 1;
                    if tm < flags.len() && word & (1 << bit) != 0 {
                        flags[tm] = true;
                    }
                }
            }
            compatibility.insert(id, flags);
        }
        Ok(compatibility)
    }

    pub(crate) fn write_tm_compatibility(&mut self, compatibility: &Compatibility) -> Result<()> {
        let pokes = self.pokes.clone();
        for row in self.tables.rows_mut(Table::Personal, "Personal")? {
            let number = int(row, "id")?;
            let Some(Some(id)) = pokes.get(number as usize).copied() else {
                continue;
            };
            let Some(flags) = compatibility.get(&id) else {
                continue;
            };
            let mut words = [0u32; 4];
            for (tm, &learns) in flags.iter().enumerate().skip(1) {
                if learns {
                    let bit = tm - 1;
                    words[bit / 32] |= 1 << (bit % 32);
                }
            }
            for (field, word) in MACHINE_FIELDS.iter().zip(words) {
                row.set_int_field(field, i128::from(word))?;
            }
        }
        Ok(())
    }
}
