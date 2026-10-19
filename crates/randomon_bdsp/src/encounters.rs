use randomon_data::{Encounter, EncounterSet, PokemonId};
use randomon_unity::Value;

use crate::constants;
use crate::error::{Result, data_error};
use crate::handler::BdspRomHandler;
use crate::tables::{Table, int, set_int};

const ZONE_SETS: [(&str, &str, &str); 12] = [
    ("ground_mons", "encRate_gr", "Grass"),
    ("tairyo", "encRate_gr", "Swarm"),
    ("swayGrass", "encRate_gr", "Poké Radar"),
    ("gbaRuby", "encRate_gr", "Ruby"),
    ("gbaSapp", "encRate_gr", "Sapphire"),
    ("gbaEme", "encRate_gr", "Emerald"),
    ("gbaFire", "encRate_gr", "FireRed"),
    ("gbaLeaf", "encRate_gr", "LeafGreen"),
    ("water_mons", "encRate_wat", "Surfing"),
    ("boro_mons", "encRate_turi_boro", "Old Rod"),
    ("ii_mons", "encRate_turi_ii", "Good Rod"),
    ("sugoi_mons", "encRate_sugoi", "Super Rod"),
];
const TIME_SETS: [(&str, &str); 2] = [("day", "Day"), ("night", "Night")];
const TIME_REPLACED_GRASS_SLOTS: [usize; 2] = [2, 3];
const ENCOUNTER_TABLES: [(Table, &str); 2] = [
    (Table::EncounterDiamond, "Brilliant Diamond"),
    (Table::EncounterPearl, "Shining Pearl"),
];
const TROPHY_GARDEN_LEVEL: i32 = 16;
const HONEY_TREE_LEVEL: i32 = 15;
const GREAT_MARSH_LEVEL: i32 = 25;
const UNDERGROUND_LEVEL: i32 = 30;

#[derive(Clone, Copy)]
enum Source {
    Zone {
        table: Table,
        row: usize,
        field: &'static str,
    },
    List {
        table: Table,
        field: &'static str,
        species_fields: &'static [&'static str],
    },
    Underground(usize),
}

impl BdspRomHandler {
    fn zone_rows(&self, table: Table) -> Result<&[Value]> {
        self.tables.rows(table, "table")
    }

    fn sources(&self, use_time_of_day: bool) -> Result<Vec<(Source, String, i32)>> {
        let mut sources = Vec::new();
        for (table, game) in ENCOUNTER_TABLES {
            for (row_index, row) in self.zone_rows(table)?.iter().enumerate() {
                let zone = int(row, "zoneID")?;
                let mut sets: Vec<(&'static str, &'static str, &str)> = ZONE_SETS.to_vec();
                if use_time_of_day {
                    for (field, name) in TIME_SETS {
                        sets.insert(1, (field, "encRate_gr", name));
                    }
                }
                for (field, rate_field, name) in sets {
                    let rate = int(row, rate_field)?;
                    let slots = row
                        .field(field)?
                        .as_array()
                        .ok_or_else(|| data_error(format!("{field} is not an array")))?;
                    let populated = slots
                        .iter()
                        .any(|slot| int(slot, "monsNo").is_ok_and(|species| species > 0));
                    if rate <= 0 || !populated {
                        continue;
                    }
                    sources.push((
                        Source::Zone {
                            table,
                            row: row_index,
                            field,
                        },
                        format!("{game} Zone {zone} {name}"),
                        rate,
                    ));
                }
            }
        }
        sources.push((
            Source::List {
                table: Table::EncounterDiamond,
                field: "urayama",
                species_fields: &["monsNo"],
            },
            "Trophy Garden".to_owned(),
            0,
        ));
        sources.push((
            Source::List {
                table: Table::EncounterDiamond,
                field: "mistu",
                species_fields: &["Normal", "Rare", "SuperRare"],
            },
            "Honey Trees".to_owned(),
            0,
        ));
        sources.push((
            Source::List {
                table: Table::EncounterDiamond,
                field: "safari",
                species_fields: &["MonsNo"],
            },
            "Great Marsh".to_owned(),
            0,
        ));
        for index in 0..self.tables.underground().len() {
            sources.push((
                Source::Underground(index),
                format!("Grand Underground {}", index + 1),
                0,
            ));
        }
        Ok(sources)
    }

    fn encounter(&self, species_number: i32, level: i32, max_level: i32) -> Encounter {
        Encounter {
            level,
            max_level,
            pokemon: self.pk(species_number),
            ..Encounter::default()
        }
    }

    fn read_source(&self, source: Source) -> Result<Vec<Encounter>> {
        let valid = |species: i32| (1..=constants::SPECIES_COUNT).contains(&species);
        let mut encounters = Vec::new();
        match source {
            Source::Zone { table, row, field } => {
                let slots = self.zone_rows(table)?[row]
                    .field(field)?
                    .as_array()
                    .unwrap_or_default();
                for slot in slots {
                    let species = int(slot, "monsNo")?;
                    if valid(species) {
                        encounters.push(self.encounter(
                            species,
                            int(slot, "minlv")?,
                            int(slot, "maxlv")?,
                        ));
                    }
                }
            }
            Source::List {
                table,
                field,
                species_fields,
            } => {
                let level = list_level(field);
                for entry in self.tables.rows(table, field)? {
                    for species_field in species_fields {
                        let species = int(entry, species_field)?;
                        if valid(species) {
                            encounters.push(self.encounter(species, level, 0));
                        }
                    }
                }
            }
            Source::Underground(index) => {
                let rows = self.tables.underground()[index]
                    .field("table")?
                    .as_array()
                    .unwrap_or_default();
                for entry in rows {
                    let species = int(entry, "monsno")?;
                    if valid(species) {
                        encounters.push(self.encounter(species, UNDERGROUND_LEVEL, 0));
                    }
                }
            }
        }
        Ok(encounters)
    }

    pub(crate) fn read_encounters(&self, use_time_of_day: bool) -> Result<Vec<EncounterSet>> {
        let mut sets = Vec::new();
        for (index, (source, name, rate)) in self.sources(use_time_of_day)?.into_iter().enumerate()
        {
            let encounters = self.read_source(source)?;
            if encounters.is_empty() {
                continue;
            }
            sets.push(EncounterSet {
                rate,
                encounters,
                display_name: Some(name),
                offset: index as i32,
                ..EncounterSet::default()
            });
        }
        Ok(sets)
    }

    fn write_species_values(
        values: &mut [&mut Value],
        species_field: &str,
        encounters: &mut std::slice::Iter<'_, Encounter>,
        levels: bool,
        data_number: impl Fn(PokemonId) -> i32,
    ) -> Result<()> {
        let valid = |species: i32| (1..=constants::SPECIES_COUNT).contains(&species);
        for value in values.iter_mut() {
            if !valid(int(value, species_field)?) {
                continue;
            }
            let Some(encounter) = encounters.next() else {
                return Ok(());
            };
            set_int(value, species_field, data_number(encounter.pokemon))?;
            if levels {
                set_int(value, "minlv", encounter.level)?;
                set_int(value, "maxlv", encounter.max_level.max(encounter.level))?;
            }
        }
        Ok(())
    }

    fn write_source(&mut self, source: Source, set: &EncounterSet) -> Result<()> {
        let numbers: Vec<i32> = set
            .encounters
            .iter()
            .map(|encounter| self.core.data.base_number(encounter.pokemon))
            .collect();
        let mut renumbered = set.encounters.clone();
        for (encounter, &number) in renumbered.iter_mut().zip(&numbers) {
            encounter.pokemon = PokemonId::from_index(number as usize);
        }
        let mut iterator = renumbered.iter();
        let number_of = |pokemon: PokemonId| pokemon.index() as i32;
        match source {
            Source::Zone { table, row, field } => {
                let slots = self.tables.rows_mut(table, "table")?[row]
                    .field_mut(field)?
                    .as_array_mut()
                    .ok_or_else(|| data_error(format!("{field} is not an array")))?;
                let mut values: Vec<&mut Value> = slots.iter_mut().collect();
                Self::write_species_values(&mut values, "monsNo", &mut iterator, true, number_of)
            }
            Source::List {
                table,
                field,
                species_fields,
            } => {
                for entry in self.tables.rows_mut(table, field)? {
                    for species_field in species_fields {
                        let mut values = vec![&mut *entry];
                        Self::write_species_values(
                            &mut values,
                            species_field,
                            &mut iterator,
                            false,
                            number_of,
                        )?;
                    }
                }
                if table == Table::EncounterDiamond {
                    let copied = self.tables.rows(table, field)?.to_vec();
                    *self.tables.rows_mut(Table::EncounterPearl, field)? = copied;
                }
                Ok(())
            }
            Source::Underground(index) => {
                let rows = self.tables.underground_mut()[index]
                    .field_mut("table")?
                    .as_array_mut()
                    .ok_or_else(|| data_error("underground table is not an array"))?;
                let mut values: Vec<&mut Value> = rows.iter_mut().collect();
                Self::write_species_values(&mut values, "monsno", &mut iterator, false, number_of)
            }
        }
    }

    pub(crate) fn write_encounters(
        &mut self,
        use_time_of_day: bool,
        encounters: &[EncounterSet],
    ) -> Result<()> {
        let mut provided = encounters.iter();
        for (source, _, _) in self.sources(use_time_of_day)? {
            if self.read_source(source)?.is_empty() {
                continue;
            }
            let set = provided
                .next()
                .ok_or_else(|| data_error("fewer encounter sets than encounter tables"))?;
            self.write_source(source, set)?;
        }
        if !use_time_of_day {
            self.copy_grass_into_time_slots()?;
        }
        Ok(())
    }

    fn copy_grass_into_time_slots(&mut self) -> Result<()> {
        for (table, _) in ENCOUNTER_TABLES {
            for row in self.tables.rows_mut(table, "table")? {
                let grass = row
                    .field("ground_mons")?
                    .as_array()
                    .unwrap_or_default()
                    .to_vec();
                for (field, _) in TIME_SETS {
                    let Some(slots) = row.field_mut(field)?.as_array_mut() else {
                        continue;
                    };
                    for (slot, &grass_slot) in slots.iter_mut().zip(&TIME_REPLACED_GRASS_SLOTS) {
                        if let Some(source) = grass.get(grass_slot)
                            && int(slot, "monsNo")? > 0
                        {
                            set_int(slot, "monsNo", int(source, "monsNo")?)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn list_level(field: &str) -> i32 {
    match field {
        "urayama" => TROPHY_GARDEN_LEVEL,
        "mistu" => HONEY_TREE_LEVEL,
        _ => GREAT_MARSH_LEVEL,
    }
}
