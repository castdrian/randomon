use std::collections::HashMap;

use randomon_settings::java_text::{java_trim, scanner_lines};

use crate::rom::GbaRom;

pub const TEXT_TERMINATOR: u8 = 0xFF;
pub const TEXT_VARIABLE: u8 = 0xFD;
const LONGEST_TOKEN: usize = 4;
const BACKSLASH: u16 = b'\\' as u16;
const LOWER_X: u16 = b'x' as u16;
const LOWER_V: u16 = b'v' as u16;

#[derive(Clone, Debug)]
pub struct TextTable {
    tokens: Vec<Option<String>>,
    reverse: HashMap<Vec<u16>, u8>,
}

impl Default for TextTable {
    fn default() -> Self {
        Self {
            tokens: vec![None; 256],
            reverse: HashMap::new(),
        }
    }
}

fn parse_hex_pair(units: &[u16]) -> Option<u8> {
    let text = String::from_utf16(units).ok()?;
    u8::from_str_radix(&text, 16).ok()
}

impl TextTable {
    pub fn from_text(text: &str) -> Self {
        let mut table = Self::default();
        table.read(text);
        table
    }

    pub fn read(&mut self, text: &str) {
        for line in scanner_lines(text) {
            if java_trim(line).is_empty() {
                continue;
            }
            let Some((code, token)) = line.split_once('=') else {
                continue;
            };
            let Ok(code) = usize::from_str_radix(code, 16) else {
                continue;
            };
            if let Some(slot) = self.tokens.get_mut(code) {
                *slot = Some(token.to_owned());
                self.reverse
                    .insert(token.encode_utf16().collect(), code as u8);
            }
        }
    }

    pub fn token(&self, code: u8) -> Option<&str> {
        self.tokens[code as usize].as_deref()
    }

    pub fn read_string(&self, rom: &GbaRom, offset: i32, max_length: usize) -> String {
        let mut text = String::new();
        let mut index = 0usize;
        while index < max_length {
            let position = offset as i64 + index as i64;
            if position < 0 || position >= i64::from(rom.len()) {
                break;
            }
            let current = rom.byte(position as i32);
            match self.token(current) {
                Some(token) => text.push_str(token),
                None if current == TEXT_TERMINATOR => break,
                None if current == TEXT_VARIABLE => {
                    let next = rom.byte(position as i32 + 1);
                    text.push_str(&format!("\\v{next:02X}"));
                    index += 1;
                }
                None => text.push_str(&format!("\\x{current:02X}")),
            }
            index += 1;
        }
        text
    }

    fn escaped_byte(&self, units: &[u16]) -> Option<(Vec<u8>, usize)> {
        if units.len() < 4 || units[0] != BACKSLASH {
            return None;
        }
        let value = parse_hex_pair(&units[2..4])?;
        match units[1] {
            LOWER_X => Some((vec![value], 4)),
            LOWER_V => Some((vec![TEXT_VARIABLE, value], 4)),
            _ => None,
        }
    }

    fn longest_token_at(&self, units: &[u16]) -> Option<(u8, usize)> {
        let longest = LONGEST_TOKEN.min(units.len());
        (1..=longest).rev().find_map(|length| {
            self.reverse
                .get(&units[..length])
                .map(|&code| (code, length))
        })
    }

    pub fn translate(&self, text: &str) -> Vec<u8> {
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut remaining: &[u16] = &units;
        let mut out = Vec::new();
        while !remaining.is_empty() {
            if let Some((bytes, consumed)) = self.escaped_byte(remaining) {
                out.extend(bytes);
                remaining = &remaining[consumed..];
            } else if let Some((code, consumed)) = self.longest_token_at(remaining) {
                out.push(code);
                remaining = &remaining[consumed..];
            } else {
                remaining = &remaining[1..];
            }
        }
        out
    }

    pub fn length(&self, text: &str) -> i32 {
        self.translate(text).len() as i32
    }
}

pub fn length_of_string_at(rom: &GbaRom, offset: i32) -> i32 {
    let mut length = 0;
    while rom.contains(offset.saturating_add(length))
        && rom.byte(offset + length) != TEXT_TERMINATOR
    {
        length += 1;
    }
    length
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::table_text;

    fn english() -> TextTable {
        TextTable::from_text(table_text("gba_english").expect("table exists"))
    }

    #[test]
    fn english_names_round_trip() {
        let table = english();
        let encoded = table.translate("TREECKO");
        assert_eq!(encoded, vec![0xCE, 0xCC, 0xBF, 0xBF, 0xBD, 0xC5, 0xC9]);
        let mut bytes = encoded.clone();
        bytes.push(TEXT_TERMINATOR);
        let rom = GbaRom::new(bytes);
        assert_eq!(table.read_string(&rom, 0, usize::MAX), "TREECKO");
        assert_eq!(length_of_string_at(&rom, 0), 7);
    }

    #[test]
    fn multi_character_tokens_win_over_single_characters() {
        let table = english();
        assert_eq!(table.translate("[Lv]5"), vec![0x34, 0xA6]);
        assert_eq!(table.translate("A\\nB"), vec![0xBB, 0xFE, 0xBC]);
        assert_eq!(
            table.translate("POKéMON"),
            vec![0xCA, 0xC9, 0xC5, 0x1B, 0xC7, 0xC9, 0xC8]
        );
    }

    #[test]
    fn escapes_and_variables_round_trip() {
        let table = english();
        let rom = GbaRom::new(vec![0x09, 0x0A, 0xFD, 0x01, 0xBB, 0xFF]);
        let text = table.read_string(&rom, 0, usize::MAX);
        assert_eq!(text, "Ì\\x0A\\v01A");
        assert_eq!(table.translate(&text), vec![0x09, 0x0A, 0xFD, 0x01, 0xBB]);
    }

    #[test]
    fn malformed_escapes_never_panic() {
        let table = english();
        assert_eq!(table.translate("\\"), Vec::<u8>::new());
        assert_eq!(table.translate("\\x"), vec![0xEC]);
        assert_eq!(table.translate("\\xZZ"), vec![0xEC, 0xD4, 0xD4]);
    }

    #[test]
    fn fixed_length_reads_stop_at_the_rom_end() {
        let table = english();
        let rom = GbaRom::new(vec![0xBB, 0xBC]);
        assert_eq!(table.read_string(&rom, 0, usize::MAX), "AB");
        assert_eq!(table.read_string(&rom, 1, 10), "B");
        assert_eq!(length_of_string_at(&rom, 0), 2);
    }
}
