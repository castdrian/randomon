use crate::error::Result;
use crate::formats::flatbuffer::{
    TableBuilder, TableRef, build_table_list, parse_table_list, scalar_table,
};

const ARCHIVE_CONTEXT: &str = "EncounterArchive";
const TABLE_CONTEXT: &str = "EncounterTable";
const SUB_TABLE_CONTEXT: &str = "EncounterSubTable";
const NEST_TABLE_CONTEXT: &str = "EncounterNestTable";
const NEST_CONTEXT: &str = "EncounterNest";
const NEST_ARCHIVE_CONTEXT: &str = "EncounterNestArchive";

pub const SWORD_HIDDEN_FILE: &str = "encount_k.bin";
pub const SWORD_SYMBOL_FILE: &str = "encount_symbol_k.bin";
pub const SHIELD_HIDDEN_FILE: &str = "encount_t.bin";
pub const SHIELD_SYMBOL_FILE: &str = "encount_symbol_t.bin";
pub const NEST_FILE: &str = "nest_hole_encount.bin";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weather {
    Normal,
    Overcast,
    Raining,
    Thunderstorm,
    IntenseSun,
    Snowing,
    Snowstorm,
    Sandstorm,
    HeavyFog,
    ShakingTrees,
    Fishing,
}

impl Weather {
    pub const ALL: [Weather; 11] = [
        Weather::Normal,
        Weather::Overcast,
        Weather::Raining,
        Weather::Thunderstorm,
        Weather::IntenseSun,
        Weather::Snowing,
        Weather::Snowstorm,
        Weather::Sandstorm,
        Weather::HeavyFog,
        Weather::ShakingTrees,
        Weather::Fishing,
    ];

    pub fn from_sub_table_index(index: usize) -> Option<Weather> {
        Weather::ALL.get(index).copied()
    }
}

scalar_table! {
    pub struct EncounterSlot {
        0 => probability: u8,
        1 => species: i32,
        2 => form: u8,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EncounterSubTable {
    pub level_min: u8,
    pub level_max: u8,
    pub slots: Vec<EncounterSlot>,
}

impl EncounterSubTable {
    fn read(table: &TableRef<'_>) -> Result<Self> {
        table.ensure_known_fields(3, SUB_TABLE_CONTEXT)?;
        Ok(EncounterSubTable {
            level_min: table.scalar(0)?,
            level_max: table.scalar(1)?,
            slots: table
                .tables(2)?
                .iter()
                .map(EncounterSlot::read)
                .collect::<Result<_>>()?,
        })
    }

    fn build(&self) -> TableBuilder {
        let mut builder = TableBuilder::new();
        builder
            .scalar(0, self.level_min)
            .scalar(1, self.level_max)
            .tables(2, self.slots.iter().map(EncounterSlot::build).collect());
        builder
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|slot| slot.species == 0)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EncounterTable {
    pub zone_id: u64,
    pub sub_tables: Vec<EncounterSubTable>,
}

impl EncounterTable {
    fn read(table: &TableRef<'_>) -> Result<Self> {
        table.ensure_known_fields(2, TABLE_CONTEXT)?;
        Ok(EncounterTable {
            zone_id: table.scalar(0)?,
            sub_tables: table
                .tables(1)?
                .iter()
                .map(EncounterSubTable::read)
                .collect::<Result<_>>()?,
        })
    }

    fn build(&self) -> TableBuilder {
        let mut builder = TableBuilder::new();
        builder.scalar(0, self.zone_id).tables(
            1,
            self.sub_tables
                .iter()
                .map(EncounterSubTable::build)
                .collect(),
        );
        builder
    }

    pub fn sub_table(&self, weather: Weather) -> Option<&EncounterSubTable> {
        self.sub_tables.get(weather as usize)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EncounterArchive {
    pub field_00: u32,
    pub tables: Vec<EncounterTable>,
}

impl EncounterArchive {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let root = TableRef::root(data)?;
        root.ensure_known_fields(2, ARCHIVE_CONTEXT)?;
        Ok(EncounterArchive {
            field_00: root.scalar(0)?,
            tables: root
                .tables(1)?
                .iter()
                .map(EncounterTable::read)
                .collect::<Result<_>>()?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut root = TableBuilder::new();
        root.scalar(0, self.field_00)
            .tables(1, self.tables.iter().map(EncounterTable::build).collect());
        root.finish()
    }

    pub fn slots(&self) -> impl Iterator<Item = &EncounterSlot> {
        self.tables
            .iter()
            .flat_map(|table| table.sub_tables.iter())
            .flat_map(|sub_table| sub_table.slots.iter())
    }

    pub fn slots_mut(&mut self) -> impl Iterator<Item = &mut EncounterSlot> {
        self.tables
            .iter_mut()
            .flat_map(|table| table.sub_tables.iter_mut())
            .flat_map(|sub_table| sub_table.slots.iter_mut())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NestEncounter {
    pub entry_index: i32,
    pub species: i32,
    pub form: i32,
    pub level_table_id: u64,
    pub ability: u8,
    pub is_gigantamax: bool,
    pub drop_table_id: u64,
    pub bonus_table_id: u64,
    pub probabilities: Vec<u32>,
    pub gender: i8,
    pub flawless_ivs: i8,
}

impl NestEncounter {
    fn read(table: &TableRef<'_>) -> Result<Self> {
        table.ensure_known_fields(11, NEST_CONTEXT)?;
        Ok(NestEncounter {
            entry_index: table.scalar(0)?,
            species: table.scalar(1)?,
            form: table.scalar(2)?,
            level_table_id: table.scalar(3)?,
            ability: table.scalar(4)?,
            is_gigantamax: table.scalar(5)?,
            drop_table_id: table.scalar(6)?,
            bonus_table_id: table.scalar(7)?,
            probabilities: table.scalars(8)?,
            gender: table.scalar(9)?,
            flawless_ivs: table.scalar(10)?,
        })
    }

    fn build(&self) -> TableBuilder {
        let mut builder = TableBuilder::new();
        builder
            .scalar(0, self.entry_index)
            .scalar(1, self.species)
            .scalar(2, self.form)
            .scalar(3, self.level_table_id)
            .scalar(4, self.ability)
            .scalar(5, self.is_gigantamax)
            .scalar(6, self.drop_table_id)
            .scalar(7, self.bonus_table_id)
            .scalars(8, &self.probabilities)
            .scalar(9, self.gender)
            .scalar(10, self.flawless_ivs);
        builder
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NestTable {
    pub table_id: u64,
    pub game_version: i32,
    pub entries: Vec<NestEncounter>,
}

impl NestTable {
    fn read(table: &TableRef<'_>) -> Result<Self> {
        table.ensure_known_fields(3, NEST_TABLE_CONTEXT)?;
        Ok(NestTable {
            table_id: table.scalar(0)?,
            game_version: table.scalar(1)?,
            entries: table
                .tables(2)?
                .iter()
                .map(NestEncounter::read)
                .collect::<Result<_>>()?,
        })
    }

    fn build(&self) -> TableBuilder {
        let mut builder = TableBuilder::new();
        builder
            .scalar(0, self.table_id)
            .scalar(1, self.game_version)
            .tables(2, self.entries.iter().map(NestEncounter::build).collect());
        builder
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NestArchive {
    pub tables: Vec<NestTable>,
}

impl NestArchive {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(NestArchive {
            tables: parse_table_list(data, NEST_ARCHIVE_CONTEXT, NestTable::read)?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        build_table_list(self.tables.iter().map(NestTable::build).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive() -> EncounterArchive {
        let slot = |probability, species, form| EncounterSlot {
            probability,
            species,
            form,
        };
        EncounterArchive {
            field_00: 2,
            tables: vec![EncounterTable {
                zone_id: 0x078B_C1FF_1A65_7844,
                sub_tables: vec![
                    EncounterSubTable {
                        level_min: 2,
                        level_max: 5,
                        slots: vec![slot(40, 819, 0), slot(30, 52, 2), slot(0, 0, 0)],
                    },
                    EncounterSubTable::default(),
                ],
            }],
        }
    }

    #[test]
    fn encounter_archive_round_trips() {
        let bytes = archive().to_bytes();
        assert_eq!(EncounterArchive::parse(&bytes), Ok(archive()));
        assert_eq!(archive().slots().count(), 3);
        assert!(archive().tables[0].sub_tables[1].is_empty());
        assert_eq!(
            archive().tables[0].sub_table(Weather::Overcast),
            Some(&EncounterSubTable::default())
        );
    }

    #[test]
    fn forms_can_be_added_to_slots_that_had_none() {
        let mut parsed = EncounterArchive::parse(&archive().to_bytes()).expect("parse");
        for slot in parsed.slots_mut() {
            slot.species = 133;
            slot.form = 1;
        }
        let reparsed = EncounterArchive::parse(&parsed.to_bytes()).expect("parse");
        assert!(
            reparsed
                .slots()
                .all(|slot| slot.species == 133 && slot.form == 1)
        );
    }

    #[test]
    fn nest_archive_round_trips() {
        let archive = NestArchive {
            tables: vec![NestTable {
                table_id: 0x1742_8356_DC61_09F6,
                game_version: 1,
                entries: vec![NestEncounter {
                    entry_index: 1,
                    species: 236,
                    level_table_id: 99,
                    ability: 3,
                    drop_table_id: 7,
                    bonus_table_id: 8,
                    probabilities: vec![35, 0, 0, 0, 0],
                    flawless_ivs: 1,
                    ..NestEncounter::default()
                }],
            }],
        };
        assert_eq!(NestArchive::parse(&archive.to_bytes()), Ok(archive));
    }

    #[test]
    fn weather_maps_sub_table_indices() {
        assert_eq!(Weather::from_sub_table_index(10), Some(Weather::Fishing));
        assert_eq!(Weather::from_sub_table_index(11), None);
    }
}
