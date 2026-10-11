use std::collections::HashMap;
use std::sync::OnceLock;

use crate::error::{FormatError, Result};

const CONTEXT: &str = "PPTxtHandler";
const TABLE_TEXT: &str = include_str!("../../assets/text/Generation5.tbl");
const LINE_TERMINATOR: u16 = 0xFFFF;
const COMPRESSED_MARKER: i32 = 0xF100;
const LAST_PLAIN_CHARACTER: i32 = 0xFFF0;
const BACKSLASH: u16 = b'\\' as u16;
const OPEN_BRACE: u16 = b'{' as u16;

struct CharacterTable {
    poke_to_text: HashMap<u16, String>,
    text_to_poke: Vec<(Vec<u16>, Vec<u16>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTexts {
    pub strings: Vec<String>,
    pub keys: Vec<i32>,
    pub unknowns: Vec<i32>,
}

fn table() -> &'static CharacterTable {
    static TABLE: OnceLock<CharacterTable> = OnceLock::new();
    TABLE.get_or_init(load_table)
}

fn scanner_lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n')
        .flat_map(|line| line.split('\r'))
        .filter(|line| !line.trim().is_empty())
}

fn load_table() -> CharacterTable {
    let mut poke_to_text = HashMap::new();
    let mut text_to_poke: HashMap<Vec<u16>, Vec<u16>> = HashMap::new();
    for line in scanner_lines(TABLE_TEXT) {
        let Some((code, text)) = line.split_once('=') else {
            continue;
        };
        let Ok(value) = u32::from_str_radix(code, 16) else {
            continue;
        };
        poke_to_text.insert(value as u16, text.to_owned());
        text_to_poke.insert(
            text.encode_utf16().collect(),
            format!("\\x{code}").encode_utf16().collect(),
        );
    }
    let mut text_to_poke: Vec<(Vec<u16>, Vec<u16>)> = text_to_poke.into_iter().collect();
    text_to_poke.sort_by(|left, right| right.0.len().cmp(&left.0.len()).then(left.0.cmp(&right.0)));
    CharacterTable {
        poke_to_text,
        text_to_poke,
    }
}

fn byte_at(data: &[u8], offset: i64) -> Result<i32> {
    usize::try_from(offset)
        .ok()
        .and_then(|index| data.get(index))
        .map(|&byte| i32::from(byte))
        .ok_or(FormatError::out_of_bounds(CONTEXT))
}

fn read_word(data: &[u8], offset: i64) -> Result<i32> {
    Ok(byte_at(data, offset)? + (byte_at(data, offset + 1)? << 8))
}

fn read_long(data: &[u8], offset: i64) -> Result<i32> {
    Ok(byte_at(data, offset)?
        .wrapping_add(byte_at(data, offset + 1)? << 8)
        .wrapping_add(byte_at(data, offset + 2)? << 16)
        .wrapping_add(byte_at(data, offset + 3)? << 24))
}

fn checked_index(data_len: usize, offset: i64, width: usize) -> Result<usize> {
    usize::try_from(offset)
        .ok()
        .filter(|&index| index + width <= data_len)
        .ok_or(FormatError::out_of_bounds(CONTEXT))
}

fn write_word(data: &mut [u8], offset: i64, value: i32) -> Result<()> {
    let index = checked_index(data.len(), offset, 2)?;
    data[index] = value as u8;
    data[index + 1] = (value >> 8) as u8;
    Ok(())
}

fn write_long(data: &mut [u8], offset: i64, value: i32) -> Result<()> {
    let index = checked_index(data.len(), offset, 4)?;
    data[index..index + 4].copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn rotate_key_right(key: i32) -> i32 {
    (((key as u32) >> 3) as i32 | (key << 13)) & 0xFFFF
}

fn rotate_key_left(key: i32) -> i32 {
    ((key << 3) | ((key as u32) >> 13) as i32) & 0xFFFF
}

fn element(chars: &[i32], index: usize) -> Result<i32> {
    chars
        .get(index)
        .copied()
        .ok_or(FormatError::out_of_bounds(CONTEXT))
}

fn keep_decompressed(uncompressed: &mut Vec<i32>, value: i32) -> bool {
    if (value & 0xFF) == 0xFF {
        return false;
    }
    if value != 0x0 && value != 0x1 {
        uncompressed.push(value);
    }
    true
}

fn decompress(chars: &[i32]) -> Result<Vec<i32>> {
    let mut uncompressed = Vec::new();
    let mut index = 1;
    let mut shift = 0;
    let mut carried = 0;
    loop {
        let current = element(chars, index)?;
        if shift >= 0x10 {
            shift -= 0x10;
            if shift > 0 {
                let value = carried | ((current << (9 - shift)) & 0x1FF);
                if !keep_decompressed(&mut uncompressed, value) {
                    break;
                }
            }
        } else {
            let value = (current >> shift) & 0x1FF;
            if !keep_decompressed(&mut uncompressed, value) {
                break;
            }
            shift += 9;
            if shift < 0x10 {
                carried = (current >> shift) & 0x1FF;
                shift += 9;
            }
            index += 1;
        }
    }
    Ok(uncompressed)
}

fn has_unicode_block(code: i32) -> bool {
    const UNASSIGNED_BMP_BLOCK: std::ops::RangeInclusive<i32> = 0x2FE0..=0x2FEF;
    !UNASSIGNED_BMP_BLOCK.contains(&code)
}

fn push_escape(units: &mut Vec<u16>, value: i32) {
    units.extend(format!("\\x{value:04X}").encode_utf16());
}

fn displayed_units(characters: &[i32]) -> Vec<u16> {
    let mut units = Vec::new();
    for &character in characters {
        if character == i32::from(LINE_TERMINATOR) {
            continue;
        }
        if character > 20 && character <= LAST_PLAIN_CHARACTER && has_unicode_block(character) {
            units.push(character as u16);
        } else {
            push_escape(&mut units, character);
        }
    }
    units
}

fn units_to_string(units: &[u16]) -> String {
    let mut text = String::with_capacity(units.len());
    for decoded in char::decode_utf16(units.iter().copied()) {
        match decoded {
            Ok(character) => text.push(character),
            Err(error) => text.push_str(&format!("\\x{:04X}", error.unpaired_surrogate())),
        }
    }
    text
}

fn poke_to_text(units: &[u16]) -> String {
    let replacements = &table().poke_to_text;
    let mut replaced: Vec<u16> = Vec::with_capacity(units.len());
    for &unit in units {
        match replacements.get(&unit) {
            Some(text) => replaced.extend(text.encode_utf16()),
            None => replaced.push(unit),
        }
    }
    units_to_string(&replaced)
}

fn text_to_poke(text: &str) -> Vec<u16> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let replacements = &table().text_to_poke;
    let mut replaced = Vec::with_capacity(units.len());
    let mut position = 0;
    while position < units.len() {
        let matched = replacements
            .iter()
            .find(|(token, _)| units[position..].starts_with(token));
        match matched {
            Some((token, replacement)) => {
                replaced.extend_from_slice(replacement);
                position += token.len();
            }
            None => {
                replaced.push(units[position]);
                position += 1;
            }
        }
    }
    replaced
}

struct EntryHeader {
    offset: i32,
    character_count: i32,
    unknown: i32,
}

struct RawFile {
    section_offsets: Vec<i32>,
    entries: Vec<Vec<i32>>,
    keys: Vec<i32>,
    unknowns: Vec<i32>,
}

fn section_offsets(data: &[u8], section_count: i32) -> Result<Vec<i32>> {
    if section_count > 3 {
        return Err(FormatError::out_of_bounds(CONTEXT));
    }
    (0..section_count)
        .map(|section| read_long(data, 12 + i64::from(section) * 4))
        .collect()
}

fn entry_headers(data: &[u8], section_offset: i32, entry_count: i32) -> Result<Vec<EntryHeader>> {
    let mut position = i64::from(section_offset) + 4;
    let mut headers = Vec::new();
    for _ in 0..entry_count {
        headers.push(EntryHeader {
            offset: read_long(data, position)?,
            character_count: read_word(data, position + 4)?,
            unknown: read_word(data, position + 6)?,
        });
        position += 8;
    }
    Ok(headers)
}

fn decrypt_entry(
    data: &[u8],
    section_offset: i32,
    header: &EntryHeader,
) -> Result<(Vec<i32>, i32)> {
    let start = i64::from(section_offset) + i64::from(header.offset);
    let count = header.character_count as usize;
    let mut characters = (0..count)
        .map(|index| read_word(data, start + 2 * index as i64))
        .collect::<Result<Vec<i32>>>()?;
    let last = *characters
        .last()
        .ok_or(FormatError::out_of_bounds(CONTEXT))?;
    let mut key = last ^ 0xFFFF;
    let mut first_key = key;
    for index in (0..count).rev() {
        characters[index] ^= key;
        if index == 0 {
            first_key = key;
        }
        key = rotate_key_right(key);
    }
    if characters[0] == COMPRESSED_MARKER {
        characters = decompress(&characters)?;
    }
    Ok((characters, first_key))
}

fn read_raw(data: &[u8]) -> Result<RawFile> {
    let section_count = read_word(data, 0)?;
    let entry_count = read_word(data, 2)?;
    read_long(data, 4)?;
    let mut raw = RawFile {
        section_offsets: Vec::new(),
        entries: Vec::new(),
        keys: Vec::new(),
        unknowns: Vec::new(),
    };
    if section_count <= 0 {
        return Ok(raw);
    }
    raw.section_offsets = section_offsets(data, section_count)?;
    let section_offset = raw.section_offsets[0];
    read_long(data, i64::from(section_offset))?;
    let headers = entry_headers(data, section_offset, entry_count)?;
    raw.unknowns = headers.iter().map(|header| header.unknown).collect();
    for header in &headers {
        let (characters, key) = decrypt_entry(data, section_offset, header)?;
        raw.entries.push(characters);
        raw.keys.push(key);
    }
    Ok(raw)
}

pub fn decode(data: &[u8]) -> Result<DecodedTexts> {
    let raw = read_raw(data)?;
    let strings = raw
        .entries
        .iter()
        .map(|characters| poke_to_text(&displayed_units(characters)))
        .collect();
    Ok(DecodedTexts {
        strings,
        keys: raw.keys,
        unknowns: raw.unknowns,
    })
}

pub fn read_texts(data: &[u8]) -> Result<Vec<String>> {
    decode(data).map(|decoded| decoded.strings)
}

fn java_parse_hex(units: &[u16]) -> Result<i32> {
    let text = String::from_utf16(units).map_err(|_| number_format())?;
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text.as_str()),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(number_format());
    }
    let magnitude = i64::from_str_radix(digits, 16).map_err(|_| number_format())?;
    Ok(if negative { -magnitude } else { magnitude } as i32)
}

fn number_format() -> FormatError {
    FormatError::invalid(CONTEXT, "java.lang.NumberFormatException")
}

fn parse_string(units: &[u16], key: i32) -> Result<Vec<i32>> {
    let mut characters = Vec::with_capacity(units.len() + 1);
    let mut index = 0;
    while index < units.len() {
        let is_literal = units[index] != BACKSLASH
            || (index + 2 < units.len() && units[index + 2] == OPEN_BRACE);
        if is_literal {
            characters.push(i32::from(units[index]));
        } else {
            let digits = units
                .get(index + 2..index + 6)
                .ok_or(FormatError::out_of_bounds(CONTEXT))?;
            characters.push(java_parse_hex(digits)?);
            index += 5;
        }
        index += 1;
    }
    characters.push(i32::from(LINE_TERMINATOR));
    let mut key = key;
    for character in &mut characters {
        *character = (*character ^ key) & 0xFFFF;
        key = rotate_key_left(key);
    }
    Ok(characters)
}

fn padding_word(last_key: i32, entry_length: usize) -> i32 {
    let mut key = last_key;
    for _ in 0..entry_length {
        key = rotate_key_left(key);
    }
    0xFFFF ^ key
}

fn make_section(strings: &[Vec<u16>], entry_count: usize, raw: &RawFile) -> Result<Vec<u8>> {
    let key_at = |index: usize| {
        raw.keys
            .get(index)
            .copied()
            .ok_or(FormatError::out_of_bounds(CONTEXT))
    };
    let mut data = Vec::with_capacity(entry_count);
    let mut size = 0usize;
    for (index, string) in strings.iter().take(entry_count).enumerate() {
        let encoded = parse_string(string, key_at(index)?)?;
        size += encoded.len() * 2;
        data.push(encoded);
    }
    if size % 4 == 2 {
        size += 2;
        let last = entry_count - 1;
        let padding = padding_word(key_at(last)?, data[last].len());
        data[last].push(padding);
    }
    let table_size = 4 + 8 * entry_count;
    size += table_size;
    let mut section = vec![0u8; size];
    write_long(&mut section, 0, size as i32)?;
    let mut position = 4i64;
    let mut offset = table_size as i32;
    for (index, encoded) in data.iter().enumerate() {
        let unknown = raw
            .unknowns
            .get(index)
            .copied()
            .ok_or(FormatError::out_of_bounds(CONTEXT))?;
        write_long(&mut section, position, offset)?;
        write_word(&mut section, position + 4, encoded.len() as i32)?;
        write_word(&mut section, position + 6, unknown)?;
        position += 8;
        offset += encoded.len() as i32 * 2;
    }
    for encoded in &data {
        for &word in encoded {
            write_word(&mut section, position, word)?;
            position += 2;
        }
    }
    Ok(section)
}

fn copy_into(target: &mut [u8], at: i64, source: &[u8]) -> Result<()> {
    let start = checked_index(target.len(), at, source.len())?;
    target[start..start + source.len()].copy_from_slice(source);
    Ok(())
}

fn source_slice(data: &[u8], offset: i32, length: i32) -> Result<&[u8]> {
    let start = usize::try_from(offset).map_err(|_| FormatError::out_of_bounds(CONTEXT))?;
    let length = usize::try_from(length).map_err(|_| FormatError::out_of_bounds(CONTEXT))?;
    data.get(start..start + length)
        .ok_or(FormatError::out_of_bounds(CONTEXT))
}

pub fn save_entry(original: &[u8], strings: &[String]) -> Result<Vec<u8>> {
    let converted: Vec<Vec<u16>> = strings.iter().map(|text| text_to_poke(text)).collect();
    let raw = read_raw(original)?;
    let section_count = read_word(original, 0)?;
    let entry_count = read_word(original, 2)? as usize;
    let header_size = read_long(original, 4)?;
    if converted.len() < entry_count {
        return Ok(original.to_vec());
    }
    let section = make_section(&converted, entry_count, &raw)?;
    let offsets = section_offsets(original, section_count)?;
    let sizes = offsets
        .iter()
        .map(|&offset| read_long(original, i64::from(offset)))
        .collect::<Result<Vec<i32>>>()?;
    let first_offset = offsets.first().copied().unwrap_or(0);
    let first_size = sizes.first().copied().unwrap_or(header_size);
    let new_length = original.len() as i64 - i64::from(first_size) + section.len() as i64;
    let mut output =
        vec![0u8; usize::try_from(new_length).map_err(|_| FormatError::out_of_bounds(CONTEXT))?];
    let shared = original.len().min(output.len());
    output[..shared].copy_from_slice(&original[..shared]);
    write_long(&mut output, 4, section.len() as i32)?;
    let second_offset = section.len() as i32 + first_offset;
    if section_count == 2 {
        write_long(&mut output, 0x10, second_offset)?;
    }
    copy_into(&mut output, i64::from(first_offset), &section)?;
    if section_count == 2 {
        let second = source_slice(original, offsets[1], sizes[1])?;
        copy_into(&mut output, i64::from(second_offset), second)?;
    }
    Ok(output)
}

pub fn internal_string_length(text: &str) -> i32 {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut length = units.len() as i32;
    let mut from = 0;
    while let Some(found) = find_escape(&units, from) {
        length -= 5;
        from = found + 1;
    }
    length
}

fn find_escape(units: &[u16], from: usize) -> Option<usize> {
    let x = u16::from(b'x');
    (from..units.len().saturating_sub(1))
        .find(|&index| units[index] == BACKSLASH && units[index + 1] == x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_maps_both_directions() {
        assert_eq!(
            table().poke_to_text.get(&0x2467).map(String::as_str),
            Some("×")
        );
        assert_eq!(
            table().poke_to_text.get(&0x21D2).map(String::as_str),
            Some("\\[angry]")
        );
        let encoded: Vec<u16> = text_to_poke("a\\[angry]b×");
        assert_eq!(String::from_utf16(&encoded).unwrap(), "a\\x21D2b\\x2467");
    }

    #[test]
    fn internal_length_discounts_escapes() {
        assert_eq!(internal_string_length("Pok\\x00E9mon"), 7);
        assert_eq!(internal_string_length("abc"), 3);
    }
}
