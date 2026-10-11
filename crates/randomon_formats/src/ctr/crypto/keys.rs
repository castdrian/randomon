use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::aes::Block;

pub const SCRAMBLER_CONSTANT: u128 = 0x1FF9E9AAC5FE0408024591DC5D52768A;
pub const NCCH_PRIMARY_SLOT: u8 = 0x2C;
pub const COMMON_KEY_SLOT: u8 = 0x3D;
pub const COMMON_KEY_COUNT: usize = 6;

const BOOT9_FULL_SIZE: usize = 0x10000;
const BOOT9_PROTECTED_SIZE: usize = 0x8000;
const BOOT9_FULL_KEY_SECTION: usize = 0xD9D0;
const BOOT9_PROTECTED_KEY_SECTION: usize = BOOT9_FULL_KEY_SECTION - BOOT9_PROTECTED_SIZE;
const BOOT9_KEY_X_SLOTS: [(u8, usize); 2] = [(0x2C, 0), (0x3D, 5)];

pub fn scramble_normal_key(key_x: &Block, key_y: &Block) -> Block {
    let key_x = u128::from_be_bytes(*key_x);
    let key_y = u128::from_be_bytes(*key_y);
    (key_x.rotate_left(2) ^ key_y)
        .wrapping_add(SCRAMBLER_CONSTANT)
        .rotate_left(87)
        .to_be_bytes()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum KeyKind {
    X,
    Y,
    Normal,
}

#[derive(Clone, Default)]
pub struct KeyStore {
    slots: BTreeMap<(u8, KeyKind), Block>,
    common_key_y: [Option<Block>; COMMON_KEY_COUNT],
    common_normal: [Option<Block>; COMMON_KEY_COUNT],
    sources: Vec<PathBuf>,
}

impl std::fmt::Debug for KeyStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KeyStore")
            .field("key_count", &self.slots.len())
            .field("sources", &self.sources)
            .finish()
    }
}

fn parse_block(text: &str) -> Option<Block> {
    let text = text.trim();
    if text.len() != 32 {
        return None;
    }
    let mut block = [0u8; 16];
    for (index, byte) in block.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(block)
}

fn parse_slot_name(name: &str) -> Option<(u8, KeyKind)> {
    let rest = name.strip_prefix("slot0x")?;
    let slot = u8::from_str_radix(rest.get(..2)?, 16).ok()?;
    let kind = match rest.get(2..)? {
        "KeyX" => KeyKind::X,
        "KeyY" => KeyKind::Y,
        "KeyN" => KeyKind::Normal,
        _ => return None,
    };
    Some((slot, kind))
}

fn parse_common_name(name: &str) -> Option<(usize, bool)> {
    let rest = name.strip_prefix("common")?;
    let (digit, is_normal) = match rest.strip_suffix('N') {
        Some(digit) => (digit, true),
        None => (rest, false),
    };
    let index: usize = digit.parse().ok()?;
    (index < COMMON_KEY_COUNT).then_some((index, is_normal))
}

impl KeyStore {
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
            && self.common_key_y.iter().all(Option::is_none)
            && self.common_normal.iter().all(Option::is_none)
    }

    pub fn sources(&self) -> &[PathBuf] {
        &self.sources
    }

    pub fn from_aes_keys_text(text: &str) -> Self {
        let mut store = KeyStore::default();
        store.merge_aes_keys_text(text);
        store
    }

    pub fn merge_aes_keys_text(&mut self, text: &str) {
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            let Some(block) = parse_block(value) else {
                continue;
            };
            let name = name.trim();
            if let Some(slot_key) = parse_slot_name(name) {
                self.slots.entry(slot_key).or_insert(block);
            } else if let Some((index, is_normal)) = parse_common_name(name) {
                let table = if is_normal {
                    &mut self.common_normal
                } else {
                    &mut self.common_key_y
                };
                table[index].get_or_insert(block);
            }
        }
    }

    pub fn merge_boot9(&mut self, boot9: &[u8]) -> bool {
        let section = match boot9.len() {
            BOOT9_FULL_SIZE => BOOT9_FULL_KEY_SECTION,
            BOOT9_PROTECTED_SIZE => BOOT9_PROTECTED_KEY_SECTION,
            _ => return false,
        };
        for (slot, index) in BOOT9_KEY_X_SLOTS {
            let start = section + index * 16;
            let Some(block) = boot9
                .get(start..start + 16)
                .and_then(|bytes| Block::try_from(bytes).ok())
            else {
                return false;
            };
            self.slots.entry((slot, KeyKind::X)).or_insert(block);
        }
        true
    }

    pub fn merge_file(&mut self, path: &Path) -> bool {
        let Ok(bytes) = std::fs::read(path) else {
            return false;
        };
        let merged = if is_boot9_file(path, &bytes) {
            self.merge_boot9(&bytes)
        } else {
            match std::str::from_utf8(&bytes) {
                Ok(text) => {
                    self.merge_aes_keys_text(text);
                    true
                }
                Err(_) => false,
            }
        };
        if merged {
            self.sources.push(path.to_path_buf());
        }
        merged
    }

    pub fn key_x(&self, slot: u8) -> Option<Block> {
        self.slots.get(&(slot, KeyKind::X)).copied()
    }

    pub fn normal_key(&self, slot: u8, key_y: &Block) -> Option<Block> {
        self.key_x(slot)
            .map(|key_x| scramble_normal_key(&key_x, key_y))
    }

    pub fn common_key(&self, index: usize) -> Option<Block> {
        if let Some(normal) = self.common_normal.get(index).copied().flatten() {
            return Some(normal);
        }
        let key_y = self.common_key_y.get(index).copied().flatten()?;
        self.normal_key(COMMON_KEY_SLOT, &key_y)
    }

    pub fn common_key_from_parts(&self, index: usize) -> Option<Block> {
        let key_y = self.common_key_y.get(index).copied().flatten()?;
        self.normal_key(COMMON_KEY_SLOT, &key_y)
    }

    pub fn stored_common_normal_key(&self, index: usize) -> Option<Block> {
        self.common_normal.get(index).copied().flatten()
    }
}

fn is_boot9_file(path: &Path, bytes: &[u8]) -> bool {
    let named_bin = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("bin"));
    named_bin && matches!(bytes.len(), BOOT9_FULL_SIZE | BOOT9_PROTECTED_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(block: &Block) -> String {
        block.iter().map(|byte| format!("{byte:02X}")).collect()
    }

    #[test]
    fn scrambler_is_rotate_xor_add_rotate() {
        let key_x = [0u8; 16];
        let key_y = [0u8; 16];
        assert_eq!(
            scramble_normal_key(&key_x, &key_y),
            SCRAMBLER_CONSTANT.rotate_left(87).to_be_bytes()
        );
        let mut key_x = [0u8; 16];
        key_x[15] = 1;
        let expected = (4u128.wrapping_add(SCRAMBLER_CONSTANT)).rotate_left(87);
        assert_eq!(scramble_normal_key(&key_x, &key_y), expected.to_be_bytes());
    }

    #[test]
    fn parses_slots_and_common_keys() {
        let key_x: Block = std::array::from_fn(|index| index as u8 * 3);
        let key_y: Block = std::array::from_fn(|index| 0xF0 ^ index as u8);
        let text = format!(
            "# comment\n\nslot0x2CKeyX={}\ncommon1={}\nslot0x3DKeyX={}\nbroken=zz\n",
            hex(&key_x),
            hex(&key_y),
            hex(&key_x)
        );
        let store = KeyStore::from_aes_keys_text(&text);
        assert_eq!(store.key_x(0x2C), Some(key_x));
        assert!(store.key_x(0x25).is_none());
        assert_eq!(
            store.common_key(1),
            Some(scramble_normal_key(&key_x, &key_y))
        );
        assert!(store.common_key(0).is_none());
    }

    #[test]
    fn reads_key_x_from_boot9_layouts() {
        let mut full = vec![0u8; BOOT9_FULL_SIZE];
        full[BOOT9_FULL_KEY_SECTION..BOOT9_FULL_KEY_SECTION + 16].fill(0xAB);
        full[BOOT9_FULL_KEY_SECTION + 80..BOOT9_FULL_KEY_SECTION + 96].fill(0xCD);
        let mut store = KeyStore::default();
        assert!(store.merge_boot9(&full));
        assert_eq!(store.key_x(0x2C), Some([0xAB; 16]));
        assert_eq!(store.key_x(0x3D), Some([0xCD; 16]));
        let mut protected = KeyStore::default();
        assert!(protected.merge_boot9(&full[BOOT9_PROTECTED_SIZE..]));
        assert_eq!(protected.key_x(0x2C), Some([0xAB; 16]));
        assert!(!KeyStore::default().merge_boot9(&[0u8; 12]));
    }
}
