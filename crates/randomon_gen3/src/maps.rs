use std::collections::HashMap;

use crate::constants as c;
use crate::handler::Gen3RomHandler;

const LARGEST_SANE_BANK_COUNT: usize = 256;
const LARGEST_SANE_MAPS_PER_BANK: i32 = 1024;
const ITEM_SCRIPT_COPYVAR_ITEM: [u8; 3] = [0x1A, 0x00, 0x80];
const ITEM_SCRIPT_COPYVAR_COUNT: [u8; 3] = [0x1A, 0x01, 0x80];
const ITEM_SCRIPT_CALL: u8 = 0x09;
const HIDDEN_ITEM_TYPES: std::ops::RangeInclusive<i32> = 5..=7;
const FRLG_REDIRECTED_TUTORS: [i32; 5] = [5, 8, 9, 10, 11];
const FRLG_DEEP_REDIRECT_TUTOR: i32 = 7;
const FRLG_DEEP_REDIRECT_DISTANCE: i32 = 0x1F;

impl Gen3RomHandler {
    fn bank_overlaps(starts: &[i32], base: i32, offset: i32) -> bool {
        starts.iter().any(|&start| base < start && offset >= start)
    }

    pub(crate) fn determine_map_bank_sizes(&mut self) {
        let headers = self.value("MapHeaders");
        let mut bank_offsets: Vec<i32> = Vec::new();
        let mut offset = headers;
        while bank_offsets.len() < LARGEST_SANE_BANK_COUNT {
            if Self::bank_overlaps(&bank_offsets, headers, offset) {
                break;
            }
            let Some(bank) = self.rom.valid_pointer(offset) else {
                break;
            };
            bank_offsets.push(bank);
            offset += 4;
        }
        let sizes: Vec<i32> = bank_offsets
            .iter()
            .map(|&base| self.count_maps_in_bank(&bank_offsets, base, headers))
            .collect();
        self.entry
            .set_value("MapBankCount", bank_offsets.len() as i32);
        self.entry
            .array_entries
            .insert("MapBankSizes".to_owned(), sizes);
    }

    fn count_maps_in_bank(&self, bank_offsets: &[i32], base: i32, headers: i32) -> i32 {
        let mut count = 0;
        let mut offset = base;
        while count < LARGEST_SANE_MAPS_PER_BANK {
            if Self::bank_overlaps(bank_offsets, base, offset)
                || (base < headers && offset >= headers)
                || self.rom.valid_pointer(offset).is_none()
            {
                break;
            }
            count += 1;
            offset += 4;
        }
        count
    }

    fn map_label_name(&self, label: i32) -> String {
        let labels = self.value("MapLabels");
        let pointer_location = if self.rom_type().is_frlg() {
            labels + (label - c::FRLG_MAP_LABELS_START) * 4
        } else {
            labels + label * 8 + 4
        };
        match self.rom.valid_pointer(pointer_location) {
            Some(pointer) => self.read_variable_string(pointer),
            None => String::new(),
        }
    }

    pub(crate) fn preprocess_maps(&mut self) {
        self.item_offsets.clear();
        let bank_count = self.value("MapBankCount");
        let sizes = self.array("MapBankSizes");
        let headers = self.value("MapHeaders");
        let mut labels: HashMap<i32, String> = HashMap::new();
        let mut names = Vec::with_capacity(bank_count.max(0) as usize);
        for bank in 0..bank_count {
            let bank_offset = self.rom.pointer(headers + bank * 4);
            let map_count = sizes.get(bank as usize).copied().unwrap_or(0);
            let mut bank_names = Vec::with_capacity(map_count.max(0) as usize);
            for map in 0..map_count {
                let header = self.rom.pointer(bank_offset + map * 4);
                let label = self.rom.int(header + c::MAP_HEADER_LABEL_OFFSET);
                let name = match labels.get(&label) {
                    Some(name) => name.clone(),
                    None => {
                        let name = self.map_label_name(label);
                        labels.insert(label, name.clone());
                        name
                    }
                };
                bank_names.push(name);
                if let Some(events) = self.rom.valid_pointer(header + 4) {
                    self.scan_map_events(events, bank, map);
                }
            }
            names.push(bank_names);
        }
        self.map_names = names;
    }

    fn is_item_ball_script(&self, script: i32) -> bool {
        self.rom.matches(script, &ITEM_SCRIPT_COPYVAR_ITEM)
            && self.rom.matches(script + 5, &ITEM_SCRIPT_COPYVAR_COUNT)
            && self.rom.byte(script + 10) == ITEM_SCRIPT_CALL
            && matches!(self.rom.byte(script + 11), 0x00 | 0x01)
    }

    fn scan_map_events(&mut self, events: i32, bank: i32, map: i32) {
        let people_count = self.rom.int(events);
        let signpost_count = self.rom.int(events + 3);
        if people_count > 0 {
            let people = self.rom.pointer(events + 4);
            self.scan_people(people, people_count);
            self.locate_script_texts(people, bank, map);
        }
        if signpost_count > 0 {
            let signposts = self.rom.pointer(events + 16);
            self.scan_hidden_items(signposts, signpost_count);
        }
    }

    fn scan_people(&mut self, people: i32, count: i32) {
        let item_ball = self.value("ItemBallPic");
        for person in 0..count {
            let base = people + person * c::PERSON_EVENT_SIZE;
            let sprite = self.rom.signed(base + 1);
            if sprite != item_ball {
                continue;
            }
            let script = self.rom.pointer(base + 16);
            if script >= 0 && self.is_item_ball_script(script) {
                self.item_offsets.push(script + 3);
            }
        }
    }

    fn script_text_location(&self, people: i32, index: usize) -> Option<i32> {
        let text = &self.entry.script_texts[index];
        let mut script = self
            .rom
            .pointer(people + (text.person_number - 1) * c::PERSON_EVENT_SIZE + 16);
        if script < 0 {
            return None;
        }
        if self.rom_type().is_frlg() && text.is_move_tutor {
            if FRLG_REDIRECTED_TUTORS.contains(&text.number) {
                script = self.rom.pointer(script + 1);
            } else if text.number == FRLG_DEEP_REDIRECT_TUTOR {
                script = self.rom.pointer(script + FRLG_DEEP_REDIRECT_DISTANCE);
            }
        }
        let look_at = script.checked_add(text.offset_in_script)?;
        let plausible = look_at >= 0
            && look_at < self.rom.len() - 2
            && matches!(self.rom.byte(look_at + 3), 0x08 | 0x09);
        plausible.then_some(look_at)
    }

    fn locate_script_texts(&mut self, people: i32, bank: i32, map: i32) {
        for index in 0..self.entry.script_texts.len() {
            let text = &self.entry.script_texts[index];
            if text.map_bank != bank || text.map_number != map {
                continue;
            }
            if let Some(location) = self.script_text_location(people, index) {
                self.entry.script_texts[index].actual_offset = location;
            }
        }
    }

    fn scan_hidden_items(&mut self, signposts: i32, count: i32) {
        for signpost in 0..count {
            let base = signposts + signpost * c::SIGNPOST_EVENT_SIZE;
            let kind = self.rom.signed(base + 5);
            if HIDDEN_ITEM_TYPES.contains(&kind) && self.rom.word(base + 8) != 0 {
                self.item_offsets.push(base + 8);
            }
        }
    }

    pub(crate) fn map_name(&self, bank: i32, map: i32) -> String {
        usize::try_from(bank)
            .ok()
            .and_then(|bank| self.map_names.get(bank))
            .and_then(|names| usize::try_from(map).ok().and_then(|map| names.get(map)))
            .cloned()
            .unwrap_or_else(|| "Unknown map".to_owned())
    }
}
