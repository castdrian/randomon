use std::collections::HashMap;
use std::fmt::Write;
use std::sync::OnceLock;

use crate::bytes::{read_long, read_word, write_long, write_word};
use crate::error::{FormatError, Result};

const GENERATION4_TBL: &str = include_str!("../../assets/text/Generation4.tbl");

pub const WRITE_KEY: i32 = 0xD00E;

const HEADER_SIZE: i32 = 4;
const POINTER_ENTRY_SIZE: i32 = 8;
const POINTER_KEY_MULTIPLIER: i32 = 0x2FD;
const TEXT_KEY_MULTIPLIER: i32 = 0x91BD3;
const TEXT_KEY_STEP: i32 = 0x493D;
const SIXTEEN_BITS: i32 = 0xFFFF;

const COMPRESSED_MARKER: i32 = 0xF100;
const VARIABLE_MARKER: i32 = 0xFFFE;
const STRING_TERMINATOR: i32 = 0xFFFF;
const COMPRESSED_TERMINATOR: i32 = 0x1FF;
const NEWLINE_CODE: i32 = 0xE000;
const PARAGRAPH_CODE: i32 = 0x25BC;
const SCROLL_CODE: i32 = 0x25BD;
const AND_CODE: i32 = 0x1C2;

const BITS_PER_CODE: usize = 9;
const BITS_PER_WORD: usize = 15;
const CODE_MASK: i32 = 0x1FF;
const CODES_PER_COMPRESSED_BLOCK: usize = 5;

const LONGEST_TOKEN: usize = 6;
const ESCAPE_LENGTH: usize = 6;

const BACKSLASH: u16 = b'\\' as u16;
const LOWER_X: u16 = b'x' as u16;
const LOWER_V: u16 = b'v' as u16;
const LOWER_Z: u16 = b'z' as u16;
const LOWER_N: u16 = b'n' as u16;
const LOWER_P: u16 = b'p' as u16;
const LOWER_L: u16 = b'l' as u16;
const AND_WORD: [u16; 3] = [b'a' as u16, b'n' as u16, b'd' as u16];

const DATA_CONTEXT: &str = "PokeTextData";
const ENCODER_CONTEXT: &str = "TextToPoke";
const TABLE_CONTEXT: &str = "Generation4.tbl";

const DECIMAL_DIGIT_ZEROS: [u16; 37] = [
    0x0030, 0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66, 0x0BE6, 0x0C66, 0x0CE6,
    0x0D66, 0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80,
    0x1A90, 0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0,
    0xFF10,
];
const LETTER_DIGIT_RANGES: [(u16, u16); 4] = [
    (b'A' as u16, b'F' as u16),
    (b'a' as u16, b'f' as u16),
    (0xFF21, 0xFF26),
    (0xFF41, 0xFF46),
];
const FIRST_LETTER_DIGIT: i32 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedText {
    pub strings: Vec<String>,
    pub compressed: bool,
}

pub fn read_strings(data: &[u8]) -> Result<DecodedText> {
    let mut text = PokeTextData::new(data);
    text.decrypt()?;
    Ok(DecodedText {
        compressed: text.compress_flag(),
        strings: text.strings,
    })
}

pub fn write_strings<S: AsRef<str>>(strings: &[S], compressed: bool) -> Result<Vec<u8>> {
    encrypt_for_rom(make_file(strings, compressed)?)
}

pub fn write_strings_utf16<S: AsRef<[u16]>>(strings: &[S], compressed: bool) -> Result<Vec<u8>> {
    encrypt_for_rom(make_file_utf16(strings, compressed)?)
}

fn encrypt_for_rom(raw: Vec<u8>) -> Result<Vec<u8>> {
    let mut text = PokeTextData::from_vec(raw);
    text.set_key(WRITE_KEY)?;
    text.encrypt()?;
    Ok(text.into_bytes())
}

pub fn make_file<S: AsRef<str>>(strings: &[S], compressed: bool) -> Result<Vec<u8>> {
    let units: Vec<Vec<u16>> = strings
        .iter()
        .map(|text| text.as_ref().encode_utf16().collect())
        .collect();
    make_file_utf16(&units, compressed)
}

pub fn make_file_utf16<S: AsRef<[u16]>>(strings: &[S], compressed: bool) -> Result<Vec<u8>> {
    let table = generation4_table()?;
    let count = strings.len() as i32;
    let mut base = count
        .wrapping_mul(POINTER_ENTRY_SIZE)
        .wrapping_add(HEADER_SIZE);
    let mut pointers = Vec::with_capacity(strings.len());
    let mut bodies = Vec::with_capacity(strings.len());
    for text in strings {
        let codes = encode_with(table, text.as_ref(), compressed)?;
        let length = codes.len() as i32;
        pointers.push(PointerEntry {
            pointer: base,
            length,
        });
        base = base.wrapping_add(length.wrapping_mul(2));
        bodies.push(codes);
    }
    let mut file = Vec::new();
    push_word(&mut file, count);
    push_word(&mut file, 0);
    for entry in &pointers {
        file.extend_from_slice(&entry.pointer.to_le_bytes());
        file.extend_from_slice(&entry.length.to_le_bytes());
    }
    for code in bodies.iter().flatten() {
        push_word(&mut file, *code);
    }
    Ok(file)
}

fn push_word(file: &mut Vec<u8>, value: i32) {
    file.extend_from_slice(&(value as u16).to_le_bytes());
}

pub fn encode_string(text: &str, compressed: bool) -> Result<Vec<i32>> {
    let units: Vec<u16> = text.encode_utf16().collect();
    encode_utf16(&units, compressed)
}

pub fn encode_utf16(text: &[u16], compressed: bool) -> Result<Vec<i32>> {
    encode_with(generation4_table()?, text, compressed)
}

fn encode_with(table: &CharacterTable, text: &[u16], compressed: bool) -> Result<Vec<i32>> {
    let mut codes = Vec::new();
    let mut rest = text;
    while let Some(&first) = rest.first() {
        rest = if first == BACKSLASH {
            encode_escape(rest, &mut codes)?
        } else {
            encode_token(table, rest, &mut codes)
        };
    }
    if compressed {
        codes = compress_codes(codes);
    }
    codes.push(STRING_TERMINATOR);
    Ok(codes)
}

fn encode_escape<'a>(text: &'a [u16], codes: &mut Vec<i32>) -> Result<&'a [u16]> {
    match unit_at(text, 1)? {
        LOWER_X => {
            codes.push(escape_value(text)?);
            Ok(&text[ESCAPE_LENGTH..])
        }
        LOWER_V => {
            codes.push(VARIABLE_MARKER);
            codes.push(escape_value(text)?);
            Ok(&text[ESCAPE_LENGTH..])
        }
        LOWER_Z => encode_variable_arguments(text, codes),
        LOWER_N => Ok(push_code(codes, NEWLINE_CODE, text, 2)),
        LOWER_P => Ok(push_code(codes, PARAGRAPH_CODE, text, 2)),
        LOWER_L => Ok(push_code(codes, SCROLL_CODE, text, 2)),
        _ if substring(text, 1, 4)? == AND_WORD => Ok(push_code(codes, AND_CODE, text, 4)),
        _ => Ok(&text[2..]),
    }
}

fn push_code<'a>(codes: &mut Vec<i32>, code: i32, text: &'a [u16], consumed: usize) -> &'a [u16] {
    codes.push(code);
    &text[consumed..]
}

fn encode_variable_arguments<'a>(text: &'a [u16], codes: &mut Vec<i32>) -> Result<&'a [u16]> {
    let mut arguments = Vec::new();
    let mut rest = text;
    while let Some(&first) = rest.first() {
        if first != BACKSLASH || unit_at(rest, 1)? != LOWER_Z {
            break;
        }
        arguments.push(escape_value(rest)?);
        rest = &rest[ESCAPE_LENGTH..];
    }
    codes.push(arguments.len() as i32);
    codes.extend(arguments);
    Ok(rest)
}

fn encode_token<'a>(table: &CharacterTable, text: &'a [u16], codes: &mut Vec<i32>) -> &'a [u16] {
    let longest_match = (1..=text.len().min(LONGEST_TOKEN))
        .rev()
        .find_map(|length| table.code_for(&text[..length]).map(|code| (length, code)));
    match longest_match {
        Some((length, code)) => push_code(codes, code, text, length),
        None => &text[1..],
    }
}

fn escape_value(text: &[u16]) -> Result<i32> {
    java_parse_hex(substring(text, 2, ESCAPE_LENGTH)?)
}

fn unit_at(text: &[u16], index: usize) -> Result<u16> {
    text.get(index)
        .copied()
        .ok_or(FormatError::out_of_bounds(ENCODER_CONTEXT))
}

fn substring(text: &[u16], start: usize, end: usize) -> Result<&[u16]> {
    text.get(start..end)
        .ok_or(FormatError::out_of_bounds(ENCODER_CONTEXT))
}

fn compress_codes(mut codes: Vec<i32>) -> Vec<i32> {
    if !codes.len().is_multiple_of(CODES_PER_COMPRESSED_BLOCK) || codes.is_empty() {
        codes.push(COMPRESSED_TERMINATOR);
    }
    let bits = codes
        .iter()
        .flat_map(|&code| (0..BITS_PER_CODE).map(move |bit| (code >> bit) & 1));
    let mut packed = vec![COMPRESSED_MARKER];
    let mut word = 0;
    for (index, bit) in bits.enumerate() {
        if index % BITS_PER_WORD == 0 && index != 0 {
            packed.push(word);
            word = 0;
        }
        word |= bit << (index % BITS_PER_WORD);
    }
    packed.push(word);
    packed
}

fn java_parse_hex(units: &[u16]) -> Result<i32> {
    let invalid = || {
        FormatError::invalid(
            "Integer.parseInt",
            format!("For input string: \"{}\"", String::from_utf16_lossy(units)),
        )
    };
    let (&first, rest) = units.split_first().ok_or_else(invalid)?;
    let (negative, digits) = match first {
        0x2D => (true, rest),
        0x2B => (false, rest),
        _ if first < u16::from(b'0') => return Err(invalid()),
        _ => (false, units),
    };
    if digits.is_empty() {
        return Err(invalid());
    }
    let limit = if negative {
        i64::from(i32::MIN)
    } else {
        -i64::from(i32::MAX)
    };
    let mut accumulated: i64 = 0;
    for &unit in digits {
        let digit = java_hex_digit(unit).ok_or_else(invalid)?;
        accumulated = accumulated * 16 - i64::from(digit);
        if accumulated < limit {
            return Err(invalid());
        }
    }
    let value = if negative { accumulated } else { -accumulated };
    i32::try_from(value).map_err(|_| invalid())
}

fn java_hex_digit(unit: u16) -> Option<i32> {
    decimal_digit(unit).or_else(|| letter_digit(unit))
}

fn decimal_digit(unit: u16) -> Option<i32> {
    DECIMAL_DIGIT_ZEROS
        .iter()
        .find(|&&zero| (zero..zero + 10).contains(&unit))
        .map(|&zero| i32::from(unit - zero))
}

fn letter_digit(unit: u16) -> Option<i32> {
    LETTER_DIGIT_RANGES
        .iter()
        .find(|&&(first, last)| (first..=last).contains(&unit))
        .map(|&(first, _)| FIRST_LETTER_DIGIT + i32::from(unit - first))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PointerEntry {
    pointer: i32,
    length: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokeTextData {
    data: Vec<u8>,
    strings: Vec<String>,
    compress_flag: bool,
}

impl PokeTextData {
    pub fn new(data: &[u8]) -> Self {
        Self::from_vec(data.to_vec())
    }

    pub fn from_vec(data: Vec<u8>) -> Self {
        Self {
            data,
            strings: Vec::new(),
            compress_flag: false,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    pub fn compress_flag(&self) -> bool {
        self.compress_flag
    }

    pub fn key(&self) -> Result<i32> {
        self.read16(2)
    }

    pub fn set_key(&mut self, key: i32) -> Result<()> {
        self.write16(key, 2)
    }

    pub fn decrypt(&mut self) -> Result<()> {
        let table = generation4_table()?;
        self.crypt_pointers(self.read16(0)?, self.read16(2)?, HEADER_SIZE)?;
        let pointers = self.pointer_list(self.read16(0)?, HEADER_SIZE)?;
        self.strings = Vec::new();
        let count = self.read16(0)?;
        for (id, entry) in (1..=count).zip(pointers) {
            self.crypt_text(entry.length, id, entry.pointer)?;
            let text = self.make_string(table, entry.length, entry.pointer)?;
            self.strings.push(text);
        }
        Ok(())
    }

    pub fn encrypt(&mut self) -> Result<()> {
        let pointers = self.pointer_list(self.read16(0)?, HEADER_SIZE)?;
        let count = self.read16(0)?;
        for (id, entry) in (1..=count).zip(pointers) {
            self.crypt_text(entry.length, id, entry.pointer)?;
        }
        self.crypt_pointers(self.read16(0)?, self.read16(2)?, HEADER_SIZE)
    }

    fn crypt_pointers(&mut self, count: i32, key: i32, start: i32) -> Result<()> {
        let key = key.wrapping_mul(POINTER_KEY_MULTIPLIER) & SIXTEEN_BITS;
        let mut offset = start;
        for index in 0..count {
            let entry_key = key.wrapping_mul(index + 1) & SIXTEEN_BITS;
            let doubled_key = entry_key | (entry_key << 16);
            self.write32(self.read32(offset)? ^ doubled_key, offset)?;
            let length_offset = offset.wrapping_add(4);
            self.write32(self.read32(length_offset)? ^ doubled_key, length_offset)?;
            offset = offset.wrapping_add(POINTER_ENTRY_SIZE);
        }
        Ok(())
    }

    fn pointer_list(&self, count: i32, start: i32) -> Result<Vec<PointerEntry>> {
        (0..count)
            .map(|index| {
                let offset = start.wrapping_add(index.wrapping_mul(POINTER_ENTRY_SIZE));
                Ok(PointerEntry {
                    pointer: self.read32(offset)?,
                    length: self.read32(offset.wrapping_add(4))?,
                })
            })
            .collect()
    }

    fn crypt_text(&mut self, count: i32, id: i32, start: i32) -> Result<()> {
        let mut key = TEXT_KEY_MULTIPLIER.wrapping_mul(id) & SIXTEEN_BITS;
        let mut offset = start;
        for _ in 0..count {
            self.write16(self.read16(offset)? ^ key, offset)?;
            key = (key + TEXT_KEY_STEP) & SIXTEEN_BITS;
            offset = offset.wrapping_add(2);
        }
        Ok(())
    }

    fn make_string(&mut self, table: &CharacterTable, count: i32, start: i32) -> Result<String> {
        let mut codes = Vec::new();
        let mut offset = start;
        for _ in 0..count {
            codes.push(self.read16(offset)?);
            offset = offset.wrapping_add(2);
        }
        if code_at(&codes, 0)? == COMPRESSED_MARKER {
            self.compress_flag = true;
            codes = decompress_codes(&codes)?;
        }
        render_codes(table, &codes)
    }

    fn checked_range(&self, offset: i32, width: usize) -> Result<usize> {
        usize::try_from(offset)
            .ok()
            .filter(|&start| start + width <= self.data.len())
            .ok_or(FormatError::out_of_bounds(DATA_CONTEXT))
    }

    fn read16(&self, offset: i32) -> Result<i32> {
        let start = self.checked_range(offset, 2)?;
        Ok(read_word(&self.data, start))
    }

    fn write16(&mut self, value: i32, offset: i32) -> Result<()> {
        let start = self.checked_range(offset, 2)?;
        write_word(&mut self.data, start, value);
        Ok(())
    }

    fn read32(&self, offset: i32) -> Result<i32> {
        let start = self.checked_range(offset, 4)?;
        Ok(read_long(&self.data, start))
    }

    fn write32(&mut self, value: i32, offset: i32) -> Result<()> {
        let start = self.checked_range(offset, 4)?;
        write_long(&mut self.data, start, value);
        Ok(())
    }
}

fn code_at(codes: &[i32], index: usize) -> Result<i32> {
    codes
        .get(index)
        .copied()
        .ok_or(FormatError::out_of_bounds(DATA_CONTEXT))
}

fn decompress_codes(words: &[i32]) -> Result<Vec<i32>> {
    let mut codes = Vec::new();
    let mut index = 1;
    let mut shift: i32 = 0;
    let mut carried = 0;
    loop {
        let word = code_at(words, index)?;
        if shift >= BITS_PER_WORD as i32 {
            shift -= BITS_PER_WORD as i32;
            if shift > 0 {
                let code = carried | ((word << (BITS_PER_CODE as i32 - shift)) & CODE_MASK);
                if code == COMPRESSED_TERMINATOR {
                    break;
                }
                codes.push(code);
            }
        } else {
            let code = (word >> shift) & CODE_MASK;
            if code == COMPRESSED_TERMINATOR {
                break;
            }
            codes.push(code);
            shift += BITS_PER_CODE as i32;
            if shift < BITS_PER_WORD as i32 {
                carried = (word >> shift) & CODE_MASK;
                shift += BITS_PER_CODE as i32;
            }
            index += 1;
        }
    }
    Ok(codes)
}

fn render_codes(table: &CharacterTable, codes: &[i32]) -> Result<String> {
    let mut text = String::new();
    let mut index = 0;
    for _ in 0..codes.len() {
        let current = code_at(codes, index)?;
        if let Some(entry) = table.text_for(current) {
            text.push_str(entry);
        } else if current == VARIABLE_MARKER {
            index = render_variable(codes, index, &mut text)?;
        } else if current == STRING_TERMINATOR {
            break;
        } else {
            push_escape(&mut text, 'x', current);
        }
        index += 1;
    }
    Ok(text)
}

fn render_variable(codes: &[i32], marker_index: usize, text: &mut String) -> Result<usize> {
    let mut index = marker_index + 1;
    push_escape(text, 'v', code_at(codes, index)?);
    index += 1;
    let argument_count = code_at(codes, index)?;
    if argument_count == 0 {
        push_escape(text, 'x', 0);
    }
    for _ in 0..argument_count {
        index += 1;
        push_escape(text, 'z', code_at(codes, index)?);
    }
    Ok(index)
}

fn push_escape(text: &mut String, kind: char, value: i32) {
    let _ = write!(text, "\\{kind}{value:04X}");
}

#[derive(Debug, Clone)]
pub struct CharacterTable {
    by_code: Vec<Option<Box<str>>>,
    by_text: HashMap<Vec<u16>, i32>,
}

impl CharacterTable {
    pub fn parse(source: &str) -> Result<Self> {
        let mut table = CharacterTable {
            by_code: vec![None; 0x10000],
            by_text: HashMap::new(),
        };
        for line in scanner_lines(source) {
            if !java_trim(line).is_empty() {
                table.add_line(line)?;
            }
        }
        Ok(table)
    }

    fn add_line(&mut self, line: &str) -> Result<()> {
        let (key, value) = line
            .split_once('=')
            .ok_or(FormatError::out_of_bounds(TABLE_CONTEXT))?;
        let key_units: Vec<u16> = key.encode_utf16().collect();
        let code = java_parse_hex(&key_units)?;
        let slot = usize::try_from(code)
            .ok()
            .and_then(|index| self.by_code.get_mut(index))
            .ok_or(FormatError::out_of_bounds(TABLE_CONTEXT))?;
        *slot = Some(value.into());
        self.by_text.insert(value.encode_utf16().collect(), code);
        Ok(())
    }

    pub fn text_for(&self, code: i32) -> Option<&str> {
        usize::try_from(code)
            .ok()
            .and_then(|index| self.by_code.get(index))
            .and_then(Option::as_deref)
    }

    pub fn code_for(&self, text: &[u16]) -> Option<i32> {
        self.by_text.get(text).copied()
    }

    pub fn texts_by_code(&self) -> impl Iterator<Item = (i32, &str)> {
        (0..)
            .zip(&self.by_code)
            .filter_map(|(code, entry)| Some((code, entry.as_deref()?)))
    }

    pub fn codes_by_text(&self) -> impl Iterator<Item = (&[u16], i32)> {
        self.by_text
            .iter()
            .map(|(text, &code)| (text.as_slice(), code))
    }
}

pub fn generation4_table() -> Result<&'static CharacterTable> {
    static TABLE: OnceLock<std::result::Result<CharacterTable, String>> = OnceLock::new();
    TABLE
        .get_or_init(|| CharacterTable::parse(GENERATION4_TBL).map_err(|error| error.to_string()))
        .as_ref()
        .map_err(|reason| FormatError::invalid(TABLE_CONTEXT, reason.clone()))
}

fn scanner_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut characters = text.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if !is_scanner_line_break(character) {
            continue;
        }
        lines.push(&text[start..index]);
        start = index + character.len_utf8();
        if character == '\r' && characters.next_if(|&(_, next)| next == '\n').is_some() {
            start += 1;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

fn is_scanner_line_break(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

fn java_trim(text: &str) -> &str {
    text.trim_matches(|character: char| character <= ' ')
}
