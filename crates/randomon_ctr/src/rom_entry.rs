use std::collections::{BTreeMap, HashMap};

use randomon_settings::java_text::{java_trim, scanner_lines};

#[derive(Clone, Debug, Default)]
pub struct RomFileEntry {
    pub path: String,
    pub expected_crc32s: [i64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPrecedence {
    ArraysBeforeNumbers,
    NumbersBeforeArrays,
}

#[derive(Clone, Debug, Default)]
pub struct CtrRomEntry {
    pub name: String,
    pub rom_code: String,
    pub title_id: String,
    pub acronym: String,
    pub type_name: String,
    pub expected_code_crc32s: [i64; 2],
    pub files: BTreeMap<String, RomFileEntry>,
    pub linked_static_offsets: BTreeMap<i32, i32>,
    pub strings: HashMap<String, String>,
    pub numbers: HashMap<String, i32>,
    pub array_entries: HashMap<String, Vec<i32>>,
}

impl CtrRomEntry {
    pub fn int(&self, key: &str) -> i32 {
        self.numbers.get(key).copied().unwrap_or(0)
    }

    pub fn number(&self, key: &str) -> Option<i32> {
        self.numbers.get(key).copied()
    }

    pub fn string(&self, key: &str) -> &str {
        self.strings.get(key).map_or("", String::as_str)
    }

    pub fn file(&self, key: &str) -> Option<&str> {
        self.files.get(key).map(|entry| entry.path.as_str())
    }

    pub fn array(&self, key: &str) -> &[i32] {
        self.array_entries.get(key).map_or(&[], Vec::as_slice)
    }

    pub fn has_array(&self, key: &str) -> bool {
        self.array_entries.contains_key(key)
    }
}

fn parse_with_radix(text: &str) -> (String, u32) {
    let lowered = java_trim(text).to_lowercase();
    if let Some(stripped) = lowered
        .strip_prefix("0x")
        .or_else(|| lowered.strip_prefix("&h"))
    {
        (stripped.to_owned(), 16)
    } else {
        (lowered, 10)
    }
}

pub fn parse_ri_int(text: &str) -> i32 {
    let (digits, radix) = parse_with_radix(text);
    i32::from_str_radix(&digits, radix).unwrap_or(0)
}

pub fn parse_ri_long(text: &str) -> i64 {
    let (digits, radix) = parse_with_radix(text);
    i64::from_str_radix(&digits, radix).unwrap_or(0)
}

fn bracket_contents(value: &str) -> &str {
    let mut characters = value.char_indices();
    let start = characters
        .next()
        .map_or(0, |(index, ch)| index + ch.len_utf8());
    let end = value
        .char_indices()
        .last()
        .map_or(value.len(), |(index, _)| index);
    value.get(start..end.max(start)).unwrap_or("")
}

fn java_split<'a>(text: &'a str, separator: &str) -> Vec<&'a str> {
    if !text.contains(separator) {
        return vec![text];
    }
    let mut parts: Vec<&str> = text.split(separator).collect();
    while parts.last() == Some(&"") {
        parts.pop();
    }
    parts
}

fn parse_file_entry(value: &str) -> RomFileEntry {
    let values = java_split(bracket_contents(value), ",");
    let crc_text = format!(
        "{}, {}",
        java_trim(values.get(1).copied().unwrap_or("")),
        java_trim(values.get(2).copied().unwrap_or(""))
    );
    let crcs = java_split(bracket_contents(&crc_text), ",");
    let crc_at = |index: usize| {
        parse_ri_long(&format!(
            "0x{}",
            java_trim(crcs.get(index).copied().unwrap_or(""))
        ))
    };
    RomFileEntry {
        path: java_trim(values[0]).to_owned(),
        expected_crc32s: [crc_at(0), crc_at(1)],
    }
}

fn parse_array(value: &str) -> Vec<i32> {
    let offsets = java_split(bracket_contents(value), ",");
    if offsets.len() == 1 && java_trim(offsets[0]).is_empty() {
        return Vec::new();
    }
    offsets.into_iter().map(parse_ri_int).collect()
}

fn is_number_key(key: &str) -> bool {
    key.ends_with("Offset") || key.ends_with("Count") || key.ends_with("Number")
}

fn is_array_value(value: &str) -> bool {
    value.starts_with('[') && value.ends_with(']')
}

fn copy_from(current: &mut CtrRomEntry, other: &CtrRomEntry) {
    current
        .linked_static_offsets
        .extend(other.linked_static_offsets.clone());
    current.array_entries.extend(other.array_entries.clone());
    current.numbers.extend(other.numbers.clone());
    current.strings.extend(other.strings.clone());
    current.files.extend(other.files.clone());
}

fn apply_line(entries: &mut [CtrRomEntry], key: &str, value: &str, precedence: KeyPrecedence) {
    let (previous, rest) = entries.split_at_mut(entries.len() - 1);
    let current = &mut rest[0];
    match key {
        "Game" => current.rom_code = value.to_owned(),
        "Type" => current.type_name = value.to_owned(),
        "TitleId" => current.title_id = value.to_owned(),
        "Acronym" => current.acronym = value.to_owned(),
        "CopyFrom" if precedence == KeyPrecedence::ArraysBeforeNumbers => {
            for other in previous.iter() {
                if other.rom_code.eq_ignore_ascii_case(value) {
                    copy_from(current, other);
                }
            }
        }
        _ if key.starts_with("File<") => {
            let name = key
                .split('<')
                .nth(1)
                .and_then(|rest| rest.split('>').next())
                .unwrap_or("")
                .to_owned();
            current.files.insert(name, parse_file_entry(value));
        }
        "CodeCRC32" => {
            let values = java_split(bracket_contents(value), ",");
            let crc_at = |index: usize| {
                parse_ri_long(&format!(
                    "0x{}",
                    java_trim(values.get(index).copied().unwrap_or(""))
                ))
            };
            current.expected_code_crc32s = [crc_at(0), crc_at(1)];
        }
        "LinkedStaticEncounterOffsets" => {
            for pair in java_split(bracket_contents(value), ",") {
                let parts = java_split(pair, ":");
                let parse = |text: &str| java_trim(text).parse::<i32>().unwrap_or(0);
                current
                    .linked_static_offsets
                    .insert(parse(parts[0]), parse(parts.get(1).copied().unwrap_or("")));
            }
        }
        _ => apply_generic_line(previous, current, key, value, precedence),
    }
}

fn apply_generic_line(
    previous: &[CtrRomEntry],
    current: &mut CtrRomEntry,
    key: &str,
    value: &str,
    precedence: KeyPrecedence,
) {
    let numbers_first = precedence == KeyPrecedence::NumbersBeforeArrays;
    if numbers_first && is_number_key(key) {
        current.numbers.insert(key.to_owned(), parse_ri_int(value));
    } else if is_array_value(value) {
        current
            .array_entries
            .insert(key.to_owned(), parse_array(value));
    } else if !numbers_first && is_number_key(key) {
        current.numbers.insert(key.to_owned(), parse_ri_int(value));
    } else if numbers_first && key == "CopyFrom" {
        for other in previous {
            if other.rom_code.eq_ignore_ascii_case(value) {
                copy_from(current, other);
            }
        }
    } else {
        current.strings.insert(key.to_owned(), value.to_owned());
    }
}

pub fn parse_rom_entries(ini: &str, precedence: KeyPrecedence) -> Vec<CtrRomEntry> {
    let mut entries: Vec<CtrRomEntry> = Vec::new();
    for line in scanner_lines(ini) {
        let mut line = java_trim(line);
        if let Some(index) = line.find("//") {
            line = java_trim(&line[..index]);
        }
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            entries.push(CtrRomEntry {
                name: bracket_contents(line).to_owned(),
                ..CtrRomEntry::default()
            });
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if entries.is_empty() {
            continue;
        }
        apply_line(&mut entries, key, java_trim(value), precedence);
    }
    entries
}

pub fn find_entry<'a>(
    entries: &'a [CtrRomEntry],
    product_code: Option<&str>,
    title_id: Option<&str>,
) -> Option<&'a CtrRomEntry> {
    let (product_code, title_id) = (product_code?, title_id?);
    entries
        .iter()
        .find(|entry| entry.rom_code == product_code && entry.title_id == title_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[First]\nGame=CTR-P-AAAA\nTitleId=0004000000000100\nType=XY\nAcronym=A\nFile<Text>=<a/0/7/4, [D82C84F1, 37523125]>\nCodeCRC32=[3778C475, BA7B14F7]\nLinkedStaticEncounterOffsets=[1:3, 2:12]\nStarterIndices=[0,1,2]\nEmpty=[]\nStaticPokemonOffset=0xEE46C // trailing\nName=Text\n\n[Second]\nGame=CTR-P-BBBB\nCopyFrom=ctr-p-aaaa\nStaticPokemonOffset=12\n";

    #[test]
    fn parses_entries_and_copies() {
        let entries = parse_rom_entries(SAMPLE, KeyPrecedence::ArraysBeforeNumbers);
        assert_eq!(entries.len(), 2);
        let first = &entries[0];
        assert_eq!(first.name, "First");
        assert_eq!(first.file("Text"), Some("a/0/7/4"));
        assert_eq!(
            first.files["Text"].expected_crc32s,
            [0xD82C84F1, 0x37523125]
        );
        assert_eq!(first.expected_code_crc32s, [0x3778C475, 0xBA7B14F7]);
        assert_eq!(first.linked_static_offsets.get(&2), Some(&12));
        assert_eq!(first.array("StarterIndices"), &[0, 1, 2]);
        assert!(first.array("Empty").is_empty());
        assert!(first.has_array("Empty"));
        assert_eq!(first.int("StaticPokemonOffset"), 0xEE46C);
        assert_eq!(first.string("Name"), "Text");
        let second = &entries[1];
        assert_eq!(second.int("StaticPokemonOffset"), 12);
        assert_eq!(second.array("StarterIndices"), &[0, 1, 2]);
        assert_eq!(second.file("Text"), Some("a/0/7/4"));
    }
}
