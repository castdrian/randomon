pub mod bytes;
pub mod ini;
pub mod ips;
pub mod search;
pub mod space;
pub mod text;

pub use bytes::{
    HeaderIds, bank_of, byte_from_flags, calculate_offset, crc32, make_gb_pointer,
    read_byte_into_flags, read_word, rom_code_matches, rom_signature_matches, write_word,
};
pub use ini::{
    IniLine, StaticOffsets, ini_lines, parse_int_array, parse_ri_int, parse_ri_long,
    parse_static_offsets,
};
pub use ips::apply_ips_patch;
pub use search::{find_unique_hex, search};
pub use space::{BankPacker, Placement, Region, bank_end, merge_regions, uniform_tail};
pub use text::{GAMEBOY_JPN_TABLE, TextTables, length_of_string_at};
