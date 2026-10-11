use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use randomon_data::constants::gen4;
use randomon_settings::java_text::{java_trim, scanner_lines};

const OFFSETS_INI: &str = include_str!("../assets/config/gen4_offsets.ini");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptEntry {
    pub file: i32,
    pub offset: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextEntry {
    pub text_index: i32,
    pub string_number: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RomFileEntry {
    pub path: String,
    pub expected_crc32: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StaticPokemonEntry {
    pub species_entries: Vec<ScriptEntry>,
    pub forme_entries: Vec<ScriptEntry>,
    pub level_entries: Vec<ScriptEntry>,
    pub game_corner_texts: Option<Vec<TextEntry>>,
}

impl StaticPokemonEntry {
    pub fn is_game_corner(&self) -> bool {
        self.game_corner_texts.is_some()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoamingPokemonEntry {
    pub species_code_offsets: Vec<i32>,
    pub level_code_offsets: Vec<i32>,
    pub species_script_offsets: Vec<ScriptEntry>,
    pub gender_offsets: Vec<ScriptEntry>,
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
    pub ignore_game_corner_statics: bool,
    pub copy_text: bool,
    pub strings: HashMap<String, String>,
    pub tweak_files: HashMap<String, String>,
    pub numbers: HashMap<String, i32>,
    pub array_entries: HashMap<String, Vec<i32>>,
    pub files: HashMap<String, RomFileEntry>,
    pub overlay_expected_crc32s: BTreeMap<i32, i64>,
    pub static_pokemon: Vec<StaticPokemonEntry>,
    pub roaming_pokemon: Vec<RoamingPokemonEntry>,
    pub marill_cry_script_entries: Vec<ScriptEntry>,
    pub tm_texts: HashMap<i32, Vec<TextEntry>>,
    pub tm_texts_game_corner: HashMap<i32, TextEntry>,
    pub tm_script_offsets_frontier: HashMap<i32, i32>,
    pub tm_texts_frontier: HashMap<i32, i32>,
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

    pub fn array_or_empty(&self, key: &str) -> &[i32] {
        self.array(key).unwrap_or(&[])
    }

    pub fn tweak(&self, key: &str) -> Option<&str> {
        self.tweak_files.get(key).map(String::as_str)
    }

    pub fn is_dp(&self) -> bool {
        self.rom_type == gen4::TYPE_DP
    }

    pub fn is_plat(&self) -> bool {
        self.rom_type == gen4::TYPE_PLAT
    }

    pub fn is_hgss(&self) -> bool {
        self.rom_type == gen4::TYPE_HGSS
    }

    fn copy_from(&mut self, other: &RomEntry) {
        self.array_entries.extend(other.array_entries.clone());
        self.numbers.extend(other.numbers.clone());
        self.strings.extend(other.strings.clone());
        self.files.extend(other.files.clone());
        if self.copy_static_pokemon {
            self.static_pokemon
                .extend(other.static_pokemon.iter().cloned());
            if self.ignore_game_corner_statics {
                self.static_pokemon.retain(|entry| !entry.is_game_corner());
            }
            self.static_pokemon_support = true;
        } else {
            self.static_pokemon_support = false;
        }
        if self.copy_roaming_pokemon {
            self.roaming_pokemon
                .extend(other.roaming_pokemon.iter().cloned());
        }
        if self.copy_text {
            self.tm_texts.extend(other.tm_texts.clone());
            self.tm_texts_game_corner
                .extend(other.tm_texts_game_corner.clone());
            self.tm_script_offsets_frontier
                .extend(other.tm_script_offsets_frontier.clone());
            self.tm_texts_frontier
                .extend(other.tm_texts_frontier.clone());
        }
        self.marill_cry_script_entries
            .extend(other.marill_cry_script_entries.iter().copied());
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

fn parse_script_entry(text: &str) -> ScriptEntry {
    let mut parts = text.split(':');
    ScriptEntry {
        file: parse_ri_int(parts.next().unwrap_or("")),
        offset: parse_ri_int(parts.next().unwrap_or("")),
    }
}

fn parse_text_entry(text: &str) -> TextEntry {
    let mut parts = text.split(':');
    TextEntry {
        text_index: parse_ri_int(parts.next().unwrap_or("")),
        string_number: parse_ri_int(parts.next().unwrap_or("")),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ElementShape {
    Hex,
    FileAndHex,
    DecimalPair,
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

fn element_matches(scanner: &mut Scanner<'_>, shape: ElementShape) -> bool {
    let digits = |scanner: &mut Scanner<'_>| scanner.eat_while(|byte| byte.is_ascii_digit()) > 0;
    let hex = |scanner: &mut Scanner<'_>| {
        scanner.eat(b"0x") && scanner.eat_while(|byte| byte.is_ascii_hexdigit()) > 0
    };
    let matched = match shape {
        ElementShape::Hex => hex(scanner),
        ElementShape::FileAndHex => digits(scanner) && scanner.eat(b":") && hex(scanner),
        ElementShape::DecimalPair => digits(scanner) && scanner.eat(b":") && digits(scanner),
    };
    if !matched {
        return false;
    }
    scanner.eat_optional(|byte| byte == b',');
    scanner.eat_optional(is_java_whitespace);
    true
}

fn group_length(text: &[u8], start: usize, shape: ElementShape) -> Option<usize> {
    let mut scanner = Scanner {
        bytes: text,
        position: start,
    };
    if !element_matches(&mut scanner, shape) {
        return None;
    }
    loop {
        if scanner.eat(b"]") {
            return Some(scanner.position - start);
        }
        if !element_matches(&mut scanner, shape) {
            return None;
        }
    }
}

struct Segment<'a> {
    name: &'a str,
    elements: Vec<&'a str>,
}

fn segment_name_end(bytes: &[u8], start: usize, numeric_names: bool) -> usize {
    let mut end = start;
    while end < bytes.len()
        && (if numeric_names {
            bytes[end].is_ascii_digit()
        } else {
            is_identifier_byte(bytes[end])
        })
    {
        end += 1;
    }
    end
}

fn segment_at<'a>(
    text: &'a str,
    start: usize,
    numeric_names: bool,
    shapes: &[ElementShape],
) -> Option<(Segment<'a>, usize)> {
    let bytes = text.as_bytes();
    let name_end = segment_name_end(bytes, start, numeric_names);
    if name_end == start || !bytes[name_end..].starts_with(b"=[") {
        return None;
    }
    let content_start = name_end + 2;
    shapes.iter().find_map(|&shape| {
        let length = group_length(bytes, content_start, shape)?;
        let end = content_start + length;
        Some((
            Segment {
                name: &text[start..name_end],
                elements: text[content_start..end - 1].split(',').collect(),
            },
            end,
        ))
    })
}

fn find_segments<'a>(
    text: &'a str,
    numeric_names: bool,
    shapes: &[ElementShape],
) -> Vec<Segment<'a>> {
    let mut segments = Vec::new();
    let mut position = 0;
    while position < text.len() {
        match segment_at(text, position, numeric_names, shapes) {
            Some((segment, end)) => {
                segments.push(segment);
                position = end;
            }
            None => position += 1,
        }
    }
    segments
}

fn script_entries(segment: &Segment<'_>) -> Vec<ScriptEntry> {
    segment
        .elements
        .iter()
        .map(|part| parse_script_entry(part))
        .collect()
}

fn parse_static_pokemon(text: &str) -> StaticPokemonEntry {
    let mut entry = StaticPokemonEntry::default();
    for segment in find_segments(text, false, &[ElementShape::FileAndHex]) {
        match segment.name {
            "Species" => entry.species_entries = script_entries(&segment),
            "Level" => entry.level_entries = script_entries(&segment),
            "Forme" => entry.forme_entries = script_entries(&segment),
            _ => {}
        }
    }
    entry
}

fn parse_static_pokemon_game_corner(text: &str) -> StaticPokemonEntry {
    let mut entry = StaticPokemonEntry {
        game_corner_texts: Some(Vec::new()),
        ..StaticPokemonEntry::default()
    };
    for segment in find_segments(text, false, &[ElementShape::FileAndHex]) {
        match segment.name {
            "Species" => entry.species_entries = script_entries(&segment),
            "Level" => entry.level_entries = script_entries(&segment),
            "Text" => {
                entry.game_corner_texts = Some(
                    segment
                        .elements
                        .iter()
                        .map(|part| parse_text_entry(part))
                        .collect(),
                )
            }
            _ => {}
        }
    }
    entry
}

fn parse_roaming_pokemon(text: &str) -> RoamingPokemonEntry {
    let mut entry = RoamingPokemonEntry::default();
    let shapes = [ElementShape::Hex, ElementShape::FileAndHex];
    for segment in find_segments(text, false, &shapes) {
        let numbers = || -> Vec<i32> {
            segment
                .elements
                .iter()
                .map(|part| parse_ri_int(part))
                .collect()
        };
        match segment.name {
            "Species" => entry.species_code_offsets = numbers(),
            "Level" => entry.level_code_offsets = numbers(),
            "Script" => entry.species_script_offsets = script_entries(&segment),
            "Gender" => entry.gender_offsets = script_entries(&segment),
            _ => {}
        }
    }
    entry
}

fn parse_tm_text(text: &str, tm_texts: &mut HashMap<i32, Vec<TextEntry>>) {
    for segment in find_segments(text, true, &[ElementShape::DecimalPair]) {
        let entries = segment
            .elements
            .iter()
            .map(|part| parse_text_entry(part))
            .collect();
        tm_texts.insert(parse_ri_int(segment.name), entries);
    }
}

fn parse_tm_text_game_corner(text: &str, tm_texts: &mut HashMap<i32, TextEntry>) {
    for element in inner_of_brackets(text).split(',') {
        let mut segments = java_trim(element).split('=');
        let tm_number = parse_ri_int(segments.next().unwrap_or(""));
        let entry = inner_of_brackets(segments.next().unwrap_or(""));
        tm_texts.insert(tm_number, parse_text_entry(entry));
    }
}

fn parse_number_pairs(text: &str, target: &mut HashMap<i32, i32>) {
    for element in inner_of_brackets(text).split(',') {
        let mut parts = element.split('=');
        let key = parse_ri_int(parts.next().unwrap_or(""));
        let value = parse_ri_int(parts.next().unwrap_or(""));
        target.insert(key, value);
    }
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

fn rom_type_named(value: &str) -> Option<i32> {
    if value.eq_ignore_ascii_case("DP") {
        Some(gen4::TYPE_DP)
    } else if value.eq_ignore_ascii_case("Plat") {
        Some(gen4::TYPE_PLAT)
    } else if value.eq_ignore_ascii_case("HGSS") {
        Some(gen4::TYPE_HGSS)
    } else {
        None
    }
}

fn apply_copy_from(roms: &mut [RomEntry], value: &str) {
    let Some((current, previous)) = roms.split_last_mut() else {
        return;
    };
    for other in previous.iter() {
        if value.eq_ignore_ascii_case(&other.name) {
            current.copy_from(other);
        }
    }
}

fn parse_marill_cry_scripts(value: &str) -> Vec<ScriptEntry> {
    inner_of_brackets(value)
        .split(',')
        .map(parse_script_entry)
        .collect()
}

fn apply_special_line(current: &mut RomEntry, key: &str, value: &str) -> bool {
    match key {
        "Game" => current.rom_code = value.to_owned(),
        "Version" => current.version = i32::from(value.parse::<i8>().unwrap_or(0)),
        "Type" => {
            if let Some(rom_type) = rom_type_named(value) {
                current.rom_type = rom_type;
            }
        }
        "Arm9CRC32" => current.arm9_expected_crc32 = parse_ri_long(&format!("0x{value}")),
        "StaticPokemon{}" => current.static_pokemon.push(parse_static_pokemon(value)),
        "RoamingPokemon{}" => current.roaming_pokemon.push(parse_roaming_pokemon(value)),
        "StaticPokemonGameCorner{}" => current
            .static_pokemon
            .push(parse_static_pokemon_game_corner(value)),
        "TMText{}" => parse_tm_text(value, &mut current.tm_texts),
        "TMTextGameCorner{}" => parse_tm_text_game_corner(value, &mut current.tm_texts_game_corner),
        "FrontierScriptTMOffsets{}" => {
            parse_number_pairs(value, &mut current.tm_script_offsets_frontier)
        }
        "FrontierTMText{}" => parse_number_pairs(value, &mut current.tm_texts_frontier),
        "StaticPokemonSupport" => current.static_pokemon_support = parse_ri_int(value) > 0,
        "CopyStaticPokemon" => current.copy_static_pokemon = parse_ri_int(value) > 0,
        "CopyRoamingPokemon" => current.copy_roaming_pokemon = parse_ri_int(value) > 0,
        "CopyText" => current.copy_text = parse_ri_int(value) > 0,
        "IgnoreGameCornerStatics" => current.ignore_game_corner_statics = parse_ri_int(value) > 0,
        _ => return false,
    }
    true
}

fn apply_generic_line(current: &mut RomEntry, key: &str, value: &str) {
    if key.starts_with("File<") {
        if let Some((name, entry)) = parse_file_line(key, value) {
            current.files.insert(name, entry);
        }
    } else if key.starts_with("OverlayCRC32<") {
        let number = key
            .split('<')
            .nth(1)
            .and_then(|rest| rest.split('>').next());
        current.overlay_expected_crc32s.insert(
            parse_ri_int(number.unwrap_or("")),
            parse_ri_long(&format!("0x{value}")),
        );
    } else if key.ends_with("Tweak") {
        current.tweak_files.insert(key.to_owned(), value.to_owned());
    } else if key.ends_with("MarillCryScripts") {
        current.marill_cry_script_entries = parse_marill_cry_scripts(value);
    } else if value.starts_with('[') && value.ends_with(']') {
        current
            .array_entries
            .insert(key.to_owned(), parse_array(value));
    } else if is_number_key(key) {
        current.numbers.insert(key.to_owned(), parse_ri_int(value));
    } else {
        current.strings.insert(key.to_owned(), value.to_owned());
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
    if !apply_special_line(current, key, value) {
        apply_generic_line(current, key, value);
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

    fn named(name: &str) -> &'static RomEntry {
        rom_entries()
            .iter()
            .find(|entry| entry.name == name)
            .expect("entry exists")
    }

    #[test]
    fn every_section_becomes_an_entry() {
        assert_eq!(rom_entries().len(), 42);
        for entry in rom_entries() {
            assert_eq!(entry.rom_code.len(), 4, "{}", entry.name);
            assert!(entry.file("Text").is_some(), "{}", entry.name);
            assert!(entry.file("Scripts").is_some(), "{}", entry.name);
            assert!(entry.arm9_expected_crc32 != 0, "{}", entry.name);
        }
    }

    #[test]
    fn diamond_parses_statics_roamers_and_tm_texts() {
        let diamond = entry_for("ADAE", 5).expect("Diamond (U)");
        assert_eq!(diamond.name, "Diamond (U)");
        assert!(diamond.is_dp());
        assert!(diamond.static_pokemon_support);
        assert_eq!(diamond.static_pokemon.len(), 17);
        assert_eq!(
            diamond.static_pokemon[0].species_entries,
            vec![
                ScriptEntry {
                    file: 342,
                    offset: 0x261
                },
                ScriptEntry {
                    file: 342,
                    offset: 0x2BE
                }
            ]
        );
        assert!(diamond.static_pokemon[12].level_entries.is_empty());
        let mesprit = &diamond.roaming_pokemon[0];
        assert_eq!(mesprit.species_code_offsets, vec![0x60580]);
        assert_eq!(mesprit.level_code_offsets, vec![0x604B6]);
        assert_eq!(mesprit.species_script_offsets.len(), 2);
        assert_eq!(
            mesprit.gender_offsets,
            vec![ScriptEntry {
                file: 344,
                offset: 0x1C
            }]
        );
        assert_eq!(diamond.tm_texts[&56].len(), 2);
        assert_eq!(
            diamond.tm_texts[&63],
            vec![TextEntry {
                text_index: 135,
                string_number: 35
            }]
        );
        assert_eq!(diamond.int("Arm9ExtensionSize"), 168);
        assert_eq!(diamond.array("ItemBallsSkip"), Some(&[40, 196][..]));
        assert_eq!(diamond.string("HasExtraPokemonNames"), "Yes");
        assert_eq!(
            diamond.file("WildPokemon"),
            Some("fielddata/encountdata/d_enc_data.narc")
        );
        assert_eq!(diamond.overlay_expected_crc32s.len(), 4);
    }

    #[test]
    fn pearl_copies_diamond_by_name() {
        let pearl = entry_for("APAE", 5).expect("Pearl (U)");
        assert_eq!(pearl.static_pokemon.len(), 17);
        assert_eq!(pearl.roaming_pokemon.len(), 2);
        assert_eq!(pearl.tm_texts.len(), 11);
        assert_eq!(pearl.int("PokemonNamesTextOffset"), 362);
        assert_eq!(
            pearl.file("PokemonStats"),
            Some("poketool/personal_pearl/personal.narc")
        );
        assert_eq!(pearl.array("HoneyTreeOffsets"), Some(&[5, 6, 7][..]));
    }

    #[test]
    fn platinum_overrides_without_copying_statics() {
        let platinum = entry_for("CPUE", 0).expect("Platinum (U)");
        assert!(platinum.is_plat());
        assert!(platinum.static_pokemon_support);
        assert_eq!(platinum.static_pokemon.len(), 20);
        assert_eq!(platinum.roaming_pokemon.len(), 5);
        assert!(
            platinum.roaming_pokemon[2]
                .species_script_offsets
                .is_empty()
        );
        assert_eq!(platinum.int("MoveTutorCount"), 38);
        assert_eq!(
            platinum.tweak("FastDistortionWorldTweak"),
            Some("pt_fast_distortion_world")
        );
        assert_eq!(platinum.int("Arm9ExtensionSize"), 424);
    }

    #[test]
    fn heartgold_parses_game_corner_frontier_and_marill() {
        let heartgold = entry_for("IPKE", 0).expect("HeartGold (U)");
        assert!(heartgold.is_hgss());
        let corner: Vec<&StaticPokemonEntry> = heartgold
            .static_pokemon
            .iter()
            .filter(|entry| entry.is_game_corner())
            .collect();
        assert_eq!(corner.len(), 7);
        assert_eq!(
            corner[0].game_corner_texts.as_deref(),
            Some(
                &[TextEntry {
                    text_index: 603,
                    string_number: 0x18
                }][..]
            )
        );
        let mareep = heartgold
            .static_pokemon
            .iter()
            .find(|entry| entry.species_entries.first().map(|e| e.offset) == Some(0x146))
            .expect("Primo's Mareep egg parses despite the stray bracket");
        assert_eq!(mareep.species_entries.len(), 2);
        assert_eq!(heartgold.marill_cry_script_entries.len(), 4);
        assert_eq!(heartgold.tm_texts_game_corner.len(), 12);
        assert_eq!(heartgold.tm_script_offsets_frontier[&40], 0xC5C);
        assert_eq!(heartgold.tm_texts_frontier[&4], 395);
        assert_eq!(heartgold.tm_texts[&59].len(), 2);
        assert_eq!(heartgold.array("StaticPokemonTrades"), Some(&[6, 7][..]));
        assert_eq!(heartgold.int("MysteryEggOffset"), 0x1C80E);
        assert_eq!(heartgold.int("MoveTutorCompatOffset"), 0);
        let soulsilver = entry_for("IPGE", 0).expect("SoulSilver (U)");
        assert_eq!(
            soulsilver.static_pokemon.len(),
            heartgold.static_pokemon.len()
        );
        assert_eq!(soulsilver.marill_cry_script_entries.len(), 4);
        assert_eq!(soulsilver.tm_texts_frontier.len(), 16);
    }

    #[test]
    fn ignore_game_corner_statics_drops_them_when_copying() {
        for entry in rom_entries()
            .iter()
            .filter(|entry| entry.ignore_game_corner_statics)
        {
            assert!(
                entry.static_pokemon.iter().all(|s| !s.is_game_corner()),
                "{}",
                entry.name
            );
        }
        assert!(
            rom_entries()
                .iter()
                .any(|entry| entry.ignore_game_corner_statics)
        );
    }

    #[test]
    fn every_region_detects_by_code_and_version() {
        for entry in rom_entries() {
            let found = entry_for(&entry.rom_code, entry.version).expect("detectable");
            assert_eq!(found.rom_code, entry.rom_code);
        }
        assert!(named("Platinum (U Rev 1)").version != named("Platinum (U)").version);
    }
}
