use std::collections::HashMap;

use randomon_data::constants::gb;
use randomon_settings::java_text::scanner_lines;

pub fn table_text(name: &str) -> Option<&'static str> {
    match name {
        "gameboy_jpn" => Some(include_str!("../assets/config/gameboy_jpn.tbl")),
        "green_translation" => Some(include_str!("../assets/config/green_translation.tbl")),
        "rby_english" => Some(include_str!("../assets/config/rby_english.tbl")),
        "rby_espita" => Some(include_str!("../assets/config/rby_espita.tbl")),
        "rby_freger" => Some(include_str!("../assets/config/rby_freger.tbl")),
        _ => None,
    }
}

pub fn patch_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "bwexp/rb_en_bwxp" => Some(include_bytes!("../assets/patches/bwexp/rb_en_bwxp.ips")),
        "bwexp/yellow_en_bwxp" => {
            Some(include_bytes!("../assets/patches/bwexp/yellow_en_bwxp.ips"))
        }
        "rb_en_critrate" => Some(include_bytes!("../assets/patches/rb_en_critrate.ips")),
        "rb_en_xaccnerf" => Some(include_bytes!("../assets/patches/rb_en_xaccnerf.ips")),
        "yellow_en_critrate" => Some(include_bytes!("../assets/patches/yellow_en_critrate.ips")),
        "yellow_en_xaccnerf" => Some(include_bytes!("../assets/patches/yellow_en_xaccnerf.ips")),
        _ => None,
    }
}

pub fn make_gb_pointer(offset: i32) -> i32 {
    if offset < gb::BANK_SIZE {
        offset
    } else {
        (offset % gb::BANK_SIZE) + gb::BANK_SIZE
    }
}

pub fn bank_of(offset: i32) -> i32 {
    offset / gb::BANK_SIZE
}

pub fn calculate_offset(bank: i32, pointer: i32) -> i32 {
    if pointer < gb::BANK_SIZE {
        pointer
    } else {
        (pointer % gb::BANK_SIZE) + bank * gb::BANK_SIZE
    }
}

pub fn read_word(data: &[u8], offset: i32) -> i32 {
    let offset = offset as usize;
    i32::from(data[offset]) + (i32::from(data[offset + 1]) << 8)
}

pub fn write_word(data: &mut [u8], offset: i32, value: i32) {
    let offset = offset as usize;
    data[offset] = (value % 0x100) as u8;
    data[offset + 1] = ((value / 0x100) % 0x100) as u8;
}

pub fn rom_signature_matches(rom: &[u8], signature: &str) -> bool {
    signature
        .bytes()
        .enumerate()
        .all(|(index, byte)| rom.get(gb::ROM_SIG_OFFSET + index) == Some(&byte))
}

#[derive(Clone, Debug)]
pub struct TextTables {
    tokens: Vec<Option<String>>,
    reverse: HashMap<Vec<u16>, u8>,
    longest_token: usize,
}

impl Default for TextTables {
    fn default() -> Self {
        Self {
            tokens: vec![None; 256],
            reverse: HashMap::new(),
            longest_token: 0,
        }
    }
}

impl TextTables {
    pub fn read_table(&mut self, name: &str) {
        let Some(text) = table_text(name) else {
            return;
        };
        for line in scanner_lines(text) {
            if randomon_settings::java_text::java_trim(line).is_empty() {
                continue;
            }
            let Some((code, token)) = line.split_once('=') else {
                continue;
            };
            let Ok(hexcode) = usize::from_str_radix(code, 16) else {
                continue;
            };
            let Some(slot) = self.tokens.get_mut(hexcode) else {
                continue;
            };
            if let Some(old) = slot.take() {
                let old_units: Vec<u16> = old.encode_utf16().collect();
                if self
                    .reverse
                    .get(&old_units)
                    .is_some_and(|&byte| i32::from(byte as i8) == hexcode as i32)
                {
                    self.reverse.remove(&old_units);
                }
            }
            *slot = Some(token.to_owned());
            let units: Vec<u16> = token.encode_utf16().collect();
            self.longest_token = self.longest_token.max(units.len());
            self.reverse.insert(units, hexcode as u8);
        }
    }

    pub fn read_string(
        &self,
        rom: &[u8],
        offset: i32,
        max_length: usize,
        text_engine_mode: bool,
    ) -> String {
        let mut text = String::new();
        for index in 0..max_length {
            let current = rom[offset as usize + index];
            match &self.tokens[current as usize] {
                Some(token) => {
                    text.push_str(token);
                    if text_engine_mode && (token == "\\r" || token == "\\e") {
                        break;
                    }
                }
                None => {
                    if current == gb::STRING_TERMINATOR {
                        break;
                    }
                    text.push_str(&format!("\\x{current:02X}"));
                }
            }
        }
        text
    }

    pub fn translate(&self, text: &str) -> Vec<u8> {
        let mut units: &[u16] = &text.encode_utf16().collect::<Vec<u16>>();
        let backslash = u16::from(b'\\');
        let lower_x = u16::from(b'x');
        let mut out = Vec::new();
        let longest = self.longest_token;
        while !units.is_empty() {
            if units[0] == backslash && units.get(1) == Some(&lower_x) && units.len() >= 4 {
                let hex = String::from_utf16_lossy(&units[2..4]);
                out.push(u8::from_str_radix(&hex, 16).unwrap_or(0));
                units = &units[4..];
                continue;
            }
            let mut skip = longest.saturating_sub(units.len());
            while skip != longest && !self.reverse.contains_key(&units[..longest - skip]) {
                skip += 1;
            }
            if skip == longest {
                units = &units[1..];
            } else {
                out.push(self.reverse[&units[..longest - skip]]);
                units = &units[longest - skip..];
            }
        }
        out
    }
}

pub fn length_of_string_at(rom: &[u8], offset: i32, text_engine_mode: bool) -> i32 {
    let mut length = 0usize;
    let base = offset as usize;
    let is_end = |byte: u8| {
        byte == gb::STRING_PRINTED_TEXT_END || byte == gb::STRING_PRINTED_TEXT_PROMPT_END
    };
    while rom[base + length] != gb::STRING_TERMINATOR
        && (!text_engine_mode || !is_end(rom[base + length]))
    {
        length += 1;
    }
    if text_engine_mode && is_end(rom[base + length]) {
        length += 1;
    }
    length as i32
}

pub fn apply_ips_patch(rom: &mut [u8], patch: &[u8]) -> Result<(), String> {
    if patch.len() < 8 || &patch[..5] != b"PATCH" {
        return Err("not a valid IPS file".to_owned());
    }
    let read_offset = |position: usize| {
        (usize::from(patch[position]) << 16)
            | (usize::from(patch[position + 1]) << 8)
            | usize::from(patch[position + 2])
    };
    let read_size =
        |position: usize| (usize::from(patch[position]) << 8) | usize::from(patch[position + 1]);
    let mut position = 5;
    while position + 2 < patch.len() {
        let write_offset = read_offset(position);
        if write_offset == 0x454f46 {
            return Ok(());
        }
        position += 3;
        if position + 1 >= patch.len() {
            return Err("abrupt ending to IPS file, entry cut off before size".to_owned());
        }
        let size = read_size(position);
        position += 2;
        if size == 0 {
            if position + 1 >= patch.len() {
                return Err("abrupt ending to IPS file, entry cut off before RLE size".to_owned());
            }
            let rle_size = read_size(position);
            if write_offset + rle_size > rom.len() {
                return Err("trying to patch data past the end of the ROM file".to_owned());
            }
            position += 2;
            if position >= patch.len() {
                return Err("abrupt ending to IPS file, entry cut off before RLE byte".to_owned());
            }
            let value = patch[position];
            position += 1;
            rom[write_offset..write_offset + rle_size].fill(value);
        } else {
            if position + size > patch.len() {
                return Err(
                    "abrupt ending to IPS file, entry cut off before end of data block".to_owned(),
                );
            }
            if write_offset + size > rom.len() {
                return Err("trying to patch data past the end of the ROM file".to_owned());
            }
            rom[write_offset..write_offset + size]
                .copy_from_slice(&patch[position..position + size]);
            position += size;
        }
    }
    Err("improperly terminated IPS file".to_owned())
}

fn kmp_table(needle: &[u8]) -> Vec<i32> {
    let mut table = vec![0i32; needle.len().max(2)];
    table[0] = -1;
    table[1] = 0;
    let mut position = 2;
    let mut candidate = 0usize;
    while position < needle.len() {
        if needle[position - 1] == needle[candidate] {
            table[position] = candidate as i32 + 1;
            position += 1;
            candidate += 1;
        } else if candidate > 0 {
            candidate = table[candidate] as usize;
        } else {
            table[position] = 0;
            position += 1;
        }
    }
    table
}

pub fn search(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let table = kmp_table(needle);
    let mut results = Vec::new();
    let mut match_start: i64 = 0;
    let mut character: i64 = 0;
    let end = haystack.len() as i64;
    while match_start + character < end {
        if needle[character as usize] == haystack[(character + match_start) as usize] {
            character += 1;
            if character as usize == needle.len() {
                results.push(match_start as usize);
                character = 0;
                match_start += needle.len() as i64;
            }
        } else {
            let fallback = i64::from(table[character as usize]);
            match_start = match_start + character - fallback;
            character = if fallback > -1 { fallback } else { 0 };
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn english_tables() -> TextTables {
        let mut tables = TextTables::default();
        tables.read_table("gameboy_jpn");
        tables.read_table("rby_english");
        tables
    }

    #[test]
    fn pointers_round_trip_through_banks() {
        assert_eq!(make_gb_pointer(0x3FFF), 0x3FFF);
        assert_eq!(make_gb_pointer(0x39DD1), 0x5DD1);
        assert_eq!(calculate_offset(bank_of(0x39DD1), 0x5DD1), 0x39DD1);
        assert_eq!(calculate_offset(0x0E, 0x1234), 0x1234);
    }

    #[test]
    fn words_are_little_endian() {
        let mut data = vec![0u8; 4];
        write_word(&mut data, 1, 0xBEEF);
        assert_eq!(data, vec![0, 0xEF, 0xBE, 0]);
        assert_eq!(read_word(&data, 1), 0xBEEF);
    }

    #[test]
    fn english_text_round_trips() {
        let tables = english_tables();
        let encoded = tables.translate("PIKACHU");
        assert_eq!(encoded, vec![0x8F, 0x88, 0x8A, 0x80, 0x82, 0x87, 0x94]);
        let mut rom = encoded.clone();
        rom.push(gb::STRING_TERMINATOR);
        assert_eq!(tables.read_string(&rom, 0, usize::MAX, false), "PIKACHU");
        assert_eq!(length_of_string_at(&rom, 0, false), 7);
    }

    #[test]
    fn unknown_bytes_use_hex_escapes() {
        let tables = english_tables();
        let rom = [0x00u8, gb::STRING_TERMINATOR];
        let text = tables.read_string(&rom, 0, usize::MAX, false);
        assert_eq!(tables.translate(&text), vec![0x00]);
    }

    #[test]
    fn search_reports_non_overlapping_matches() {
        let haystack = [1u8, 2, 1, 2, 1, 2, 3];
        assert_eq!(search(&haystack, &[1, 2, 1]), vec![0]);
        assert_eq!(search(&haystack, &[2, 3]), vec![5]);
        assert!(search(&haystack, &[3, 3]).is_empty());
    }

    #[test]
    fn ips_patches_apply_records_and_rle() {
        let patch = [
            b'P', b'A', b'T', b'C', b'H', 0, 0, 1, 0, 2, 0xAA, 0xBB, 0, 0, 4, 0, 0, 0, 3, 0xCC,
            b'E', b'O', b'F',
        ];
        let mut rom = vec![0u8; 8];
        apply_ips_patch(&mut rom, &patch).expect("valid patch");
        assert_eq!(rom, vec![0, 0xAA, 0xBB, 0, 0xCC, 0xCC, 0xCC, 0]);
        assert!(apply_ips_patch(&mut rom, b"NOPE....").is_err());
    }

    #[test]
    fn every_offsets_patch_is_embedded() {
        for entry in crate::rom_entry::rom_entries() {
            for patch in entry.tweak_files.values() {
                assert!(patch_bytes(patch).is_some(), "missing {patch}");
            }
            if let Some(table) = &entry.extra_table_file {
                assert!(table_text(table).is_some(), "missing {table}");
            }
        }
    }
}
