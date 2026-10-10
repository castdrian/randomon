pub fn read_word(data: &[u8], offset: usize) -> i32 {
    data[offset] as i32 | (data[offset + 1] as i32) << 8
}

pub fn read_word_be(data: &[u8], offset: usize) -> i32 {
    data[offset + 1] as i32 | (data[offset] as i32) << 8
}

pub fn read_long(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub fn read_long_be(data: &[u8], offset: usize) -> i32 {
    i32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub fn read_full_long(data: &[u8], offset: usize) -> i64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    i64::from_le_bytes(bytes)
}

pub fn read_le_sized(data: &[u8], offset: usize, size: usize) -> i32 {
    (0..size).fold(0i32, |value, index| {
        value | ((data[offset + index] as i32) << (index * 8))
    })
}

pub fn write_le_sized(data: &mut [u8], offset: usize, size: usize, value: i32) {
    for index in 0..size {
        data[offset + index] = (value >> (index * 8)) as u8;
    }
}

pub fn write_word(data: &mut [u8], offset: usize, value: i32) {
    data[offset] = value as u8;
    data[offset + 1] = (value >> 8) as u8;
}

pub fn write_word_divmod(data: &mut [u8], offset: usize, value: i32) {
    data[offset] = (value % 0x100) as u8;
    data[offset + 1] = ((value / 0x100) % 0x100) as u8;
}

pub fn write_word_be(data: &mut [u8], offset: usize, value: i32) {
    data[offset] = (value >> 8) as u8;
    data[offset + 1] = value as u8;
}

pub fn write_long(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn write_long_be(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

pub fn write_full_long(data: &mut [u8], offset: usize, value: i64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

pub fn three_bytes(amount: i32) -> [u8; 3] {
    [amount as u8, (amount >> 8) as u8, (amount >> 16) as u8]
}

pub fn read_relative_pointer(data: &[u8], offset: usize) -> i32 {
    read_long(data, offset)
        .wrapping_add(offset as i32)
        .wrapping_add(4)
}

pub fn write_relative_pointer(data: &mut [u8], offset: usize, pointer: i32) {
    let relative = pointer.wrapping_sub((offset as i32).wrapping_add(4));
    write_long(data, offset, relative);
}

pub fn matches_at(data: &[u8], offset: usize, needle: &[u8]) -> bool {
    needle
        .iter()
        .enumerate()
        .all(|(index, &byte)| data.get(offset + index) == Some(&byte))
}

pub fn read_byte_into_flags(
    data: &[u8],
    flags: &mut [bool],
    offset_into_flags: usize,
    offset_into_data: usize,
) {
    let byte = data[offset_into_data];
    for bit in 0..8 {
        if let Some(flag) = flags.get_mut(offset_into_flags + bit) {
            *flag = (byte >> bit) & 1 == 1;
        }
    }
}

pub fn byte_from_flags(flags: &[bool], offset_into_flags: usize) -> u8 {
    (0..8).fold(0u8, |byte, bit| match flags.get(offset_into_flags + bit) {
        Some(true) => byte | (1 << bit),
        _ => byte,
    })
}

pub fn crc32(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}
