use randomon_data::constants::gb;

pub fn make_gb_pointer(offset: i32) -> i32 {
    if offset < gb::BANK_SIZE {
        offset
    } else {
        (offset % gb::BANK_SIZE) + gb::BANK_SIZE
    }
}

pub fn bank_of(offset: i32) -> i32 {
    offset / gb::BANK_SIZE
}

pub fn calculate_offset(bank: i32, pointer: i32) -> i32 {
    if pointer < gb::BANK_SIZE {
        pointer
    } else {
        (pointer % gb::BANK_SIZE) + bank * gb::BANK_SIZE
    }
}

pub fn read_word(data: &[u8], offset: i32) -> i32 {
    let offset = offset as usize;
    i32::from(data[offset]) + (i32::from(data[offset + 1]) << 8)
}

pub fn write_word(data: &mut [u8], offset: i32, value: i32) {
    let offset = offset as usize;
    data[offset] = (value % 0x100) as u8;
    data[offset + 1] = ((value / 0x100) % 0x100) as u8;
}

pub fn read_byte_into_flags(
    data: &[u8],
    flags: &mut [bool],
    offset_into_flags: usize,
    offset: i32,
) {
    let value = data[offset as usize];
    for bit in 0..8 {
        if let Some(flag) = flags.get_mut(offset_into_flags + bit) {
            *flag = (value >> bit) & 1 == 1;
        }
    }
}

pub fn byte_from_flags(flags: &[bool], offset_into_flags: usize) -> u8 {
    (0..8)
        .filter(|&bit| flags.get(offset_into_flags + bit) == Some(&true))
        .fold(0u8, |value, bit| value | (1 << bit))
}

fn ascii_matches_at(rom: &[u8], start: usize, text: &str) -> bool {
    text.bytes()
        .enumerate()
        .all(|(index, byte)| rom.get(start + index) == Some(&byte))
}

pub fn rom_signature_matches(rom: &[u8], signature: &str) -> bool {
    ascii_matches_at(rom, gb::ROM_SIG_OFFSET, signature)
}

pub fn rom_code_matches(rom: &[u8], code: &str) -> bool {
    ascii_matches_at(rom, gb::ROM_CODE_OFFSET, code)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderIds {
    pub version: i32,
    pub non_japanese: i32,
    pub crc_in_header: i32,
}

impl HeaderIds {
    pub fn read(rom: &[u8]) -> Option<Self> {
        if rom.len() <= gb::CRC_OFFSET + 1 {
            return None;
        }
        Some(Self {
            version: i32::from(rom[gb::VERSION_OFFSET]),
            non_japanese: i32::from(rom[gb::JP_FLAG_OFFSET]),
            crc_in_header: (i32::from(rom[gb::CRC_OFFSET]) << 8)
                | i32::from(rom[gb::CRC_OFFSET + 1]),
        })
    }
}

pub fn crc32(bytes: &[u8]) -> i64 {
    i64::from(crc32fast::hash(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointers_round_trip_through_banks() {
        assert_eq!(make_gb_pointer(0x3FFF), 0x3FFF);
        assert_eq!(make_gb_pointer(0x39DD1), 0x5DD1);
        assert_eq!(calculate_offset(bank_of(0x39DD1), 0x5DD1), 0x39DD1);
        assert_eq!(calculate_offset(0x0E, 0x1234), 0x1234);
    }

    #[test]
    fn words_are_little_endian() {
        let mut data = vec![0u8; 4];
        write_word(&mut data, 1, 0xBEEF);
        assert_eq!(data, vec![0, 0xEF, 0xBE, 0]);
        assert_eq!(read_word(&data, 1), 0xBEEF);
    }

    #[test]
    fn flags_round_trip_through_bytes() {
        let mut flags = vec![false; 11];
        read_byte_into_flags(&[0b1010_0101], &mut flags, 1, 0);
        assert_eq!(
            flags,
            vec![
                false, true, false, true, false, false, true, false, true, false, false
            ]
        );
        assert_eq!(byte_from_flags(&flags, 1), 0b1010_0101);
        assert_eq!(byte_from_flags(&flags, 5), 0b0000_1010);
    }

    #[test]
    fn crc32_matches_the_reference_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
