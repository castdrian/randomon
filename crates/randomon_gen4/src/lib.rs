mod bytes;
mod encounters;
mod evolutions;
mod handler;
mod items;
mod moves;
mod patches;
mod pokedex;
mod pokemon;
mod rom_entry;
mod rom_handler;
mod starters;
mod statics;
mod text;
mod trades;
mod trainers;
mod tweaks;

use randomon_core::{Result, RomHandler, RomHandlerFactory};
use randomon_java::SharedRandomSource;

pub use handler::{Gen4RomHandler, detect_rom};
pub use patches::patch_bytes;
pub use rom_entry::{
    RoamingPokemonEntry, RomEntry, ScriptEntry, StaticPokemonEntry, TextEntry, entry_for,
    rom_entries,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Gen4Factory;

impl RomHandlerFactory for Gen4Factory {
    fn generation(&self) -> i32 {
        4
    }

    fn is_loadable(&self, rom: &[u8]) -> bool {
        detect_rom(rom)
    }

    fn create(
        &self,
        rom: Vec<u8>,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>> {
        Ok(Box::new(Gen4RomHandler::load(
            rom,
            file_name,
            random_source,
        )?))
    }
}
