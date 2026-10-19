use std::path::{Path, PathBuf};

use crate::engine::{self, EngineGame};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["gb", "gbc", "gba", "nds", "3ds", "cci", "cxi"];

#[derive(Debug, Clone)]
pub struct LoadedRom {
    pub path: PathBuf,
    pub file_name: String,
    pub platform: Platform,
    pub size_bytes: u64,
    pub rom_name: String,
    pub engine: Option<EngineGame>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    GameBoy,
    GameBoyColor,
    GameBoyAdvance,
    NintendoDs,
    Nintendo3ds,
    NintendoSwitch,
}

impl Platform {
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "gb" => Some(Self::GameBoy),
            "gbc" => Some(Self::GameBoyColor),
            "gba" => Some(Self::GameBoyAdvance),
            "nds" => Some(Self::NintendoDs),
            "3ds" | "cci" | "cxi" => Some(Self::Nintendo3ds),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::GameBoy => "Game Boy",
            Self::GameBoyColor => "Game Boy Color",
            Self::GameBoyAdvance => "Game Boy Advance",
            Self::NintendoDs => "Nintendo DS",
            Self::Nintendo3ds => "Nintendo 3DS",
            Self::NintendoSwitch => "Nintendo Switch",
        }
    }

    pub fn supports_game_updates(self) -> bool {
        self == Self::Nintendo3ds
    }
}

pub fn switch_romfs_containing(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .filter(|ancestor| ancestor.is_dir())
        .find(|ancestor| engine::registry().detect_file(ancestor).is_some())
        .map(Path::to_path_buf)
}

fn romfs_display_name(romfs: &Path) -> String {
    let name = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    match romfs.parent() {
        Some(parent) if name(romfs).eq_ignore_ascii_case("romfs") => {
            format!("{}/romfs", name(parent))
        }
        _ => name(romfs),
    }
}

fn inspect_switch(romfs: PathBuf) -> Result<LoadedRom, String> {
    let engine = engine::inspect_checked(&romfs, 0)?
        .ok_or("That folder isn't a supported game or romfs folder.")?;
    Ok(LoadedRom {
        file_name: romfs_display_name(&romfs),
        platform: Platform::NintendoSwitch,
        size_bytes: 0,
        rom_name: engine.rom_name.clone(),
        engine: Some(engine),
        path: romfs,
    })
}

pub fn inspect(path: PathBuf) -> Result<LoadedRom, String> {
    let Some(platform) = Platform::from_path(&path) else {
        return match switch_romfs_containing(&path) {
            Some(romfs) => inspect_switch(romfs),
            None if path.is_dir() => {
                Err("That folder isn't a supported game or romfs folder.".to_owned())
            }
            None => Err("This file type isn't a supported ROM.".to_owned()),
        };
    };
    let size_bytes = std::fs::metadata(&path)
        .map_err(|error| format!("Couldn't read the file: {error}"))?
        .len();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let engine = engine::inspect_checked(&path, size_bytes)?;
    let rom_name = engine
        .as_ref()
        .map(|game| game.rom_name.clone())
        .unwrap_or_else(|| provisional_rom_name(&path));
    Ok(LoadedRom {
        path,
        file_name,
        platform,
        size_bytes,
        rom_name,
        engine,
    })
}

fn provisional_rom_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod switch_tests {
    use super::*;

    #[test]
    fn loads_a_bdsp_romfs_folder() {
        let Some(romfs) = std::env::var_os("RANDOMON_BDSP_ROMFS").map(PathBuf::from) else {
            return;
        };
        let loaded = inspect(romfs).expect("loads");
        assert_eq!(loaded.platform, Platform::NintendoSwitch);
        assert_eq!(loaded.engine.expect("engine").generation, 8);
    }
}
