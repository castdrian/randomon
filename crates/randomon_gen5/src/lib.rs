mod encounters;
mod evolutions;
mod handler;
mod items;
mod moves;
mod patches;
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

pub use handler::{Gen5RomHandler, detect_rom};

#[derive(Clone, Copy, Debug, Default)]
pub struct Gen5Factory;

impl RomHandlerFactory for Gen5Factory {
    fn generation(&self) -> i32 {
        5
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
        Ok(Box::new(Gen5RomHandler::load(
            rom,
            file_name,
            random_source,
        )?))
    }
}
