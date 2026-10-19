use std::borrow::Cow;

use crate::bytes::{bytes_at, i32_at, u16_at, u32_at, u64_at};
use crate::error::{Result, invalid};
use crate::formats::hash::fnv1a_64;

pub const MAGIC: u32 = 0x4254_4841;
const CONTEXT: &str = "AHTB";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashName {
    pub hash: u64,
    pub name_bytes: Vec<u8>,
}

impl HashName {
    pub fn new(name: &str) -> Self {
        HashName {
            hash: fnv1a_64(name),
            name_bytes: name.as_bytes().to_vec(),
        }
    }

    pub fn name(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.name_bytes)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HashTable {
    pub entries: Vec<HashName>,
}

impl HashTable {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if u32_at(data, 0, CONTEXT)? != MAGIC {
            return Err(invalid(CONTEXT, "missing AHTB magic"));
        }
        let count = usize::try_from(i32_at(data, 4, CONTEXT)?)
            .map_err(|_| invalid(CONTEXT, "negative entry count"))?;
        let mut offset = 8;
        let mut entries = Vec::new();
        for _ in 0..count {
            let hash = u64_at(data, offset, CONTEXT)?;
            let length = usize::from(u16_at(data, offset + 8, CONTEXT)?);
            let bytes = bytes_at(data, offset + 10, length, CONTEXT)?;
            let (terminator, text) = bytes
                .split_last()
                .ok_or_else(|| invalid(CONTEXT, "entry name has no terminator"))?;
            if *terminator != 0 {
                return Err(invalid(CONTEXT, "entry name is not NUL terminated"));
            }
            entries.push(HashName {
                hash,
                name_bytes: text.to_vec(),
            });
            offset += 10 + length;
        }
        if offset != data.len() {
            return Err(invalid(
                CONTEXT,
                format!("{} trailing bytes", data.len() - offset),
            ));
        }
        Ok(HashTable { entries })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let count =
            i32::try_from(self.entries.len()).map_err(|_| invalid(CONTEXT, "too many entries"))?;
        let mut output = Vec::new();
        output.extend_from_slice(&MAGIC.to_le_bytes());
        output.extend_from_slice(&count.to_le_bytes());
        for entry in &self.entries {
            let length = u16::try_from(entry.name_bytes.len() + 1)
                .map_err(|_| invalid(CONTEXT, format!("name {:?} is too long", entry.name())))?;
            output.extend_from_slice(&entry.hash.to_le_bytes());
            output.extend_from_slice(&length.to_le_bytes());
            output.extend_from_slice(&entry.name_bytes);
            output.push(0);
        }
        Ok(output)
    }

    pub fn name_of(&self, hash: u64) -> Option<Cow<'_, str>> {
        self.entries
            .iter()
            .find(|entry| entry.hash == hash)
            .map(HashName::name)
    }

    pub fn index_of(&self, hash: u64) -> Option<usize> {
        self.entries.iter().position(|entry| entry.hash == hash)
    }

    pub fn names(&self) -> impl Iterator<Item = Cow<'_, str>> {
        self.entries.iter().map(HashName::name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_table_round_trips() {
        let table = HashTable {
            entries: vec![HashName::new("MONSNAME_000"), HashName::new("a_0101")],
        };
        let bytes = table.to_bytes().expect("write");
        let parsed = HashTable::parse(&bytes).expect("parse");
        assert_eq!(parsed, table);
        assert_eq!(
            parsed.name_of(fnv1a_64("a_0101")).as_deref(),
            Some("a_0101")
        );
        assert_eq!(parsed.index_of(fnv1a_64("MONSNAME_000")), Some(0));
        assert_eq!(parsed.names().count(), 2);
    }

    #[test]
    fn malformed_tables_are_rejected() {
        assert!(HashTable::parse(b"AHTB").is_err());
        assert!(HashTable::parse(b"ABCD\0\0\0\0").is_err());
        let mut bytes = HashTable {
            entries: vec![HashName::new("x")],
        }
        .to_bytes()
        .expect("write");
        let last = bytes.len() - 1;
        bytes[last] = b'y';
        assert!(HashTable::parse(&bytes).is_err());
        bytes.push(0);
        assert!(HashTable::parse(&bytes).is_err());
    }

    #[test]
    fn non_utf8_names_round_trip() {
        let table = HashTable {
            entries: vec![HashName {
                hash: 1,
                name_bytes: b"msg_\x82\x92".to_vec(),
            }],
        };
        let parsed = HashTable::parse(&table.to_bytes().expect("write")).expect("parse");
        assert_eq!(parsed, table);
        assert!(parsed.entries[0].name().contains('\u{FFFD}'));
    }
}
