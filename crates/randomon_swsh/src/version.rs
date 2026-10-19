use std::path::{Path, PathBuf};

use crate::bytes::{array_at, u32_at, u64_at};
use crate::error::{Result, invalid};
use crate::formats::encounters;
use crate::formats::personal::{EXPANSION_RECORD_SIZE, record_size_for};
use crate::paths;

const NPDM_CONTEXT: &str = "main.npdm";
const NSO_CONTEXT: &str = "main";
const NPDM_MAGIC: &[u8; 4] = b"META";
const ACI0_MAGIC: &[u8; 4] = b"ACI0";
const NSO_MAGIC: &[u8; 4] = b"NSO0";
const ACI0_OFFSET_FIELD: usize = 0x70;
const ACI0_PROGRAM_ID: usize = 0x10;
const LEGACY_TITLE_ID_OFFSET: usize = 0x290;
const NSO_BUILD_ID_OFFSET: usize = 0x40;
pub const BUILD_ID_SIZE: usize = 0x20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataVersion {
    Base,
    Expansion,
}

impl DataVersion {
    pub fn name(self) -> &'static str {
        match self {
            DataVersion::Base => "1.0.x base game",
            DataVersion::Expansion => "1.2.0+ with Isle of Armor and Crown Tundra",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Game {
    Sword,
    Shield,
}

impl Game {
    pub const ALL: [Game; 2] = [Game::Sword, Game::Shield];

    pub fn title_id(self) -> u64 {
        match self {
            Game::Sword => 0x0100_ABF0_0896_8000,
            Game::Shield => 0x0100_8DB0_08C2_C000,
        }
    }

    pub fn title_id_text(self) -> String {
        format!("{:016X}", self.title_id())
    }

    pub fn name(self) -> &'static str {
        match self {
            Game::Sword => "Pokémon Sword",
            Game::Shield => "Pokémon Shield",
        }
    }

    pub fn from_title_id(title_id: u64) -> Option<Game> {
        Game::ALL
            .into_iter()
            .find(|game| game.title_id() == title_id)
    }

    pub fn hidden_encounter_file(self) -> &'static str {
        match self {
            Game::Sword => encounters::SWORD_HIDDEN_FILE,
            Game::Shield => encounters::SHIELD_HIDDEN_FILE,
        }
    }

    pub fn symbol_encounter_file(self) -> &'static str {
        match self {
            Game::Sword => encounters::SWORD_SYMBOL_FILE,
            Game::Shield => encounters::SHIELD_SYMBOL_FILE,
        }
    }
}

pub fn read_title_id(npdm: &[u8]) -> Result<u64> {
    if array_at::<4>(npdm, 0, NPDM_CONTEXT)? != *NPDM_MAGIC {
        return Err(invalid(NPDM_CONTEXT, "missing META magic"));
    }
    let aci0 = u32_at(npdm, ACI0_OFFSET_FIELD, NPDM_CONTEXT)? as usize;
    if array_at::<4>(npdm, aci0, NPDM_CONTEXT).ok() == Some(*ACI0_MAGIC) {
        return u64_at(npdm, aci0 + ACI0_PROGRAM_ID, NPDM_CONTEXT);
    }
    u64_at(npdm, LEGACY_TITLE_ID_OFFSET, NPDM_CONTEXT)
}

pub fn read_build_id(nso: &[u8]) -> Result<[u8; BUILD_ID_SIZE]> {
    if array_at::<4>(nso, 0, NSO_CONTEXT)? != *NSO_MAGIC {
        return Err(invalid(NSO_CONTEXT, "missing NSO0 magic"));
    }
    array_at(nso, NSO_BUILD_ID_OFFSET, NSO_CONTEXT)
}

pub fn build_id_text(build_id: &[u8; BUILD_ID_SIZE]) -> String {
    let significant = build_id
        .iter()
        .rposition(|&byte| byte != 0)
        .map_or(0, |index| index + 1)
        .max(0x10);
    build_id[..significant]
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    pub game: Option<Game>,
    pub version: DataVersion,
    pub title_id: Option<u64>,
    pub build_id: Option<[u8; BUILD_ID_SIZE]>,
}

pub fn looks_like_swsh_romfs(romfs: &Path) -> bool {
    paths::REQUIRED
        .iter()
        .all(|relative| romfs.join(relative).exists())
}

pub fn data_version(romfs: &Path) -> Option<DataVersion> {
    let length = std::fs::metadata(romfs.join(paths::PERSONAL)).ok()?.len();
    let size = record_size_for(usize::try_from(length).ok()?).ok()?;
    Some(if size == EXPANSION_RECORD_SIZE {
        DataVersion::Expansion
    } else {
        DataVersion::Base
    })
}

pub fn sibling_exefs(romfs: &Path) -> Option<PathBuf> {
    let candidate = romfs.parent()?.join("exefs");
    candidate.is_dir().then_some(candidate)
}

pub fn title_id_from_folders(romfs: &Path) -> Option<u64> {
    romfs.ancestors().find_map(|folder| {
        let name = folder.file_name()?.to_str()?;
        let title_id = u64::from_str_radix(name, 16).ok()?;
        Game::from_title_id(title_id).map(|_| title_id)
    })
}

pub fn detect(romfs: &Path) -> Option<Detection> {
    detect_with_exefs(romfs, sibling_exefs(romfs).as_deref())
}

pub fn detect_with_exefs(romfs: &Path, exefs: Option<&Path>) -> Option<Detection> {
    if !looks_like_swsh_romfs(romfs) {
        return None;
    }
    let version = data_version(romfs)?;
    let title_id = exefs
        .and_then(|exefs| std::fs::read(exefs.join("main.npdm")).ok())
        .and_then(|npdm| read_title_id(&npdm).ok())
        .or_else(|| title_id_from_folders(romfs));
    let build_id = exefs
        .and_then(|exefs| std::fs::read(exefs.join("main")).ok())
        .and_then(|nso| read_build_id(&nso).ok());
    Some(Detection {
        game: title_id.and_then(Game::from_title_id),
        version,
        title_id,
        build_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn npdm(title_id: u64, with_aci0: bool) -> Vec<u8> {
        let mut data = vec![0u8; 0x500];
        data[..4].copy_from_slice(NPDM_MAGIC);
        if with_aci0 {
            data[ACI0_OFFSET_FIELD..ACI0_OFFSET_FIELD + 4].copy_from_slice(&0x450u32.to_le_bytes());
            data[0x450..0x454].copy_from_slice(ACI0_MAGIC);
            data[0x460..0x468].copy_from_slice(&title_id.to_le_bytes());
        } else {
            data[LEGACY_TITLE_ID_OFFSET..LEGACY_TITLE_ID_OFFSET + 8]
                .copy_from_slice(&title_id.to_le_bytes());
        }
        data
    }

    #[test]
    fn title_ids_identify_games() {
        let shield = read_title_id(&npdm(Game::Shield.title_id(), true)).expect("title");
        assert_eq!(Game::from_title_id(shield), Some(Game::Shield));
        let sword = read_title_id(&npdm(Game::Sword.title_id(), false)).expect("title");
        assert_eq!(Game::from_title_id(sword), Some(Game::Sword));
        assert_eq!(Game::Shield.title_id_text(), "01008DB008C2C000");
        assert_eq!(Game::from_title_id(1), None);
        assert!(read_title_id(&[0; 8]).is_err());
        assert_eq!(
            title_id_from_folders(Path::new("/dumps/01008DB008C2C000/romfs")),
            Some(Game::Shield.title_id())
        );
        assert_eq!(title_id_from_folders(Path::new("/dumps/romfs")), None);
    }

    #[test]
    fn build_ids_come_from_the_nso_header() {
        let mut nso = vec![0u8; 0x100];
        nso[..4].copy_from_slice(NSO_MAGIC);
        nso[0x40] = 0xB5;
        nso[0x41] = 0x6C;
        let build_id = read_build_id(&nso).expect("build id");
        assert_eq!(build_id_text(&build_id), "B56C0000000000000000000000000000");
        assert!(read_build_id(&[0; 0x60]).is_err());
    }

    #[test]
    fn games_name_their_encounter_files() {
        assert_eq!(Game::Sword.hidden_encounter_file(), "encount_k.bin");
        assert_eq!(Game::Shield.symbol_encounter_file(), "encount_symbol_t.bin");
    }

    #[test]
    fn non_romfs_directories_are_not_detected() {
        assert_eq!(detect(Path::new("/definitely/not/a/romfs")), None);
    }
}
