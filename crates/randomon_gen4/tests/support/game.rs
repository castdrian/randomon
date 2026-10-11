use std::collections::BTreeMap;

use randomon_data::constants::{gen4, items, moves, species};
use randomon_formats::nds::NarcArchive;
use randomon_formats::text::poketext;
use randomon_gen4::{RomEntry, patch_bytes};

use super::blob::{Allocator, hex, ips_ranges, longs, put_bytes, put_u8, put_u16, put_u32, words};
use super::nds::NdsImage;

pub const TRAINER_COUNT: usize = 930;
const MAP_COUNT: usize = 200;
const MAP_NAME_COUNT: usize = 220;
const CLASS_COUNT: usize = 130;
const DPPT_WILD_FILES: usize = 183;
const HGSS_WILD_FILES: usize = 142;
const ITEM_BALLS: usize = 340;
const ITEM_DATA_FILES: usize = 520;
const PERSONAL_FILES: usize = 508;
const DTCM_TAIL: usize = 0x40;
const ITCM_SIZE: usize = 0x40;
const ARM9_START: usize = 0x4000;

#[derive(Clone, Copy, Default)]
pub struct Quirks {
    pub empty_headbutt_tree: bool,
    pub empty_bug_contest_day: bool,
}

pub struct GameBuilder {
    entry: &'static RomEntry,
    quirks: Quirks,
    text: BTreeMap<i32, Vec<String>>,
    scripts: BTreeMap<i32, Vec<u8>>,
    overlays: BTreeMap<i32, Vec<u8>>,
    files: BTreeMap<String, Vec<u8>>,
    arm9: Vec<u8>,
    arm9_allocator: Allocator,
    overlay_allocators: BTreeMap<i32, Allocator>,
}

pub fn pokemon_name(number: usize) -> String {
    format!("PKMN{number:03}")
}

pub fn move_name(number: usize) -> String {
    format!("Move{number:03}")
}

fn valid_species(seed: usize) -> u32 {
    (seed % 493) as u32 + 1
}

fn regular_species(seed: usize) -> u32 {
    let candidate = valid_species(seed);
    if candidate == species::UNOWN as u32 {
        202
    } else {
        candidate
    }
}

fn type_byte(seed: usize) -> u8 {
    let types = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10, 0x11,
    ];
    types[seed % types.len()]
}

fn tm_moves() -> Vec<u32> {
    (0..92)
        .map(|index| ((index * 5 + 7) % 460 + 1) as u32)
        .collect()
}

fn narc(files: Vec<Vec<u8>>) -> Vec<u8> {
    let mut archive = NarcArchive::new();
    archive.files = files;
    archive.to_bytes()
}

fn file_path(entry: &RomEntry, key: &str) -> Option<String> {
    entry.file(key).map(str::to_owned)
}

impl GameBuilder {
    pub fn new(entry: &'static RomEntry, quirks: Quirks) -> Self {
        Self {
            entry,
            quirks,
            text: BTreeMap::new(),
            scripts: BTreeMap::new(),
            overlays: BTreeMap::new(),
            files: BTreeMap::new(),
            arm9: Vec::new(),
            arm9_allocator: Allocator::new(0, 0, Vec::new()),
            overlay_allocators: BTreeMap::new(),
        }
    }

    fn int(&self, key: &str) -> i32 {
        self.entry.int(key)
    }

    fn array(&self, key: &str) -> Vec<i32> {
        self.entry
            .array(key)
            .map(<[i32]>::to_vec)
            .unwrap_or_default()
    }

    fn is_hgss(&self) -> bool {
        self.entry.rom_type == gen4::TYPE_HGSS
    }

    fn is_dp(&self) -> bool {
        self.entry.rom_type == gen4::TYPE_DP
    }

    fn put_file(&mut self, key: &str, data: Vec<u8>) {
        if let Some(path) = file_path(self.entry, key) {
            self.files.insert(path, data);
        }
    }

    fn text_entry(&mut self, index: i32, minimum: usize) -> &mut Vec<String> {
        let strings = self.text.entry(index).or_default();
        while strings.len() < minimum {
            let number = strings.len();
            strings.push(format!("Text {index} line {number}"));
        }
        strings
    }

    fn set_text(&mut self, index: i32, number: usize, value: String) {
        let strings = self.text_entry(index, number + 1);
        strings[number] = value;
    }

    fn script(&mut self, index: i32, minimum: usize) -> &mut Vec<u8> {
        let script = self
            .scripts
            .entry(index)
            .or_insert_with(|| vec![0x02, 0x00]);
        super::blob::grow(script, minimum);
        script
    }

    fn overlay(&mut self, number: i32) -> &mut Vec<u8> {
        self.overlays
            .entry(number)
            .or_insert_with(|| vec![0x11; 0x100])
    }

    fn overlay_alloc(&mut self, number: i32, length: usize) -> usize {
        let allocator = self
            .overlay_allocators
            .entry(number)
            .or_insert_with(|| Allocator::new(0x100, usize::MAX, Vec::new()));
        allocator.alloc(length)
    }

    fn reserve_overlay(&mut self, number: i32, start: usize, end: usize) {
        self.overlay_allocators
            .entry(number)
            .or_insert_with(|| Allocator::new(0x100, usize::MAX, Vec::new()))
            .reserve(start, end);
    }

    fn place_in_overlay(&mut self, number: i32, bytes: &[u8], extra: usize) -> usize {
        let start = self.overlay_alloc(number, bytes.len() + extra);
        put_bytes(self.overlay(number), start, bytes);
        super::blob::grow(self.overlay(number), start + bytes.len() + extra);
        start
    }

    fn place_in_arm9(&mut self, bytes: &[u8], extra: usize) -> usize {
        let start = self.arm9_allocator.alloc(bytes.len() + extra);
        put_bytes(&mut self.arm9, start, bytes);
        start
    }

    pub fn build(mut self) -> Vec<u8> {
        self.prepare_arm9_layout();
        self.build_text();
        self.build_pokemon();
        self.build_moves();
        self.build_trainers();
        self.build_encounters();
        self.build_statics();
        self.build_starters();
        self.build_items();
        self.build_tms_and_tutors();
        self.build_misc_arm9();
        self.build_overlay_tables();
        self.build_scripts_and_text_files();
        self.finish_arm9();
        let overlay_count = self
            .entry
            .overlay_expected_crc32s
            .keys()
            .chain(self.overlays.keys())
            .copied()
            .max()
            .unwrap_or(0)
            + 1;
        let overlays: Vec<Vec<u8>> = (0..overlay_count)
            .map(|number| {
                self.overlays
                    .get(&number)
                    .cloned()
                    .unwrap_or_else(|| vec![0x11; 0x100])
            })
            .collect();
        for path in self.entry.files.values().map(|file| file.path.clone()) {
            self.files
                .entry(path)
                .or_insert_with(|| narc(vec![vec![0; 4]]));
        }
        NdsImage {
            code: self.entry.rom_code.clone(),
            version: self.entry.version as u8,
            arm9: self.arm9,
            overlays,
            files: self.files,
        }
        .build()
    }

    fn arm9_tweak_ranges(&self) -> Vec<(usize, usize)> {
        [
            "FastestTextTweak",
            "NewIndexToMusicTweak",
            "NewRoamerSubroutineTweak",
            "NewCatchingTutorialSubroutineTweak",
        ]
        .iter()
        .filter_map(|tweak| self.entry.tweak(tweak))
        .filter_map(patch_bytes)
        .flat_map(ips_ranges)
        .collect()
    }

    fn extension_tweaks(&self) -> Vec<(usize, usize)> {
        [
            "NewIndexToMusicTweak",
            "NewRoamerSubroutineTweak",
            "NewCatchingTutorialSubroutineTweak",
        ]
        .iter()
        .filter_map(|tweak| self.entry.tweak(tweak))
        .filter_map(patch_bytes)
        .flat_map(ips_ranges)
        .collect()
    }

    fn fixed_arm9_ranges(&self) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        let mut add = |start: i32, length: i32| {
            if start > 0 {
                ranges.push((start as usize, (start + length) as usize));
            }
        };
        add(
            self.int("HiddenItemTableOffset"),
            self.int("HiddenItemCount") * 8,
        );
        add(self.int("MapTableARM9Offset"), MAP_COUNT as i32 * 24);
        if !self.is_hgss() {
            add(self.int("FossilTableOffset"), 2 + 7 * 4);
        }
        add(self.int("CatchingTutorialOpponentMonOffset"), 4);
        add(self.int("CatchingTutorialPlayerMonOffset"), 1);
        add(self.int("CatchingTutorialPlayerLevelOffset"), 1);
        if self.is_dp() && !self.entry.roaming_pokemon.is_empty() {
            add(self.int("RoamingPokemonFunctionStartOffset"), 0x100);
            for roamer in &self.entry.roaming_pokemon {
                for &offset in &roamer.species_code_offsets {
                    add(offset, 2);
                }
                for &offset in &roamer.level_code_offsets {
                    add(offset, 1);
                }
            }
        }
        ranges
    }

    fn prepare_arm9_layout(&mut self) {
        let extend_by = self.int("Arm9ExtensionSize").max(0) as usize;
        let mut reserved = self.fixed_arm9_ranges();
        let extension = self.extension_tweaks();
        let extension_end = extension.iter().map(|&(_, end)| end).max();
        let dtcm_start = match extension_end {
            Some(end) if end > extend_by => end - extend_by,
            _ => {
                let used = reserved
                    .iter()
                    .chain(self.arm9_tweak_ranges().iter())
                    .map(|&(_, end)| end)
                    .max()
                    .unwrap_or(0);
                used.max(0x20000) + 0x4000
            }
        };
        let itcm_start = dtcm_start - ITCM_SIZE;
        reserved.extend(
            self.arm9_tweak_ranges()
                .into_iter()
                .filter(|&(start, _)| start < itcm_start),
        );
        self.arm9 = vec![0u8; dtcm_start + DTCM_TAIL];
        let length = self.arm9.len();
        for byte in &mut self.arm9[length - 0x10..] {
            *byte = 0x55;
        }
        self.arm9_allocator = Allocator::new(ARM9_START, itcm_start, reserved);
        let prefix = hex(self.entry.string("TCMCopyingPrefix"));
        if !prefix.is_empty() {
            let pointers = self.place_in_arm9(&prefix, 12) + prefix.len();
            let destinations = dtcm_start + 0x20;
            put_u32(
                &mut self.arm9,
                pointers,
                (destinations as u32) + gen4::ARM9_OFFSET as u32,
            );
            put_u32(
                &mut self.arm9,
                pointers + 4,
                length as u32 + gen4::ARM9_OFFSET as u32,
            );
            put_u32(
                &mut self.arm9,
                pointers + 8,
                itcm_start as u32 + gen4::ARM9_OFFSET as u32,
            );
            put_u32(&mut self.arm9, destinations, 0x01FF_8000);
            put_u32(&mut self.arm9, destinations + 4, ITCM_SIZE as u32);
            put_u32(&mut self.arm9, destinations + 8, 0x027E_0000);
            put_u32(&mut self.arm9, destinations + 12, 0x20);
        }
    }

    fn finish_arm9(&mut self) {
        let length = self.arm9.len();
        for byte in &mut self.arm9[length - 0x10..] {
            *byte = 0x55;
        }
    }

    fn build_text(&mut self) {
        let names = self.int("PokemonNamesTextOffset");
        let mut pokemon_names = vec![String::from("-----")];
        pokemon_names.extend((1..=493).map(pokemon_name));
        self.text.insert(names, pokemon_names.clone());
        if self
            .entry
            .string("HasExtraPokemonNames")
            .eq_ignore_ascii_case("Yes")
        {
            self.text.insert(
                names + 1,
                pokemon_names
                    .iter()
                    .map(|name| format!("a {name}"))
                    .collect(),
            );
        }
        let mut move_names = vec![String::from("-")];
        move_names.extend((1..=467).map(move_name));
        self.text
            .insert(self.int("MoveNamesTextOffset"), move_names);
        self.text.insert(
            self.int("MoveDescriptionsTextOffset"),
            (0..=467)
                .map(|number| {
                    format!("A move with the\\nnumber {number} that hits\\nthe foe hard.")
                })
                .collect(),
        );
        self.text.insert(
            self.int("AbilityNamesTextOffset"),
            (0..=123).map(|number| format!("Ability{number}")).collect(),
        );
        self.text.insert(
            self.int("ItemNamesTextOffset"),
            (0..=536).map(|number| format!("Item{number}")).collect(),
        );
        self.text.insert(
            self.int("ItemDescriptionsTextOffset"),
            (0..=536)
                .map(|number| format!("An item\\nnumbered {number}."))
                .collect(),
        );
        let mut trainer_names = vec![String::new()];
        trainer_names.extend((1..=TRAINER_COUNT).map(|number| {
            if number % 97 == 0 {
                format!("Ann\\andBo{number}")
            } else {
                format!("Trainer{number}")
            }
        }));
        self.text
            .insert(self.int("TrainerNamesTextOffset"), trainer_names);
        self.text.insert(
            self.int("TrainerClassesTextOffset"),
            (0..CLASS_COUNT)
                .map(|number| format!("Class{number}"))
                .collect(),
        );
        let mut map_names: Vec<String> = (0..MAP_NAME_COUNT)
            .map(|number| format!("Place {number}"))
            .collect();
        map_names[0] = "Route 1".to_owned();
        map_names[1] = "Route 10".to_owned();
        self.text.insert(self.int("MapNamesTextOffset"), map_names);
        self.text_entry(self.int("StarterScreenTextOffset"), 8);
        self.text.insert(
            self.int("PokedexSpeciesTextOffset"),
            (0..=493).map(|number| format!("Kind{number}")).collect(),
        );
        self.text_entry(self.int("StarterLocationTextOffset"), 40);
        self.text_entry(self.int("MiscUITextOffset"), 400);
    }

    fn base_stats(&self, number: usize) -> Vec<u8> {
        let mut stats = vec![0u8; 44];
        for (stat, value) in stats.iter_mut().take(6).enumerate() {
            *value = ((number * (stat + 3)) % 120 + 20) as u8;
        }
        stats[6] = type_byte(number);
        stats[7] = if number.is_multiple_of(3) {
            type_byte(number)
        } else {
            type_byte(number / 3 + 5)
        };
        stats[8] = ((number * 7) % 250 + 3) as u8;
        let common = if number.is_multiple_of(4) {
            items::ORAN_BERRY
        } else {
            0
        };
        let rare = if number.is_multiple_of(8) {
            items::SITRUS_BERRY
        } else {
            common
        };
        stats[12..14].copy_from_slice(&(common as u16).to_le_bytes());
        stats[14..16].copy_from_slice(&(rare as u16).to_le_bytes());
        stats[16] = match number % 11 {
            0 => 0xFE,
            1 => 0xFF,
            2 => 0,
            _ => 0x7F,
        };
        stats[19] = (number % 6) as u8;
        stats[22] = (number % 123 + 1) as u8;
        stats[23] = if number.is_multiple_of(5) {
            0
        } else {
            ((number * 3) % 123 + 1) as u8
        };
        for byte in 0..13 {
            stats[28 + byte] = ((number * 31 + byte * 17) % 256) as u8;
        }
        stats
    }

    fn evolution_entry(&self, number: usize) -> Vec<u8> {
        let mut entry = vec![0u8; 44];
        let mut slot = 0;
        let mut add = |method: u32, extra: u32, target: u32| {
            let offset = slot * 6;
            entry[offset..offset + 6].copy_from_slice(&words(&[method, extra, target]));
            slot += 1;
        };
        match number as i32 {
            species::EEVEE => {
                add(2, 0, species::ESPEON as u32);
                add(3, 0, species::UMBREON as u32);
                add(7, items::FIRE_STONE as u32, species::FLAREON as u32);
                if self.is_hgss() {
                    add(25, 0, species::LEAFEON as u32);
                    add(26, 0, species::GLACEON as u32);
                }
            }
            species::NINCADA => {
                add(13, 20, species::NINJASK as u32);
                add(14, 20, species::SHEDINJA as u32);
            }
            species::SLOWPOKE => {
                add(4, 37, species::SLOWBRO as u32);
                add(6, items::KINGS_ROCK as u32, species::SLOWKING as u32);
            }
            species::ONIX => add(6, items::METAL_COAT as u32, species::STEELIX as u32),
            species::KADABRA => add(5, 0, species::ALAKAZAM as u32),
            species::PILOSWINE => add(20, moves::ANCIENT_POWER as u32, species::MAMOSWINE as u32),
            species::MANTYKE => add(21, species::REMORAID as u32, species::MANTINE as u32),
            species::FEEBAS => add(15, 170, species::MILOTIC as u32),
            species::MAGNETON => add(24, 0, species::MAGNEZONE as u32),
            species::SNEASEL => add(19, items::RAZOR_CLAW as u32, species::WEAVILE as u32),
            species::GLIGAR => add(18, items::RAZOR_FANG as u32, species::GLISCOR as u32),
            species::CHANSEY => add(1, 0, species::BLISSEY as u32),
            _ if !number.is_multiple_of(3) && number < 480 => {
                add(4, (number % 40 + 10) as u32, number as u32 + 1)
            }
            _ => {}
        }
        entry
    }

    fn build_pokemon(&mut self) {
        let personal: Vec<Vec<u8>> = (0..PERSONAL_FILES)
            .map(|number| self.base_stats(number))
            .collect();
        self.put_file("PokemonStats", narc(personal));
        let evolutions: Vec<Vec<u8>> = (0..=493)
            .map(|number| self.evolution_entry(number))
            .collect();
        self.put_file("PokemonEvolutions", narc(evolutions));
        let mut movesets = Vec::new();
        for file in 0..PERSONAL_FILES {
            let mut moveset = Vec::new();
            if file != 0 && file != 494 && file != 495 {
                for slot in 0..6usize {
                    let mv = (file * 11 + slot * 37) % 467 + 1;
                    let level = 1 + slot * 7 + file % 5;
                    moveset.push((mv & 0xFF) as u8);
                    moveset.push(((level << 1) as u8) | u8::from(mv > 255));
                }
            }
            moveset.extend_from_slice(&[0xFF, 0xFF]);
            if moveset.len() % 4 != 0 {
                moveset.extend_from_slice(&[0, 0]);
            }
            movesets.push(moveset);
        }
        self.put_file("PokemonMovesets", narc(movesets));
        self.put_file("BabyPokemon", vec![0u8; 494 * 2 + 2]);
        self.put_file("PokemonGraphics", narc(vec![vec![0; 16]; 6]));
    }

    fn build_moves(&mut self) {
        let effects = [
            0, 1, 2, 3, 4, 5, 6, 10, 18, 31, 33, 42, 43, 48, 49, 50, 66, 68, 75, 76, 80, 106, 118,
            138, 145, 148, 151, 155, 166, 182, 198, 200, 205, 206, 253, 262, 263, 267, 272, 273,
            276,
        ];
        let files: Vec<Vec<u8>> = (0..=467usize)
            .map(|number| {
                let mut data = vec![0u8; 16];
                data[0..2].copy_from_slice(&(effects[number % effects.len()] as u16).to_le_bytes());
                data[2] = (number % 3) as u8;
                data[3] = if number % 3 == 2 {
                    0
                } else {
                    ((number * 3) % 140 + 20) as u8
                };
                data[4] = if number == moves::CURSE as usize {
                    0x09
                } else {
                    type_byte(number + 2)
                };
                data[5] = if number % 9 == 0 {
                    0
                } else {
                    (number % 50 + 51) as u8
                };
                data[6] = ((number % 7) * 5 + 5) as u8;
                data[7] = (number % 4 * 10) as u8;
                data[8..10]
                    .copy_from_slice(&(if number % 5 == 0 { 16u16 } else { 0 }).to_le_bytes());
                data[10] = if number % 13 == 0 {
                    0xFF
                } else {
                    (number % 2) as u8
                };
                data[11] = (number % 2) as u8;
                data
            })
            .collect();
        self.put_file("MoveData", narc(files));
    }

    fn trainer_pokemon_size(&self, poketype: usize) -> usize {
        let mut size = 6;
        if !self.is_dp() {
            size += 2;
        }
        if poketype & 1 != 0 {
            size += 8;
        }
        if poketype & 2 != 0 {
            size += 2;
        }
        size
    }

    fn build_trainers(&mut self) {
        let mut trainer_data = vec![vec![0u8; 20]];
        let mut trainer_pokemon = vec![vec![0u8; 8]];
        for index in 1..=TRAINER_COUNT {
            let poketype = index % 4;
            let count = index % 6 + 1;
            let mut data = vec![0u8; 20];
            data[0] = poketype as u8;
            data[1] = (index % CLASS_COUNT) as u8;
            data[3] = count as u8;
            data[16] = u8::from(index % 50 == 0);
            trainer_data.push(data);
            let size = self.trainer_pokemon_size(poketype);
            let mut pokes = vec![0u8; size * count];
            for slot in 0..count {
                let base = slot * size;
                let species_number = if index == 0x1F0 && slot == count - 1 {
                    species::CYNDAQUIL as usize
                } else {
                    valid_species(index * 7 + slot * 3) as usize
                };
                pokes[base] = ((index * 3) % 256) as u8;
                pokes[base + 1] = if index % 7 == 0 { 0x20 } else { 0 };
                pokes[base + 2] = (index % 60 + slot + 2) as u8;
                pokes[base + 4] = (species_number & 0xFF) as u8;
                pokes[base + 5] = (species_number >> 8) as u8;
                let mut offset = base + 6;
                if poketype & 2 != 0 {
                    pokes[offset..offset + 2]
                        .copy_from_slice(&(items::ORAN_BERRY as u16).to_le_bytes());
                    offset += 2;
                }
                if poketype & 1 != 0 {
                    for mv in 0..4 {
                        let value = ((index + mv * 29) % 467 + 1) as u16;
                        pokes[offset + mv * 2..offset + mv * 2 + 2]
                            .copy_from_slice(&value.to_le_bytes());
                    }
                }
            }
            trainer_pokemon.push(pokes);
        }
        self.put_file("TrainerData", narc(trainer_data));
        self.put_file("TrainerPokemon", narc(trainer_pokemon));
        let mut sub_seq = vec![vec![0u8; 0x100]; 6];
        sub_seq[self.int("TrainerEndFileNumber").max(0) as usize]
            [self.int("TrainerEndTextBoxOffset").max(0) as usize % 0x100] = 4;
        self.put_file("BattleSkillSubSeq", narc(sub_seq));
    }

    fn dppt_area(&self, index: usize) -> Vec<u8> {
        let mut data = vec![0u8; 424];
        let grass_rate = if index == DPPT_WILD_FILES - 1 {
            0
        } else {
            10 + index % 20
        };
        put_u32(&mut data, 0, grass_rate as u32);
        for slot in 0..12 {
            put_u32(&mut data, 4 + slot * 8, (index % 30 + slot + 2) as u32);
            put_u32(
                &mut data,
                8 + slot * 8,
                regular_species(index * 13 + slot * 7),
            );
        }
        if index >= 160 {
            for slot in 0..4 {
                if (index + slot).is_multiple_of(2) {
                    put_u32(
                        &mut data,
                        108 + slot * 4,
                        regular_species(index + slot * 31),
                    );
                }
            }
            for slot in [0usize, 1, 6, 7, 8, 9, 10, 14, 19] {
                let offset = 100 + slot * 4 + if slot >= 10 { 24 } else { 0 };
                put_u32(&mut data, offset, regular_species(index * 3 + slot));
            }
        }
        let mut offset = 204;
        for set in 0..5 {
            let enabled =
                set == 0 || (index >= 160 && set != 1) || (set == 1 && index.is_multiple_of(2));
            if enabled {
                put_u32(&mut data, offset, 5 + set as u32);
            }
            for slot in 0..5 {
                let min = (index % 20 + 5) as u32;
                put_u32(&mut data, offset + 4 + slot * 8, (min << 8) + min + 5);
                put_u32(
                    &mut data,
                    offset + 8 + slot * 8,
                    regular_species(index * 5 + slot * 11 + set),
                );
            }
            offset += 44;
        }
        data
    }

    fn hgss_area(&self, index: usize) -> Vec<u8> {
        let mut data = vec![0u8; 196];
        let rates = [
            20u8,
            if index.is_multiple_of(2) { 10 } else { 0 },
            if index.is_multiple_of(5) { 5 } else { 0 },
            15,
            if index.is_multiple_of(3) { 15 } else { 0 },
            if index.is_multiple_of(4) { 15 } else { 0 },
        ];
        data[..6].copy_from_slice(&rates);
        for slot in 0..12 {
            data[8 + slot] = (index % 40 + slot + 2) as u8;
            for time in 0..3 {
                put_u16(
                    &mut data,
                    20 + time * 24 + slot * 2,
                    regular_species(index * 13 + slot * 7 + time),
                );
            }
        }
        if index.is_multiple_of(6) {
            for slot in 0..4 {
                put_u16(&mut data, 92 + slot * 2, regular_species(index + slot * 41));
            }
        }
        let mut offset = 100;
        for (set, amount) in [0usize, 5, 2, 5, 5, 5].into_iter().enumerate().skip(1) {
            for slot in 0..amount {
                data[offset + slot * 4] = (index % 30 + 5) as u8;
                data[offset + slot * 4 + 1] = (index % 30 + 10) as u8;
                put_u16(
                    &mut data,
                    offset + slot * 4 + 2,
                    regular_species(index * 5 + slot * 11 + set),
                );
            }
            offset += 4 * amount;
        }
        if index.is_multiple_of(7) {
            put_u16(&mut data, offset, regular_species(index + 3));
            put_u16(&mut data, offset + 2, regular_species(index + 4));
        }
        if index.is_multiple_of(3) {
            put_u16(&mut data, offset + 4, regular_species(index + 5));
        }
        if index.is_multiple_of(9) {
            put_u16(&mut data, offset + 6, regular_species(index + 6));
        }
        data
    }

    fn build_encounters(&mut self) {
        if self.is_hgss() {
            let wild: Vec<Vec<u8>> = (0..HGSS_WILD_FILES)
                .map(|index| self.hgss_area(index))
                .collect();
            self.put_file("WildPokemon", narc(wild));
            let headbutt: Vec<Vec<u8>> = (0..MAP_COUNT)
                .map(|index| {
                    if index % 5 == 3 {
                        return vec![1, 0, 1, 0];
                    }
                    let mut data = vec![0u8; 4 + 18 * 4];
                    data[0] = 3;
                    if !(self.quirks.empty_headbutt_tree && index == 7) {
                        for slot in 0..18 {
                            let gap = if self.quirks.empty_headbutt_tree {
                                slot % 4 == 3
                            } else {
                                slot >= 14
                            };
                            if !gap {
                                put_u16(&mut data, 4 + slot * 4, regular_species(index * 3 + slot));
                                data[6 + slot * 4] = (index % 20 + 3) as u8;
                                data[7 + slot * 4] = (index % 20 + 9) as u8;
                            }
                        }
                    }
                    data
                })
                .collect();
            self.put_file("HeadbuttPokemon", narc(headbutt));
            let mut bcc = vec![0u8; 320];
            for block in 0..4 {
                if self.quirks.empty_bug_contest_day && block == 2 {
                    continue;
                }
                for slot in 0..10 {
                    let offset = block * 80 + slot * 8;
                    put_u16(&mut bcc, offset, regular_species(block * 10 + slot + 10));
                    bcc[offset + 2] = 10;
                    bcc[offset + 3] = 15;
                }
            }
            self.put_file("BCCWilds", bcc);
        } else {
            let wild: Vec<Vec<u8>> = (0..DPPT_WILD_FILES)
                .map(|index| self.dppt_area(index))
                .collect();
            self.put_file("WildPokemon", narc(wild));
            let mut extra = vec![vec![0u8; 4]; 11];
            extra[0] = longs(&[species::FEEBAS as u32]);
            for file in self.array("HoneyTreeOffsets") {
                extra[file as usize] = longs(
                    &(0..6)
                        .map(|slot| regular_species(file as usize * 6 + slot))
                        .collect::<Vec<_>>(),
                );
            }
            extra[8] = longs(
                &(0..16)
                    .map(|slot| regular_species(slot * 9 + 1))
                    .collect::<Vec<_>>(),
            );
            for file in [9usize, 10] {
                extra[file] = longs(
                    &(0..32)
                        .map(|slot| regular_species(file * 32 + slot))
                        .collect::<Vec<_>>(),
                );
            }
            self.put_file("ExtraEncounters", narc(extra));
            let overlay = self.int("EncounterOvlNumber");
            let feebas =
                self.place_in_overlay(overlay, &hex(gen4::FEEBAS_LEVEL_PREFIX_DPPT), 8) + 6;
            put_bytes(self.overlay(overlay), feebas, &[20, 0, 0, 0, 10]);
            let honey =
                self.place_in_overlay(overlay, &hex(gen4::HONEY_TREE_LEVEL_PREFIX_DPPT), 160) + 10;
            put_bytes(self.overlay(overlay), honey + 46, &[0x0B, 0x2E]);
            put_u8(self.overlay(overlay), honey + 102, 15);
            put_u32(self.overlay(overlay), honey + 148, 0x1746);
        }
        let area_size = [
            "PokedexAreaDataDungeonIndex",
            "PokedexAreaDataOverworldIndex",
            "PokedexAreaDataDungeonSpecialPreNationalIndex",
            "PokedexAreaDataDungeonSpecialPostNationalIndex",
            "PokedexAreaDataOverworldSpecialPreNationalIndex",
            "PokedexAreaDataOverworldSpecialPostNationalIndex",
            "PokedexAreaDataDungeonSpecialIndex",
            "PokedexAreaDataOverworldSpecialIndex",
        ]
        .iter()
        .map(|key| self.int(key) as usize + 3 * 495 + 1)
        .max()
        .unwrap_or(0);
        self.put_file("PokedexAreaData", narc(vec![vec![0u8; 4]; area_size]));
        let mut map_table = vec![0u8; MAP_COUNT * 16];
        map_table[0] = 1;
        self.put_file("MapTableFile", map_table);
        let base = self.int("MapTableARM9Offset") as usize;
        let two_bytes = self.int("MapTableNameIndexSize") == 2;
        for map in 0..MAP_COUNT {
            let header = base + map * 24;
            let name = (map % MAP_NAME_COUNT) as u32;
            if two_bytes {
                put_u16(&mut self.arm9, header + 18, name);
            } else {
                put_u8(&mut self.arm9, header + 18, name);
            }
            if self.is_hgss() {
                put_u8(
                    &mut self.arm9,
                    header,
                    if map < HGSS_WILD_FILES {
                        map as u32
                    } else {
                        255
                    },
                );
            } else {
                put_u16(
                    &mut self.arm9,
                    header + 14,
                    if map < DPPT_WILD_FILES {
                        map as u32
                    } else {
                        65535
                    },
                );
            }
        }
    }

    fn static_species_for(&self, index: usize, has_forme: bool) -> u32 {
        if has_forme {
            species::GIRATINA as u32
        } else {
            regular_species(index * 17 + 140)
        }
    }

    fn build_statics(&mut self) {
        let entries = self.entry.static_pokemon.clone();
        for (index, entry) in entries.iter().enumerate() {
            for (number, level) in entry.level_entries.iter().enumerate() {
                let script = self.script(level.file, level.offset as usize + 1);
                script[level.offset as usize] = (30 + index + number) as u8;
            }
            for forme in &entry.forme_entries {
                let script = self.script(forme.file, forme.offset as usize + 1);
                script[forme.offset as usize] = 1;
            }
            let value = self.static_species_for(index, !entry.forme_entries.is_empty());
            for species_entry in &entry.species_entries {
                let script = self.script(species_entry.file, species_entry.offset as usize + 2);
                put_u16(script, species_entry.offset as usize, value);
            }
            if let Some(texts) = &entry.game_corner_texts {
                for text in texts {
                    let line = format!("{}\\nCoins", pokemon_name(value as usize));
                    self.set_text(text.text_index, text.string_number as usize, line);
                }
            }
        }
        let roamers = self.entry.roaming_pokemon.clone();
        for (index, roamer) in roamers.iter().enumerate() {
            for entry in roamer
                .species_script_offsets
                .iter()
                .chain(&roamer.gender_offsets)
            {
                let script = self.script(entry.file, entry.offset as usize + 2);
                put_u16(script, entry.offset as usize, 0);
            }
            if self.is_dp() {
                for &offset in &roamer.species_code_offsets {
                    put_u16(
                        &mut self.arm9,
                        offset as usize,
                        [species::MESPRIT, species::CRESSELIA][index % 2] as u32,
                    );
                }
                for &offset in &roamer.level_code_offsets {
                    put_u8(&mut self.arm9, offset as usize, 50);
                }
            }
        }
        if self.is_dp() && !roamers.is_empty() {
            let function = self.int("RoamingPokemonFunctionStartOffset") as usize;
            put_bytes(&mut self.arm9, function + 42, &[0x7A, 0x27, 0xBF, 0x00]);
            put_u32(&mut self.arm9, function + 244, species::DARKRAI as u32);
        }
        let trades = self.array("StaticPokemonTrades");
        let trade_scripts = self.array("StaticPokemonTradeScripts");
        let trade_levels = self.array("StaticPokemonTradeLevelOffsets");
        for (index, _) in trades.iter().enumerate() {
            if let (Some(&script), Some(&offset)) =
                (trade_scripts.get(index), trade_levels.get(index))
            {
                let script = self.script(script, offset as usize + 1);
                script[offset as usize] = 20;
            }
        }
        if self.int("MysteryEggOffset") > 0 {
            let field = self.int("FieldOvlNumber");
            let offset = self.int("MysteryEggOffset") as usize;
            self.reserve_overlay(field, offset, offset + 1);
            put_u8(self.overlay(field), offset, species::TOGEPI as u32);
        }
        if self.int("FossilTableOffset") > 0 {
            let base = self.int("FossilTableOffset") as usize;
            let level_script = self.int("FossilLevelScriptNumber");
            let level_offset = self.int("FossilLevelOffset") as usize;
            self.script(level_script, level_offset + 1)[level_offset] = 20;
            let fossils = [138u32, 140, 142, 345, 347, 408, 410];
            if self.is_hgss() {
                let overlay = self.int("FossilTableOvlNumber");
                self.reserve_overlay(overlay, base, base + 32);
                for (slot, &fossil) in fossils.iter().enumerate() {
                    put_u16(self.overlay(overlay), base + 2 + slot * 4, fossil);
                }
            } else {
                for (slot, &fossil) in fossils.iter().enumerate() {
                    put_u16(&mut self.arm9, base + 2 + slot * 4, fossil);
                }
            }
        }
    }

    fn build_starters(&mut self) {
        let overlay = self.int("StarterPokemonOvlNumber");
        if self.is_hgss() {
            let mut code = words(&[152, 0, 155, 0, 158]);
            code.extend_from_slice(&[0, 0, 0]);
            code.extend_from_slice(gen4::HGSS_STARTER_CODE_SUFFIX);
            self.place_in_arm9(&code, 0);
            self.place_in_overlay(overlay, &hex(gen4::STARTER_CRIES_PREFIX), 12);
            for &file in gen4::HGSS_FILES_WITH_RIVAL_SCRIPT {
                let script = self.script(file, 0x40);
                put_bytes(script, 0x10, gen4::HGSS_RIVAL_SCRIPT_MAGIC);
                put_u32(script, 0x10 + 13, 4);
                put_u8(script, 0x10 + 21, 0x11);
                put_u16(script, 0x10 + 25, species::CYNDAQUIL as u32);
            }
            return;
        }
        let offset = self.int("StarterPokemonOffset") as usize;
        self.reserve_overlay(overlay, offset, offset + 12);
        put_bytes(self.overlay(overlay), offset, &longs(&[387, 390, 393]));
        let graphics = hex(self.entry.string("StarterPokemonGraphicsPrefix"));
        if !graphics.is_empty() {
            let start = self.place_in_overlay(overlay, &graphics, 0x60) + graphics.len();
            for delta in 0..0x60 {
                put_u8(self.overlay(overlay), start + delta, (0x30 + delta) as u32);
            }
            let inner = hex(self.entry.string("StarterPokemonGraphicsPrefixInner"));
            self.place_in_overlay(overlay, &inner, 4);
        }
        let held_script = self.int("StarterPokemonScriptOffset");
        let held_offset = self.int("StarterPokemonHeldItemOffset") as usize;
        put_u16(
            self.script(held_script, held_offset + 2),
            held_offset,
            items::ORAN_BERRY as u32,
        );
        let rival_files = if self.entry.rom_type == gen4::TYPE_PLAT {
            gen4::PT_FILES_WITH_RIVAL_SCRIPT
        } else {
            gen4::DP_FILES_WITH_RIVAL_SCRIPT
        };
        for &file in rival_files {
            let script = self.script(file, 0x40);
            put_bytes(script, 0x08, gen4::DPPT_RIVAL_SCRIPT_MAGIC);
            put_u32(script, 0x08 + 13, 8);
            put_u16(script, 0x08 + 25, 0xE5);
        }
        let tag_files = if self.entry.rom_type == gen4::TYPE_PLAT {
            gen4::PT_FILES_WITH_TAG_SCRIPT
        } else {
            gen4::DP_FILES_WITH_TAG_SCRIPT
        };
        for &file in tag_files {
            let script = self.script(file, 0x60);
            let base = 0x08;
            put_bytes(script, base, gen4::DPPT_TAG_BATTLE_SCRIPT_MAGIC1);
            put_bytes(script, base + 10, gen4::DPPT_TAG_BATTLE_SCRIPT_MAGIC2);
            put_u32(script, base + 19, 17);
            put_u16(script, base + 40, 0x1B);
            put_u16(script, base + 0x21, species::TURTWIG as u32);
        }
    }

    fn field_item(&self, index: usize) -> u32 {
        match index % 6 {
            0 => (items::TM01 as usize + index % 92) as u32,
            1 => items::HM01 as u32,
            2 => 440,
            _ => (index % 300 + 1) as u32,
        }
    }

    fn build_items(&mut self) {
        let set_var = if self.is_hgss() {
            gen4::HGSS_SET_VAR_SCRIPT
        } else {
            gen4::DPPT_SET_VAR_SCRIPT
        } as u32;
        let mut script = vec![0u8; ITEM_BALLS * 4 + 2 + ITEM_BALLS * 6];
        let blocks = ITEM_BALLS * 4 + 2;
        for index in 0..ITEM_BALLS {
            let target = blocks + index * 6;
            put_u32(&mut script, index * 4, (target - (index * 4 + 4)) as u32);
            let command = if index % 10 == 9 { 0x99 } else { set_var };
            put_bytes(
                &mut script,
                target,
                &words(&[
                    command,
                    gen4::ITEM_SCRIPT_VARIABLE as u32,
                    self.field_item(index),
                ]),
            );
        }
        put_u16(
            &mut script,
            ITEM_BALLS * 4,
            gen4::SCRIPT_LIST_TERMINATOR as u32,
        );
        self.scripts
            .insert(self.int("ItemBallsScriptOffset"), script);
        let hidden = self.int("HiddenItemTableOffset") as usize;
        for index in 0..self.int("HiddenItemCount").max(0) as usize {
            let item = (index % 250 + 1) as u32;
            put_u16(&mut self.arm9, hidden + index * 8, item);
        }
        let mut shops = hex(self.entry.string("ShopDataPrefix"));
        for shop in 0..self.int("ShopCount").max(0) as usize {
            for slot in 0..(shop % 5 + 2) {
                shops.extend_from_slice(&words(&[((shop * 7 + slot * 3) % 300 + 1) as u32]));
            }
            if shop % 4 == 0 {
                shops.extend_from_slice(&words(&[0]));
            }
            shops.extend_from_slice(&words(&[0xFFFF]));
        }
        self.place_in_arm9(&shops, 0);
        self.put_file("ItemData", narc(vec![vec![0x33u8; 36]; ITEM_DATA_FILES]));
        let battle = self.int("BattleOvlNumber");
        let mut common = hex(gen4::PICKUP_TABLE_LOCATOR);
        common.extend(words(&(0..14).map(|slot| slot + 20).collect::<Vec<_>>()));
        self.place_in_overlay(battle, &common, 0);
        let mut rare = hex(gen4::RARE_PICKUP_TABLE_LOCATOR);
        rare.extend(words(&(0..8).map(|slot| slot + 200).collect::<Vec<_>>()));
        self.place_in_overlay(battle, &rare, 0);
        let mut trades = Vec::new();
        let trade_count = if self.is_hgss() { 11 } else { 4 };
        for index in 0..trade_count {
            let mut data = vec![0u8; 0x54];
            put_u32(&mut data, 0, regular_species(index * 23 + 5));
            for iv in 0..6 {
                put_u32(&mut data, 4 + iv * 4, (index + iv) as u32 % 32);
            }
            put_u16(&mut data, 0x20, 12345 + index as u32);
            put_u32(&mut data, 0x3C, items::ORAN_BERRY as u32);
            put_u32(&mut data, 0x4C, regular_species(index * 29 + 7));
            trades.push(data);
        }
        let trade_text = self.int("IngameTradesTextOffset");
        let names: Vec<String> = (0..trade_count * 2)
            .map(|index| format!("Nick{index}"))
            .collect();
        self.text.insert(trade_text, names);
        let static_trades = self.array("StaticPokemonTrades");
        let regular: Vec<usize> = (0..trade_count)
            .filter(|index| !static_trades.contains(&(*index as i32)))
            .collect();
        for (position, &offset) in self
            .array("IngameTradePersonTextOffsets")
            .iter()
            .enumerate()
        {
            if offset <= 0 {
                continue;
            }
            let Some(&trade) = regular.get(position) else {
                continue;
            };
            let given = u32::from_le_bytes(trades[trade][0..4].try_into().unwrap()) as usize;
            let line = format!("I want to trade my\\n{} for yours!", pokemon_name(given));
            self.set_text(offset, 1, line.clone());
            if self.is_hgss() && position == 6 {
                self.set_text(offset + 1, 0, line);
            }
        }
        if self.is_hgss() {
            let kenya = self.int("KenyaTextOffset");
            self.set_text(
                kenya,
                2,
                format!(
                    "Take good care of\\n{}!",
                    pokemon_name(species::SPEAROW as usize)
                ),
            );
        }
        self.put_file("InGameTrades", narc(trades));
    }

    fn build_tms_and_tutors(&mut self) {
        let prefix = if self.is_hgss() {
            gen4::HGSS_TM_DATA_PREFIX
        } else {
            gen4::DPPT_TM_DATA_PREFIX
        };
        let mut table = hex(prefix);
        let mut tms = tm_moves();
        tms.extend([15, 19, 57, 70, 432, 249, 127, 431]);
        table.extend(words(&tms));
        self.place_in_arm9(&table, 0);
        let palettes = if self.is_dp() {
            gen4::DP_ITEM_PALETTES_PREFIX
        } else {
            gen4::PTHGSS_ITEM_PALETTES_PREFIX
        };
        self.place_in_arm9(&hex(palettes), 92 * 8);
        let tm_list = tm_moves();
        let tm_texts = self.entry.tm_texts.clone();
        for (&tm, entries) in &tm_texts {
            let mv = tm_list[(tm - 1) as usize] as usize;
            let mut name = move_name(mv);
            if self.is_hgss() && mv == moves::ROAR as usize {
                name = name.to_uppercase();
            }
            for entry in entries {
                self.set_text(
                    entry.text_index,
                    entry.string_number as usize,
                    format!("That TM contains\\n{name}. Use it well!"),
                );
            }
        }
        let corner = self.entry.tm_texts_game_corner.clone();
        for (&tm, entry) in &corner {
            let mv = tm_list[(tm - 1) as usize] as usize;
            self.set_text(
                entry.text_index,
                entry.string_number as usize,
                format!("{}\\nTM{tm:02}", move_name(mv).to_uppercase()),
            );
        }
        let frontier_script = self.int("FrontierScriptNumber");
        let frontier = self.entry.tm_script_offsets_frontier.clone();
        for (&tm, &offset) in &frontier {
            put_u16(
                self.script(frontier_script, offset as usize + 2),
                offset as usize,
                tm_list[(tm - 1) as usize],
            );
        }
        let misc = self.int("MiscUITextOffset");
        let frontier_text = self.entry.tm_texts_frontier.clone();
        for (&tm, &number) in &frontier_text {
            self.set_text(
                misc,
                number as usize,
                format!(
                    "{}\\nBP cost",
                    move_name(tm_list[(tm - 1) as usize] as usize)
                ),
            );
        }
        if self.is_dp() {
            return;
        }
        let field = self.int("FieldOvlNumber");
        let tutor_offset = self.int("MoveTutorMovesOffset") as usize;
        let tutor_count = self.int("MoveTutorCount") as usize;
        let tutor_bytes = self.int("MoveTutorBytesCount") as usize;
        self.reserve_overlay(
            field,
            tutor_offset,
            tutor_offset + tutor_count * tutor_bytes,
        );
        for index in 0..tutor_count {
            let mv = if index == tutor_count - 1 && self.is_hgss() {
                moves::HEADBUTT as u32
            } else {
                ((index * 13 + 3) % 467 + 1) as u32
            };
            put_u16(self.overlay(field), tutor_offset + index * tutor_bytes, mv);
        }
        let compat_offset = self.int("MoveTutorCompatOffset") as usize;
        let compat_bytes = self.int("MoveTutorCompatBytesCount") as usize;
        let compat: Vec<u8> = (0..505 * compat_bytes)
            .map(|index| ((index * 37) % 256) as u8)
            .collect();
        if self.is_hgss() {
            self.put_file("MoveTutorCompat", compat);
            let script = self.script(gen4::ILEX_FOREST_SCRIPT_FILE, 0x1100);
            for &offset in gen4::HEADBUTT_TUTOR_SCRIPT_OFFSETS {
                put_u16(script, offset as usize, moves::HEADBUTT as u32);
            }
            self.set_text(
                gen4::ILEX_FOREST_STRINGS_FILE,
                3,
                format!(
                    "I'll teach your Pokémon\\n{}!",
                    move_name(moves::HEADBUTT as usize)
                ),
            );
        } else {
            let overlay = self.int("MoveTutorCompatOvlNumber");
            self.reserve_overlay(overlay, compat_offset, compat_offset + compat.len());
            put_bytes(self.overlay(overlay), compat_offset, &compat);
        }
    }

    fn build_misc_arm9(&mut self) {
        let mut friendship = hex(gen4::FRIENDSHIP_VALUE_FOR_EVO_LOCATOR);
        friendship.resize(48, 0x01);
        friendship[22] = 220;
        friendship[44] = 220;
        self.place_in_arm9(&friendship, 0);
        self.place_in_arm9(&hex(gen4::SHEDINJA_SPECIES_LOCATOR), 4);
        self.place_in_arm9(&hex(gen4::double_battle_fix_prefix(self.entry.rom_type)), 4);
        for key in [
            "DoubleBattleFlagReturnPrefix",
            "DoubleBattleWalkingPrefix1",
            "DoubleBattleWalkingPrefix2",
            "DoubleBattleTextBoxPrefix",
        ] {
            let prefix = hex(self.entry.string(key));
            if !prefix.is_empty() {
                self.place_in_arm9(&prefix, 8);
            }
        }
        let running = hex(gen4::run_without_running_shoes_prefix(self.entry.rom_type));
        self.place_in_arm9(&running, 0x10);
        if self.is_hgss() {
            let mut table = hex(self.entry.string("IndexToMusicPrefix"));
            if !table.is_empty() {
                for (slot, special) in self.array("SpecialMusicStatics").iter().enumerate() {
                    table.extend(words(&[*special as u32 | ((slot as u32 % 8) << 10)]));
                }
                table.extend(words(&[0x3FF]));
                self.place_in_arm9(&table, 0);
            }
        }
        let opponent = self.int("CatchingTutorialOpponentMonOffset") as usize;
        if opponent > 0 {
            put_u32(&mut self.arm9, opponent, species::BIDOOF as u32);
        }
    }

    fn build_overlay_tables(&mut self) {
        let battle = self.int("BattleOvlNumber");
        let mut table = Vec::new();
        for attacker in [0x00u8, 0x07, 0x11, 0x0A] {
            for defender in [0x05u8, 0x08, 0x07] {
                let effectiveness = match (attacker, defender) {
                    (0x00, 0x07) | (0x07, 0x00) => 0,
                    (_, 0x08) | (0x00, 0x05) => 5,
                    _ => 20,
                };
                table.extend_from_slice(&[attacker, defender, effectiveness]);
            }
        }
        table.extend_from_slice(&[0xFE, 0xFE, 0x00, 0x00, 0x07, 0x00, 0xFF, 0xFF, 0x00]);
        self.place_in_overlay(battle, &table, 0);
        self.place_in_overlay(battle, &hex(gen4::PERFECT_ODDS_BRANCH_LOCATOR), 4);
        for prefix in [
            gen4::HP_BAR_SPEED_PREFIX,
            gen4::EXP_BAR_SPEED_PREFIX,
            gen4::BOTH_BARS_SPEED_PREFIX,
        ] {
            self.place_in_overlay(battle, &hex(prefix), 4);
        }
        let field = self.int("FieldOvlNumber");
        if !self.is_hgss() {
            let mut egg = hex(gen4::DPPT_EGG_MOVE_TABLE_PREFIX);
            for species_number in (1..=490usize).step_by(5) {
                egg.extend(words(&[
                    20000 + species_number as u32,
                    ((species_number * 3) % 467 + 1) as u32,
                    ((species_number * 7) % 467 + 1) as u32,
                ]));
            }
            egg.extend(words(&[0xFFFF]));
            self.place_in_overlay(field, &egg, 0);
            if self.entry.rom_type == gen4::TYPE_PLAT {
                self.place_in_overlay(field, &hex(gen4::DISTORTION_WORLD_GROUND_CHECK_PREFIX), 64);
            }
            let intro = self.int("IntroOvlNumber");
            for prefix in gen4::DPPT_INTRO_PREFIXES {
                let start = self.place_in_overlay(intro, &hex(prefix), 2) + prefix.len() / 2;
                put_u16(self.overlay(intro), start, species::MUNCHLAX as u32);
            }
        } else {
            let mut egg = Vec::new();
            for species_number in (1..=490usize).step_by(5) {
                egg.extend(words(&[
                    20000 + species_number as u32,
                    ((species_number * 3) % 467 + 1) as u32,
                ]));
            }
            egg.extend(words(&[0xFFFF]));
            self.put_file("EggMoves", narc(vec![egg]));
            self.place_in_overlay(field, &hex(gen4::LYRA_ETHAN_MARILL_SPRITE_PREFIX), 4);
            let marill_texts = self.array("MarillTextFiles");
            for text in marill_texts {
                self.set_text(
                    text,
                    1,
                    format!(
                        "My {} loves\\nto play!",
                        pokemon_name(species::MARILL as usize)
                    ),
                );
            }
            let cries = self.entry.marill_cry_script_entries.clone();
            for entry in cries {
                put_u16(
                    self.script(entry.file, entry.offset as usize + 2),
                    entry.offset as usize,
                    species::MARILL as u32,
                );
            }
        }
    }

    fn patch_end(&self, tweak: &str) -> usize {
        self.entry
            .tweak(tweak)
            .and_then(patch_bytes)
            .map(ips_ranges)
            .and_then(|ranges| ranges.iter().map(|&(_, end)| end).max())
            .unwrap_or(0)
    }

    fn build_scripts_and_text_files(&mut self) {
        let national = self.int("NationalDexScriptOffset");
        let national_size = self
            .patch_end("NationalDexAtStartTweak")
            .saturating_sub(if self.is_hgss() { 4 } else { 0 });
        self.script(national, national_size.max(2));
        if self.entry.tweak("FastDistortionWorldTweak").is_some() {
            let size = self
                .patch_end("FastDistortionWorldTweak")
                .saturating_sub(12);
            self.script(gen4::PT_SPEAR_PILLAR_PORTAL_SCRIPT_FILE, size.max(2));
        }
        let script_count = self.scripts.keys().max().copied().unwrap_or(0) as usize + 1;
        let scripts: Vec<Vec<u8>> = (0..script_count)
            .map(|index| {
                self.scripts
                    .get(&(index as i32))
                    .cloned()
                    .unwrap_or_else(|| vec![0x02, 0x00])
            })
            .collect();
        self.put_file("Scripts", narc(scripts));
        self.put_file("Events", narc(vec![vec![0x45; 16]; 3]));
        let text_count = self.text.keys().max().copied().unwrap_or(0) as usize + 2;
        let text: Vec<Vec<u8>> = (0..text_count)
            .map(|index| {
                let strings = self
                    .text
                    .get(&(index as i32))
                    .cloned()
                    .unwrap_or_else(|| vec![format!("Text {index}"), "Filler".to_owned()]);
                poketext::write_strings(&strings, false).expect("encodable synthetic text")
            })
            .collect();
        self.put_file("Text", narc(text));
    }
}
