pub mod bytes;
pub mod held_items;
pub mod progress_writer;
pub mod rom;
pub mod rom_entry;
pub mod strings;

pub use rom::{BoxedSource, CtrIdentity, CtrRom, RomSource};
pub use rom_entry::{CtrRomEntry, KeyPrecedence, RomFileEntry, find_entry, parse_rom_entries};

pub const TYPE_XY: i32 = 0;
pub const TYPE_ORAS: i32 = 1;
pub const TYPE_SM: i32 = 2;
pub const TYPE_USUM: i32 = 3;
