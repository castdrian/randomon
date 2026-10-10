mod gb;
mod handler;
mod maps;
mod moves;
mod rom_entry;

use randomon_core::{Result, RomHandler, RomHandlerFactory};
use randomon_java::SharedRandomSource;

pub use handler::{Gen1RomHandler, detect_rom};

const MAX_LOADABLE_FILE_SIZE: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default)]
pub struct Gen1Factory;

impl RomHandlerFactory for Gen1Factory {
    fn generation(&self) -> i32 {
        1
    }

    fn is_loadable(&self, rom: &[u8]) -> bool {
        rom.len() <= MAX_LOADABLE_FILE_SIZE && detect_rom(rom)
    }

    fn create(
        &self,
        rom: Vec<u8>,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>> {
        Ok(Box::new(Gen1RomHandler::load(
            rom,
            file_name,
            random_source,
        )?))
    }
}
