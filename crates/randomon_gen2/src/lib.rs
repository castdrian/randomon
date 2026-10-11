mod assets;
pub mod constants;
mod encounters;
mod field;
mod handler;
pub mod items;
mod learnsets;
mod maps;
mod moves;
mod rom_entry;
mod rom_handler;
mod statics;
mod trainer_space;
mod trainers;
mod tweaks;

use randomon_core::{Result, RomHandler, RomHandlerFactory};
use randomon_java::SharedRandomSource;

pub use handler::{Gen2RomHandler, detect_rom};

const MAX_LOADABLE_FILE_SIZE: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default)]
pub struct Gen2Factory;

impl RomHandlerFactory for Gen2Factory {
    fn generation(&self) -> i32 {
        2
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
        Ok(Box::new(Gen2RomHandler::load(
            rom,
            file_name,
            random_source,
        )?))
    }
}
