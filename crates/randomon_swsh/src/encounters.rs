use randomon_core::Movesets;
use randomon_data::{Encounter, EncounterSet, moves_at_level};

use crate::formats::encounters::{EncounterArchive, Weather};
use crate::handler::SwshRomHandler;

const ARCHIVE_NAMES: [&str; 4] = [
    "Sword Hidden",
    "Sword Overworld",
    "Shield Hidden",
    "Shield Overworld",
];
const NEST_GAMES: [&str; 2] = ["Sword", "Shield"];
const NEST_BASE_LEVEL: i32 = 15;
const NEST_LEVEL_PER_RANK: i32 = 10;
const UNDERGROUND_NOT_GIGANTAMAX: u32 = 1;
const UNDERGROUND_GIGANTAMAX: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Wild {
        archive: usize,
        table: usize,
        sub_table: usize,
    },
    Nest(usize),
    Underground,
}

fn weather_name(index: usize) -> String {
    Weather::from_sub_table_index(index).map_or_else(
        || format!("Table {index}"),
        |weather| format!("{weather:?}"),
    )
}

fn nest_level(probabilities: &[u32]) -> i32 {
    let rank = probabilities
        .iter()
        .position(|&probability| probability > 0)
        .unwrap_or(0) as i32;
    NEST_BASE_LEVEL + rank * NEST_LEVEL_PER_RANK
}

impl SwshRomHandler {
    fn archives(&self) -> [&EncounterArchive; 4] {
        let wild = &self.files.data.wild;
        [
            &wild.sword_hidden,
            &wild.sword_symbol,
            &wild.shield_hidden,
            &wild.shield_symbol,
        ]
    }

    fn encounter(&self, species: i32, form: i32, level: i32, max_level: i32) -> Option<Encounter> {
        let pokemon = self.pokemon_with_forme(species, form)?;
        let forme_number = if self.core.data.pk(pokemon).base_forme.is_some() {
            form
        } else {
            0
        };
        Some(Encounter {
            level,
            max_level,
            pokemon,
            forme_number,
            ..Encounter::default()
        })
    }

    fn sources(&self) -> Vec<(Source, String, i32)> {
        let mut sources = Vec::new();
        for (archive, encounters) in self.archives().into_iter().enumerate() {
            for (table, zone) in encounters.tables.iter().enumerate() {
                for (sub_table, slots) in zone.sub_tables.iter().enumerate() {
                    if slots.is_empty() {
                        continue;
                    }
                    let rate = slots
                        .slots
                        .iter()
                        .map(|slot| i32::from(slot.probability))
                        .sum();
                    sources.push((
                        Source::Wild {
                            archive,
                            table,
                            sub_table,
                        },
                        format!(
                            "{} {:016X} {}",
                            ARCHIVE_NAMES[archive],
                            zone.zone_id,
                            weather_name(sub_table)
                        ),
                        rate,
                    ));
                }
            }
        }
        for (index, nest) in self.files.data.nests.tables.iter().enumerate() {
            let game = usize::try_from(nest.game_version - 1)
                .ok()
                .and_then(|game| NEST_GAMES.get(game))
                .copied()
                .unwrap_or("Shared");
            sources.push((
                Source::Nest(index),
                format!("{game} Max Raid {:016X}", nest.table_id),
                0,
            ));
        }
        if self.files.data.underground.is_some() {
            sources.push((Source::Underground, "Dynamax Adventures".to_owned(), 0));
        }
        sources
    }

    fn read_source(&self, source: Source) -> Vec<Encounter> {
        let data = &self.files.data;
        match source {
            Source::Wild {
                archive,
                table,
                sub_table,
            } => {
                let slots = &self.archives()[archive].tables[table].sub_tables[sub_table];
                slots
                    .slots
                    .iter()
                    .filter_map(|slot| {
                        self.encounter(
                            slot.species,
                            i32::from(slot.form),
                            i32::from(slots.level_min),
                            i32::from(slots.level_max),
                        )
                    })
                    .collect()
            }
            Source::Nest(index) => data.nests.tables[index]
                .entries
                .iter()
                .filter_map(|entry| {
                    self.encounter(
                        entry.species,
                        entry.form,
                        nest_level(&entry.probabilities),
                        0,
                    )
                })
                .collect(),
            Source::Underground => data
                .underground
                .iter()
                .flat_map(|archive| archive.encounters.iter())
                .filter_map(|entry| {
                    self.encounter(entry.species, i32::from(entry.form), entry.level as i32, 0)
                })
                .collect(),
        }
    }

    pub(crate) fn read_encounters(&self) -> Vec<EncounterSet> {
        let dynamax_banned = self.dynamax_banned();
        let mut sets = Vec::new();
        for (index, (source, name, rate)) in self.sources().into_iter().enumerate() {
            let encounters = self.read_source(source);
            if encounters.is_empty() {
                continue;
            }
            let banned_pokemon = match source {
                Source::Wild { .. } => Vec::new(),
                Source::Nest(_) | Source::Underground => dynamax_banned.clone(),
            };
            sets.push(EncounterSet {
                rate,
                encounters,
                banned_pokemon,
                display_name: Some(name),
                offset: index as i32,
            });
        }
        sets
    }

    fn targets(
        &self,
        originals: &[(usize, (i32, i32))],
        set: &EncounterSet,
    ) -> Vec<(usize, (i32, i32))> {
        originals
            .iter()
            .zip(&set.encounters)
            .map(|(&(index, original), encounter)| {
                (
                    index,
                    self.placement(encounter.pokemon, encounter.forme_number, original),
                )
            })
            .collect()
    }

    fn present_entries(
        &self,
        entries: impl Iterator<Item = (i32, i32)>,
    ) -> Vec<(usize, (i32, i32))> {
        entries
            .enumerate()
            .filter(|(_, (species, form))| self.pokemon_with_forme(*species, *form).is_some())
            .collect()
    }

    fn write_wild(&mut self, archive: usize, table: usize, sub_table: usize, set: &EncounterSet) {
        let slots = &self.archives()[archive].tables[table].sub_tables[sub_table];
        let originals = self.present_entries(
            slots
                .slots
                .iter()
                .map(|slot| (slot.species, i32::from(slot.form))),
        );
        let targets = self.targets(&originals, set);
        let min_level = set.encounters.iter().map(|encounter| encounter.level).min();
        let max_level = set
            .encounters
            .iter()
            .map(|encounter| encounter.max_level.max(encounter.level))
            .max();
        let Some(encounters) = self.files.data.wild.all_mut().into_iter().nth(archive) else {
            return;
        };
        let slots = &mut encounters.tables[table].sub_tables[sub_table];
        for (index, (species, form)) in targets {
            let slot = &mut slots.slots[index];
            if slot.species != species || i32::from(slot.form) != form {
                slot.species = species;
                slot.form = form as u8;
            }
        }
        if let (Some(min_level), Some(max_level)) = (min_level, max_level) {
            let min_level = min_level.clamp(1, 100) as u8;
            let max_level = (max_level.clamp(1, 100) as u8).max(min_level);
            if slots.level_min != min_level || slots.level_max != max_level {
                slots.level_min = min_level;
                slots.level_max = max_level;
            }
        }
    }

    fn write_nest(&mut self, index: usize, set: &EncounterSet) {
        let entries = &self.files.data.nests.tables[index].entries;
        let originals =
            self.present_entries(entries.iter().map(|entry| (entry.species, entry.form)));
        let targets: Vec<(usize, (i32, i32), bool)> = self
            .targets(&originals, set)
            .into_iter()
            .map(|(position, (species, form))| {
                (
                    position,
                    (species, form),
                    self.cannot_dynamax(species, form),
                )
            })
            .collect();
        let entries = &mut self.files.data.nests.tables[index].entries;
        for (position, (species, form), blocked) in targets {
            let entry = &mut entries[position];
            if (entry.species != species || entry.form != form) && !blocked {
                entry.species = species;
                entry.form = form;
                entry.is_gigantamax = false;
            }
        }
    }

    fn write_underground(&mut self, set: &EncounterSet, movesets: &Movesets) {
        let Some(archive) = self.files.data.underground.clone() else {
            return;
        };
        let originals = self.present_entries(
            archive
                .encounters
                .iter()
                .map(|entry| (entry.species, i32::from(entry.form))),
        );
        let mut updated = archive.clone();
        for ((position, (species, form)), encounter) in self
            .targets(&originals, set)
            .into_iter()
            .zip(&set.encounters)
        {
            let entry = &mut updated.encounters[position];
            if (entry.species == species && i32::from(entry.form) == form)
                || self.cannot_dynamax(species, form)
            {
                continue;
            }
            entry.species = species;
            entry.form = form as u8;
            if entry.gigantamax_state == UNDERGROUND_GIGANTAMAX {
                entry.gigantamax_state = UNDERGROUND_NOT_GIGANTAMAX;
            }
            let shown = self.alt_forme_of(encounter.pokemon, form);
            let moves = movesets.get(&shown.number()).map_or([0; 4], |movepool| {
                moves_at_level(movepool, entry.level as i32, 0)
            });
            entry.set_moves(moves.map(|move_number| move_number.max(0) as u32));
        }
        if updated != archive {
            self.files.data.underground = Some(updated);
        }
    }

    pub(crate) fn write_encounters(&mut self, encounters: &[EncounterSet]) {
        let movesets = self.read_movesets();
        let mut provided = encounters.iter();
        for (source, _, _) in self.sources() {
            if self.read_source(source).is_empty() {
                continue;
            }
            let Some(set) = provided.next() else {
                return;
            };
            match source {
                Source::Wild {
                    archive,
                    table,
                    sub_table,
                } => self.write_wild(archive, table, sub_table, set),
                Source::Nest(index) => self.write_nest(index, set),
                Source::Underground => self.write_underground(set, &movesets),
            }
        }
    }
}
