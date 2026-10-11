use randomon_data::Type;
use randomon_formats::search::find_hex;

pub fn read_word(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset]) | (i32::from(data[offset + 1]) << 8)
}

pub fn write_word(data: &mut [u8], offset: usize, value: i32) {
    data[offset] = (value & 0xFF) as u8;
    data[offset + 1] = ((value >> 8) & 0xFF) as u8;
}

pub fn read_long(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub fn write_long(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn write_long_big_endian(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

pub fn read_byte_into_flags(
    data: &[u8],
    flags: &mut [bool],
    offset_into_flags: usize,
    offset_into_data: usize,
) {
    let byte = data[offset_into_data];
    for bit in 0..8 {
        if offset_into_flags + bit < flags.len() {
            flags[offset_into_flags + bit] = (byte >> bit) & 1 == 1;
        }
    }
}

pub fn byte_from_flags(flags: &[bool], offset_into_flags: usize) -> u8 {
    (0..8)
        .filter(|bit| offset_into_flags + bit < flags.len() && flags[offset_into_flags + bit])
        .fold(0u8, |byte, bit| byte | (1 << bit))
}

pub fn find(data: &[u8], hex: &str) -> i32 {
    find_hex(data, hex).map_or(-1, |found| found.upr_code())
}

pub fn prefix_length(hex: &str) -> i32 {
    (hex.len() / 2) as i32
}

pub fn find_after_prefix(data: &[u8], hex: &str) -> Option<usize> {
    let offset = find(data, hex);
    (offset > 0).then(|| (offset + prefix_length(hex)) as usize)
}

pub fn type_tm_palette_number(kind: Option<Type>, is_gen7: bool) -> i32 {
    match kind {
        Some(Type::Dark) => 309,
        Some(Type::Dragon) => 310,
        Some(Type::Psychic) => 311,
        Some(Type::Normal) => 312,
        Some(Type::Poison) => 313,
        Some(Type::Ice) => 314,
        Some(Type::Fighting) => 315,
        Some(Type::Fire) => 316,
        Some(Type::Water) => 317,
        Some(Type::Flying) => 323,
        Some(Type::Grass) => 318,
        Some(Type::Rock) => 319,
        Some(Type::Electric) => 320,
        Some(Type::Ground) => 321,
        Some(Type::Steel) => 324,
        Some(Type::Bug) => 325,
        Some(Type::Fairy) => {
            if is_gen7 {
                555
            } else {
                546
            }
        }
        _ => 322,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_round_trip() {
        let mut flags = vec![false; 10];
        read_byte_into_flags(&[0b1010_0101], &mut flags, 1, 0);
        assert_eq!(byte_from_flags(&flags, 1), 0b1010_0101);
        assert_eq!(byte_from_flags(&flags, 5), 0b0000_1010);
    }

    #[test]
    fn find_reports_upstream_codes() {
        let data = [1u8, 2, 3, 1, 2];
        assert_eq!(find(&data, "0203"), 1);
        assert_eq!(find(&data, "0102"), -2);
        assert_eq!(find(&data, "09"), -1);
        assert_eq!(find(&data, "010"), -3);
        assert_eq!(find_after_prefix(&data, "0203"), Some(3));
    }
}
