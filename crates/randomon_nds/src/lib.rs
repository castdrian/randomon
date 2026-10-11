mod arm9;
mod data;
mod detect;
mod palette;
mod rom;

pub use arm9::{Arm9Extension, extend_arm9};
pub use data::{
    find, get_byte_from_flags, read_byte, read_byte_into_flags, read_full_int, read_long,
    read_relative_pointer, read_word, signed_byte, write_full_int, write_long,
    write_relative_pointer, write_word,
};
pub use detect::{rom_code_from_bytes, version_from_bytes};
pub use palette::type_tm_palette_number;
pub use rom::{DsRom, format_error};
