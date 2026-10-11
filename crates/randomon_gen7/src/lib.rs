mod constants;
mod encounters;
mod handler;
mod held_items;
mod items;
mod moves;
mod pokemon;
mod rom_entry;
mod rom_handler;
mod statics;
mod tables;
mod trainers;
mod tweaks;

use std::path::Path;

use randomon_core::{DetectedGame, Result, RomHandler, RomHandlerFactory};
use randomon_ctr::rom::{bytes_source, identify_bytes, identify_file, open_file_source};
use randomon_java::SharedRandomSource;

pub use handler::Gen7RomHandler;
pub use rom_entry::entry_for;

#[derive(Clone, Copy, Debug, Default)]
pub struct Gen7Factory;

impl RomHandlerFactory for Gen7Factory {
    fn generation(&self) -> i32 {
        7
    }

    fn is_loadable(&self, rom: &[u8]) -> bool {
        entry_for(&identify_bytes(rom)).is_some()
    }

    fn create(
        &self,
        rom: Vec<u8>,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>> {
        Ok(Box::new(Gen7RomHandler::load(
            bytes_source(rom),
            file_name,
            random_source,
        )?))
    }

    fn is_loadable_file(&self, path: &Path) -> bool {
        entry_for(&identify_file(path)).is_some()
    }

    fn detect_file(&self, path: &Path) -> Option<DetectedGame> {
        entry_for(&identify_file(path)).map(|entry| DetectedGame {
            generation: 7,
            rom_name: format!("Pokemon {}", entry.name),
            rom_code: entry.rom_code.clone(),
        })
    }

    fn create_from_file(
        &self,
        path: &Path,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>> {
        Ok(Box::new(Gen7RomHandler::load(
            open_file_source(path)?,
            file_name,
            random_source,
        )?))
    }
}
