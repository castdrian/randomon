mod assets;
pub mod constants;
mod detect;
mod encounters;
mod evolutions;
mod field;
mod hack;
mod handler;
pub mod items;
mod learnsets;
mod maps;
mod moves;
mod rom;
mod rom_entry;
mod rom_handler;
mod starters;
mod statics;
mod text;
mod text_format;
mod tms;
mod trainers;
mod tweaks;

#[cfg(any(test, feature = "synthetic"))]
pub mod synthetic;

use randomon_core::{Result, RomHandler, RomHandlerFactory};
use randomon_java::SharedRandomSource;

pub use detect::detect_rom;
pub use handler::Gen3RomHandler;

#[derive(Clone, Copy, Debug, Default)]
pub struct Gen3Factory;

impl RomHandlerFactory for Gen3Factory {
    fn generation(&self) -> i32 {
        3
    }

    fn is_loadable(&self, rom: &[u8]) -> bool {
        rom.len() <= constants::SIZE_32M && detect::detect_loadable(rom)
    }

    fn create(
        &self,
        rom: Vec<u8>,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>> {
        Ok(Box::new(Gen3RomHandler::load(
            rom,
            file_name,
            random_source,
        )?))
    }
}
