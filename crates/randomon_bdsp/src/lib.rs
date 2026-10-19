mod assets;
mod bundles;
mod constants;
mod encounters;
mod error;
mod evolutions;
mod exefs;
mod handler;
mod items;
mod moves;
mod rom_handler;
mod species;
mod starters;
mod statics;
mod tables;
mod text;
mod trades;
mod trainers;
mod tweaks;

use std::path::{Path, PathBuf};

use randomon_core::{DetectedGame, RandomizationError, Result, RomHandler, RomHandlerFactory};
use randomon_java::SharedRandomSource;

pub use assets::{BundleId, path_ids};
pub use bundles::{Bundles, data_directory, looks_like_bdsp_romfs};
pub use error::{BdspError, data_error};
pub use exefs::{Game, STARTER_OFFSETS, StarterOffsets, movz_w, starter_patch, starter_patches};
pub use handler::{BdspRomHandler, DEFAULT_STARTERS, EXEFS_PATCH_FOLDER};
pub use items::{decode_item_argument, encode_item_argument};

pub const GENERATION: i32 = 8;
pub const ROM_NAME: &str = "Pokemon Brilliant Diamond / Shining Pearl";
pub const ROM_CODE: &str = "BDSP";

pub fn romfs_root(path: &Path) -> Option<PathBuf> {
    [path.to_path_buf(), path.join("romfs")]
        .into_iter()
        .find(|candidate| candidate.is_dir() && looks_like_bdsp_romfs(candidate))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BdspFactory;

impl RomHandlerFactory for BdspFactory {
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
    ) -> Result<Box<dyn RomHandler>> {
        Err(RandomizationError::UnsupportedRom(file_name.to_owned()))
    }

    fn is_loadable_file(&self, path: &Path) -> bool {
        romfs_root(path).is_some()
    }

    fn detect_file(&self, path: &Path) -> Option<DetectedGame> {
        romfs_root(path).map(|_| DetectedGame {
            generation: GENERATION,
            rom_name: ROM_NAME.to_owned(),
            rom_code: ROM_CODE.to_owned(),
        })
    }

    fn create_from_file(
        &self,
        path: &Path,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>> {
        let romfs = romfs_root(path)
            .ok_or_else(|| RandomizationError::UnsupportedRom(path.display().to_string()))?;
        Ok(Box::new(BdspRomHandler::load(
            &romfs,
            file_name,
            random_source,
        )?))
    }
}
