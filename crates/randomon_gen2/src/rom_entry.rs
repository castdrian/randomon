use std::collections::HashMap;
use std::sync::OnceLock;

use randomon_gb::{
    IniLine, ini_lines, parse_int_array, parse_ri_int, parse_ri_long, parse_static_offsets,
};

const OFFSETS_INI: &str = include_str!("../assets/config/gen2_offsets.ini");

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StaticPokemonEntry {
    pub species_offsets: Vec<i32>,
    pub level_offsets: Vec<i32>,
    pub is_game_corner: bool,
}

impl StaticPokemonEntry {
    fn parse(text: &str, is_game_corner: bool) -> Self {
        let offsets = parse_static_offsets(text);
        Self {
            species_offsets: offsets.species_offsets,
            level_offsets: offsets.level_offsets,
            is_game_corner,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TmTextEntry {
    pub number: i32,
    pub offset: i32,
    pub template: String,
}

#[derive(Clone, Debug, Default)]
pub struct RomEntry {
    pub name: String,
    pub rom_code: Option<String>,
    pub version: i32,
    pub non_japanese: i32,
    pub extra_table_file: Option<String>,
    pub is_crystal: bool,
    pub expected_crc32: i64,
    pub crc_in_header: i32,
    pub code_tweaks: HashMap<String, String>,
    pub tm_texts: Vec<TmTextEntry>,
    pub entries: HashMap<String, i32>,
    pub array_entries: HashMap<String, Vec<i32>>,
    pub strings: HashMap<String, String>,
    pub static_pokemon: Vec<StaticPokemonEntry>,
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

    pub fn has_array(&self, key: &str) -> bool {
        self.array_entries.contains_key(key)
    }

    pub fn string(&self, key: &str) -> &str {
        self.strings.get(key).map_or("", String::as_str)
    }
}

fn parse_tm_text(value: &str) -> Option<TmTextEntry> {
    if !(value.starts_with('[') && value.ends_with(']')) || value.len() < 2 {
        return None;
    }
    let parts: Vec<&str> = value[1..value.len() - 1].splitn(3, ',').collect();
    (parts.len() == 3).then(|| TmTextEntry {
        number: parse_ri_int(parts[0]),
        offset: parse_ri_int(parts[1]),
        template: parts[2].to_owned(),
    })
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
    current.strings.extend(
        source
            .strings
            .iter()
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    if copy_static {
        current
            .static_pokemon
            .extend(source.static_pokemon.iter().cloned());
        current.entries.insert("StaticPokemonSupport".to_owned(), 1);
    } else {
        current.entries.insert("StaticPokemonSupport".to_owned(), 0);
        current.entries.remove("StaticPokemonOddEggOffset");
        current.entries.remove("StaticPokemonOddEggDataSize");
    }
    if copy_tm_text {
        current.tm_texts.extend(source.tm_texts.iter().cloned());
    }
    current.extra_table_file = source.extra_table_file.clone();
}

fn apply_header_line(entry: &mut RomEntry, key: &str, value: &str) -> bool {
    match key {
        "Game" => entry.rom_code = Some(value.to_owned()),
        "Version" => entry.version = parse_ri_int(value),
        "NonJapanese" => entry.non_japanese = parse_ri_int(value),
        "Type" => entry.is_crystal = value.eq_ignore_ascii_case("Crystal"),
        "ExtraTableFile" => entry.extra_table_file = Some(value.to_owned()),
        "CRCInHeader" => entry.crc_in_header = parse_ri_int(value),
        "CRC32" => entry.expected_crc32 = parse_ri_long(&format!("0x{value}")),
        _ => return false,
    }
    true
}

fn apply_value_line(entry: &mut RomEntry, key: &str, value: &str) {
    if key.ends_with("Tweak") {
        entry.code_tweaks.insert(key.to_owned(), value.to_owned());
    } else if key.ends_with("Locator") || key.ends_with("Prefix") {
        entry.strings.insert(key.to_owned(), value.to_owned());
    } else if let Some(parsed) = parse_int_array(value) {
        entry.array_entries.insert(key.to_owned(), parsed);
    } else {
        entry.entries.insert(key.to_owned(), parse_ri_int(value));
    }
}

fn apply_line(roms: &mut [RomEntry], key: &str, value: &str) {
    let Some(current_index) = roms.len().checked_sub(1) else {
        return;
    };
    let (previous, current) = roms.split_at_mut(current_index);
    let current = &mut current[0];
    match key {
        "StaticPokemon{}" => current
            .static_pokemon
            .push(StaticPokemonEntry::parse(value, false)),
        "StaticPokemonGameCorner{}" => current
            .static_pokemon
            .push(StaticPokemonEntry::parse(value, true)),
        "TMText[]" => current.tm_texts.extend(parse_tm_text(value)),
        "CopyFrom" => {
            for other in previous.iter() {
                if value.eq_ignore_ascii_case(&other.name) {
                    apply_copy_from(current, other);
                }
            }
        }
        _ => {
            if !apply_header_line(current, key, value) {
                apply_value_line(current, key, value);
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
        assert_eq!(rom_entries().len(), 22);
    }

    #[test]
    fn gold_entry_is_parsed() {
        let gold = entry("Gold (U)");
        assert!(!gold.is_crystal);
        assert_eq!(gold.rom_code.as_deref(), Some("AAUE"));
        assert_eq!(gold.expected_crc32, 0x6BDE_3C3E);
        assert_eq!(gold.value("PokemonStatsOffset"), 0x51B0B);
        assert_eq!(
            gold.array("StarterHeldItems"),
            &[0x1800F8, 0x18013A, 0x180176]
        );
        assert_eq!(gold.array("TradesUnused"), &[] as &[i32]);
        assert_eq!(gold.string("GuaranteedCatchPrefix"), "D147FA19D1FE03");
        assert_eq!(gold.static_pokemon.len(), 26);
        assert!(gold.static_pokemon[20].is_game_corner);
        assert_eq!(gold.static_pokemon[14].species_offsets, vec![0x15924F]);
        assert!(gold.static_pokemon[14].level_offsets.is_empty());
        assert_eq!(gold.tm_texts[0].template, "That is\\n%m.\\e");
        assert_eq!(
            gold.code_tweaks.get("BWXPTweak").map(String::as_str),
            Some("bwexp/gs_en_bwxp")
        );
    }

    #[test]
    fn copied_entries_inherit_and_drop_odd_eggs() {
        let crystal = entry("Crystal (U 1.1)");
        assert!(crystal.is_crystal);
        assert_eq!(crystal.value("StaticPokemonSupport"), 1);
        assert_eq!(crystal.value("StaticPokemonOddEggOffset"), 0x1FB56E);
        assert_eq!(
            crystal.static_pokemon.len(),
            entry("Crystal (U)").static_pokemon.len()
        );
        let gold_jp = entry("Gold (J 1.1)");
        assert_eq!(gold_jp.value("StaticPokemonSupport"), 0);
        assert!(gold_jp.extra_table_file.is_none());
    }
}
