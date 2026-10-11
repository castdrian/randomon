use std::collections::BTreeMap;

use randomon_data::constants::species;

use crate::assets::table_text;
use crate::constants as c;
use crate::items;
use crate::rom::{POINTER_BASE, pointer_to_hex};
use crate::rom_entry::{RomEntry, RomType, rom_entries};
use crate::text::TextTable;

pub const SYNTHETIC_SIZE: usize = c::SIZE_16M;
const CODE_AREA_START: i32 = 0x200;
const CODE_AREA_END: i32 = 0x7000;
const DATA_GAP_AFTER_FREE_SPACE: i32 = 0x10_0000;
const REAL_POKEMON: i32 = 386;
const JOHTO_END: i32 = 251;
const HOENN_INTERNAL_START: i32 = 277;
const HOENN_DEX_START: i32 = 252;
const WILD_HEADER_COUNT: i32 = 26;
const TYPE_BYTES: [u8; 17] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
    0x11,
];
const GENDER_RATIOS: [u8; 7] = [0x00, 0x1F, 0x3F, 0x7F, 0xBF, 0xFE, 0xFF];
const EFFECTS: [u8; 24] = [
    0, 0, 0, 1, 2, 4, 6, 10, 18, 31, 33, 43, 48, 50, 68, 76, 80, 118, 148, 166, 182, 198, 204, 212,
];
const SPECIAL_HOENN: [(i32, i32); 8] = [
    (404, species::KYOGRE),
    (405, species::GROUDON),
    (406, species::RAYQUAZA),
    (407, species::LATIAS),
    (408, species::LATIOS),
    (409, species::JIRACHI),
    (410, species::DEOXYS),
    (411, species::CHIMECHO),
];

#[derive(Clone, Copy, Debug)]
pub struct SyntheticOptions {
    pub forge_crc: bool,
    pub hack_pointers: bool,
    pub jambo_movesets: bool,
}

impl Default for SyntheticOptions {
    fn default() -> Self {
        Self {
            forge_crc: true,
            hack_pointers: false,
            jambo_movesets: false,
        }
    }
}

pub fn entry_names() -> Vec<&'static str> {
    rom_entries()
        .iter()
        .map(|entry| entry.name.as_str())
        .collect()
}

pub fn expected_crc32(name: &str) -> i64 {
    entry_named(name).expected_crc32
}

pub fn static_entry_count(name: &str) -> usize {
    entry_named(name).static_pokemon.len()
}

pub fn static_has_level(name: &str, index: usize) -> bool {
    entry_named(name)
        .static_pokemon
        .get(index)
        .is_some_and(|entry| !entry.level_offsets.is_empty())
}

pub fn roamer_count(name: &str) -> usize {
    entry_named(name).roaming_pokemon.len()
}

pub fn has_tweak(name: &str, key: &str) -> bool {
    entry_named(name).tweak(key).is_some()
}

pub fn encode_english(text: &str) -> Vec<u8> {
    TextTable::from_text(table_text("gba_english").expect("table")).translate(text)
}

fn entry_named(name: &str) -> &'static RomEntry {
    rom_entries()
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("no entry named {name}"))
}

pub fn build(name: &str) -> Vec<u8> {
    build_with(name, SyntheticOptions::default())
}

pub fn build_with(name: &str, options: SyntheticOptions) -> Vec<u8> {
    let mut builder = Builder::new(entry_named(name), options);
    builder.build_all();
    builder.finish()
}

pub fn internal_to_dex(internal: i32) -> i32 {
    if internal <= JOHTO_END {
        return internal;
    }
    if internal < HOENN_INTERNAL_START {
        return REAL_POKEMON + (internal - JOHTO_END);
    }
    hoenn_mapping()
        .into_iter()
        .find(|&(slot, _)| slot == internal)
        .map_or(0, |(_, dex)| dex)
}

pub fn dex_to_internal(dex: i32) -> i32 {
    if dex <= JOHTO_END {
        return dex;
    }
    hoenn_mapping()
        .into_iter()
        .find(|&(_, number)| number == dex)
        .map_or(0, |(slot, _)| slot)
}

fn hoenn_mapping() -> Vec<(i32, i32)> {
    let mut mapping: Vec<(i32, i32)> = SPECIAL_HOENN.to_vec();
    let mut free_dex: Vec<i32> = (HOENN_DEX_START..=REAL_POKEMON)
        .filter(|dex| !SPECIAL_HOENN.iter().any(|&(_, special)| special == *dex))
        .collect();
    free_dex.reverse();
    for internal in HOENN_INTERNAL_START..=411 {
        if SPECIAL_HOENN.iter().any(|&(slot, _)| slot == internal) {
            continue;
        }
        if let Some(dex) = free_dex.pop() {
            mapping.push((internal, dex));
        }
    }
    mapping
}

pub fn pokemon_name(dex: i32, japanese: bool) -> String {
    let letters: Vec<char> = ('A'..='Z').collect();
    let first = letters[(dex as usize / 26) % 26];
    let second = letters[dex as usize % 26];
    if japanese {
        format!("P{first}{second}")
    } else {
        format!("MON{first}{second}")
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn below(&mut self, bound: i32) -> i32 {
        (self.next() % bound.max(1) as u64) as i32
    }

    fn range(&mut self, low: i32, high: i32) -> i32 {
        low + self.below(high - low + 1)
    }

    fn pick<T: Copy>(&mut self, values: &[T]) -> T {
        values[self.below(values.len() as i32) as usize]
    }
}

struct Builder {
    rom: Vec<u8>,
    entry: RomEntry,
    options: SyntheticOptions,
    table: TextTable,
    used: Vec<(i32, i32, String)>,
    data_cursor: i32,
    code_cursor: i32,
    rng: Rng,
    names_offset: i32,
    stats_offset: i32,
    move_data_offset: i32,
    move_names_offset: i32,
    ability_names_offset: i32,
    item_data_offset: i32,
    movesets_offset: i32,
    evolutions_offset: i32,
    tmhm_offset: i32,
    trainers_offset: i32,
    descriptions_offset: i32,
}

impl Builder {
    fn new(entry: &RomEntry, options: SyntheticOptions) -> Self {
        let mut rom = vec![0u8; SYNTHETIC_SIZE];
        let free_space = entry.value("FreeSpace") as usize;
        rom[free_space..].fill(0xFF);
        let table = TextTable::from_text(
            table_text(entry.table_file.as_deref().unwrap_or("gba_english")).expect("table"),
        );
        let seed = entry.name.bytes().fold(0x1234_5678u64, |hash, byte| {
            hash.wrapping_mul(31).wrapping_add(u64::from(byte))
        });
        Self {
            rom,
            entry: entry.clone(),
            options,
            table,
            used: Vec::new(),
            data_cursor: free_space as i32 + DATA_GAP_AFTER_FREE_SPACE,
            code_cursor: CODE_AREA_START,
            rng: Rng(seed),
            names_offset: 0,
            stats_offset: 0,
            move_data_offset: 0,
            move_names_offset: 0,
            ability_names_offset: 0,
            item_data_offset: 0,
            movesets_offset: 0,
            evolutions_offset: 0,
            tmhm_offset: 0,
            trainers_offset: 0,
            descriptions_offset: 0,
        }
    }

    fn value(&self, key: &str) -> i32 {
        self.entry.value(key)
    }

    fn is_fire_red_10(&self) -> bool {
        self.entry.rom_code == "BPRE" && self.entry.version == 0
    }

    fn is_japanese(&self) -> bool {
        self.entry.table_file.as_deref() == Some("gba_jpn")
    }

    fn rom_type(&self) -> RomType {
        self.entry.rom_type
    }

    fn reserve(&mut self, offset: i32, bytes: &[u8], what: &str) {
        let end = offset + bytes.len() as i32;
        assert!(
            offset >= 0 && end as usize <= self.rom.len(),
            "{what} at {offset:#X} is outside the ROM"
        );
        for (start, finish, owner) in &self.used {
            let overlaps = offset < *finish && *start < end;
            let same_extent = *start == offset && *finish == end;
            let identical = same_extent
                && (self.rom[offset as usize..end as usize] == *bytes
                    || (owner == what && what.starts_with("static")));
            assert!(
                !overlaps || identical,
                "{what} at {offset:#X}..{end:#X} overlaps {owner} at {start:#X}..{finish:#X}"
            );
        }
        self.used.push((offset, end, what.to_owned()));
    }

    fn put(&mut self, offset: i32, bytes: &[u8], what: &str) {
        self.reserve(offset, bytes, what);
        self.rom[offset as usize..offset as usize + bytes.len()].copy_from_slice(bytes);
    }

    fn put_word(&mut self, offset: i32, value: i32, what: &str) {
        self.put(offset, &(value as u16).to_le_bytes(), what);
    }

    fn put_pointer(&mut self, offset: i32, target: i32, what: &str) {
        self.put(offset, &(target + POINTER_BASE).to_le_bytes(), what);
    }

    fn alloc(&mut self, length: i32) -> i32 {
        let offset = (self.data_cursor + 3) & !3;
        self.data_cursor = offset + length.max(1);
        assert!(
            (self.data_cursor as usize) < SYNTHETIC_SIZE - 0x2_0000,
            "synthetic data area is full"
        );
        offset
    }

    fn alloc_bytes(&mut self, bytes: &[u8], what: &str) -> i32 {
        let offset = self.alloc(bytes.len() as i32);
        self.put(offset, bytes, what);
        offset
    }

    fn code(&mut self, bytes: &[u8], what: &str) -> i32 {
        let offset = (self.code_cursor + 3) & !3;
        self.code_cursor = offset + bytes.len() as i32 + 8;
        assert!(self.code_cursor < CODE_AREA_END, "code area is full");
        self.put(offset, bytes, what);
        offset
    }

    fn code_hex(&mut self, hex: &str, what: &str) -> i32 {
        let bytes = randomon_formats::search::parse_hex_bytes(hex).expect("hex");
        self.code(&bytes, what)
    }

    fn text(&self, text: &str) -> Vec<u8> {
        let mut bytes = self.table.translate(text);
        bytes.push(0xFF);
        bytes
    }

    fn fixed_text(&self, text: &str, length: i32) -> Vec<u8> {
        let mut bytes = self.table.translate(text);
        bytes.truncate(length as usize - 1);
        bytes.push(0xFF);
        bytes.resize(length as usize, 0);
        bytes
    }

    fn alloc_text(&mut self, text: &str, what: &str) -> i32 {
        let bytes = self.text(text);
        self.alloc_bytes(&bytes, what)
    }

    fn table_location(&mut self, key: &str, length: i32) -> i32 {
        if self.rom_type().is_ruby_or_sapphire() || key_is_fixed(key) {
            let fixed = self.value(key);
            if fixed > 0 {
                return fixed;
            }
        }
        self.alloc(length)
    }

    fn build_all(&mut self) {
        self.write_header();
        self.write_signatures();
        self.write_pokedex_order();
        self.write_pokemon_names();
        self.write_pokemon_stats();
        self.write_moves();
        self.write_ability_names();
        self.write_items();
        self.write_evolutions();
        self.write_movesets();
        self.write_compatibility();
        self.write_egg_moves();
        self.write_starters();
        self.write_trainers();
        self.write_world();
        self.write_statics();
        self.write_trades();
        self.write_shops();
        self.write_pickup_table();
        self.write_type_effectiveness();
        self.write_tm_tables();
        self.write_tweak_locators();
        self.write_header_pointers();
        if self.options.hack_pointers || self.is_fire_red_10() {
            self.write_hack_pointers();
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.options.forge_crc && self.entry.expected_crc32 >= 0 {
            forge_crc32(&mut self.rom, self.entry.expected_crc32 as u32);
        }
        self.rom
    }

    fn write_header(&mut self) {
        let mut code = self.entry.rom_code.clone().into_bytes();
        if self.entry.name == "Emerald (T-Eng)" {
            self.put(c::ROM_NAME_OFFSET as i32, b"YJencrypted", "unofficial name");
            code = b"ZZZZ".to_vec();
        }
        self.put(c::ROM_CODE_OFFSET as i32, &code, "rom code");
        self.put(
            c::ROM_VERSION_OFFSET as i32,
            &[self.entry.version as u8],
            "rom version",
        );
    }

    fn write_signatures(&mut self) {
        let wild = self.code_hex(c::WILD_POKEMON_POINTER_PREFIX, "wild prefix");
        self.reserve(wild + 12, &[0; 4], "wild pointer");
        let maps = self.code_hex(c::MAP_BANKS_POINTER_PREFIX, "map prefix");
        self.reserve(maps + 12, &[0; 4], "map pointer");
        let label_prefix = if self.rom_type().is_frlg() {
            c::FRLG_MAP_LABELS_POINTER_PREFIX
        } else {
            c::RSE_MAP_LABELS_POINTER_PREFIX
        };
        let labels = self.code_hex(label_prefix, "label prefix");
        self.reserve(labels + 12, &[0; 4], "label pointer");
        let mut order_prefixes = Vec::new();
        for _ in 0..3 {
            let prefix = self.code_hex(c::POKEDEX_ORDER_POINTER_PREFIX, "order prefix");
            order_prefixes.push(prefix);
        }
        self.reserve(order_prefixes[1] + 16, &[0; 4], "order pointer");
        self.entry.set_value("SignatureWild", wild + 12);
        self.entry.set_value("SignatureMaps", maps + 12);
        self.entry.set_value("SignatureLabels", labels + 12);
        self.entry
            .set_value("SignatureOrder", order_prefixes[1] + 16);
        if self.rom_type().is_ruby_or_sapphire() {
            let mut names = vec![0u8; 4];
            names.extend(
                randomon_formats::search::parse_hex_bytes(c::RS_POKEMON_NAMES_POINTER_SUFFIX)
                    .expect("hex"),
            );
            let location = self.code(&names, "names suffix");
            self.entry.set_value("SignatureNames", location);
            let mut sprites =
                randomon_formats::search::parse_hex_bytes(c::RS_FRONT_SPRITES_POINTER_PREFIX)
                    .expect("hex");
            sprites.extend((self.value("PokemonFrontSprites") + POINTER_BASE).to_le_bytes());
            sprites.extend(
                randomon_formats::search::parse_hex_bytes(c::RS_FRONT_SPRITES_POINTER_SUFFIX)
                    .expect("hex"),
            );
            self.code(&sprites, "front sprites signature");
            let mut palettes =
                randomon_formats::search::parse_hex_bytes(c::RS_POKEMON_PALETTES_POINTER_PREFIX)
                    .expect("hex");
            palettes.extend((self.value("PokemonNormalPalettes") + POINTER_BASE).to_le_bytes());
            palettes.extend(
                randomon_formats::search::parse_hex_bytes(c::RS_POKEMON_PALETTES_POINTER_SUFFIX)
                    .expect("hex"),
            );
            self.code(&palettes, "palettes signature");
        }
    }

    fn patch_reserved_pointer(&mut self, location: i32, target: i32) {
        self.rom[location as usize..location as usize + 4]
            .copy_from_slice(&(target + POINTER_BASE).to_le_bytes());
    }

    fn pokemon_count(&self) -> i32 {
        self.value("PokemonCount")
    }

    fn write_pokedex_order(&mut self) {
        let count = self.pokemon_count();
        let bytes: Vec<u8> = (1..=count)
            .flat_map(|internal| (internal_to_dex(internal) as u16).to_le_bytes())
            .collect();
        let order = self.alloc_bytes(&bytes, "pokedex order");
        let location = self.value("SignatureOrder");
        self.patch_reserved_pointer(location, order);
    }

    fn name_for_internal(&self, internal: i32) -> String {
        let dex = internal_to_dex(internal);
        if dex > REAL_POKEMON {
            "?".to_owned()
        } else {
            pokemon_name(dex, self.is_japanese())
        }
    }

    fn write_pokemon_names(&mut self) {
        let count = self.pokemon_count();
        let length = self.value("PokemonNameLength");
        let mut bytes = vec![0u8; length as usize];
        for internal in 1..=count {
            let name = self.name_for_internal(internal);
            bytes.extend(self.fixed_text(&name, length));
        }
        bytes.extend(std::iter::repeat_n(0xFF, length as usize));
        let offset = self.alloc_bytes(&bytes, "pokemon names");
        self.names_offset = offset;
        if self.rom_type().is_ruby_or_sapphire() {
            let location = self.value("SignatureNames");
            self.patch_reserved_pointer(location, offset);
        }
    }

    fn stats_for(&mut self, internal: i32) -> [u8; 28] {
        if internal_to_dex(internal) > REAL_POKEMON {
            return c::EMPTY_POKEMON_SIG;
        }
        let mut stats = [0u8; 28];
        for stat in stats.iter_mut().take(6) {
            *stat = self.rng.range(20, 150) as u8;
        }
        stats[6] = self.rng.pick(&TYPE_BYTES);
        stats[7] = if self.rng.below(2) == 0 {
            stats[6]
        } else {
            self.rng.pick(&TYPE_BYTES)
        };
        stats[8] = self.rng.range(3, 255) as u8;
        let common = self
            .rng
            .pick(&[0, items::ORAN_BERRY, items::NUGGET, items::LEFTOVERS]);
        let rare = if self.rng.below(3) == 0 {
            common
        } else {
            self.rng.pick(&[0, items::SITRUS_BERRY, items::STAR_PIECE])
        };
        stats[12..14].copy_from_slice(&(common as u16).to_le_bytes());
        stats[14..16].copy_from_slice(&(rare as u16).to_le_bytes());
        stats[16] = self.rng.pick(&GENDER_RATIOS);
        stats[19] = self.rng.range(0, 5) as u8;
        stats[22] = self.rng.range(1, c::HIGHEST_ABILITY_INDEX) as u8;
        stats[23] = if self.rng.below(2) == 0 {
            0
        } else {
            self.rng.range(1, c::HIGHEST_ABILITY_INDEX) as u8
        };
        stats
    }

    fn write_pokemon_stats(&mut self) {
        let count = self.pokemon_count();
        let mut bytes = vec![0u8; c::BASE_STATS_ENTRY_SIZE as usize];
        for internal in 1..=count {
            let stats = self.stats_for(internal);
            bytes.extend(stats);
        }
        let offset = self.table_location("PokemonStats", bytes.len() as i32);
        self.put(offset, &bytes, "pokemon stats");
        self.stats_offset = offset;
    }

    fn move_count(&self) -> i32 {
        self.value("MoveCount")
    }

    fn write_moves(&mut self) {
        let count = self.move_count();
        let name_length = self.value("MoveNameLength");
        let mut data = vec![0u8; c::MOVE_DATA_ENTRY_SIZE as usize];
        let mut names = vec![0u8; name_length as usize];
        for number in 1..=count {
            let effect = self.rng.pick(&EFFECTS);
            let power = if self.rng.below(4) == 0 {
                0
            } else {
                self.rng.range(10, 150)
            };
            let accuracy = if number == randomon_data::constants::moves::SWIFT {
                0
            } else {
                self.rng.range(50, 100)
            };
            let entry = [
                effect,
                power as u8,
                self.rng.pick(&TYPE_BYTES),
                accuracy as u8,
                self.rng.range(5, 40) as u8,
                self.rng.pick(&[0, 10, 30, 100]) as u8,
                self.rng.pick(&[0, 16]) as u8,
                0,
                self.rng.below(2) as u8,
                0,
                0,
                0,
            ];
            data.extend(entry);
            let name = if self.is_japanese() {
                format!("W{}", number)
            } else {
                format!("MOVE {number}")
            };
            names.extend(self.fixed_text(&name, name_length));
        }
        let data_offset = self.table_location("MoveData", data.len() as i32);
        self.put(data_offset, &data, "move data");
        self.move_data_offset = data_offset;
        let names_offset = self.table_location("MoveNames", names.len() as i32);
        self.put(names_offset, &names, "move names");
        self.move_names_offset = names_offset;
        self.write_move_descriptions();
    }

    fn write_move_descriptions(&mut self) {
        let table = self.value("MoveDescriptions");
        if table <= 0 {
            return;
        }
        let mut pointers = Vec::new();
        for number in 1..=self.move_count() {
            let text = format!(
                "Move {number} deals damage\\nand may lower Sp. Atk.\\nA long-\\nwinded text."
            );
            let pointer = self.alloc_text(&text, "move description");
            pointers.extend((pointer + POINTER_BASE).to_le_bytes());
        }
        pointers.extend([0, 0, 0, 0]);
        self.put(table, &pointers, "move description table");
        self.descriptions_offset = table;
    }

    fn write_ability_names(&mut self) {
        let length = self.value("AbilityNameLength");
        let mut bytes = Vec::new();
        for index in 0..=c::HIGHEST_ABILITY_INDEX {
            let name = if self.is_japanese() {
                format!("A{index}")
            } else {
                format!("ABILITY{index}")
            };
            bytes.extend(self.fixed_text(&name, length));
        }
        let offset = self.table_location("AbilityNames", bytes.len() as i32);
        self.put(offset, &bytes, "ability names");
        self.ability_names_offset = offset;
    }

    fn write_items(&mut self) {
        let count = self.value("ItemCount");
        let size = self.value("ItemEntrySize");
        let mut bytes = Vec::new();
        for item in 0..=count {
            let mut entry = vec![0u8; size as usize];
            let name = if self.is_japanese() {
                format!("I{item}")
            } else {
                format!("ITEM{item}")
            };
            let name_bytes = self.text(&name);
            entry[..name_bytes.len().min(14)]
                .copy_from_slice(&name_bytes[..name_bytes.len().min(14)]);
            entry[16..18].copy_from_slice(&((item * 10) as u16).to_le_bytes());
            bytes.extend(entry);
        }
        let offset = self.table_location("ItemData", bytes.len() as i32);
        self.put(offset, &bytes, "item data");
        self.item_data_offset = offset;
        for item in 0..=count {
            let text = format!("Item {item} text.");
            let pointer = self.alloc_text(&text, "item description");
            let location = offset + item * size + c::ITEM_DATA_DESCRIPTION_OFFSET;
            self.rom[location as usize..location as usize + 4]
                .copy_from_slice(&(pointer + POINTER_BASE).to_le_bytes());
        }
    }

    fn evolutions_for(&self, dex: i32) -> Vec<(i32, i32, i32)> {
        match dex {
            species::EEVEE => vec![
                (7, items::WATER_STONE, species::VAPOREON),
                (7, items::THUNDERSTONE, species::JOLTEON),
                (7, items::FIRE_STONE, species::FLAREON),
                (2, 0, species::ESPEON),
                (3, 0, species::UMBREON),
            ],
            species::POLIWHIRL => vec![
                (7, items::WATER_STONE, species::POLIWRATH),
                (6, items::KINGS_ROCK, species::POLITOED),
            ],
            species::SLOWPOKE => vec![
                (4, 37, species::SLOWBRO),
                (6, items::KINGS_ROCK, species::SLOWKING),
            ],
            species::SEADRA => vec![(6, items::DRAGON_SCALE, species::KINGDRA)],
            species::HAUNTER => vec![(5, 0, species::GENGAR)],
            species::CLAMPERL => vec![
                (6, items::DEEP_SEA_TOOTH, species::HUNTAIL),
                (6, items::DEEP_SEA_SCALE, species::GOREBYSS),
            ],
            species::FEEBAS => vec![(15, 170, species::MILOTIC)],
            species::NINCADA => vec![(13, 20, species::NINJASK), (14, 20, species::SHEDINJA)],
            species::TYROGUE => vec![
                (8, 20, species::HITMONLEE),
                (10, 20, species::HITMONCHAN),
                (9, 20, species::HITMONTOP),
            ],
            species::WURMPLE => vec![(11, 7, species::SILCOON), (12, 7, species::CASCOON)],
            species::GOLBAT => vec![(1, 0, species::CROBAT)],
            _ if dex % 3 == 1 && dex < 380 => vec![(4, 16, dex + 1)],
            _ if dex % 3 == 2 && dex < 380 => vec![(4, 36, dex + 1)],
            _ => Vec::new(),
        }
    }

    fn write_evolutions(&mut self) {
        let count = self.pokemon_count();
        let mut bytes = vec![0u8; c::EVOLUTION_ENTRY_SIZE as usize];
        for internal in 1..=count {
            let dex = internal_to_dex(internal);
            let mut entry = vec![0u8; c::EVOLUTION_ENTRY_SIZE as usize];
            if dex <= REAL_POKEMON {
                for (slot, (method, extra, target)) in
                    self.evolutions_for(dex).into_iter().take(5).enumerate()
                {
                    let base = slot * 8;
                    entry[base..base + 2].copy_from_slice(&(method as u16).to_le_bytes());
                    entry[base + 2..base + 4].copy_from_slice(&(extra as u16).to_le_bytes());
                    entry[base + 4..base + 6]
                        .copy_from_slice(&(dex_to_internal(target) as u16).to_le_bytes());
                }
            }
            bytes.extend(entry);
        }
        let offset = self.value("PokemonEvolutions");
        self.put(offset, &bytes, "evolutions");
        self.evolutions_offset = offset;
    }

    fn moveset_bytes(&mut self, dex: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let count = self.rng.range(4, 12);
        let mut level = 1;
        for slot in 0..count {
            let number = if dex == 1 && slot == 0 {
                0x150
            } else {
                self.rng.range(1, self.move_count())
            };
            if self.options.jambo_movesets {
                bytes.extend((number as u16).to_le_bytes());
                bytes.push(level as u8);
            } else {
                bytes.push((number & 0xFF) as u8);
                bytes.push((((level << 1) & 0xFE) + i32::from(number > 255)) as u8);
            }
            level += self.rng.range(0, 6);
        }
        if self.options.jambo_movesets {
            bytes.extend([0x00, 0x00, 0xFF]);
        } else {
            bytes.extend([0xFF, 0xFF]);
        }
        bytes
    }

    fn write_movesets(&mut self) {
        let count = self.pokemon_count();
        let mut pointers = vec![0u8; 4];
        for internal in 1..=count {
            let dex = internal_to_dex(internal);
            let moves = if dex > REAL_POKEMON {
                if self.options.jambo_movesets {
                    vec![1, 0, 1, 0, 0, 0xFF]
                } else {
                    vec![1, 2, 0xFF, 0xFF]
                }
            } else {
                self.moveset_bytes(dex)
            };
            let location = self.alloc_bytes(&moves, "moveset");
            pointers.extend((location + POINTER_BASE).to_le_bytes());
        }
        pointers.extend([0, 0, 0, 0]);
        let offset = self.value("PokemonMovesets");
        self.put(offset, &pointers, "moveset pointers");
        self.movesets_offset = offset;
    }

    fn random_bits(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.rng.below(256) as u8).collect()
    }

    fn write_compatibility(&mut self) {
        let count = self.pokemon_count() + 1;
        let tmhm = self.random_bits((count * c::TMHM_COMPAT_ENTRY_SIZE) as usize);
        let offset = self.value("PokemonTMHMCompat");
        self.put(offset, &tmhm, "tmhm compatibility");
        self.tmhm_offset = offset;
        if matches!(
            self.rom_type(),
            RomType::Emerald | RomType::FireRedLeafGreen
        ) {
            let tutor_count = self.value("MoveTutorMoves");
            let tutors: Vec<u8> = (0..tutor_count)
                .flat_map(|index| ((index * 7 + 5) as u16).to_le_bytes())
                .collect();
            let tutor_offset = self.value("MoveTutorData");
            self.put(tutor_offset, &tutors, "tutor moves");
            let bytes_per = ((tutor_count + 7) & !7) / 8;
            let compat = self.random_bits((count * bytes_per) as usize);
            self.put(
                tutor_offset + tutor_count * 2,
                &compat,
                "tutor compatibility",
            );
        }
    }

    fn write_egg_moves(&mut self) {
        let mut words: Vec<i32> = Vec::new();
        for dex in (1..=REAL_POKEMON).step_by(7) {
            words.push(dex_to_internal(dex) + c::EGG_MOVES_SPECIES_MARKER);
            for _ in 0..self.rng.range(1, 4) {
                let mv = self.rng.range(1, self.move_count());
                words.push(mv);
            }
        }
        words.push(c::EGG_MOVES_TERMINATOR);
        let bytes: Vec<u8> = words
            .iter()
            .flat_map(|&word| (word as u16).to_le_bytes())
            .collect();
        let offset = self.value("EggMoves");
        self.put(offset, &bytes, "egg moves");
    }

    fn write_starters(&mut self) {
        let base = self.value("StarterPokemon");
        let starters = [1, 4, 7].map(dex_to_internal);
        if self.rom_type().is_frlg() {
            let offsets = [
                base,
                base + c::FRLG_STARTER2_OFFSET,
                base + c::FRLG_STARTER3_OFFSET,
            ];
            for (index, &offset) in offsets.iter().enumerate() {
                self.put_word(offset, starters[index], "starter");
                self.put_word(
                    offset + c::FRLG_STARTER_REPEAT_OFFSET,
                    starters[(index + 1) % 3],
                    "starter repeat",
                );
            }
            self.put_word(base + c::FRLG_STARTER_ITEMS_OFFSET, 0, "starter item");
            self.write_oak_texts();
        } else {
            let starters = [252, 255, 258].map(dex_to_internal);
            for (index, &starter) in starters.iter().enumerate() {
                self.put_word(base + index as i32 * 2, starter, "starter");
            }
            let items = self.value("StarterItems");
            self.put(items, &[0, 0x21, 0, 0x32], "starter item code");
        }
    }

    fn write_oak_texts(&mut self) {
        if matches!(self.entry.region_letter(), Some('J' | 'B')) {
            return;
        }
        for dex in [1, 4, 7] {
            let name = pokemon_name(dex, false);
            let text = format!("{name} is a nice POKéMON, a very long sentence indeed. Take it?");
            let bytes = self.text(&text);
            self.code(&bytes, "oak text");
        }
    }

    fn trainer_team(&mut self, poketype: i32, size: i32) -> Vec<u8> {
        let mut bytes = Vec::new();
        for _ in 0..size {
            let dex = self.rng.range(1, REAL_POKEMON);
            let mut block = vec![0u8; if poketype & 1 == 1 { 16 } else { 8 }];
            block[0] = self.rng.below(256) as u8;
            block[2..4].copy_from_slice(&(self.rng.range(2, 70) as u16).to_le_bytes());
            block[4..6].copy_from_slice(&(dex_to_internal(dex) as u16).to_le_bytes());
            let moves_start = match poketype {
                1 => Some(6),
                2 => {
                    block[6] = items::ORAN_BERRY as u8;
                    None
                }
                3 => {
                    block[6..8].copy_from_slice(&(items::SITRUS_BERRY as u16).to_le_bytes());
                    Some(8)
                }
                _ => None,
            };
            if let Some(start) = moves_start {
                for slot in 0..4 {
                    let mv = self.rng.range(1, self.move_count());
                    block[start + slot * 2..start + slot * 2 + 2]
                        .copy_from_slice(&(mv as u16).to_le_bytes());
                }
            }
            bytes.extend(block);
        }
        bytes
    }

    fn write_trainers(&mut self) {
        let count = self.value("TrainerCount");
        let size = self.value("TrainerEntrySize");
        let name_length = self.value("TrainerNameLength");
        let classes = self.value("TrainerClassCount");
        let mut bytes = vec![0u8; size as usize];
        for index in 1..count {
            let poketype = self.rng.below(4);
            let team_size = self.rng.range(1, 6);
            let team = self.trainer_team(poketype, team_size);
            let pointer = self.alloc_bytes(&team, "trainer team");
            let mut entry = vec![0u8; size as usize];
            entry[0] = poketype as u8;
            entry[1] = self.rng.below(classes) as u8;
            entry[2] = if index % 5 == 0 { 0x80 } else { 0x00 };
            let name = if self.is_japanese() {
                format!("T{}", index % 100)
            } else {
                format!("TRAINER{}", index % 100)
            };
            let name_bytes = self.fixed_text(&name, name_length);
            entry[4..4 + name_bytes.len()].copy_from_slice(&name_bytes);
            entry[(size - 16) as usize] = u8::from(index % 7 == 0);
            entry[(size - 8) as usize] = team_size as u8;
            entry[(size - 4) as usize..size as usize]
                .copy_from_slice(&(pointer + POINTER_BASE).to_le_bytes());
            bytes.extend(entry);
        }
        bytes.extend(vec![0u8; size as usize]);
        let offset = self.value("TrainerData");
        self.put(offset, &bytes, "trainers");
        self.trainers_offset = offset;
        let class_length = self.value("TrainerClassNameLength");
        let mut class_names = Vec::new();
        for class in 0..classes {
            let name = if self.is_japanese() {
                format!("C{class}")
            } else {
                format!("CLASS{class}")
            };
            class_names.extend(self.fixed_text(&name, class_length));
        }
        let class_offset = self.value("TrainerClassNames");
        self.put(class_offset, &class_names, "trainer classes");
        if self.rom_type().is_emerald() {
            self.write_mossdeep_steven();
        }
    }

    fn write_mossdeep_steven(&mut self) {
        let offset = self.value("MossdeepStevenTeamOffset");
        let mut bytes = Vec::new();
        for dex in [species::METANG, species::SKARMORY, species::AGGRON] {
            let mut entry = vec![0u8; c::MOSSDEEP_STEVEN_POKEMON_SIZE as usize];
            entry[0..2].copy_from_slice(&(dex_to_internal(dex) as u16).to_le_bytes());
            entry[2] = 31;
            entry[3] = 42;
            for slot in 0..4 {
                let mv = 10 + slot as i32;
                entry[12 + slot * 2..14 + slot * 2].copy_from_slice(&(mv as u16).to_le_bytes());
            }
            bytes.extend(entry);
        }
        self.put(offset, &bytes, "mossdeep steven");
    }

    fn label_for(&self, bank: i32, map: i32) -> i32 {
        let base = (bank * 7 + map) % 40;
        if self.rom_type().is_frlg() {
            c::FRLG_MAP_LABELS_START + base
        } else {
            base
        }
    }

    fn write_map_labels(&mut self) -> i32 {
        let mut names = Vec::new();
        for label in 0..40 {
            let name = if self.is_japanese() {
                format!("M{label}")
            } else {
                format!("ROUTE {label}")
            };
            names.push(self.alloc_text(&name, "map label"));
        }
        let mut table = Vec::new();
        for pointer in names {
            if !self.rom_type().is_frlg() {
                table.extend([0u8; 4]);
            }
            table.extend((pointer + POINTER_BASE).to_le_bytes());
        }
        self.alloc_bytes(&table, "map labels")
    }

    fn item_script(&mut self, item: i32) -> i32 {
        let mut script = vec![0x1A, 0x00, 0x80];
        script.extend((item as u16).to_le_bytes());
        script.extend([0x1A, 0x01, 0x80, 0x01, 0x00, 0x09, 0x01]);
        self.alloc_bytes(&script, "item ball script")
    }

    fn field_items(&self) -> Vec<i32> {
        let mut field: Vec<i32> = Vec::new();
        let required = match self.rom_type() {
            RomType::FireRedLeafGreen => c::FRLG_REQUIRED_FIELD_TMS,
            RomType::Emerald => c::E_REQUIRED_FIELD_TMS,
            _ => c::RS_REQUIRED_FIELD_TMS,
        };
        for &tm in required {
            field.push(c::TM_ITEM_OFFSET + tm - 1);
        }
        field.push(c::TM_ITEM_OFFSET + 3);
        field.push(c::TM_ITEM_OFFSET + 3);
        for item in [
            items::POTION,
            items::RARE_CANDY,
            items::NUGGET,
            items::ORAN_BERRY,
            items::LEFTOVERS,
            items::SUPER_POTION,
            items::REVIVE,
            items::MACH_BIKE,
        ] {
            field.push(item);
        }
        field
    }

    fn person_entry(&self, sprite: i32, script: i32) -> Vec<u8> {
        let mut person = vec![0u8; c::PERSON_EVENT_SIZE as usize];
        person[1] = sprite as u8;
        let pointer = if script > 0 { script + POINTER_BASE } else { 0 };
        person[16..20].copy_from_slice(&pointer.to_le_bytes());
        person
    }

    fn script_text_people(&mut self, bank: i32, map: i32) -> BTreeMap<i32, i32> {
        let texts: Vec<_> = self
            .entry
            .script_texts
            .iter()
            .filter(|text| text.map_bank == bank && text.map_number == map)
            .cloned()
            .collect();
        let mut by_person: BTreeMap<i32, Vec<_>> = BTreeMap::new();
        for text in texts {
            by_person.entry(text.person_number).or_default().push(text);
        }
        let mut scripts = BTreeMap::new();
        for (person, texts) in by_person {
            let redirect = self.rom_type().is_frlg()
                && texts
                    .iter()
                    .all(|text| text.is_move_tutor && [5, 7, 8, 9, 10, 11].contains(&text.number));
            let size = texts
                .iter()
                .map(|text| text.offset_in_script + 4)
                .max()
                .unwrap_or(4);
            let mut block = vec![0u8; size as usize];
            for text in &texts {
                let message = self.alloc_text("Original script text.", "script text");
                let at = text.offset_in_script as usize;
                block[at..at + 4].copy_from_slice(&(message + POINTER_BASE).to_le_bytes());
            }
            let real_script = self.alloc_bytes(&block, "script block");
            let entry_script = if redirect {
                let deep = texts.iter().any(|text| text.number == 7);
                let distance = if deep { 0x1F } else { 1 };
                let mut wrapper = vec![0u8; distance + 4];
                wrapper[distance..distance + 4]
                    .copy_from_slice(&(real_script + POINTER_BASE).to_le_bytes());
                self.alloc_bytes(&wrapper, "script redirect")
            } else {
                real_script
            };
            scripts.insert(person, entry_script);
        }
        scripts
    }

    fn map_events(&mut self, bank: i32, map: i32, items: &mut Vec<i32>) -> i32 {
        let item_ball = self.value("ItemBallPic");
        let text_people = self.script_text_people(bank, map);
        let text_person_count = text_people.keys().copied().max().unwrap_or(0);
        let mut people = Vec::new();
        for person in 1..=text_person_count {
            let script = text_people.get(&person).copied().unwrap_or(0);
            people.extend(self.person_entry(3, script));
        }
        for _ in 0..2 {
            if let Some(item) = items.pop() {
                let script = self.item_script(item);
                people.extend(self.person_entry(item_ball, script));
            }
        }
        let mut signposts = Vec::new();
        if let Some(item) = items.pop() {
            let mut signpost = vec![0u8; c::SIGNPOST_EVENT_SIZE as usize];
            signpost[5] = 6;
            signpost[8..10].copy_from_slice(&(item as u16).to_le_bytes());
            signposts.extend(signpost);
        }
        let mut coins = vec![0u8; c::SIGNPOST_EVENT_SIZE as usize];
        coins[5] = 7;
        signposts.extend(coins);
        let people_count = people.len() / c::PERSON_EVENT_SIZE as usize;
        let signpost_count = signposts.len() / c::SIGNPOST_EVENT_SIZE as usize;
        let people_offset = if people.is_empty() {
            0
        } else {
            self.alloc_bytes(&people, "people events")
        };
        let signpost_offset = self.alloc_bytes(&signposts, "signpost events");
        let mut events = vec![0u8; 20];
        events[0] = people_count as u8;
        events[3] = signpost_count as u8;
        if people_offset > 0 {
            events[4..8].copy_from_slice(&(people_offset + POINTER_BASE).to_le_bytes());
        }
        events[16..20].copy_from_slice(&(signpost_offset + POINTER_BASE).to_le_bytes());
        self.alloc_bytes(&events, "map events")
    }

    fn write_world(&mut self) {
        let labels = self.write_map_labels();
        let location = self.value("SignatureLabels");
        self.patch_reserved_pointer(location, labels);
        let bank_count = self
            .entry
            .script_texts
            .iter()
            .map(|text| text.map_bank + 1)
            .max()
            .unwrap_or(0)
            .max(4);
        let mut field_items = self.field_items();
        let mut bank_tables = Vec::new();
        for bank in 0..bank_count {
            let map_count = self
                .entry
                .script_texts
                .iter()
                .filter(|text| text.map_bank == bank)
                .map(|text| text.map_number + 1)
                .max()
                .unwrap_or(0)
                .max(6);
            let mut pointers = Vec::new();
            for map in 0..map_count {
                let events = self.map_events(bank, map, &mut field_items);
                let mut header = vec![0u8; 0x1C];
                header[4..8].copy_from_slice(&(events + POINTER_BASE).to_le_bytes());
                header[c::MAP_HEADER_LABEL_OFFSET as usize] = self.label_for(bank, map) as u8;
                let header_offset = self.alloc_bytes(&header, "map header");
                pointers.extend((header_offset + POINTER_BASE).to_le_bytes());
            }
            pointers.extend([0u8; 4]);
            bank_tables.push(self.alloc_bytes(&pointers, "map bank"));
        }
        let mut banks = Vec::new();
        for table in bank_tables {
            banks.extend((table + POINTER_BASE).to_le_bytes());
        }
        banks.extend([0u8; 4]);
        let headers = self.alloc_bytes(&banks, "map bank table");
        let location = self.value("SignatureMaps");
        self.patch_reserved_pointer(location, headers);
        self.write_wild_encounters(bank_count);
    }

    fn wild_area(&mut self, slots: i32) -> i32 {
        let mut data = Vec::new();
        for _ in 0..slots {
            let level = self.rng.range(2, 60);
            data.push(level as u8);
            data.push((level + self.rng.range(0, 5)) as u8);
            let dex = self.rng.range(1, REAL_POKEMON);
            data.extend((dex_to_internal(dex) as u16).to_le_bytes());
        }
        let data_offset = self.alloc_bytes(&data, "wild slots");
        let mut area = vec![self.rng.range(1, 30) as u8, 0, 0, 0];
        area.extend((data_offset + POINTER_BASE).to_le_bytes());
        self.alloc_bytes(&area, "wild area")
    }

    fn write_wild_encounters(&mut self, bank_count: i32) {
        let mut headers = Vec::new();
        let mut first_grass = 0;
        for index in 0..WILD_HEADER_COUNT {
            let grass = if index == WILD_HEADER_COUNT - 1 {
                first_grass
            } else {
                self.wild_area(c::GRASS_SLOTS)
            };
            if index == 0 {
                first_grass = grass;
            }
            let water = self.wild_area(c::SURFING_SLOTS);
            let rock = if index % 4 == 0 {
                self.wild_area(c::ROCK_SMASH_SLOTS)
            } else {
                0
            };
            let fishing = self.wild_area(c::FISHING_SLOTS);
            let mut header = vec![(index % bank_count) as u8, (index % 5) as u8, 0, 0];
            for area in [grass, water, rock, fishing] {
                let pointer = if area > 0 { area + POINTER_BASE } else { 0 };
                header.extend(pointer.to_le_bytes());
            }
            headers.extend(header);
        }
        let mut terminator = vec![0xFF, 0xFF];
        terminator.resize(c::WILD_HEADER_SIZE as usize, 0);
        headers.extend(terminator);
        let table = self.alloc_bytes(&headers, "wild headers");
        let location = self.value("SignatureWild");
        self.patch_reserved_pointer(location, table);
    }

    fn write_static_entry(&mut self, index: usize, roaming: bool, dex: i32, level: i32) {
        let entry = if roaming {
            self.entry.roaming_pokemon[index].clone()
        } else {
            self.entry.static_pokemon[index].clone()
        };
        for &offset in &entry.species_offsets {
            if offset >= self.value("FreeSpace") {
                continue;
            }
            self.put_word(offset, dex_to_internal(dex), "static species");
        }
        for &offset in &entry.level_offsets {
            if offset >= self.value("FreeSpace") {
                continue;
            }
            self.put(offset, &[level as u8], "static level");
        }
    }

    fn write_statics(&mut self) {
        for index in 0..self.entry.static_pokemon.len() {
            let dex = [
                species::LILEEP,
                species::GROUDON,
                species::REGIROCK,
                species::LATIAS,
                species::KECLEON,
                species::VOLTORB,
            ][index % 6];
            self.write_static_entry(index, false, dex, 20 + index as i32);
        }
        self.write_roamers();
        if self.entry.tweak("GhostMarowakTweak").is_some() {
            let species_offsets = self.entry.array("GhostMarowakSpeciesOffsets").to_vec();
            let free = self.value("FreeSpace");
            for offset in species_offsets.into_iter().filter(|&offset| offset < free) {
                self.put_word(offset, species::MAROWAK, "ghost marowak");
            }
            let level_offsets = self.entry.array("GhostMarowakLevelOffsets").to_vec();
            for offset in level_offsets.into_iter().filter(|&offset| offset < free) {
                self.put(offset, &[30], "ghost marowak level");
            }
        }
    }

    fn write_roamers(&mut self) {
        let roamers = self.entry.roaming_pokemon.len();
        match self.rom_type() {
            RomType::Ruby => {
                for roamer in self.entry.roaming_pokemon.clone() {
                    for offset in roamer.level_offsets {
                        self.put(offset, &[40], "roamer level");
                    }
                }
            }
            RomType::Sapphire => {
                for index in 0..roamers {
                    self.write_static_entry(index, true, species::LATIAS, 40);
                }
            }
            RomType::FireRedLeafGreen => {
                let dexes = [species::RAIKOU, species::ENTEI, species::SUICUNE];
                for index in 0..roamers {
                    self.write_static_entry(index, true, dexes[index % 3], 50);
                }
            }
            RomType::Emerald => {
                let roamers = self.entry.roaming_pokemon.clone();
                if let Some(latios) = roamers.first() {
                    for &offset in &latios.species_offsets {
                        self.put_word(offset, 0x5D8C, "unpatched roamer");
                    }
                    for &offset in &latios.level_offsets {
                        self.put(offset, &[40], "roamer level");
                    }
                }
                if let Some(latias) = roamers.get(1) {
                    for &offset in &latias.species_offsets {
                        self.put_word(offset, dex_to_internal(species::LATIAS), "roamer");
                    }
                    for &offset in &latias.level_offsets {
                        self.put(offset, &[40], "roamer level");
                    }
                }
            }
        }
    }

    fn write_trades(&mut self) {
        let table = self.value("TradeTableOffset");
        let size = self.value("TradeTableSize");
        let mut bytes = Vec::new();
        for index in 0..size {
            let mut trade = vec![0u8; c::TRADE_ENTRY_LENGTH as usize];
            let nickname = self.fixed_text(&format!("NICK{index}"), 12);
            trade[..12].copy_from_slice(&nickname);
            trade[12..14].copy_from_slice(&(dex_to_internal(10 + index) as u16).to_le_bytes());
            for iv in 0..6 {
                trade[14 + iv] = (5 + iv) as u8;
            }
            trade[24..26].copy_from_slice(&((1000 + index) as u16).to_le_bytes());
            trade[40..42].copy_from_slice(&(items::ORAN_BERRY as u16).to_le_bytes());
            let ot = self.fixed_text(&format!("OT{index}"), 11);
            trade[43..54].copy_from_slice(&ot);
            trade[56..58].copy_from_slice(&(dex_to_internal(20 + index) as u16).to_le_bytes());
            bytes.extend(trade);
        }
        self.put(table, &bytes, "trades");
    }

    fn write_shops(&mut self) {
        let mut offsets = self.entry.array("ShopItemOffsets").to_vec();
        let original = offsets.clone();
        offsets.sort_unstable();
        for (index, &offset) in original.iter().enumerate() {
            let next = offsets
                .iter()
                .copied()
                .find(|&other| other > offset)
                .unwrap_or(offset + 64);
            let capacity = (((next - offset) / 2) - 1).clamp(1, 6);
            let mut bytes = Vec::new();
            for slot in 0..capacity {
                let item = [
                    items::POTION,
                    items::POKE_BALL,
                    items::REPEL,
                    items::ANTIDOTE,
                    items::FULL_HEAL,
                    items::ESCAPE_ROPE,
                ][(slot as usize + index) % 6];
                bytes.extend((item as u16).to_le_bytes());
            }
            bytes.extend([0, 0]);
            self.put(offset, &bytes, "shop");
        }
    }

    fn write_pickup_table(&mut self) {
        let Some(locator) = self
            .entry
            .string("PickupTableStartLocator")
            .map(str::to_owned)
        else {
            return;
        };
        let count = self.value("PickupItemCount");
        let entry_size = if self.rom_type().is_emerald() { 2 } else { 4 };
        let mut bytes = randomon_formats::search::parse_hex_bytes(&locator).expect("hex");
        let locator_length = bytes.len();
        bytes.resize((count * entry_size) as usize, 0);
        for index in 0..count {
            let at = (index * entry_size) as usize;
            if at >= locator_length {
                let item = items::POTION + index;
                bytes[at..at + 2].copy_from_slice(&(item as u16).to_le_bytes());
            }
        }
        self.code(&bytes, "pickup table");
    }

    fn write_type_effectiveness(&mut self) {
        let offset = self.value("TypeEffectivenessOffset");
        let mut bytes = vec![
            0x0A, 0x0C, 20, 0x0B, 0x0A, 20, 0x07, 0x08, 5, 0x11, 0x08, 5, 0x0D, 0x04, 0, 0x01,
            0x00, 20, 0x0C, 0x0A, 5, 0x0E, 0x01, 20, 0x06, 0x0E, 20, 0x0F, 0x10, 20,
        ];
        bytes.extend([
            0xFE, 0xFE, 0x00, 0x00, 0x07, 0, 0x01, 0x07, 0, 0xFF, 0xFF, 0x00,
        ]);
        self.put(offset, &bytes, "type effectiveness");
    }

    fn write_tm_tables(&mut self) {
        let tms: Vec<u8> = (0..c::TM_COUNT)
            .flat_map(|index| ((index * 5 + 3) as u16).to_le_bytes())
            .collect();
        let offset = self.value("TmMoves");
        self.put(offset, &tms, "tm moves");
        let duplicate = self.value("TmMovesDuplicate");
        if duplicate > 0 {
            self.put(duplicate, &tms, "tm moves duplicate");
        }
    }

    fn write_tweak_locators(&mut self) {
        let mut deoxys =
            randomon_formats::search::parse_hex_bytes(c::DEOXYS_OBEY_CODE).expect("hex");
        deoxys.resize(c::MEW_OBEY_OFFSET_FROM_DEOXYS_OBEY as usize, 0x11);
        deoxys.extend([0x97, 0x28]);
        self.code(&deoxys, "deoxys obedience");
        if self.rom_type().is_frlg() {
            self.code_hex(c::LEVEL_EVO_KANTO_DEX_CHECK_CODE, "level evolution check");
            self.code_hex(c::STONE_EVO_KANTO_DEX_CHECK_CODE, "stone evolution check");
        }
        let mut friendship =
            randomon_formats::search::parse_hex_bytes(c::FRIENDSHIP_VALUE_FOR_EVO_LOCATOR)
                .expect("hex");
        friendship.resize(70, 0x33);
        friendship[c::HAPPINESS_DAY_LOCATOR_DISTANCE as usize] = c::HAPPINESS_FOR_EVOLUTION;
        friendship[c::HAPPINESS_NIGHT_LOCATOR_DISTANCE as usize] = c::HAPPINESS_FOR_EVOLUTION;
        self.code(&friendship, "friendship locator");
        self.code_hex(c::PERFECT_ODDS_BRANCH_LOCATOR, "perfect odds");
        let mut shoes = randomon_formats::search::parse_hex_bytes(c::running_shoes_check_prefix(
            self.rom_type(),
        ))
        .expect("hex");
        shoes.resize(c::RUNNING_SHOES_BRANCH_DISTANCE as usize + 2, 0x77);
        self.code(&shoes, "running shoes");
        if let Some(prefix) = self.entry.string("DeoxysStatPrefix").map(str::to_owned) {
            let mut stats = randomon_formats::search::parse_hex_bytes(&prefix).expect("hex");
            for stat in [50, 180, 20, 150, 180, 20] {
                stats.extend((stat as u16).to_le_bytes());
            }
            self.code(&stats, "deoxys stats");
        }
        self.write_national_dex_scripts();
    }

    fn write_national_dex_scripts(&mut self) {
        match self.rom_type() {
            RomType::FireRedLeafGreen => {
                self.code_hex(c::FRLG_POKEDEX_SCRIPT_IDENTIFIER, "dex script");
                self.code_hex(c::FRLG_NAT_DEX_FLAG_CHECKER, "dex flag check");
                self.code_hex(c::FRLG_NAT_DEX_FLAG_CHECKER, "dex flag check");
                self.code_hex(c::FRLG_OAKS_LAB_KANTO_DEX_CHECKER, "oak lab check");
                self.code_hex(c::FRLG_OAK_OUTSIDE_HOUSE_CHECK, "oak house check");
                let mut aide =
                    randomon_formats::search::parse_hex_bytes(c::FRLG_OAK_AIDE_CHECK_PREFIX)
                        .expect("hex");
                aide.extend([0x02, 0xD1]);
                self.code(&aide, "oak aide check");
            }
            rom_type => {
                let identifier = if rom_type.is_emerald() {
                    c::E_POKEDEX_SCRIPT_IDENTIFIER
                } else {
                    c::RS_POKEDEX_SCRIPT_IDENTIFIER
                };
                let text = self.alloc_text("Original dex text.", "dex text");
                let mut script = vec![0x5A, 0x5B, 0x5C, 0x5D];
                script.extend((text + POINTER_BASE).to_le_bytes());
                script.extend(randomon_formats::search::parse_hex_bytes(identifier).expect("hex"));
                let start = self.code(&script, "dex script");
                let pointer =
                    randomon_formats::search::parse_hex_bytes(&pointer_to_hex(start)).expect("hex");
                self.code(&pointer, "dex script pointer");
            }
        }
    }

    fn write_header_pointers(&mut self) {
        if self.rom_type().is_ruby_or_sapphire() {
            return;
        }
        let pointers = [
            (c::EFRLG_POKEMON_NAMES_POINTER, self.names_offset),
            (c::EFRLG_MOVE_NAMES_POINTER, self.move_names_offset),
            (c::EFRLG_ABILITY_NAMES_POINTER, self.ability_names_offset),
            (c::EFRLG_ITEM_DATA_POINTER, self.item_data_offset),
            (c::EFRLG_MOVE_DATA_POINTER, self.move_data_offset),
            (c::EFRLG_POKEMON_STATS_POINTER, self.stats_offset),
            (c::EFRLG_FRONT_SPRITES_POINTER, 0x10_0000),
            (c::EFRLG_POKEMON_PALETTES_POINTER, 0x10_4000),
        ];
        for (location, target) in pointers {
            self.put_pointer(location, target, "header pointer");
        }
    }

    fn write_hack_pointers(&mut self) {
        let pointers = [
            (0x3EA7C, self.movesets_offset),
            (0x43C68, self.tmhm_offset),
            (0x42F6C, self.evolutions_offset),
            (
                0x120C30,
                self.value("MoveTutorData") + self.value("MoveTutorMoves") * 2,
            ),
            (0xE5440, self.descriptions_offset),
            (0xFC00, self.trainers_offset),
        ];
        for (location, target) in pointers {
            self.put_pointer(location, target, "hack pointer");
        }
        if self.options.jambo_movesets {
            self.put(0x3EB20, &0x4708_4918u32.to_le_bytes(), "jambo signature");
            let routine = self.alloc(80);
            self.put_pointer(0x3EB84, routine, "jambo routine");
            self.put_pointer(routine + 75, self.movesets_offset, "jambo movesets");
        }
    }
}

fn key_is_fixed(key: &str) -> bool {
    !matches!(
        key,
        "PokemonStats" | "MoveData" | "MoveNames" | "AbilityNames" | "ItemData"
    )
}

fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (index, slot) in table.iter_mut().enumerate() {
        let mut value = index as u32;
        for _ in 0..8 {
            value = if value & 1 == 1 {
                (value >> 1) ^ 0xEDB8_8320
            } else {
                value >> 1
            };
        }
        *slot = value;
    }
    table
}

pub fn forge_crc32(rom: &mut [u8], wanted: u32) {
    let length = rom.len();
    let table = crc_table();
    let mut reverse = [0u8; 256];
    for (index, value) in table.iter().enumerate() {
        reverse[(value >> 24) as usize] = index as u8;
    }
    let prefix_state = !(randomon_gb::crc32(&rom[..length - 4]) as u32);
    let mut state = !wanted;
    for _ in 0..4 {
        let index = reverse[(state >> 24) as usize];
        state = ((state ^ table[index as usize]) << 8) | u32::from(index);
    }
    let patch = (state ^ prefix_state).to_le_bytes();
    rom[length - 4..].copy_from_slice(&patch);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_forging_hits_the_requested_checksum() {
        let mut data = vec![7u8; 4096];
        forge_crc32(&mut data, 0xDD88_761C);
        assert_eq!(randomon_gb::crc32(&data), 0xDD88_761C);
    }

    #[test]
    fn synthetic_pokedex_mapping_is_a_bijection() {
        for dex in 1..=REAL_POKEMON {
            assert_eq!(internal_to_dex(dex_to_internal(dex)), dex);
        }
        assert_eq!(dex_to_internal(species::LATIOS), 408);
        assert_eq!(internal_to_dex(260), 395);
    }
}
