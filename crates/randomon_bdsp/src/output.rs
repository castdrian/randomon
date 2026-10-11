use std::path::{Path, PathBuf};

use crate::error::{BdspError, Result};
use crate::exefs::Game;

pub const MOD_NAME: &str = "randomon";
pub const EXEFS_PATCH_FOLDER: &str = "randomon_bdsp";
pub const LOG_FILE_NAME: &str = "randomon-bdsp-log.txt";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputLayout {
    #[default]
    Atmosphere,
    Yuzu,
    Ryujinx,
    Emulator,
}

impl OutputLayout {
    pub const ALL: [OutputLayout; 4] = [
        OutputLayout::Atmosphere,
        OutputLayout::Yuzu,
        OutputLayout::Ryujinx,
        OutputLayout::Emulator,
    ];

    pub fn parse(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "atmosphere" | "switch" => Some(Self::Atmosphere),
            "yuzu" | "suyu" | "sudachi" | "citron" | "eden" => Some(Self::Yuzu),
            "ryujinx" | "ryubing" => Some(Self::Ryujinx),
            "emulator" => Some(Self::Emulator),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Atmosphere => "Atmosphere (Switch)",
            Self::Yuzu => "Yuzu-family emulators",
            Self::Ryujinx => "Ryujinx",
            Self::Emulator => "Plain romfs/exefs folders",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFile {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RandomizedMod {
    pub romfs_files: Vec<(String, Vec<u8>)>,
    pub exefs_patches: Vec<(String, Vec<u8>)>,
    pub log: String,
}

fn mod_roots(layout: OutputLayout, game: Game) -> (PathBuf, PathBuf) {
    let title = game.title_id();
    match layout {
        OutputLayout::Atmosphere => (
            PathBuf::from("atmosphere/contents")
                .join(title)
                .join("romfs"),
            PathBuf::from("atmosphere/exefs_patches").join(EXEFS_PATCH_FOLDER),
        ),
        OutputLayout::Yuzu => {
            let root = PathBuf::from(title).join(MOD_NAME);
            (root.join("romfs"), root.join("exefs"))
        }
        OutputLayout::Ryujinx => {
            let root = PathBuf::from("mods/contents").join(title).join(MOD_NAME);
            (root.join("romfs"), root.join("exefs"))
        }
        OutputLayout::Emulator => (PathBuf::from("romfs"), PathBuf::from("exefs")),
    }
}

impl RandomizedMod {
    pub fn files(&self, layout: OutputLayout, games: &[Game]) -> Vec<OutputFile> {
        let mut files = Vec::new();
        let games: &[Game] = if layout == OutputLayout::Emulator {
            &games[..games.len().min(1)]
        } else {
            games
        };
        for &game in games {
            let (romfs_root, exefs_root) = mod_roots(layout, game);
            for (relative, bytes) in &self.romfs_files {
                files.push(OutputFile {
                    path: romfs_root.join("Data").join(relative),
                    bytes: bytes.clone(),
                });
            }
            for (build_id, bytes) in &self.exefs_patches {
                let path = exefs_root.join(format!("{build_id}.ips"));
                if !files.iter().any(|file| file.path == path) {
                    files.push(OutputFile {
                        path,
                        bytes: bytes.clone(),
                    });
                }
            }
        }
        files.push(OutputFile {
            path: PathBuf::from(LOG_FILE_NAME),
            bytes: self.log.clone().into_bytes(),
        });
        files
    }

    pub fn write_to(
        &self,
        directory: &Path,
        layout: OutputLayout,
        games: &[Game],
    ) -> Result<Vec<PathBuf>> {
        let mut written = Vec::new();
        for file in self.files(layout, games) {
            let path = directory.join(&file.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| BdspError::Io(parent.to_path_buf(), error.to_string()))?;
            }
            std::fs::write(&path, &file.bytes)
                .map_err(|error| BdspError::Io(path.clone(), error.to_string()))?;
            written.push(path);
        }
        Ok(written)
    }
}

pub fn detect_games(romfs: &Path) -> Vec<Game> {
    let found = romfs
        .ancestors()
        .filter_map(|ancestor| ancestor.file_name()?.to_str())
        .find_map(Game::from_title_id);
    match found {
        Some(game) => vec![game],
        None => Game::ALL.to_vec(),
    }
}
