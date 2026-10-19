use crate::bytes::put_bytes;
use crate::error::{Result, invalid};
use crate::formats::ahtb::HashTable;
use crate::formats::flatbuffer::{TableRef, VectorRef};

pub const PACK_FILE: &str = "bin/archive/field/resident/placement.gfpak";
pub const AREA_NAME_TABLE: &str = "AreaNameHashTable.tbl";
pub const STARTER_AREA_FILE: &str = "a_0101.bin";
const CONTEXT: &str = "PlacementZoneArchive";

const ZONE_CRITTERS: usize = 2;
const ZONE_FIELD_ITEMS: usize = 6;
const ZONE_HIDDEN_ITEMS: usize = 19;
const CRITTER_SPECIES: usize = 2;
const CRITTER_FORM: usize = 3;
const FIELD_ITEM_MODEL: usize = 2;
const FIELD_ITEM_HASHES: usize = 6;
const FIELD_ITEM_QUANTITIES: usize = 7;
const HIDDEN_ITEM_CHANCES: usize = 2;
const CHANCE_HASH: usize = 0;
const CHANCE_PROBABILITY: usize = 1;
const CHANCE_QUANTITY: usize = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Critter {
    pub species: u32,
    pub form: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldItem {
    pub model: String,
    pub item_hashes: Vec<u64>,
    pub quantities: Vec<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HiddenItemChance {
    pub hash: u64,
    pub chance: i32,
    pub quantity: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiddenItem {
    pub chances: Vec<HiddenItemChance>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlacementZone {
    pub critters: Vec<Critter>,
    pub field_items: Vec<FieldItem>,
    pub hidden_items: Vec<HiddenItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    position: Option<usize>,
    size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementArchive {
    pub zones: Vec<PlacementZone>,
    original: Vec<PlacementZone>,
    slots: Vec<Slot>,
    raw: Vec<u8>,
}

fn scalar_slot(table: &TableRef<'_>, index: usize, size: usize) -> Result<Slot> {
    Ok(Slot {
        position: table.field_position(index)?,
        size,
    })
}

fn vector_slots(vector: Option<VectorRef<'_>>, size: usize) -> Result<Vec<Slot>> {
    let Some(vector) = vector else {
        return Ok(Vec::new());
    };
    (0..vector.len())
        .map(|index| {
            Ok(Slot {
                position: Some(vector.element_position(index, size)?),
                size,
            })
        })
        .collect()
}

struct ZoneReader<'a> {
    slots: &'a mut Vec<Slot>,
}

impl ZoneReader<'_> {
    fn critter(&mut self, holder: &TableRef<'_>) -> Result<Critter> {
        self.slots.push(scalar_slot(holder, CRITTER_SPECIES, 4)?);
        self.slots.push(scalar_slot(holder, CRITTER_FORM, 4)?);
        Ok(Critter {
            species: holder.scalar(CRITTER_SPECIES)?,
            form: holder.scalar(CRITTER_FORM)?,
        })
    }

    fn field_item(&mut self, holder: &TableRef<'_>) -> Result<FieldItem> {
        let Some(item) = holder.table(0)? else {
            return Ok(FieldItem::default());
        };
        self.slots
            .extend(vector_slots(item.vector(FIELD_ITEM_HASHES)?, 8)?);
        self.slots
            .extend(vector_slots(item.vector(FIELD_ITEM_QUANTITIES)?, 4)?);
        Ok(FieldItem {
            model: item.string(FIELD_ITEM_MODEL)?.unwrap_or_default(),
            item_hashes: item.scalars(FIELD_ITEM_HASHES)?,
            quantities: item.scalars(FIELD_ITEM_QUANTITIES)?,
        })
    }

    fn hidden_item(&mut self, holder: &TableRef<'_>) -> Result<HiddenItem> {
        let Some(item) = holder.table(0)? else {
            return Ok(HiddenItem::default());
        };
        let mut chances = Vec::new();
        for chance in item.tables(HIDDEN_ITEM_CHANCES)? {
            self.slots.push(scalar_slot(&chance, CHANCE_HASH, 8)?);
            self.slots
                .push(scalar_slot(&chance, CHANCE_PROBABILITY, 4)?);
            self.slots.push(scalar_slot(&chance, CHANCE_QUANTITY, 4)?);
            chances.push(HiddenItemChance {
                hash: chance.scalar(CHANCE_HASH)?,
                chance: chance.scalar(CHANCE_PROBABILITY)?,
                quantity: chance.scalar(CHANCE_QUANTITY)?,
            });
        }
        Ok(HiddenItem { chances })
    }

    fn zone(&mut self, zone: &TableRef<'_>) -> Result<PlacementZone> {
        let critters = zone
            .tables(ZONE_CRITTERS)?
            .iter()
            .map(|holder| self.critter(holder))
            .collect::<Result<_>>()?;
        let field_items = zone
            .tables(ZONE_FIELD_ITEMS)?
            .iter()
            .map(|holder| self.field_item(holder))
            .collect::<Result<_>>()?;
        let hidden_items = zone
            .tables(ZONE_HIDDEN_ITEMS)?
            .iter()
            .map(|holder| self.hidden_item(holder))
            .collect::<Result<_>>()?;
        Ok(PlacementZone {
            critters,
            field_items,
            hidden_items,
        })
    }
}

fn flatten(zones: &[PlacementZone]) -> Vec<u64> {
    let mut values = Vec::new();
    for zone in zones {
        for critter in &zone.critters {
            values.extend([u64::from(critter.species), u64::from(critter.form)]);
        }
        for item in &zone.field_items {
            values.extend(item.item_hashes.iter().copied());
            values.extend(item.quantities.iter().map(|&quantity| u64::from(quantity)));
        }
        for item in &zone.hidden_items {
            for chance in &item.chances {
                values.extend([
                    chance.hash,
                    u64::from(chance.chance as u32),
                    u64::from(chance.quantity as u32),
                ]);
            }
        }
    }
    values
}

fn shape(zones: &[PlacementZone]) -> Vec<usize> {
    zones
        .iter()
        .flat_map(|zone| {
            let mut sizes = vec![
                zone.critters.len(),
                zone.field_items.len(),
                zone.hidden_items.len(),
            ];
            sizes.extend(
                zone.field_items
                    .iter()
                    .flat_map(|item| [item.item_hashes.len(), item.quantities.len()]),
            );
            sizes.extend(zone.hidden_items.iter().map(|item| item.chances.len()));
            sizes
        })
        .collect()
}

impl PlacementArchive {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let root = TableRef::root(data)?;
        let mut slots = Vec::new();
        let mut reader = ZoneReader { slots: &mut slots };
        let zones = root
            .tables(0)?
            .iter()
            .map(|zone| reader.zone(zone))
            .collect::<Result<Vec<_>>>()?;
        if flatten(&zones).len() != slots.len() {
            return Err(invalid(CONTEXT, "placement values and slots disagree"));
        }
        Ok(PlacementArchive {
            original: zones.clone(),
            zones,
            slots,
            raw: data.to_vec(),
        })
    }

    pub fn is_modified(&self) -> bool {
        self.zones != self.original
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        if shape(&self.zones) != shape(&self.original) {
            return Err(invalid(
                CONTEXT,
                "placement entries can only be changed in place",
            ));
        }
        if self
            .zones
            .iter()
            .zip(&self.original)
            .any(|(zone, original)| {
                zone.field_items
                    .iter()
                    .zip(&original.field_items)
                    .any(|(item, before)| item.model != before.model)
            })
        {
            return Err(invalid(CONTEXT, "field item models cannot be changed"));
        }
        let mut data = self.raw.clone();
        let current = flatten(&self.zones);
        let before = flatten(&self.original);
        for ((value, previous), slot) in current.iter().zip(&before).zip(&self.slots) {
            if value == previous {
                continue;
            }
            let position = slot.position.ok_or_else(|| {
                invalid(
                    CONTEXT,
                    "cannot set a value whose field is absent from the file",
                )
            })?;
            put_bytes(
                &mut data,
                position,
                &value.to_le_bytes()[..slot.size],
                CONTEXT,
            )?;
        }
        Ok(data)
    }

    pub fn can_set_critter_form(&self, zone: usize, critter: usize) -> bool {
        self.critter_slot_index(zone, critter)
            .and_then(|index| self.slots.get(index + 1))
            .is_some_and(|slot| slot.position.is_some())
    }

    fn critter_slot_index(&self, zone: usize, critter: usize) -> Option<usize> {
        if critter >= self.original.get(zone)?.critters.len() {
            return None;
        }
        let preceding = flatten(self.original.get(..zone)?).len();
        Some(preceding + critter * 2)
    }
}

pub fn area_file_names(table: &HashTable) -> Vec<String> {
    table.names().map(|name| format!("{name}.bin")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::flatbuffer::TableBuilder;

    fn sample() -> Vec<u8> {
        let critter = |species: u32, form: u32| {
            let mut holder = TableBuilder::new();
            holder
                .scalar(CRITTER_SPECIES, species)
                .scalar(CRITTER_FORM, form);
            holder
        };
        let mut field_item = TableBuilder::new();
        field_item
            .string(FIELD_ITEM_MODEL, "unit_obj_itemred01")
            .scalars(FIELD_ITEM_HASHES, &[11u64])
            .scalars(FIELD_ITEM_QUANTITIES, &[1u32]);
        let mut field_holder = TableBuilder::new();
        field_holder.table(0, field_item);
        let mut chance = TableBuilder::new();
        chance
            .scalar(CHANCE_HASH, 22u64)
            .scalar(CHANCE_PROBABILITY, 50i32)
            .scalar(CHANCE_QUANTITY, 1i32);
        let mut hidden_item = TableBuilder::new();
        hidden_item.tables(HIDDEN_ITEM_CHANCES, vec![chance]);
        let mut hidden_holder = TableBuilder::new();
        hidden_holder.table(0, hidden_item);
        let mut zone = TableBuilder::new();
        zone.tables(ZONE_CRITTERS, vec![critter(831, 0), critter(813, 1)])
            .tables(ZONE_FIELD_ITEMS, vec![field_holder])
            .tables(ZONE_HIDDEN_ITEMS, vec![hidden_holder]);
        let mut root = TableBuilder::new();
        root.tables(0, vec![zone]);
        root.finish()
    }

    #[test]
    fn placement_values_are_read() {
        let archive = PlacementArchive::parse(&sample()).expect("parse");
        let zone = &archive.zones[0];
        assert_eq!(
            zone.critters,
            vec![
                Critter {
                    species: 831,
                    form: 0
                },
                Critter {
                    species: 813,
                    form: 1
                }
            ]
        );
        assert_eq!(zone.field_items[0].model, "unit_obj_itemred01");
        assert_eq!(zone.field_items[0].item_hashes, vec![11]);
        assert_eq!(zone.hidden_items[0].chances[0].chance, 50);
        assert_eq!(archive.to_bytes().expect("write"), sample());
        assert!(!archive.is_modified());
    }

    #[test]
    fn present_values_are_patched_in_place() {
        let mut archive = PlacementArchive::parse(&sample()).expect("parse");
        archive.zones[0].critters[1].species = 25;
        archive.zones[0].critters[1].form = 8;
        archive.zones[0].field_items[0].item_hashes[0] = 99;
        archive.zones[0].hidden_items[0].chances[0].quantity = 3;
        let bytes = archive.to_bytes().expect("write");
        assert_eq!(bytes.len(), sample().len());
        let reparsed = PlacementArchive::parse(&bytes).expect("parse");
        assert_eq!(reparsed.zones, archive.zones);
    }

    #[test]
    fn absent_values_and_shape_changes_are_rejected() {
        let mut archive = PlacementArchive::parse(&sample()).expect("parse");
        assert!(!archive.can_set_critter_form(0, 0));
        assert!(archive.can_set_critter_form(0, 1));
        assert!(!archive.can_set_critter_form(0, 5));
        archive.zones[0].critters[0].form = 1;
        assert!(archive.to_bytes().is_err());
        let mut archive = PlacementArchive::parse(&sample()).expect("parse");
        archive.zones[0].field_items[0].item_hashes.push(1);
        assert!(archive.to_bytes().is_err());
        let mut archive = PlacementArchive::parse(&sample()).expect("parse");
        archive.zones[0].field_items[0].model = "other".to_string();
        assert!(archive.to_bytes().is_err());
    }
}
