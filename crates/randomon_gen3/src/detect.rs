use crate::constants as c;
use crate::rom::{find_in, find_multiple_in};
use crate::rom_entry::{RomEntry, rom_entries};

fn ascii_matches_at(rom: &[u8], start: usize, text: &str) -> bool {
    text.bytes()
        .enumerate()
        .all(|(index, byte)| rom.get(start + index) == Some(&byte))
}

pub fn is_unofficial_emerald(rom: &[u8]) -> bool {
    ascii_matches_at(rom, c::ROM_NAME_OFFSET, c::UNOFFICIAL_EMERALD_ROM_NAME)
}

pub fn normalize_unofficial_emerald(rom: &mut [u8]) {
    if !is_unofficial_emerald(rom) || rom.len() <= c::HEADER_CHECKSUM_OFFSET {
        return;
    }
    let code = c::UNOFFICIAL_EMERALD_ROM_CODE.as_bytes();
    rom[c::ROM_CODE_OFFSET..c::ROM_CODE_OFFSET + code.len()].copy_from_slice(code);
    rom[c::HEADER_CHECKSUM_OFFSET] = c::UNOFFICIAL_EMERALD_HEADER_CHECKSUM;
}

fn effective_rom_code(rom: &[u8]) -> Option<[u8; 4]> {
    if is_unofficial_emerald(rom) {
        let mut code = [0u8; 4];
        code.copy_from_slice(c::UNOFFICIAL_EMERALD_ROM_CODE.as_bytes());
        return Some(code);
    }
    let slice = rom.get(c::ROM_CODE_OFFSET..c::ROM_CODE_OFFSET + 4)?;
    let mut code = [0u8; 4];
    code.copy_from_slice(slice);
    Some(code)
}

fn code_matches(code: &[u8; 4], expected: &str) -> bool {
    let expected = expected.as_bytes();
    expected.len() <= code.len() && &code[..expected.len()] == expected
}

pub fn matching_entry(rom: &[u8]) -> Option<&'static RomEntry> {
    let code = effective_rom_code(rom)?;
    let version = i32::from(*rom.get(c::ROM_VERSION_OFFSET)?);
    rom_entries()
        .iter()
        .find(|entry| code_matches(&code, &entry.rom_code) && entry.version == version)
}

pub fn has_supported_size(size: usize) -> bool {
    matches!(size, c::SIZE_8M | c::SIZE_16M | c::SIZE_32M)
}

fn has_engine_signatures(rom: &[u8]) -> bool {
    find_in(rom, c::WILD_POKEMON_POINTER_PREFIX) != -1
        && find_in(rom, c::MAP_BANKS_POINTER_PREFIX) != -1
        && find_multiple_in(rom, c::POKEDEX_ORDER_POINTER_PREFIX).len() == 3
}

pub fn detect_rom_inner(rom: &[u8], rom_size: usize) -> bool {
    has_supported_size(rom_size) && has_engine_signatures(rom) && matching_entry(rom).is_some()
}

pub fn detect_rom(rom: &[u8]) -> bool {
    detect_rom_inner(rom, rom.len())
}

pub fn detect_loadable(rom: &[u8]) -> bool {
    let prefix = &rom[..rom.len().min(c::DETECTION_PREFIX_BYTES)];
    detect_rom_inner(prefix, rom.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(code: &str, version: u8) -> Vec<u8> {
        let mut rom = vec![0u8; 0x100];
        rom[c::ROM_CODE_OFFSET..c::ROM_CODE_OFFSET + 4].copy_from_slice(code.as_bytes());
        rom[c::ROM_VERSION_OFFSET] = version;
        rom
    }

    #[test]
    fn entries_match_on_code_and_version() {
        assert_eq!(
            matching_entry(&header("AXVE", 1)).map(|e| e.name.as_str()),
            Some("Ruby (E)")
        );
        assert_eq!(
            matching_entry(&header("BPRE", 1)).map(|e| e.name.as_str()),
            Some("Fire Red (U) 1.1")
        );
        assert!(matching_entry(&header("BPRE", 9)).is_none());
        assert!(matching_entry(&header("ZZZZ", 0)).is_none());
        assert!(matching_entry(&[0u8; 8]).is_none());
    }

    #[test]
    fn unofficial_emerald_is_renamed_before_matching() {
        let mut rom = header("BPEJ", 0);
        rom[c::ROM_NAME_OFFSET..c::ROM_NAME_OFFSET + 11].copy_from_slice(b"YJencrypted");
        assert_eq!(
            matching_entry(&rom).map(|e| e.name.as_str()),
            Some("Emerald (T-Eng)")
        );
        normalize_unofficial_emerald(&mut rom);
        assert_eq!(&rom[c::ROM_CODE_OFFSET..c::ROM_CODE_OFFSET + 4], b"BPET");
        assert_eq!(rom[c::HEADER_CHECKSUM_OFFSET], 0x66);
    }

    #[test]
    fn detection_requires_size_and_engine_signatures() {
        assert!(!detect_rom(&header("AXVE", 0)));
        let mut rom = vec![0u8; c::SIZE_8M];
        rom[..0x100].copy_from_slice(&header("AXVE", 0));
        assert!(!detect_rom(&rom));
        assert!(has_supported_size(c::SIZE_32M));
        assert!(!has_supported_size(c::SIZE_32M + 1));
    }
}
