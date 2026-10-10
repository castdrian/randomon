pub const MIN_ROM_SIZE: usize = 0x80000;
pub const MAX_ROM_SIZE: usize = 0x200000;
pub const JP_FLAG_OFFSET: usize = 0x14A;
pub const VERSION_OFFSET: usize = 0x14C;
pub const HEADER_CHECKSUM_OFFSET: usize = 0x14D;
pub const CRC_OFFSET: usize = 0x14E;
pub const ROM_SIG_OFFSET: usize = 0x134;
pub const IS_GBC_OFFSET: usize = 0x143;
pub const ROM_CODE_OFFSET: usize = 0x13F;
pub const BANK_SIZE: i32 = 0x4000;

const HEADER_CHECKSUM_RANGE: std::ops::Range<usize> = 0x134..0x14D;

pub fn crc_in_header(rom: &[u8]) -> i32 {
    (rom[CRC_OFFSET] as i32) << 8 | rom[CRC_OFFSET + 1] as i32
}

pub fn rom_sig_matches(rom: &[u8], signature: &str) -> bool {
    ascii_matches_at(rom, ROM_SIG_OFFSET, signature)
}

pub fn rom_code_matches(rom: &[u8], code: &str) -> bool {
    ascii_matches_at(rom, ROM_CODE_OFFSET, code)
}

fn ascii_matches_at(rom: &[u8], offset: usize, text: &str) -> bool {
    crate::java::strings::encode_us_ascii(text)
        .iter()
        .enumerate()
        .all(|(index, &byte)| rom[offset + index] == byte)
}

pub fn make_gb_pointer(offset: i32) -> i32 {
    if offset < BANK_SIZE {
        offset
    } else {
        (offset % BANK_SIZE) + BANK_SIZE
    }
}

pub fn bank_of(offset: i32) -> i32 {
    offset / BANK_SIZE
}

pub fn calculate_offset(bank: i32, pointer: i32) -> i32 {
    if pointer < BANK_SIZE {
        pointer
    } else {
        (pointer % BANK_SIZE) + bank * BANK_SIZE
    }
}

pub fn compute_header_checksum(rom: &[u8]) -> u8 {
    rom[HEADER_CHECKSUM_RANGE]
        .iter()
        .fold(0u8, |sum, &byte| sum.wrapping_sub(byte).wrapping_sub(1))
}

pub fn compute_global_checksum(rom: &[u8]) -> u16 {
    rom.iter()
        .enumerate()
        .filter(|(index, _)| *index != CRC_OFFSET && *index != CRC_OFFSET + 1)
        .fold(0u16, |sum, (_, &byte)| sum.wrapping_add(byte as u16))
}

pub fn header_checksum_is_valid(rom: &[u8]) -> bool {
    compute_header_checksum(rom) == rom[HEADER_CHECKSUM_OFFSET]
}

pub fn global_checksum_is_valid(rom: &[u8]) -> bool {
    compute_global_checksum(rom) as i32 == crc_in_header(rom)
}
