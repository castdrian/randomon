use randomon_core::{Compatibility, RandomizationError, Result};
use randomon_data::IngameTrade;
use randomon_data::constants::gb;
use randomon_gb::{byte_from_flags, make_gb_pointer, read_byte_into_flags};

use crate::constants::{
    self as c, BS_MT_COMPAT_OFFSET, BS_TMHM_COMPAT_OFFSET, HM_COUNT, POKEMON_COUNT, TM_COUNT,
};
use crate::handler::{Gen2RomHandler, pokemon};

const TUTOR_MOVE_COUNT: usize = 3;
const TUTOR_COMPAT_BITS: std::ops::RangeInclusive<usize> = 1..=3;

struct TradeLayout {
    table: i32,
    size: i32,
    nickname_length: i32,
    ot_length: i32,
    entry_length: i32,
    unused: &'static [i32],
}

impl Gen2RomHandler {
    pub(crate) fn tm_moves(&self) -> Vec<i32> {
        let offset = self.value("TMMovesOffset");
        (0..TM_COUNT)
            .map(|index| self.int(offset + index))
            .collect()
    }

    pub(crate) fn hm_moves(&self) -> Vec<i32> {
        let offset = self.value("TMMovesOffset") + TM_COUNT;
        (0..HM_COUNT)
            .map(|index| self.int(offset + index))
            .collect()
    }

    pub(crate) fn write_tm_moves(&mut self, moves: &[i32]) -> Result<()> {
        let offset = self.value("TMMovesOffset");
        for index in 0..TM_COUNT {
            let mv = *moves
                .get(index as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("moveIndexes"))?;
            self.set_byte(offset + index, mv as u8);
        }
        let names = self.read_move_names();
        for text in &self.entry.tm_texts {
            let mv = *moves
                .get((text.number - 1) as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("moveIndexes"))?;
            let name = names
                .get(mv as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("moveNames"))?;
            let rendered = text.template.replace("%m", name);
            self.write_variable_string(&rendered, text.offset, true);
        }
        Ok(())
    }

    pub(crate) fn tmhm_compatibility(&self) -> Compatibility {
        let mut compatibility = Compatibility::new();
        for number in 1..=POKEMON_COUNT {
            let offset = self.stats_offset(number) + BS_TMHM_COMPAT_OFFSET;
            let mut flags = vec![false; (TM_COUNT + HM_COUNT + 1) as usize];
            for byte in 0..8 {
                read_byte_into_flags(&self.rom, &mut flags, byte * 8 + 1, offset + byte as i32);
            }
            compatibility.insert(pokemon(number), flags);
        }
        compatibility
    }

    pub(crate) fn write_tmhm_compatibility(&mut self, compatibility: &Compatibility) {
        for (&id, flags) in compatibility {
            let offset = self.stats_offset(self.core.data.pk(id).number) + BS_TMHM_COMPAT_OFFSET;
            for byte in 0..8usize {
                let mut value = byte_from_flags(flags, byte * 8 + 1);
                if self.entry.is_crystal && byte == 7 {
                    value |= self.byte(offset + 7) & 0b0000_1110;
                }
                self.set_byte(offset + byte as i32, value);
            }
        }
    }

    pub(crate) fn tutor_moves(&self) -> Vec<i32> {
        if !self.entry.is_crystal {
            return Vec::new();
        }
        self.array("MoveTutorMoves")
            .iter()
            .map(|&offset| self.int(offset))
            .collect()
    }

    pub(crate) fn write_tutor_moves(&mut self, moves: &[i32]) {
        if !self.entry.is_crystal || moves.len() != TUTOR_MOVE_COUNT {
            return;
        }
        for (&offset, &mv) in self.array("MoveTutorMoves").iter().zip(moves) {
            self.set_byte(offset, mv as u8);
        }
        let menu_pointer = self.value("MoveTutorMenuOffset");
        let menu_space = self.value("MoveTutorMenuNewSpace");
        if menu_pointer > 0 && menu_space > 0 {
            self.write_tutor_menu(moves, menu_pointer, menu_space);
        }
    }

    fn write_tutor_menu(&mut self, moves: &[i32], menu_pointer: i32, menu_space: i32) {
        let names = self.read_move_names();
        let mut entries: Vec<String> = moves
            .iter()
            .map(|&mv| names.get(mv as usize).cloned().unwrap_or_default())
            .collect();
        entries.push(c::MT_MENU_CANCEL_STRING.to_owned());
        let mut offset = menu_space;
        self.set_byte(offset, c::MT_MENU_INIT_BYTE);
        self.set_byte(offset + 1, 0x4);
        offset += 2;
        for entry in entries {
            let translated = self.translate(&entry);
            self.write_bytes(offset, &translated);
            offset += translated.len() as i32;
            self.set_byte(offset, gb::STRING_TERMINATOR);
            offset += 1;
        }
        self.set_word(menu_pointer, make_gb_pointer(menu_space));
    }

    pub(crate) fn tutor_compatibility(&self) -> Compatibility {
        let mut compatibility = Compatibility::new();
        if !self.entry.is_crystal {
            return compatibility;
        }
        for number in 1..=POKEMON_COUNT {
            let value = self.byte(self.stats_offset(number) + BS_MT_COMPAT_OFFSET);
            let mut flags = vec![false; 4];
            for bit in TUTOR_COMPAT_BITS {
                flags[bit] = (value >> bit) & 1 == 1;
            }
            compatibility.insert(pokemon(number), flags);
        }
        compatibility
    }

    pub(crate) fn write_tutor_compatibility(&mut self, compatibility: &Compatibility) {
        if !self.entry.is_crystal {
            return;
        }
        for (&id, flags) in compatibility {
            let offset = self.stats_offset(self.core.data.pk(id).number) + BS_MT_COMPAT_OFFSET;
            let mut value = self.byte(offset) & 0x01;
            for bit in TUTOR_COMPAT_BITS {
                if flags.get(bit) == Some(&true) {
                    value |= 1 << bit;
                }
            }
            self.set_byte(offset, value);
        }
    }

    pub(crate) fn current_field_tms(&self) -> Vec<i32> {
        let allowed = c::allowed_items();
        let mut tms = Vec::new();
        for &offset in &self.maps.item_offsets {
            let item = self.int(offset);
            if allowed.is_tm(item) {
                let tm = c::tm_number_for_item(item);
                if !tms.contains(&tm) {
                    tms.push(tm);
                }
            }
        }
        tms
    }

    pub(crate) fn write_field_tms(&mut self, tms: &[i32]) -> Result<()> {
        let allowed = c::allowed_items();
        let mut iterator = tms.iter();
        let mut given = [0i32; 256];
        for offset in self.maps.item_offsets.clone() {
            let item = self.int(offset);
            if !allowed.is_tm(item) {
                continue;
            }
            if given[item as usize] != 0 {
                self.set_byte(offset, given[item as usize] as u8);
            } else {
                let tm = *iterator
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("fieldTMs"))?;
                let replacement = c::item_for_tm_number(tm);
                given[item as usize] = replacement;
                self.set_byte(offset, replacement as u8);
            }
        }
        Ok(())
    }

    pub(crate) fn regular_field_items(&self) -> Vec<i32> {
        let allowed = c::allowed_items();
        self.maps
            .item_offsets
            .iter()
            .map(|&offset| self.int(offset))
            .filter(|&item| allowed.is_allowed(item) && !allowed.is_tm(item))
            .collect()
    }

    pub(crate) fn write_regular_field_items(&mut self, items: &[i32]) -> Result<()> {
        let allowed = c::allowed_items();
        let mut iterator = items.iter();
        for offset in self.maps.item_offsets.clone() {
            let item = self.int(offset);
            if allowed.is_allowed(item) && !allowed.is_tm(item) {
                let replacement = iterator
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("fieldItems"))?;
                self.set_byte(offset, *replacement as u8);
            }
        }
        Ok(())
    }

    fn trade_layout(&self) -> TradeLayout {
        let nickname_length = self.value("TradeNameLength");
        let ot_length = self.value("TradeOTLength");
        let mut entry_length = nickname_length + ot_length + 9;
        if entry_length % 2 != 0 {
            entry_length += 1;
        }
        TradeLayout {
            table: self.value("TradeTableOffset"),
            size: self.value("TradeTableSize"),
            nickname_length,
            ot_length,
            entry_length,
            unused: self.array("TradesUnused"),
        }
    }

    fn used_trade_offsets(layout: &TradeLayout) -> Vec<i32> {
        let mut unused_index = 0;
        let mut offsets = Vec::new();
        for entry in 0..layout.size {
            if layout.unused.get(unused_index) == Some(&entry) {
                unused_index += 1;
                continue;
            }
            offsets.push(layout.table + entry * layout.entry_length);
        }
        offsets
    }

    pub(crate) fn read_ingame_trades(&self) -> Vec<IngameTrade> {
        let layout = self.trade_layout();
        let names = layout.nickname_length;
        Self::used_trade_offsets(&layout)
            .into_iter()
            .map(|offset| {
                let attack_defense = self.int(offset + 3 + names);
                let speed_special = self.int(offset + 4 + names);
                IngameTrade {
                    requested_pokemon: Some(self.pokemon_at(offset + 1)),
                    given_pokemon: self.pokemon_at(offset + 2),
                    nickname: self.read_string(offset + 3, names as usize),
                    ivs: vec![
                        (attack_defense >> 4) & 0xF,
                        attack_defense & 0xF,
                        (speed_special >> 4) & 0xF,
                        speed_special & 0xF,
                    ],
                    item: self.int(offset + 5 + names),
                    ot_id: self.word(offset + 6 + names),
                    ot_name: self.read_string(offset + 8 + names, layout.ot_length as usize),
                    ..IngameTrade::default()
                }
            })
            .collect()
    }

    pub(crate) fn write_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        let layout = self.trade_layout();
        let names = layout.nickname_length;
        let can_change_text = self.value("CanChangeTrainerText") > 0;
        for (index, offset) in Self::used_trade_offsets(&layout).into_iter().enumerate() {
            let trade = trades
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds("trades"))?;
            let requested = trade
                .requested_pokemon
                .ok_or(RandomizationError::NullPointer("requestedPokemon"))?;
            let requested_number = self.number_of(requested);
            let given_number = self.number_of(trade.given_pokemon);
            self.set_byte(offset + 1, requested_number);
            self.set_byte(offset + 2, given_number);
            if can_change_text {
                self.write_fixed_string(&trade.nickname, offset + 3, names);
            }
            let iv = |slot: usize| trade.ivs.get(slot).copied().unwrap_or(0);
            self.set_byte(offset + 3 + names, (iv(0) << 4 | iv(1)) as u8);
            self.set_byte(offset + 4 + names, (iv(2) << 4 | iv(3)) as u8);
            self.set_byte(offset + 5 + names, trade.item as u8);
            self.set_word(offset + 6 + names, trade.ot_id);
            if can_change_text {
                self.write_fixed_string(&trade.ot_name, offset + 8 + names, layout.ot_length);
            }
            self.set_byte(offset + 8 + names + layout.ot_length, 0);
        }
        Ok(())
    }
}
