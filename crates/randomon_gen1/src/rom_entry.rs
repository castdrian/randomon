use std::collections::HashMap;
use std::sync::OnceLock;

use randomon_data::Type;
use randomon_gb::{
    IniLine, ini_lines, parse_int_array, parse_ri_int, parse_ri_long, parse_static_offsets,
};

const OFFSETS_INI: &str = include_str!("../assets/config/gen1_offsets.ini");

#[derive(Clone, Debug, Default)]
pub struct StaticPokemonEntry {
    pub species_offsets: Vec<i32>,
    pub level_offsets: Vec<i32>,
}

#[derive(Clone, Debug)]
pub struct TmTextEntry {
    pub number: i32,
    pub offset: i32,
    pub template: String,
}

#[derive(Clone, Debug, Default)]
pub struct RomEntry {
    pub name: String,
    pub rom_name: Option<String>,
    pub version: i32,
    pub non_japanese: i32,
    pub extra_table_file: Option<String>,
    pub is_yellow: bool,
    pub expected_crc32: i64,
    pub crc_in_header: i32,
    pub tweak_files: HashMap<String, String>,
    pub tm_texts: Vec<TmTextEntry>,
    pub entries: HashMap<String, i32>,
    pub array_entries: HashMap<String, Vec<i32>>,
    pub static_pokemon: Vec<StaticPokemonEntry>,
    pub ghost_marowak_offsets: Vec<i32>,
    pub extra_type_lookup: HashMap<i32, Type>,
    pub extra_type_reverse: HashMap<Type, i32>,
}

impl RomEntry {
    fn new(name: String) -> Self {
        Self {
            name,
            expected_crc32: -1,
            crc_in_header: -1,
            ..Self::default()
        }
    }

    pub fn value(&self, key: &str) -> i32 {
        self.entries.get(key).copied().unwrap_or(0)
    }

    pub fn array(&self, key: &str) -> &[i32] {
        self.array_entries.get(key).map_or(&[], Vec::as_slice)
    }
}

fn parse_static_pokemon(text: &str) -> StaticPokemonEntry {
    let offsets = parse_static_offsets(text);
    StaticPokemonEntry {
        species_offsets: offsets.species_offsets,
        level_offsets: offsets.level_offsets,
    }
}

fn apply_copy_from(current: &mut RomEntry, source: &RomEntry) {
    let copy_static = current.value("CopyStaticPokemon") == 1;
    let copy_tm_text = current.value("CopyTMText") == 1;
    current.array_entries.extend(
        source
            .array_entries
            .iter()
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    current.entries.extend(
        source
            .entries
            .iter()
            .map(|(key, value)| (key.clone(), *value)),
    );
    if copy_static {
        current
            .static_pokemon
            .extend(source.static_pokemon.iter().cloned());
        current.ghost_marowak_offsets = source.ghost_marowak_offsets.clone();
        current.entries.insert("StaticPokemonSupport".to_owned(), 1);
    } else {
        current.entries.insert("StaticPokemonSupport".to_owned(), 0);
    }
    if copy_tm_text {
        current.tm_texts.extend(source.tm_texts.iter().cloned());
    }
    current.extra_table_file = source.extra_table_file.clone();
}

fn apply_line(roms: &mut [RomEntry], key: &str, value: &str) {
    let Some(current_index) = roms.len().checked_sub(1) else {
        return;
    };
    match key {
        "StaticPokemon{}" => {
            let parsed = parse_static_pokemon(value);
            roms[current_index].static_pokemon.push(parsed);
        }
        "StaticPokemonGhostMarowak{}" => {
            let parsed = parse_static_pokemon(value);
            roms[current_index].ghost_marowak_offsets = parsed.species_offsets.clone();
            roms[current_index].static_pokemon.push(parsed);
        }
        "TMText[]" => {
            if value.starts_with('[') && value.ends_with(']') {
                let inner = &value[1..value.len() - 1];
                let parts: Vec<&str> = inner.splitn(3, ',').collect();
                if parts.len() == 3 {
                    roms[current_index].tm_texts.push(TmTextEntry {
                        number: parse_ri_int(parts[0]),
                        offset: parse_ri_int(parts[1]),
                        template: parts[2].to_owned(),
                    });
                }
            }
        }
        "Game" => roms[current_index].rom_name = Some(value.to_owned()),
        "Version" => roms[current_index].version = parse_ri_int(value),
        "NonJapanese" => roms[current_index].non_japanese = parse_ri_int(value),
        "Type" => roms[current_index].is_yellow = value.eq_ignore_ascii_case("Yellow"),
        "ExtraTableFile" => roms[current_index].extra_table_file = Some(value.to_owned()),
        "CRCInHeader" => roms[current_index].crc_in_header = parse_ri_int(value),
        "CRC32" => roms[current_index].expected_crc32 = parse_ri_long(&format!("0x{value}")),
        "ExtraTypes" => {
            let inner = if value.len() >= 2 {
                &value[1..value.len() - 1]
            } else {
                ""
            };
            for part in inner.split(',') {
                let mut pieces = part.split('=');
                let (Some(id), Some(name)) = (pieces.next(), pieces.next()) else {
                    continue;
                };
                if let (Ok(id), Some(kind)) = (
                    i32::from_str_radix(id, 16),
                    Type::from_java_name(name.trim()),
                ) {
                    roms[current_index].extra_type_lookup.insert(id, kind);
                    roms[current_index].extra_type_reverse.insert(kind, id);
                }
            }
        }
        "CopyFrom" => {
            let (previous, current) = roms.split_at_mut(current_index);
            let current = &mut current[0];
            for other in previous.iter() {
                if value.eq_ignore_ascii_case(&other.name) {
                    apply_copy_from(current, other);
                }
            }
        }
        _ if key.ends_with("Tweak") => {
            roms[current_index]
                .tweak_files
                .insert(key.to_owned(), value.to_owned());
        }
        _ => {
            if let Some(parsed) = parse_int_array(value) {
                roms[current_index]
                    .array_entries
                    .insert(key.to_owned(), parsed);
            } else {
                roms[current_index]
                    .entries
                    .insert(key.to_owned(), parse_ri_int(value));
            }
        }
    }
}

fn load_rom_info() -> Vec<RomEntry> {
    let mut roms: Vec<RomEntry> = Vec::new();
    for line in ini_lines(OFFSETS_INI) {
        match line {
            IniLine::Section(name) => roms.push(RomEntry::new(name.to_owned())),
            IniLine::Entry(key, value) => apply_line(&mut roms, key, value),
        }
    }
    roms
}

pub fn rom_entries() -> &'static [RomEntry] {
    static ENTRIES: OnceLock<Vec<RomEntry>> = OnceLock::new();
    ENTRIES.get_or_init(load_rom_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> &'static RomEntry {
        rom_entries()
            .iter()
            .find(|entry| entry.name == name)
            .expect("entry exists")
    }

    #[test]
    fn every_section_becomes_an_entry() {
        assert_eq!(rom_entries().len(), 20);
    }

    #[test]
    fn yellow_copies_red_and_overrides_offsets() {
        let yellow = entry("Yellow (U)");
        assert!(yellow.is_yellow);
        assert_eq!(yellow.rom_name.as_deref(), Some("POKEMON YELLOW"));
        assert_eq!(yellow.expected_crc32, 0x7D52_7D62);
        assert_eq!(yellow.value("PokedexOrder"), 0x410B1);
        assert_eq!(yellow.value("StaticPokemonSupport"), 1);
        assert_eq!(
            yellow.value("InternalPokemonCount"),
            entry("Red (U)").value("InternalPokemonCount")
        );
        assert_eq!(
            yellow.array("StarterOffsets1"),
            &[0x18F19, 0x1CB41, 0x1CB66]
        );
        assert_eq!(yellow.array("TradesUnused"), &[2, 4, 6]);
        assert_eq!(
            yellow.tweak_files.get("BWXPTweak").map(String::as_str),
            Some("bwexp/yellow_en_bwxp")
        );
    }

    #[test]
    fn static_pokemon_and_tm_texts_are_parsed() {
        let yellow = entry("Yellow (U)");
        let bulbasaur = &yellow.static_pokemon[1];
        assert_eq!(bulbasaur.species_offsets, vec![0x1CF7B, 0x1CF8C, 0x1CFEB]);
        assert_eq!(bulbasaur.level_offsets, vec![0x1CF8B]);
        assert_eq!(yellow.ghost_marowak_offsets.len(), 6);
        let first_text = &yellow.tm_texts[0];
        assert_eq!(first_text.number, 6);
        assert_eq!(first_text.offset, 0x0B0EE4);
        assert_eq!(first_text.template, "\\pTM06 contains\\n%m!\\e");
    }
}
