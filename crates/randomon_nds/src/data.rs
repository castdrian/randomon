use randomon_formats::bytes;
use randomon_formats::search::{HexFind, find_hex};

fn index(offset: i32) -> usize {
    usize::try_from(offset).expect("array offsets are not negative")
}

pub fn read_byte(data: &[u8], offset: i32) -> i32 {
    i32::from(data[index(offset)])
}

pub fn signed_byte(data: &[u8], offset: i32) -> i32 {
    i32::from(data[index(offset)] as i8)
}

pub fn read_word(data: &[u8], offset: i32) -> i32 {
    bytes::read_word(data, index(offset))
}

pub fn read_long(data: &[u8], offset: i32) -> i32 {
    bytes::read_long(data, index(offset))
}

pub fn read_full_int(data: &[u8], offset: i32) -> i32 {
    read_long(data, offset)
}

pub fn read_relative_pointer(data: &[u8], offset: i32) -> i32 {
    read_long(data, offset).wrapping_add(offset).wrapping_add(4)
}

pub fn write_word(data: &mut [u8], offset: i32, value: i32) {
    bytes::write_word(data, index(offset), value);
}

pub fn write_long(data: &mut [u8], offset: i32, value: i32) {
    bytes::write_long(data, index(offset), value);
}

pub fn write_full_int(data: &mut [u8], offset: i32, value: i32) {
    write_long(data, offset, value);
}

pub fn write_relative_pointer(data: &mut [u8], offset: i32, pointer: i32) {
    write_long(data, offset, pointer.wrapping_sub(offset.wrapping_add(4)));
}

pub fn read_byte_into_flags(
    data: &[u8],
    flags: &mut [bool],
    offset_into_flags: usize,
    offset_into_data: i32,
) {
    bytes::read_byte_into_flags(data, flags, offset_into_flags, index(offset_into_data));
}

pub fn get_byte_from_flags(flags: &[bool], offset_into_flags: usize) -> u8 {
    bytes::byte_from_flags(flags, offset_into_flags)
}

pub fn find(data: &[u8], hex: &str) -> i32 {
    find_hex(data, hex).map_or(HexFind::NotFound.upr_code(), HexFind::upr_code)
}
