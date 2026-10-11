use randomon_formats::search::{HexFind, find_hex, find_hex_multiple, parse_hex_bytes};

pub const POINTER_BASE: i32 = 0x0800_0000;
pub const FREE_SPACE_BYTE: u8 = 0xFF;
pub const LARGEST_ROM_SIZE: usize = 0x200_0000;
const OUT_OF_RANGE_BYTE: u8 = 0xFF;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GbaRom {
    bytes: Vec<u8>,
}

impl GbaRom {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> i32 {
        self.bytes.len() as i32
    }

    fn index(&self, offset: i32) -> Option<usize> {
        usize::try_from(offset)
            .ok()
            .filter(|&index| index < self.bytes.len())
    }

    pub fn contains(&self, offset: i32) -> bool {
        self.index(offset).is_some()
    }

    pub fn contains_range(&self, offset: i32, length: i32) -> bool {
        length >= 0
            && offset >= 0
            && i64::from(offset) + i64::from(length) <= self.bytes.len() as i64
    }

    pub fn byte(&self, offset: i32) -> u8 {
        self.index(offset)
            .map_or(OUT_OF_RANGE_BYTE, |index| self.bytes[index])
    }

    pub fn int(&self, offset: i32) -> i32 {
        i32::from(self.byte(offset))
    }

    pub fn signed(&self, offset: i32) -> i32 {
        i32::from(self.byte(offset) as i8)
    }

    pub fn word(&self, offset: i32) -> i32 {
        self.int(offset) + (self.int(offset.wrapping_add(1)) << 8)
    }

    pub fn long(&self, offset: i32) -> i32 {
        (0..4).fold(0i32, |value, index| {
            value.wrapping_add(self.int(offset.wrapping_add(index)) << (8 * index))
        })
    }

    pub fn pointer(&self, offset: i32) -> i32 {
        self.long(offset).wrapping_sub(POINTER_BASE)
    }

    pub fn is_valid_target(&self, pointer: i32) -> bool {
        self.contains(pointer)
    }

    pub fn valid_pointer(&self, offset: i32) -> Option<i32> {
        let pointer = self.pointer(offset);
        self.is_valid_target(pointer).then_some(pointer)
    }

    pub fn set_byte(&mut self, offset: i32, value: u8) {
        if let Some(index) = self.index(offset) {
            self.bytes[index] = value;
        }
    }

    pub fn set_word(&mut self, offset: i32, value: i32) {
        self.set_byte(offset, (value % 0x100) as u8);
        self.set_byte(offset.wrapping_add(1), ((value / 0x100) % 0x100) as u8);
    }

    pub fn set_long(&mut self, offset: i32, value: i32) {
        for (index, byte) in value.to_le_bytes().into_iter().enumerate() {
            self.set_byte(offset.wrapping_add(index as i32), byte);
        }
    }

    pub fn set_pointer(&mut self, offset: i32, pointer: i32) {
        self.set_long(offset, pointer.wrapping_add(POINTER_BASE));
    }

    pub fn write_bytes(&mut self, offset: i32, data: &[u8]) {
        for (index, &byte) in data.iter().enumerate() {
            self.set_byte(offset.wrapping_add(index as i32), byte);
        }
    }

    pub fn write_hex(&mut self, offset: i32, hex: &str) {
        if !hex.len().is_multiple_of(2) {
            return;
        }
        if let Ok(data) = parse_hex_bytes(hex) {
            self.write_bytes(offset, &data);
        }
    }

    pub fn slice(&self, offset: i32, length: i32) -> Option<&[u8]> {
        self.contains_range(offset, length)
            .then(|| &self.bytes[offset as usize..(offset + length) as usize])
    }

    pub fn copy_within(&mut self, source: i32, destination: i32, length: i32) {
        if self.contains_range(source, length) && self.contains_range(destination, length) {
            let source = source as usize;
            self.bytes
                .copy_within(source..source + length as usize, destination as usize);
        }
    }

    pub fn matches(&self, offset: i32, needle: &[u8]) -> bool {
        self.slice(offset, needle.len() as i32)
            .is_some_and(|found| found == needle)
    }

    pub fn find(&self, hex: &str) -> i32 {
        find_in(&self.bytes, hex)
    }

    pub fn find_multiple(&self, hex: &str) -> Vec<i32> {
        find_multiple_in(&self.bytes, hex)
    }

    pub fn search(&self, needle: &[u8]) -> Vec<i32> {
        self.search_range(0, self.len(), needle)
    }

    pub fn search_range(&self, begin: i32, end: i32, needle: &[u8]) -> Vec<i32> {
        if needle.len() < 2 || begin < 0 || end < begin {
            return Vec::new();
        }
        let end = (end as usize).min(self.bytes.len());
        randomon_formats::search::search_range(&self.bytes, begin as usize, end, needle)
            .map(|found| found.into_iter().map(|offset| offset as i32).collect())
            .unwrap_or_default()
    }

    pub fn upstream_free_space(&self, amount: i32, start: i32, long_aligned: bool) -> i32 {
        let amount = amount.max(0) as usize;
        let start = start.max(0) as usize;
        randomon_formats::search::free_space_finder(
            &self.bytes,
            FREE_SPACE_BYTE,
            amount,
            start,
            long_aligned,
        )
        .unwrap_or(-1)
    }

    pub fn allocate(&mut self, amount: i32, start: i32, long_aligned: bool) -> Option<i32> {
        let found = self.upstream_free_space(amount, start, long_aligned);
        if found >= start && self.contains_range(found, amount) {
            return Some(found);
        }
        if self.bytes.len() >= LARGEST_ROM_SIZE {
            return None;
        }
        let search_start = self.bytes.len() as i32;
        self.bytes.resize(LARGEST_ROM_SIZE, FREE_SPACE_BYTE);
        let found = self.upstream_free_space(amount, search_start.max(start), long_aligned);
        (found >= start && self.contains_range(found, amount)).then_some(found)
    }

    pub fn apply_ips(&mut self, patch: &[u8]) -> Result<(), String> {
        let extent = ips_extent(patch);
        if extent > self.bytes.len() && extent <= LARGEST_ROM_SIZE {
            self.bytes.resize(extent, FREE_SPACE_BYTE);
        }
        randomon_formats::ips::apply_patch(&mut self.bytes, patch)
            .map_err(|error| error.to_string())
    }
}

const IPS_HEADER_LENGTH: usize = 5;
const IPS_EOF: usize = 0x454F46;

fn ips_extent(patch: &[u8]) -> usize {
    let read = |position: usize, length: usize| {
        patch.get(position..position + length).map(|bytes| {
            bytes
                .iter()
                .fold(0usize, |value, &byte| (value << 8) | usize::from(byte))
        })
    };
    let mut extent = 0;
    let mut position = IPS_HEADER_LENGTH;
    while let Some(offset) = read(position, 3) {
        if offset == IPS_EOF {
            break;
        }
        let Some(size) = read(position + 3, 2) else {
            break;
        };
        if size == 0 {
            let Some(run) = read(position + 5, 2) else {
                break;
            };
            extent = extent.max(offset + run);
            position += 8;
        } else {
            extent = extent.max(offset + size);
            position += 5 + size;
        }
    }
    extent
}

pub fn find_in(bytes: &[u8], hex: &str) -> i32 {
    if hex.len() < 4 {
        return if hex.len().is_multiple_of(2) { -1 } else { -3 };
    }
    match find_hex(bytes, hex) {
        Ok(found) => found.upr_code(),
        Err(_) => HexFind::NotFound.upr_code(),
    }
}

pub fn find_multiple_in(bytes: &[u8], hex: &str) -> Vec<i32> {
    if hex.len() < 4 {
        return Vec::new();
    }
    find_hex_multiple(bytes, hex)
        .map(|found| found.into_iter().map(|offset| offset as i32).collect())
        .unwrap_or_default()
}

pub fn pointer_to_hex(pointer: i32) -> String {
    pointer
        .wrapping_add(POINTER_BASE)
        .to_le_bytes()
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_are_little_endian_and_pointers_are_rebased() {
        let rom = GbaRom::new(vec![0x34, 0x12, 0x00, 0x08, 0xFE]);
        assert_eq!(rom.word(0), 0x1234);
        assert_eq!(rom.long(0), 0x0800_1234);
        assert_eq!(rom.pointer(0), 0x1234);
        assert_eq!(rom.signed(4), -2);
    }

    #[test]
    fn out_of_range_access_never_panics() {
        let mut rom = GbaRom::new(vec![0; 4]);
        assert_eq!(rom.byte(-1), 0xFF);
        assert_eq!(rom.byte(100), 0xFF);
        assert_eq!(rom.word(3), 0xFF00);
        rom.set_word(3, 0xBEEF);
        rom.set_long(-2, 0x7FFF_FFFF);
        assert_eq!(rom.bytes(), &[0xFF, 0x7F, 0x00, 0xEF]);
        assert!(rom.valid_pointer(0).is_none());
    }

    #[test]
    fn java_word_writes_keep_the_remainder_semantics() {
        let mut rom = GbaRom::new(vec![0; 2]);
        rom.set_word(0, -1);
        assert_eq!(rom.bytes(), &[0xFF, 0x00]);
    }

    #[test]
    fn ips_patches_grow_short_roms_with_free_space() {
        let patch = [
            b'P', b'A', b'T', b'C', b'H', 0, 0, 6, 0, 2, 0xAA, 0xBB, 0, 0, 1, 0, 0, 0, 3, 0xCC,
            b'E', b'O', b'F',
        ];
        assert_eq!(ips_extent(&patch), 8);
        let mut rom = GbaRom::new(vec![0; 4]);
        rom.apply_ips(&patch).expect("patch applies");
        assert_eq!(rom.bytes(), &[0, 0xCC, 0xCC, 0xCC, 0xFF, 0xFF, 0xAA, 0xBB]);
    }

    #[test]
    fn pointer_hex_is_little_endian() {
        assert_eq!(pointer_to_hex(0x123456), "56341208");
    }

    #[test]
    fn find_follows_upstream_codes() {
        let rom = GbaRom::new(vec![1, 2, 3, 1, 2, 3, 4]);
        assert_eq!(rom.find("0304"), 5);
        assert_eq!(rom.find("0102"), -2);
        assert_eq!(rom.find("0909"), -1);
        assert_eq!(rom.find("010"), -3);
        assert_eq!(rom.find(""), -1);
        assert_eq!(rom.find_multiple("0102"), vec![0, 3]);
    }

    #[test]
    fn allocation_uses_upstream_alignment_then_grows_the_rom() {
        let mut bytes = vec![0u8; 64];
        bytes[16..48].fill(FREE_SPACE_BYTE);
        let mut rom = GbaRom::new(bytes);
        assert_eq!(rom.upstream_free_space(8, 0, true), 20);
        assert_eq!(rom.upstream_free_space(8, 0, false), 18);
        assert_eq!(rom.allocate(8, 0, true), Some(20));
        let mut full = GbaRom::new(vec![0u8; 64]);
        let grown = full.allocate(16, 8, true).expect("grows");
        assert!(grown >= 64);
        assert_eq!(full.len() as usize, LARGEST_ROM_SIZE);
    }
}
