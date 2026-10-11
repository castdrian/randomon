use std::collections::HashMap;

use randomon_data::constants::gb;
use randomon_settings::java_text::{java_trim, scanner_lines};

pub const GAMEBOY_JPN_TABLE: &str = include_str!("../assets/gameboy_jpn.tbl");

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
    pub fn read_table(&mut self, text: &str) {
        for line in scanner_lines(text) {
            if java_trim(line).is_empty() {
                continue;
            }
            let Some((code, token)) = line.split_once('=') else {
                continue;
            };
            let Ok(hexcode) = usize::from_str_radix(code, 16) else {
                continue;
            };
            self.insert_token(hexcode, token);
        }
    }

    fn insert_token(&mut self, hexcode: usize, token: &str) {
        let Some(slot) = self.tokens.get_mut(hexcode) else {
            return;
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

#[cfg(test)]
mod tests {
    use super::*;

    const LATIN_TABLE: &str = "80=A\n81=B\n82=C\n87=H\n88=I\n8A=K\n8F=P\n94=U\n7F= \n";

    fn tables() -> TextTables {
        let mut tables = TextTables::default();
        tables.read_table(GAMEBOY_JPN_TABLE);
        tables.read_table(LATIN_TABLE);
        tables
    }

    #[test]
    fn text_round_trips() {
        let tables = tables();
        let encoded = tables.translate("PIKACHU");
        assert_eq!(encoded, vec![0x8F, 0x88, 0x8A, 0x80, 0x82, 0x87, 0x94]);
        let mut rom = encoded.clone();
        rom.push(gb::STRING_TERMINATOR);
        assert_eq!(tables.read_string(&rom, 0, usize::MAX, false), "PIKACHU");
        assert_eq!(length_of_string_at(&rom, 0, false), 7);
    }

    #[test]
    fn unknown_bytes_use_hex_escapes() {
        let tables = tables();
        let rom = [0x00u8, gb::STRING_TERMINATOR];
        let text = tables.read_string(&rom, 0, usize::MAX, false);
        assert_eq!(tables.translate(&text), vec![0x00]);
    }
}
