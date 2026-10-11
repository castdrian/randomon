#![allow(dead_code)]

pub mod blob;
pub mod game;
pub mod nds;

use randomon_gen4::{RomEntry, entry_for, rom_entries};

pub use game::Quirks;

pub fn build_rom(entry: &'static RomEntry, quirks: Quirks) -> Vec<u8> {
    game::GameBuilder::new(entry, quirks).build()
}

pub fn entry(code: &str, version: i32) -> &'static RomEntry {
    entry_for(code, version).expect("known Gen 4 ROM entry")
}

pub fn all_entries() -> &'static [RomEntry] {
    rom_entries()
}
