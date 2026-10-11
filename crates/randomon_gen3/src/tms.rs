use randomon_core::Compatibility;
use randomon_formats::bytes::byte_from_flags;

use crate::constants as c;
use crate::handler::Gen3RomHandler;
use crate::text_format::{
    TextBreaks, format_text_with_replacements, rewrite_description_for_new_line_size,
};

const GBA_BREAKS: TextBreaks<'static> = TextBreaks {
    newline: "\\n",
    extra_line: "\\l",
    new_paragraph: "\\p",
};
const ITEM_IMAGE_ENTRY_SIZE: i32 = 8;
const ITEM_IMAGE_PALETTE_OFFSET: i32 = 4;

impl Gen3RomHandler {
    pub(crate) fn tm_moves(&self) -> Vec<i32> {
        let offset = self.value("TmMoves");
        (0..c::TM_COUNT)
            .map(|index| self.rom.word(offset + index * 2))
            .collect()
    }

    pub(crate) fn write_tm_moves(&mut self, moves: &[i32]) {
        let offset = self.value("TmMoves");
        for (index, &mv) in moves.iter().take(c::TM_COUNT as usize).enumerate() {
            self.rom.set_word(offset + index as i32 * 2, mv);
        }
        let duplicate = self.value("TmMovesDuplicate");
        if duplicate > 0 {
            self.rom.copy_within(offset, duplicate, c::TM_COUNT * 2);
        }
        self.update_tm_palettes(moves);
        if !self.update_tm_descriptions(moves) {
            return;
        }
        self.update_script_texts(moves, false);
    }

    fn update_tm_palettes(&mut self, moves: &[i32]) {
        let images = self.value("ItemImages");
        if images <= 0 {
            return;
        }
        let palettes = self.array("TmPals");
        for (index, &number) in moves.iter().take(c::TM_COUNT as usize).enumerate() {
            let Some(mv) = self.core.data.move_by_number(number) else {
                continue;
            };
            let Some(&palette) = palettes.get(usize::from(c::type_to_byte(mv.kind))) else {
                continue;
            };
            let location = images
                + (c::TM_ITEM_OFFSET + index as i32) * ITEM_IMAGE_ENTRY_SIZE
                + ITEM_IMAGE_PALETTE_OFFSET;
            self.rom.set_pointer(location, palette);
        }
    }

    fn item_description_line_size(&self) -> i32 {
        if self.rom_type().is_frlg() {
            c::FRLG_ITEM_DESC_CHARS_PER_LINE
        } else {
            c::RSE_ITEM_DESC_CHARS_PER_LINE
        }
    }

    fn update_tm_descriptions(&mut self, moves: &[i32]) -> bool {
        let descriptions = self.value("MoveDescriptions");
        if descriptions <= 0 {
            return true;
        }
        let item_data = self.value("ItemData");
        let entry_size = self.value("ItemEntrySize");
        let line_size = self.item_description_line_size();
        let free_space = self.value("FreeSpace");
        for (index, &number) in moves.iter().take(c::TM_COUNT as usize).enumerate() {
            let item_base = item_data + (index as i32 + c::TM_ITEM_OFFSET) * entry_size;
            let text_pointer = self.rom.pointer(descriptions + (number - 1) * 4);
            let move_description = self.read_variable_string(text_pointer);
            let table = &self.table;
            let description = rewrite_description_for_new_line_size(
                &move_description,
                "\\n",
                line_size,
                |text| table.length(text),
            );
            let needed = self.translate(&description).len() as i32 + 1;
            let Some(space) = self.rom.allocate(needed, free_space, true) else {
                self.log_line("Couldn't insert new item description.");
                return false;
            };
            self.write_variable_string(&description, space);
            self.rom
                .set_pointer(item_base + c::ITEM_DATA_DESCRIPTION_OFFSET, space);
        }
        true
    }

    fn log_line(&mut self, line: &str) {
        let separator = self.core.engine.log.line_separator().to_owned();
        self.log(&format!("{line}{separator}"));
    }

    fn format_script_text(&self, template: &str, move_name: &str) -> String {
        let placeholder_name = move_name.replace(' ', "_");
        let unformatted = template.replace("[move]", &placeholder_name);
        let table = &self.table;
        let formatted = format_text_with_replacements(
            &unformatted,
            &GBA_BREAKS,
            c::REGULAR_TEXTBOX_CHARS_PER_LINE,
            |text| table.length(text),
        );
        formatted.replace(&placeholder_name, move_name)
    }

    pub(crate) fn update_script_texts(&mut self, moves: &[i32], move_tutors: bool) {
        let free_space = self.value("FreeSpace");
        for index in 0..self.entry.script_texts.len() {
            let text = self.entry.script_texts[index].clone();
            if text.actual_offset <= 0 || text.is_move_tutor != move_tutors {
                continue;
            }
            if self.rom.valid_pointer(text.actual_offset).is_none() {
                let message = if move_tutors {
                    "Couldn't insert new Move Tutor text."
                } else {
                    "Couldn't insert new TM text. Skipping remaining TM text updates."
                };
                self.log_line(message);
                return;
            }
            let move_index = if move_tutors {
                text.number
            } else {
                text.number - 1
            };
            let Some(move_name) = usize::try_from(move_index)
                .ok()
                .and_then(|slot| moves.get(slot))
                .and_then(|&number| self.move_name(number))
            else {
                continue;
            };
            let new_text = self.format_script_text(&text.template, &move_name);
            let needed = self.translate(&new_text).len() as i32 + 1;
            let Some(space) = self.rom.allocate(needed, free_space, true) else {
                let message = if move_tutors {
                    "Couldn't insert new Move Tutor text."
                } else {
                    "Couldn't insert new TM text."
                };
                self.log_line(message);
                return;
            };
            self.write_variable_string(&new_text, space);
            self.repoint_nearby_copies(text.actual_offset, space);
        }
    }

    fn repoint_nearby_copies(&mut self, actual_offset: i32, new_text: i32) {
        let Some(needle) = self.rom.slice(actual_offset, 4).map(<[u8]>::to_vec) else {
            return;
        };
        let start = (actual_offset - c::POINTER_SEARCH_RADIUS).max(0);
        let end = (actual_offset + c::POINTER_SEARCH_RADIUS).min(self.rom.len());
        for location in self.rom.search_range(start, end, &needle) {
            self.rom.set_pointer(location, new_text);
        }
    }

    fn compatibility_flags(&self, offset: i32, bytes: i32, flag_count: usize) -> Vec<bool> {
        let mut flags = vec![false; flag_count];
        for index in 0..bytes {
            let value = self.rom.byte(offset + index);
            for bit in 0..8 {
                if let Some(flag) = flags.get_mut((index * 8 + 1 + bit) as usize) {
                    *flag = (value >> bit) & 1 == 1;
                }
            }
        }
        flags
    }

    pub(crate) fn tmhm_compatibility(&self) -> Compatibility {
        let base = self.value("PokemonTMHMCompat");
        let flag_count = (c::TM_COUNT + c::HM_COUNT + 1) as usize;
        self.real_pokemon()
            .into_iter()
            .map(|id| {
                let offset = base + self.internal_of(id) * c::TMHM_COMPAT_ENTRY_SIZE;
                (
                    id,
                    self.compatibility_flags(offset, c::TMHM_COMPAT_ENTRY_SIZE, flag_count),
                )
            })
            .collect()
    }

    fn write_compatibility_flags(&mut self, offset: i32, bytes: i32, flags: &[bool]) {
        for index in 0..bytes {
            let value = byte_from_flags(flags, (index * 8 + 1) as usize);
            self.rom.set_byte(offset + index, value);
        }
    }

    pub(crate) fn write_tmhm_compatibility(&mut self, compatibility: &Compatibility) {
        let base = self.value("PokemonTMHMCompat");
        for (&id, flags) in compatibility {
            let offset = base + self.internal_of(id) * c::TMHM_COMPAT_ENTRY_SIZE;
            self.write_compatibility_flags(offset, c::TMHM_COMPAT_ENTRY_SIZE, flags);
        }
    }

    pub(crate) fn has_tutors(&self) -> bool {
        matches!(
            self.rom_type(),
            crate::rom_entry::RomType::Emerald | crate::rom_entry::RomType::FireRedLeafGreen
        )
    }

    pub(crate) fn tutor_moves(&self) -> Vec<i32> {
        if !self.has_tutors() {
            return Vec::new();
        }
        let offset = self.value("MoveTutorData");
        (0..self.value("MoveTutorMoves"))
            .map(|index| self.rom.word(offset + index * 2))
            .collect()
    }

    pub(crate) fn write_tutor_moves(&mut self, moves: &[i32]) {
        if !self.has_tutors() {
            return;
        }
        let count = self.value("MoveTutorMoves");
        if count as usize != moves.len() {
            return;
        }
        let offset = self.value("MoveTutorData");
        for (index, &mv) in moves.iter().enumerate() {
            self.rom.set_word(offset + index as i32 * 2, mv);
        }
        self.update_script_texts(moves, true);
    }

    fn tutor_bytes(&self) -> i32 {
        ((self.value("MoveTutorMoves") + 7) & !7) / 8
    }

    pub(crate) fn tutor_compatibility(&self) -> Compatibility {
        if !self.has_tutors() {
            return Compatibility::new();
        }
        let base = self.value("MoveTutorCompatibility");
        let bytes = self.tutor_bytes();
        let flag_count = (self.value("MoveTutorMoves").max(0) + 1) as usize;
        self.real_pokemon()
            .into_iter()
            .map(|id| {
                let offset = base + self.internal_of(id) * bytes;
                (id, self.compatibility_flags(offset, bytes, flag_count))
            })
            .collect()
    }

    pub(crate) fn write_tutor_compatibility(&mut self, compatibility: &Compatibility) {
        if !self.has_tutors() {
            return;
        }
        let base = self.value("MoveTutorCompatibility");
        let bytes = self.tutor_bytes();
        for (&id, flags) in compatibility {
            let offset = base + self.internal_of(id) * bytes;
            self.write_compatibility_flags(offset, bytes, flags);
        }
    }
}
