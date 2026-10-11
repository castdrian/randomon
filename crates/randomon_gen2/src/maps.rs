use randomon_data::constants::gb;
use randomon_gb::{TextTables, bank_of, calculate_offset, read_word};

use crate::constants::{MAP_GROUP_COUNT, MAPS_IN_LAST_GROUP};
use crate::rom_entry::RomEntry;

const HIDDEN_ITEM_SIGNPOST: u8 = 7;
const SIGNPOST_SIZE: i32 = 5;
const PERSON_SIZE: i32 = 13;
const WARP_SIZE: i32 = 5;
const TRIGGER_SIZE: i32 = 8;
const MAP_HEADER_SIZE: i32 = 9;
const MAPS_PER_GROUP_SLOTS: usize = 100;

pub struct MapData {
    pub item_offsets: Vec<i32>,
    names: Vec<Vec<Option<String>>>,
}

impl MapData {
    pub fn name(&self, bank: usize, number: usize) -> String {
        self.names
            .get(bank)
            .and_then(|group| group.get(number))
            .cloned()
            .flatten()
            .unwrap_or_else(|| "null".to_owned())
    }
}

fn load_landmark_names(rom: &[u8], entry: &RomEntry, tables: &TextTables) -> Vec<String> {
    let table = entry.value("LandmarkTableOffset");
    let bank = bank_of(table);
    (0..entry.value("LandmarkCount"))
        .map(|index| {
            let name_offset = calculate_offset(bank, read_word(rom, table + index * 4 + 2));
            tables
                .read_string(rom, name_offset, usize::MAX, false)
                .replace("\\x1F", " ")
        })
        .collect()
}

struct MapReader<'a> {
    rom: &'a [u8],
    landmarks: Vec<String>,
    item_offsets: Vec<i32>,
    names: Vec<Vec<Option<String>>>,
}

impl MapReader<'_> {
    fn byte(&self, offset: i32) -> i32 {
        i32::from(self.rom[offset as usize])
    }

    fn word(&self, offset: i32) -> i32 {
        read_word(self.rom, offset)
    }

    fn process_map_at(&mut self, offset: i32, map_bank: usize, map_number: usize) {
        let second_header = calculate_offset(self.byte(offset), self.word(offset + 3));
        let landmark = self.byte(offset + 5) as usize;
        self.names[map_bank][map_number] = self.landmarks.get(landmark).cloned();
        let event_bank = self.byte(second_header + 6);
        let mut events = calculate_offset(event_bank, self.word(second_header + 9)) + 2;
        let warp_count = self.byte(events);
        events += 1 + warp_count * WARP_SIZE;
        let trigger_count = self.byte(events);
        events += 1 + trigger_count * TRIGGER_SIZE;
        let signpost_count = self.byte(events);
        events += 1;
        self.collect_hidden_items(events, signpost_count, event_bank);
        events += signpost_count * SIGNPOST_SIZE;
        let people_count = self.byte(events);
        events += 1;
        self.collect_item_balls(events, people_count, event_bank);
    }

    fn collect_hidden_items(&mut self, start: i32, count: i32, bank: i32) {
        for signpost in 0..count {
            let base = start + signpost * SIGNPOST_SIZE;
            if self.rom[(base + 2) as usize] == HIDDEN_ITEM_SIGNPOST {
                let script = calculate_offset(bank, self.word(base + 3));
                self.item_offsets.push(script + 2);
            }
        }
    }

    fn collect_item_balls(&mut self, start: i32, count: i32, bank: i32) {
        for person in 0..count {
            let base = start + person * PERSON_SIZE;
            if self.rom[(base + 7) as usize] & 1 == 1 {
                self.item_offsets
                    .push(calculate_offset(bank, self.word(base + 9)));
            }
        }
    }
}

pub fn load_maps(rom: &[u8], entry: &RomEntry, tables: &TextTables) -> MapData {
    let headers = entry.value("MapHeaders");
    let headers_bank = bank_of(headers);
    let mut reader = MapReader {
        rom,
        landmarks: load_landmark_names(rom, entry, tables),
        item_offsets: Vec::new(),
        names: vec![vec![None; MAPS_PER_GROUP_SLOTS]; (MAP_GROUP_COUNT + 1) as usize],
    };
    let group_offsets: Vec<i32> = (0..MAP_GROUP_COUNT)
        .map(|group| calculate_offset(headers_bank, read_word(rom, headers + group * 2)))
        .collect();
    for (group, &start) in group_offsets.iter().enumerate() {
        let is_last = group as i32 == MAP_GROUP_COUNT - 1;
        let max_offset = if is_last {
            (headers_bank + 1) * gb::BANK_SIZE
        } else {
            group_offsets[group + 1]
        };
        let max_map = if is_last {
            MAPS_IN_LAST_GROUP as usize
        } else {
            usize::MAX
        };
        let mut offset = start;
        let mut map = 0usize;
        while offset < max_offset && map < max_map {
            reader.process_map_at(offset, group + 1, map + 1);
            offset += MAP_HEADER_SIZE;
            map += 1;
        }
    }
    MapData {
        item_offsets: reader.item_offsets,
        names: reader.names,
    }
}
