use crate::bytes::{
    bytes_at, i32_at, put_bytes, put_u8, put_u16, put_u32, u8_at, u16_at, u32_at, u64_at,
};
use crate::error::{Result, invalid};

pub const RECORD_SIZE: usize = 0x30;
pub const MACHINE_COUNT: usize = 200;
pub const TM_COUNT: usize = 100;
pub const TR_COUNT: usize = 100;
pub const SHARED_RECORD: u16 = 0;
const INDEX_TABLE_OFFSET: usize = 0x44;
const RECORDS_POINTER_OFFSET: usize = 0x40;
const MACHINE_ENTRY_SIZE: usize = 4;
const CONTEXT: &str = "item.dat";
const HASH_CONTEXT: &str = "item_hash_to_index.dat";
const POUCH_MASK: u8 = 0x0F;

pub mod pouch {
    pub const MEDICINE: u8 = 0;
    pub const BALLS: u8 = 1;
    pub const BATTLE: u8 = 2;
    pub const BERRIES: u8 = 3;
    pub const ITEMS: u8 = 4;
    pub const MACHINES: u8 = 5;
    pub const TREASURES: u8 = 6;
    pub const INGREDIENTS: u8 = 7;
    pub const KEY: u8 = 8;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRecord {
    pub price: u32,
    pub watts_price: u32,
    pub alternate_price: u32,
    pub pouch: u8,
    pub field_effect: u8,
    pub can_use_on_pokemon: u8,
    pub sprite: u16,
    pub group_type: u8,
    pub group_index: u8,
    pub boosts: [u8; 4],
    pub raw: Vec<u8>,
}

impl ItemRecord {
    pub fn parse(record: &[u8]) -> Result<Self> {
        if record.len() != RECORD_SIZE {
            return Err(invalid(
                CONTEXT,
                format!("item record of {} bytes", record.len()),
            ));
        }
        let mut boosts = [0u8; 4];
        boosts.copy_from_slice(bytes_at(record, 0x1F, 4, CONTEXT)?);
        Ok(ItemRecord {
            price: u32_at(record, 0x00, CONTEXT)?,
            watts_price: u32_at(record, 0x04, CONTEXT)?,
            alternate_price: u32_at(record, 0x08, CONTEXT)?,
            pouch: u8_at(record, 0x11, CONTEXT)? & POUCH_MASK,
            field_effect: u8_at(record, 0x13, CONTEXT)?,
            can_use_on_pokemon: u8_at(record, 0x15, CONTEXT)?,
            sprite: u16_at(record, 0x1A, CONTEXT)?,
            group_type: u8_at(record, 0x1C, CONTEXT)?,
            group_index: u8_at(record, 0x1D, CONTEXT)?,
            boosts,
            raw: record.to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut record = self.raw.clone();
        if record.len() != RECORD_SIZE {
            return Err(invalid(
                CONTEXT,
                format!("item record of {} bytes", record.len()),
            ));
        }
        put_u32(&mut record, 0x00, self.price, CONTEXT)?;
        put_u32(&mut record, 0x04, self.watts_price, CONTEXT)?;
        put_u32(&mut record, 0x08, self.alternate_price, CONTEXT)?;
        let pouch_byte = (u8_at(&record, 0x11, CONTEXT)? & !POUCH_MASK) | (self.pouch & POUCH_MASK);
        put_u8(&mut record, 0x11, pouch_byte, CONTEXT)?;
        put_u8(&mut record, 0x13, self.field_effect, CONTEXT)?;
        put_u8(&mut record, 0x15, self.can_use_on_pokemon, CONTEXT)?;
        put_u16(&mut record, 0x1A, self.sprite, CONTEXT)?;
        put_u8(&mut record, 0x1C, self.group_type, CONTEXT)?;
        put_u8(&mut record, 0x1D, self.group_index, CONTEXT)?;
        put_bytes(&mut record, 0x1F, &self.boosts, CONTEXT)?;
        Ok(record)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Machine {
    pub item: u16,
    pub move_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTable {
    pub item_records: Vec<u16>,
    pub records: Vec<ItemRecord>,
    pub machines: Vec<Machine>,
    machine_table_offset: usize,
    records_offset: usize,
    raw: Vec<u8>,
}

impl ItemTable {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let item_count = usize::from(u16_at(data, 0, CONTEXT)?);
        let machine_pointer = usize::from(u16_at(data, 2, CONTEXT)?);
        let record_count = usize::from(u16_at(data, 4, CONTEXT)?);
        let records_offset = usize::try_from(i32_at(data, RECORDS_POINTER_OFFSET, CONTEXT)?)
            .map_err(|_| invalid(CONTEXT, "negative record table offset"))?;
        let item_records = (0..item_count)
            .map(|item| u16_at(data, INDEX_TABLE_OFFSET + item * 2, CONTEXT))
            .collect::<Result<Vec<_>>>()?;
        if let Some(bad) = item_records
            .iter()
            .find(|&&record| usize::from(record) >= record_count)
        {
            return Err(invalid(
                CONTEXT,
                format!("item points at record {bad} of {record_count}"),
            ));
        }
        let records = bytes_at(data, records_offset, record_count * RECORD_SIZE, CONTEXT)?
            .as_chunks::<RECORD_SIZE>()
            .0
            .iter()
            .map(|record| ItemRecord::parse(record))
            .collect::<Result<Vec<_>>>()?;
        let machine_table_offset = INDEX_TABLE_OFFSET + machine_pointer * 2;
        let machines = (0..MACHINE_COUNT)
            .map(|index| {
                let entry = machine_table_offset + index * MACHINE_ENTRY_SIZE;
                Ok(Machine {
                    item: u16_at(data, entry, CONTEXT)?,
                    move_id: u16_at(data, entry + 2, CONTEXT)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if machine_table_offset + MACHINE_COUNT * MACHINE_ENTRY_SIZE > records_offset {
            return Err(invalid(CONTEXT, "machine table overlaps the item records"));
        }
        Ok(ItemTable {
            item_records,
            records,
            machines,
            machine_table_offset,
            records_offset,
            raw: data.to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut data = self.raw.clone();
        if self.machines.len() != MACHINE_COUNT {
            return Err(invalid(
                CONTEXT,
                format!(
                    "expected {MACHINE_COUNT} machines, got {}",
                    self.machines.len()
                ),
            ));
        }
        for (index, machine) in self.machines.iter().enumerate() {
            let entry = self.machine_table_offset + index * MACHINE_ENTRY_SIZE;
            put_u16(&mut data, entry, machine.item, CONTEXT)?;
            put_u16(&mut data, entry + 2, machine.move_id, CONTEXT)?;
        }
        for (index, record) in self.records.iter().enumerate() {
            put_bytes(
                &mut data,
                self.records_offset + index * RECORD_SIZE,
                &record.to_bytes()?,
                CONTEXT,
            )?;
        }
        Ok(data)
    }

    pub fn item_count(&self) -> usize {
        self.item_records.len()
    }

    pub fn record_index(&self, item: u16) -> Option<u16> {
        self.item_records.get(usize::from(item)).copied()
    }

    pub fn record(&self, item: u16) -> Option<&ItemRecord> {
        self.records.get(usize::from(self.record_index(item)?))
    }

    pub fn is_shared_record(&self, item: u16) -> bool {
        match self.record_index(item) {
            Some(record) => {
                record == SHARED_RECORD
                    || self
                        .item_records
                        .iter()
                        .filter(|&&other| other == record)
                        .count()
                        > 1
            }
            None => true,
        }
    }

    pub fn has_own_record(&self, item: u16) -> bool {
        !self.is_shared_record(item)
    }

    pub fn record_mut(&mut self, item: u16) -> Result<&mut ItemRecord> {
        if self.is_shared_record(item) {
            return Err(invalid(
                CONTEXT,
                format!("item {item} has no record of its own"),
            ));
        }
        let record = usize::from(self.record_index(item).unwrap_or(SHARED_RECORD));
        self.records
            .get_mut(record)
            .ok_or_else(|| invalid(CONTEXT, format!("item {item} has no record")))
    }

    pub fn price(&self, item: u16) -> Option<u32> {
        self.record(item).map(|record| record.price)
    }

    pub fn set_price(&mut self, item: u16, price: u32) -> Result<()> {
        self.record_mut(item)?.price = price;
        Ok(())
    }

    pub fn pouch(&self, item: u16) -> Option<u8> {
        self.record(item).map(|record| record.pouch)
    }

    pub fn tm_moves(&self) -> Vec<u16> {
        self.machines
            .iter()
            .take(TM_COUNT)
            .map(|machine| machine.move_id)
            .collect()
    }

    pub fn tr_moves(&self) -> Vec<u16> {
        self.machines
            .iter()
            .skip(TM_COUNT)
            .take(TR_COUNT)
            .map(|machine| machine.move_id)
            .collect()
    }

    pub fn set_tm_moves(&mut self, moves: &[u16]) -> Result<()> {
        self.set_machine_moves(0, TM_COUNT, moves)
    }

    pub fn set_tr_moves(&mut self, moves: &[u16]) -> Result<()> {
        self.set_machine_moves(TM_COUNT, TR_COUNT, moves)
    }

    fn set_machine_moves(&mut self, start: usize, count: usize, moves: &[u16]) -> Result<()> {
        if moves.len() != count {
            return Err(invalid(
                CONTEXT,
                format!("expected {count} machine moves, got {}", moves.len()),
            ));
        }
        let machines = self
            .machines
            .get_mut(start..start + count)
            .ok_or_else(|| invalid(CONTEXT, "machine table is too short"))?;
        for (machine, &move_id) in machines.iter_mut().zip(moves) {
            machine.move_id = move_id;
        }
        Ok(())
    }

    pub fn machine_for_item(&self, item: u16) -> Option<(usize, Machine)> {
        self.machines
            .iter()
            .enumerate()
            .find(|(_, machine)| machine.item == item)
            .map(|(index, machine)| (index, *machine))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemHash {
    pub hash: u64,
    pub item: u32,
    pub padding: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemHashTable {
    pub entries: Vec<ItemHash>,
}

impl ItemHashTable {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let count = u32_at(data, 0, HASH_CONTEXT)? as usize;
        if data.len() != 4 + count.saturating_mul(16) {
            return Err(invalid(
                HASH_CONTEXT,
                format!("{count} entries do not fill {} bytes", data.len()),
            ));
        }
        let entries = (0..count)
            .map(|index| {
                let entry = 4 + index * 16;
                Ok(ItemHash {
                    hash: u64_at(data, entry, HASH_CONTEXT)?,
                    item: u32_at(data, entry + 8, HASH_CONTEXT)?,
                    padding: u32_at(data, entry + 12, HASH_CONTEXT)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(ItemHashTable { entries })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let count = u32::try_from(self.entries.len())
            .map_err(|_| invalid(HASH_CONTEXT, "too many entries"))?;
        let mut data = Vec::with_capacity(4 + self.entries.len() * 16);
        data.extend_from_slice(&count.to_le_bytes());
        for entry in &self.entries {
            data.extend_from_slice(&entry.hash.to_le_bytes());
            data.extend_from_slice(&entry.item.to_le_bytes());
            data.extend_from_slice(&entry.padding.to_le_bytes());
        }
        Ok(data)
    }

    pub fn hash_of(&self, item: u32) -> Option<u64> {
        if item == 0 {
            return None;
        }
        self.entries
            .iter()
            .find(|entry| entry.item == item)
            .map(|entry| entry.hash)
    }

    pub fn item_of(&self, hash: u64) -> Option<u32> {
        self.entries
            .iter()
            .find(|entry| entry.hash == hash)
            .map(|entry| entry.item)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn sample_item_data() -> Vec<u8> {
        let item_count = 4usize;
        let machine_pointer = item_count + 1;
        let machine_offset = INDEX_TABLE_OFFSET + machine_pointer * 2;
        let records_offset = machine_offset + MACHINE_COUNT * MACHINE_ENTRY_SIZE + 8;
        let record_count = 3usize;
        let mut data = vec![0u8; records_offset + record_count * RECORD_SIZE];
        data[0..2].copy_from_slice(&(item_count as u16).to_le_bytes());
        data[2..4].copy_from_slice(&(machine_pointer as u16).to_le_bytes());
        data[4..6].copy_from_slice(&(record_count as u16).to_le_bytes());
        data[0x40..0x44].copy_from_slice(&(records_offset as i32).to_le_bytes());
        for (item, record) in [0u16, 1, 2, 0].iter().enumerate() {
            let at = INDEX_TABLE_OFFSET + item * 2;
            data[at..at + 2].copy_from_slice(&record.to_le_bytes());
        }
        for index in 0..MACHINE_COUNT {
            let at = machine_offset + index * 4;
            data[at..at + 2].copy_from_slice(&(1000 + index as u16).to_le_bytes());
            data[at + 2..at + 4].copy_from_slice(&(index as u16 + 1).to_le_bytes());
        }
        let first = records_offset + RECORD_SIZE;
        data[first..first + 4].copy_from_slice(&200u32.to_le_bytes());
        data[first + 0x11] = 0x21;
        data
    }

    #[test]
    fn item_table_round_trips() {
        let data = sample_item_data();
        let table = ItemTable::parse(&data).expect("parse");
        assert_eq!(table.item_count(), 4);
        assert_eq!(table.price(1), Some(200));
        assert_eq!(table.pouch(1), Some(pouch::BALLS));
        assert_eq!(table.tm_moves().len(), TM_COUNT);
        assert_eq!(table.tr_moves()[0], 101);
        assert_eq!(
            table.machine_for_item(1100).map(|(index, _)| index),
            Some(100)
        );
        assert_eq!(table.to_bytes().expect("write"), data);
    }

    #[test]
    fn shared_records_cannot_be_edited() {
        let mut table = ItemTable::parse(&sample_item_data()).expect("parse");
        assert!(table.is_shared_record(0));
        assert!(table.is_shared_record(3));
        assert!(table.is_shared_record(99));
        assert!(table.set_price(3, 5).is_err());
        assert!(table.has_own_record(2));
        table.set_price(2, 1234).expect("set");
        let reparsed = ItemTable::parse(&table.to_bytes().expect("write")).expect("parse");
        assert_eq!(reparsed.price(2), Some(1234));
        assert_eq!(reparsed.price(0), Some(0));
    }

    #[test]
    fn machine_moves_can_be_replaced() {
        let mut table = ItemTable::parse(&sample_item_data()).expect("parse");
        let moves: Vec<u16> = (0..TM_COUNT as u16).map(|index| 500 + index).collect();
        table.set_tm_moves(&moves).expect("set");
        assert!(table.set_tr_moves(&moves[..3]).is_err());
        let reparsed = ItemTable::parse(&table.to_bytes().expect("write")).expect("parse");
        assert_eq!(reparsed.tm_moves(), moves);
        assert_eq!(reparsed.tr_moves()[0], 101);
    }

    #[test]
    fn pouch_writes_keep_the_high_nibble() {
        let mut table = ItemTable::parse(&sample_item_data()).expect("parse");
        table.record_mut(1).expect("record").pouch = pouch::KEY;
        let bytes = table.to_bytes().expect("write");
        let reparsed = ItemTable::parse(&bytes).expect("parse");
        assert_eq!(reparsed.pouch(1), Some(pouch::KEY));
        assert_eq!(reparsed.records[1].raw[0x11] & 0xF0, 0x20);
    }

    #[test]
    fn malformed_item_data_is_rejected() {
        let mut data = sample_item_data();
        data[INDEX_TABLE_OFFSET] = 9;
        assert!(ItemTable::parse(&data).is_err());
        assert!(ItemTable::parse(&sample_item_data()[..0x50]).is_err());
    }

    #[test]
    fn item_hash_table_round_trips() {
        let table = ItemHashTable {
            entries: vec![
                ItemHash {
                    hash: 0xCBF2_9CE4_8422_2645,
                    item: 0,
                    padding: 0,
                },
                ItemHash {
                    hash: 42,
                    item: 1,
                    padding: 0,
                },
            ],
        };
        let bytes = table.to_bytes().expect("write");
        let parsed = ItemHashTable::parse(&bytes).expect("parse");
        assert_eq!(parsed, table);
        assert_eq!(parsed.hash_of(1), Some(42));
        assert_eq!(parsed.hash_of(0), None);
        assert_eq!(parsed.item_of(42), Some(1));
        assert!(ItemHashTable::parse(&bytes[..bytes.len() - 1]).is_err());
    }
}
