mod bytes;
mod constants;
mod encounters;
mod error;
mod evolutions;
pub mod formats;
mod handler;
mod io;
mod items;
mod moves;
pub mod names;
pub mod paths;
mod rom_handler;
mod romfs;
mod species;
mod starters;
mod statics;
mod trades;
mod trainers;
pub mod version;

use std::path::{Path, PathBuf};

use randomon_core::{DetectedGame, RandomizationError, RomHandler, RomHandlerFactory};
use randomon_java::SharedRandomSource;

pub use error::{Result, SwshError, invalid};
pub use handler::{EXEFS_PATCH_FOLDER, SwshRomHandler};
pub use io::RomfsLayers;
pub use names::{NameLists, TextRole};
pub use romfs::{GameData, GameFiles, ModifiedFile, PlacementArea, WildEncounters};
pub use version::{DataVersion, Detection, Game, detect, detect_with_exefs, looks_like_swsh_romfs};

pub const GENERATION: i32 = 8;
pub const ROM_NAME: &str = "Pokemon Sword / Shield";
pub const SWORD_ROM_NAME: &str = "Pokemon Sword";
pub const SHIELD_ROM_NAME: &str = "Pokemon Shield";
pub const ROM_CODE: &str = "SWSH";

pub fn romfs_root(path: &Path) -> Option<PathBuf> {
    [path.to_path_buf(), path.join("romfs")]
        .into_iter()
        .find(|candidate| candidate.is_dir() && looks_like_swsh_romfs(candidate))
}

pub fn rom_name_for(romfs: &Path) -> &'static str {
    match detect(romfs).and_then(|detection| detection.game) {
        Some(Game::Sword) => SWORD_ROM_NAME,
        Some(Game::Shield) => SHIELD_ROM_NAME,
        None => ROM_NAME,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SwshFactory;

impl RomHandlerFactory for SwshFactory {
    fn generation(&self) -> i32 {
        GENERATION
    }

    fn is_loadable(&self, _rom: &[u8]) -> bool {
        false
    }

    fn create(
        &self,
        _rom: Vec<u8>,
        file_name: &str,
        _random_source: &SharedRandomSource,
    ) -> randomon_core::Result<Box<dyn RomHandler>> {
        Err(RandomizationError::UnsupportedRom(file_name.to_owned()))
    }

    fn is_loadable_file(&self, path: &Path) -> bool {
        romfs_root(path).is_some()
    }

    fn detect_file(&self, path: &Path) -> Option<DetectedGame> {
        romfs_root(path).map(|romfs| DetectedGame {
            generation: GENERATION,
            rom_name: rom_name_for(&romfs).to_owned(),
            rom_code: ROM_CODE.to_owned(),
        })
    }

    fn create_from_file(
        &self,
        path: &Path,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> randomon_core::Result<Box<dyn RomHandler>> {
        let romfs = romfs_root(path)
            .ok_or_else(|| RandomizationError::UnsupportedRom(path.display().to_string()))?;
        Ok(Box::new(SwshRomHandler::load(
            &romfs,
            file_name,
            random_source,
        )?))
    }
}
