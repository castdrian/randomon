use std::collections::HashMap;
use std::sync::OnceLock;

use randomon_data::constants::gen5;
use randomon_settings::java_text::{java_trim, scanner_lines};

const OFFSETS_INI: &str = include_str!("../assets/config/gen5_offsets.ini");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OffsetWithinEntry {
    pub entry: i32,
    pub offset: i32,
}

pub type FileEntry = OffsetWithinEntry;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RomFileEntry {
    pub path: String,
    pub expected_crc32: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StaticPokemonEntry {
    pub species_entries: Vec<FileEntry>,
    pub forme_entries: Vec<FileEntry>,
    pub level_entries: Vec<FileEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoamingPokemonEntry {
    pub species_overlay_offsets: Vec<i32>,
    pub level_overlay_offsets: Vec<i32>,
    pub species_script_offsets: Vec<FileEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TradeScript {
    pub file_number: i32,
    pub requested_offsets: Vec<i32>,
    pub given_offsets: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct RomEntry {
    pub name: String,
    pub rom_code: String,
    pub version: i32,
    pub rom_type: i32,
    pub arm9_expected_crc32: i64,
    pub static_pokemon_support: bool,
    pub copy_static_pokemon: bool,
    pub copy_roaming_pokemon: bool,
    pub copy_trade_scripts: bool,
    pub is_black: bool,
    pub strings: HashMap<String, String>,
    pub numbers: HashMap<String, i32>,
    pub tweak_files: HashMap<String, String>,
    pub array_entries: HashMap<String, Vec<i32>>,
    pub offset_array_entries: HashMap<String, Vec<OffsetWithinEntry>>,
    pub files: HashMap<String, RomFileEntry>,
    pub overlay_expected_crc32s: HashMap<i32, i64>,
    pub static_pokemon: Vec<StaticPokemonEntry>,
    pub static_pokemon_fake_ball: Vec<StaticPokemonEntry>,
    pub roaming_pokemon: Vec<RoamingPokemonEntry>,
    pub trade_scripts: Vec<TradeScript>,
}

impl RomEntry {
    pub fn int(&self, key: &str) -> i32 {
        self.numbers.get(key).copied().unwrap_or(0)
    }

    pub fn string(&self, key: &str) -> &str {
        self.strings.get(key).map_or("", String::as_str)
    }

    pub fn file(&self, key: &str) -> Option<&str> {
        self.files.get(key).map(|entry| entry.path.as_str())
    }

    pub fn array(&self, key: &str) -> Option<&[i32]> {
        self.array_entries.get(key).map(Vec::as_slice)
    }

    pub fn tweak(&self, key: &str) -> Option<&str> {
        self.tweak_files.get(key).map(String::as_str)
    }

    pub fn is_bw2(&self) -> bool {
        self.rom_type == gen5::TYPE_BW2
    }

    fn copy_from(&mut self, other: &RomEntry) {
        self.array_entries.extend(other.array_entries.clone());
        self.numbers.extend(other.numbers.clone());
        self.strings.extend(other.strings.clone());
        self.offset_array_entries
            .extend(other.offset_array_entries.clone());
        self.files.extend(other.files.clone());
        if self.copy_static_pokemon {
            self.static_pokemon
                .extend(other.static_pokemon.iter().cloned());
            self.static_pokemon_fake_ball
                .extend(other.static_pokemon_fake_ball.iter().cloned());
            self.static_pokemon_support = true;
        } else {
            self.static_pokemon_support = false;
        }
        if self.copy_trade_scripts {
            self.trade_scripts
                .extend(other.trade_scripts.iter().cloned());
        }
        if self.copy_roaming_pokemon {
            self.roaming_pokemon
                .extend(other.roaming_pokemon.iter().cloned());
        }
    }
}

fn digits_and_radix(text: &str) -> (String, u32) {
    let lowered = java_trim(text).to_lowercase();
    match lowered
        .strip_prefix("0x")
        .or_else(|| lowered.strip_prefix("&h"))
    {
        Some(hex) => (hex.to_owned(), 16),
        None => (lowered, 10),
    }
}

pub fn parse_ri_int(text: &str) -> i32 {
    let (digits, radix) = digits_and_radix(text);
    i32::from_str_radix(&digits, radix).unwrap_or(0)
}

pub fn parse_ri_long(text: &str) -> i64 {
    let (digits, radix) = digits_and_radix(text);
    i64::from_str_radix(&digits, radix).unwrap_or(0)
}

fn inner_of_brackets(value: &str) -> &str {
    let mut characters = value.chars();
    characters.next();
    characters.next_back();
    characters.as_str()
}

fn parse_file_entry(text: &str) -> FileEntry {
    let mut parts = text.split(':');
    FileEntry {
        entry: parse_ri_int(parts.next().unwrap_or("")),
        offset: parse_ri_int(parts.next().unwrap_or("")),
    }
}

struct Scanner<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Scanner<'_> {
    fn eat_while(&mut self, predicate: impl Fn(u8) -> bool) -> usize {
        let start = self.position;
        while self.position < self.bytes.len() && predicate(self.bytes[self.position]) {
            self.position += 1;
        }
        self.position - start
    }

    fn eat(&mut self, expected: &[u8]) -> bool {
        if self.bytes[self.position..].starts_with(expected) {
            self.position += expected.len();
            true
        } else {
            false
        }
    }

    fn eat_optional(&mut self, predicate: impl Fn(u8) -> bool) {
        if self.position < self.bytes.len() && predicate(self.bytes[self.position]) {
            self.position += 1;
        }
    }
}

fn is_identifier_byte(byte: u8) -> bool {
    (b'A'..=b'z').contains(&byte)
}

fn is_java_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

fn element_matches(scanner: &mut Scanner<'_>, with_file: bool) -> bool {
    if with_file && (scanner.eat_while(|byte| byte.is_ascii_digit()) == 0 || !scanner.eat(b":")) {
        return false;
    }
    if !scanner.eat(b"0x") || scanner.eat_while(|byte| byte.is_ascii_hexdigit()) == 0 {
        return false;
    }
    scanner.eat_optional(|byte| byte == b',');
    scanner.eat_optional(is_java_whitespace);
    true
}

fn group_length(text: &[u8], start: usize, with_file: bool) -> Option<usize> {
    let mut scanner = Scanner {
        bytes: text,
        position: start,
    };
    if !element_matches(&mut scanner, with_file) {
        return None;
    }
    loop {
        if scanner.eat(b"]") {
            return Some(scanner.position - start);
        }
        if !element_matches(&mut scanner, with_file) {
            return None;
        }
    }
}

struct Segment<'a> {
    name: &'a str,
    elements: Vec<&'a str>,
}

fn find_segments<'a>(text: &'a str, alternatives: &[bool]) -> Vec<Segment<'a>> {
    let bytes = text.as_bytes();
    let mut segments = Vec::new();
    let mut position = 0;
    while position < bytes.len() {
        let matched = segment_at(text, position, alternatives);
        match matched {
            Some((segment, end)) => {
                segments.push(segment);
                position = end;
            }
            None => position += 1,
        }
    }
    segments
}

fn segment_at<'a>(
    text: &'a str,
    start: usize,
    alternatives: &[bool],
) -> Option<(Segment<'a>, usize)> {
    let bytes = text.as_bytes();
    let mut name_end = start;
    while name_end < bytes.len() && is_identifier_byte(bytes[name_end]) {
        name_end += 1;
    }
    if name_end == start || !bytes[name_end..].starts_with(b"=[") {
        return None;
    }
    let content_start = name_end + 2;
    alternatives.iter().find_map(|&with_file| {
        let length = group_length(bytes, content_start, with_file)?;
        let end = content_start + length;
        let content = &text[content_start..end - 1];
        Some((
            Segment {
                name: &text[start..name_end],
                elements: content.split(',').collect(),
            },
            end,
        ))
    })
}

fn parse_static_pokemon(text: &str) -> StaticPokemonEntry {
    let mut entry = StaticPokemonEntry::default();
    for segment in find_segments(text, &[true]) {
        let entries: Vec<FileEntry> = segment
            .elements
            .iter()
            .map(|part| parse_file_entry(part))
            .collect();
        match segment.name {
            "Species" => entry.species_entries = entries,
            "Level" => entry.level_entries = entries,
            "Forme" => entry.forme_entries = entries,
            _ => {}
        }
    }
    entry
}

fn parse_roaming_pokemon(text: &str) -> RoamingPokemonEntry {
    let mut entry = RoamingPokemonEntry::default();
    for segment in find_segments(text, &[false, true]) {
        match segment.name {
            "Species" => {
                entry.species_overlay_offsets = segment
                    .elements
                    .iter()
                    .map(|part| parse_ri_int(part))
                    .collect()
            }
            "Level" => {
                entry.level_overlay_offsets = segment
                    .elements
                    .iter()
                    .map(|part| parse_ri_int(part))
                    .collect()
            }
            "Script" => {
                entry.species_script_offsets = segment
                    .elements
                    .iter()
                    .map(|part| parse_file_entry(part))
                    .collect()
            }
            _ => {}
        }
    }
    entry
}

fn parse_trade_script(value: &str) -> TradeScript {
    let mut script = TradeScript::default();
    for offset in inner_of_brackets(value).split(',') {
        let parts: Vec<&str> = offset.split(':').collect();
        script.file_number = parse_ri_int(parts.first().copied().unwrap_or(""));
        script
            .requested_offsets
            .push(parse_ri_int(parts.get(1).copied().unwrap_or("")));
        script
            .given_offsets
            .push(parse_ri_int(parts.get(2).copied().unwrap_or("")));
    }
    script
}

fn parse_offsets_within_entries(value: &str) -> Vec<OffsetWithinEntry> {
    inner_of_brackets(value)
        .split(',')
        .map(parse_file_entry)
        .collect()
}

fn parse_file_line(key: &str, value: &str) -> Option<(String, RomFileEntry)> {
    let name = key.split('<').nth(1)?.split('>').next()?.to_owned();
    let values: Vec<&str> = inner_of_brackets(value).split(',').collect();
    Some((
        name,
        RomFileEntry {
            path: java_trim(values.first()?).to_owned(),
            expected_crc32: parse_ri_long(&format!("0x{}", java_trim(values.get(1)?))),
        },
    ))
}

fn parse_array(value: &str) -> Vec<i32> {
    let offsets: Vec<&str> = inner_of_brackets(value).split(',').collect();
    if offsets.len() == 1 && java_trim(offsets[0]).is_empty() {
        return Vec::new();
    }
    offsets.iter().map(|offset| parse_ri_int(offset)).collect()
}

fn is_number_key(key: &str) -> bool {
    ["Offset", "Count", "Number", "Size", "Index"]
        .iter()
        .any(|suffix| key.ends_with(suffix))
}

fn apply_copy_from(roms: &mut [RomEntry], value: &str) {
    let Some((current, previous)) = roms.split_last_mut() else {
        return;
    };
    for other in previous.iter() {
        if value.eq_ignore_ascii_case(&other.rom_code) {
            current.copy_from(other);
        }
    }
}

fn apply_line(roms: &mut [RomEntry], key: &str, value: &str) {
    if key == "CopyFrom" {
        apply_copy_from(roms, value);
        return;
    }
    let Some(current) = roms.last_mut() else {
        return;
    };
    match key {
        "Game" => current.rom_code = value.to_owned(),
        "Version" => current.version = value.parse().unwrap_or(0),
        "Type" => {
            current.rom_type = if value.eq_ignore_ascii_case("BW2") {
                gen5::TYPE_BW2
            } else {
                gen5::TYPE_BW
            }
        }
        "Arm9CRC32" => current.arm9_expected_crc32 = parse_ri_long(&format!("0x{value}")),
        "StaticPokemon{}" => current.static_pokemon.push(parse_static_pokemon(value)),
        "StaticPokemonFakeBall{}" => current
            .static_pokemon_fake_ball
            .push(parse_static_pokemon(value)),
        "RoamingPokemon{}" => current.roaming_pokemon.push(parse_roaming_pokemon(value)),
        "TradeScript[]" => current.trade_scripts.push(parse_trade_script(value)),
        "StaticPokemonSupport" => current.static_pokemon_support = parse_ri_int(value) > 0,
        "CopyStaticPokemon" => current.copy_static_pokemon = parse_ri_int(value) > 0,
        "CopyRoamingPokemon" => current.copy_roaming_pokemon = parse_ri_int(value) > 0,
        "CopyTradeScripts" => current.copy_trade_scripts = parse_ri_int(value) > 0,
        _ if key.starts_with("File<") => {
            if let Some((name, entry)) = parse_file_line(key, value) {
                current.files.insert(name, entry);
            }
        }
        _ if key.starts_with("OverlayCRC32<") => {
            let number = key
                .split('<')
                .nth(1)
                .and_then(|rest| rest.split('>').next());
            current.overlay_expected_crc32s.insert(
                parse_ri_int(number.unwrap_or("")),
                parse_ri_long(&format!("0x{value}")),
            );
        }
        _ if key.starts_with("StarterOffsets") => {
            current
                .offset_array_entries
                .insert(key.to_owned(), parse_offsets_within_entries(value));
        }
        _ if key.ends_with("Tweak") => {
            current.tweak_files.insert(key.to_owned(), value.to_owned());
        }
        "IsBlack" => current.is_black = parse_ri_int(value) > 0,
        _ if value.starts_with('[') && value.ends_with(']') => {
            current
                .array_entries
                .insert(key.to_owned(), parse_array(value));
        }
        _ if is_number_key(key) => {
            current.numbers.insert(key.to_owned(), parse_ri_int(value));
        }
        _ => {
            current.strings.insert(key.to_owned(), value.to_owned());
        }
    }
}

fn strip_comment(line: &str) -> &str {
    let trimmed = java_trim(line);
    match trimmed.find("//") {
        Some(position) => java_trim(&trimmed[..position]),
        None => trimmed,
    }
}

fn load_rom_info() -> Vec<RomEntry> {
    let mut roms: Vec<RomEntry> = Vec::new();
    for raw_line in scanner_lines(OFFSETS_INI) {
        let line = strip_comment(raw_line);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            roms.push(RomEntry {
                name: inner_of_brackets(line).to_owned(),
                ..RomEntry::default()
            });
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        apply_line(&mut roms, key, java_trim(value));
    }
    roms
}

pub fn rom_entries() -> &'static [RomEntry] {
    static ENTRIES: OnceLock<Vec<RomEntry>> = OnceLock::new();
    ENTRIES.get_or_init(load_rom_info)
}

pub fn entry_for(rom_code: &str, version: i32) -> Option<&'static RomEntry> {
    rom_entries()
        .iter()
        .find(|entry| entry.rom_code == rom_code && entry.version == version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(code: &str) -> &'static RomEntry {
        entry_for(code, 0).expect("entry exists")
    }

    #[test]
    fn every_section_becomes_an_entry() {
        assert_eq!(rom_entries().len(), 28);
    }

    #[test]
    fn black2_copies_black_and_overrides() {
        let black2 = entry("IREO");
        assert_eq!(black2.rom_type, gen5::TYPE_BW2);
        assert!(black2.static_pokemon_support);
        assert_eq!(black2.file("MoveData"), Some("a/0/2/1"));
        assert_eq!(black2.file("Scripts"), Some("a/0/5/6"));
        assert_eq!(black2.int("RoamerOvlNumber"), 10);
        assert_eq!(black2.int("FieldOvlNumber"), 36);
        assert!(black2.roaming_pokemon.is_empty());
        assert!(black2.trade_scripts.is_empty());
        assert_eq!(black2.arm9_expected_crc32, 0xFAF9_D733);
        assert_eq!(black2.overlay_expected_crc32s.len(), 8);
        assert_eq!(black2.array("TradesUnused"), Some(&[25][..]));
        assert_eq!(
            black2.string("TCMCopyingPrefix"),
            "1030A0E3013053E2FDFFFF1AF8FFFFEA"
        );
        assert_eq!(
            black2.tweak("FastestTextTweak"),
            Some("instant_text/b2_instant_text")
        );
        let kyurem_black = &black2.static_pokemon[6];
        assert_eq!(
            kyurem_black.forme_entries,
            vec![
                FileEntry {
                    entry: 1208,
                    offset: 0xD8D
                },
                FileEntry {
                    entry: 1208,
                    offset: 0xD9B
                }
            ]
        );
        assert_eq!(black2.offset_array_entries["StarterOffsets1"].len(), 3);
    }

    #[test]
    fn black_roamers_and_trade_scripts_parse() {
        let black = entry("IRBO");
        assert!(black.is_black);
        let thundurus = &black.roaming_pokemon[0];
        assert_eq!(thundurus.species_overlay_offsets, vec![0x95D8, 0x940C]);
        assert_eq!(thundurus.level_overlay_offsets, vec![0x930E]);
        assert_eq!(thundurus.species_script_offsets.len(), 2);
        let first_trade = &black.trade_scripts[0];
        assert_eq!(first_trade.file_number, 46);
        assert_eq!(first_trade.requested_offsets, vec![0x81, 0x97]);
        assert_eq!(first_trade.given_offsets, vec![0x86, 0x9C]);
        let white = entry("IRAO");
        assert_eq!(white.roaming_pokemon.len(), 2);
        assert_eq!(white.trade_scripts.len(), 5);
        assert!(!white.is_black);
    }
}
