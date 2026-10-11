use std::collections::HashMap;
use std::sync::OnceLock;

use randomon_gb::{
    IniLine, ini_lines, parse_int_array, parse_ri_int, parse_ri_long, parse_static_offsets,
};

use crate::assets::OFFSETS_INI;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RomType {
    #[default]
    Ruby,
    Sapphire,
    Emerald,
    FireRedLeafGreen,
}

impl RomType {
    fn parse(text: &str) -> Option<Self> {
        let lowered = text.to_ascii_lowercase();
        match lowered.as_str() {
            "ruby" => Some(Self::Ruby),
            "sapp" => Some(Self::Sapphire),
            "em" => Some(Self::Emerald),
            "frlg" => Some(Self::FireRedLeafGreen),
            _ => None,
        }
    }

    pub fn is_ruby_or_sapphire(self) -> bool {
        matches!(self, Self::Ruby | Self::Sapphire)
    }

    pub fn is_frlg(self) -> bool {
        self == Self::FireRedLeafGreen
    }

    pub fn is_emerald(self) -> bool {
        self == Self::Emerald
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StaticPokemonEntry {
    pub species_offsets: Vec<i32>,
    pub level_offsets: Vec<i32>,
}

impl StaticPokemonEntry {
    fn parse(text: &str) -> Self {
        let offsets = parse_static_offsets(text);
        Self {
            species_offsets: offsets.species_offsets,
            level_offsets: offsets.level_offsets,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptTextEntry {
    pub number: i32,
    pub map_bank: i32,
    pub map_number: i32,
    pub person_number: i32,
    pub offset_in_script: i32,
    pub actual_offset: i32,
    pub template: String,
    pub is_move_tutor: bool,
}

impl ScriptTextEntry {
    fn parse(value: &str, is_move_tutor: bool) -> Option<Self> {
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        let parts: Vec<&str> = inner.splitn(6, ',').collect();
        let [number, bank, map, person, offset, template] = parts.as_slice() else {
            return None;
        };
        Some(Self {
            number: parse_ri_int(number),
            map_bank: parse_ri_int(bank),
            map_number: parse_ri_int(map),
            person_number: parse_ri_int(person),
            offset_in_script: parse_ri_int(offset),
            actual_offset: 0,
            template: (*template).to_owned(),
            is_move_tutor,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct RomEntry {
    pub name: String,
    pub rom_code: String,
    pub table_file: Option<String>,
    pub version: i32,
    pub rom_type: RomType,
    pub copy_static_pokemon: bool,
    pub entries: HashMap<String, i32>,
    pub array_entries: HashMap<String, Vec<i32>>,
    pub strings: HashMap<String, String>,
    pub static_pokemon: Vec<StaticPokemonEntry>,
    pub roaming_pokemon: Vec<StaticPokemonEntry>,
    pub script_texts: Vec<ScriptTextEntry>,
    pub code_tweaks: HashMap<String, String>,
    pub expected_crc32: i64,
}

impl RomEntry {
    fn new(name: String) -> Self {
        Self {
            name,
            expected_crc32: -1,
            ..Self::default()
        }
    }

    pub fn value(&self, key: &str) -> i32 {
        self.entries.get(key).copied().unwrap_or(0)
    }

    pub fn set_value(&mut self, key: &str, value: i32) {
        self.entries.insert(key.to_owned(), value);
    }

    pub fn array(&self, key: &str) -> &[i32] {
        self.array_entries.get(key).map_or(&[], Vec::as_slice)
    }

    pub fn has_array(&self, key: &str) -> bool {
        self.array_entries.contains_key(key)
    }

    pub fn string(&self, key: &str) -> Option<&str> {
        self.strings.get(key).map(String::as_str)
    }

    pub fn tweak(&self, key: &str) -> Option<&str> {
        self.code_tweaks.get(key).map(String::as_str)
    }

    pub fn region_letter(&self) -> Option<char> {
        self.rom_code.chars().nth(3)
    }
}

fn apply_copy_from(current: &mut RomEntry, source: &RomEntry) {
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
    let copy_tm_text = current.value("CopyTMText") == 1;
    if current.copy_static_pokemon {
        current
            .static_pokemon
            .extend(source.static_pokemon.iter().cloned());
        current
            .roaming_pokemon
            .extend(source.roaming_pokemon.iter().cloned());
        current.set_value("StaticPokemonSupport", 1);
    } else {
        current.set_value("StaticPokemonSupport", 0);
    }
    if copy_tm_text {
        current
            .script_texts
            .extend(source.script_texts.iter().cloned());
    }
    current.table_file = source.table_file.clone();
}

fn apply_value_line(entry: &mut RomEntry, key: &str, value: &str) {
    if key.ends_with("Locator") || key.ends_with("Prefix") {
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
            .push(StaticPokemonEntry::parse(value)),
        "RoamingPokemon{}" => current
            .roaming_pokemon
            .push(StaticPokemonEntry::parse(value)),
        "TMText[]" => current
            .script_texts
            .extend(ScriptTextEntry::parse(value, false)),
        "MoveTutorText[]" => current
            .script_texts
            .extend(ScriptTextEntry::parse(value, true)),
        "Game" => current.rom_code = value.to_owned(),
        "Version" => current.version = parse_ri_int(value),
        "Type" => {
            if let Some(rom_type) = RomType::parse(value) {
                current.rom_type = rom_type;
            }
        }
        "TableFile" => current.table_file = Some(value.to_owned()),
        "CopyStaticPokemon" => current.copy_static_pokemon = parse_ri_int(value) > 0,
        "CRC32" => current.expected_crc32 = parse_ri_long(&format!("0x{value}")),
        _ if key.ends_with("Tweak") => {
            current.code_tweaks.insert(key.to_owned(), value.to_owned());
        }
        "CopyFrom" => {
            for other in previous.iter() {
                if value.eq_ignore_ascii_case(&other.name) {
                    apply_copy_from(current, other);
                }
            }
        }
        _ => apply_value_line(current, key, value),
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

    pub fn entry(name: &str) -> &'static RomEntry {
        rom_entries()
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("entry {name} exists"))
    }

    #[test]
    fn every_section_becomes_an_entry() {
        assert_eq!(rom_entries().len(), 46);
        let codes: Vec<&str> = rom_entries().iter().map(|e| e.rom_code.as_str()).collect();
        for code in [
            "AXVE", "AXPE", "BPEE", "BPRE", "BPGE", "AXVJ", "BPEJ", "BPET",
        ] {
            assert!(codes.contains(&code), "{code} is supported");
        }
    }

    #[test]
    fn ruby_entry_is_parsed() {
        let ruby = entry("Ruby (U)");
        assert_eq!(ruby.rom_code, "AXVE");
        assert_eq!(ruby.rom_type, RomType::Ruby);
        assert_eq!(ruby.table_file.as_deref(), Some("gba_english"));
        assert_eq!(ruby.value("PokemonStats"), 0x1FEC18);
        assert_eq!(ruby.value("TrainerCount"), 0x2B6);
        assert_eq!(ruby.array("EliteFourIndices"), &[261, 262, 263, 264, 335]);
        assert_eq!(ruby.array("TradesUnused"), &[] as &[i32]);
        assert_eq!(
            ruby.string("PickupTableStartLocator"),
            Some("16001E00170028000200320044003C")
        );
        assert_eq!(ruby.static_pokemon.len(), 18);
        assert_eq!(
            ruby.static_pokemon[2].species_offsets,
            vec![0x1A04A3, 0x15DE26, 0x15DE2D]
        );
        assert!(ruby.static_pokemon[15].level_offsets.is_empty());
        assert_eq!(ruby.roaming_pokemon.len(), 1);
        assert_eq!(
            ruby.roaming_pokemon[0].level_offsets,
            vec![0x13425C, 0x13426C]
        );
        assert_eq!(ruby.expected_crc32, 0xF081_5EE7);
        assert_eq!(
            ruby.tweak("StaticFirstBattleTweak"),
            Some("hardcoded_statics/rs_firstbattle")
        );
        let tm = &ruby.script_texts[1];
        assert_eq!(
            (tm.number, tm.map_bank, tm.map_number, tm.person_number),
            (4, 14, 0, 1)
        );
        assert_eq!(tm.offset_in_script, 0x74);
        assert!(tm.template.contains("LIZA: [move]!"));
        assert!(!tm.is_move_tutor);
    }

    #[test]
    fn copied_entries_inherit_values_and_tm_texts() {
        let ruby_eu = entry("Ruby (E)");
        assert_eq!(ruby_eu.value("PokemonStats"), 0x1FEC30);
        assert_eq!(ruby_eu.value("TrainerEntrySize"), 40);
        assert_eq!(
            ruby_eu.script_texts.len(),
            entry("Ruby (U)").script_texts.len()
        );
        assert_eq!(ruby_eu.static_pokemon.len(), 18);
        assert_eq!(ruby_eu.value("StaticPokemonSupport"), 1);
        let ruby_12 = entry("Ruby (U/E) 1.2");
        assert!(ruby_12.copy_static_pokemon);
        assert_eq!(ruby_12.static_pokemon.len(), 18);
        assert_eq!(ruby_12.roaming_pokemon.len(), 1);
        assert_eq!(ruby_12.value("StaticPokemonSupport"), 1);
        assert_eq!(ruby_12.version, 2);
    }

    #[test]
    fn frlg_entries_have_tutors_and_roamers() {
        let fire_red = entry("Fire Red (U) 1.0");
        assert_eq!(fire_red.rom_type, RomType::FireRedLeafGreen);
        assert_eq!(fire_red.value("MoveTutorMoves"), 15);
        assert_eq!(fire_red.roaming_pokemon.len(), 3);
        assert!(fire_red.script_texts.iter().any(|text| text.is_move_tutor));
        let leaf_green = entry("Leaf Green (U) 1.0");
        assert_eq!(
            leaf_green.array("BattleTrappersBanned"),
            &[55, 56, 57, 58, 59]
        );
        assert_eq!(leaf_green.value("GhostMarowakOffset"), 25);
    }

    #[test]
    fn translated_emerald_inherits_from_the_japanese_release() {
        let translated = entry("Emerald (T-Eng)");
        let japanese = entry("Emerald (J)");
        assert_eq!(translated.rom_type, RomType::Emerald);
        assert_eq!(translated.table_file, japanese.table_file);
        assert_eq!(
            translated.static_pokemon.len(),
            japanese.static_pokemon.len()
        );
    }
}
