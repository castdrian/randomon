use randomon_core::engine::pool::random_pokemon_incl_formes;
use randomon_core::{RandomizationError, Result};
use randomon_ctr::bytes::{
    find, find_after_prefix, read_long, write_long, write_long_big_endian, write_word,
};
use randomon_java::JavaRng;

use crate::constants;
use crate::handler::Gen6RomHandler;

impl Gen6RomHandler {
    pub(crate) fn apply_fastest_text(&mut self) {
        let patches: [(&str, [u8; 4]); 2] = [
            (
                constants::FASTEST_TEXT_PREFIXES[0],
                [0x03, 0x40, 0xA0, 0xE3],
            ),
            (
                constants::FASTEST_TEXT_PREFIXES[1],
                [0x03, 0x50, 0xA0, 0xE3],
            ),
        ];
        for (prefix, instruction) in patches {
            if let Some(offset) = find_after_prefix(&self.code, prefix) {
                self.code[offset..offset + 4].copy_from_slice(&instruction);
            }
        }
    }

    pub(crate) fn patch_for_national_dex(&mut self) {
        let offset = find(&self.code, constants::NATIONAL_DEX_FUNCTION_LOCATOR);
        if offset > 0 {
            let offset = offset as usize;
            self.code[offset..offset + 4].copy_from_slice(&[0x01, 0x00, 0xA0, 0xE3]);
        }
        if self.is_xy() {
            let offset = find(&self.code, constants::XY_GET_DEX_FLAG_FUNCTION_LOCATOR);
            if offset > 0 {
                let offset = offset as usize;
                write_long(&mut self.code, offset, 0);
                self.code[offset + 7] = 0xE3;
                self.code[offset + 11] = 0xEA;
            }
        } else if let Some(offset) = find_after_prefix(
            &self.code,
            constants::ORAS_GET_HOENN_DEX_CAUGHT_FUNCTION_PREFIX,
        ) {
            self.code[offset + 31] = 0xEA;
            write_long_big_endian(&mut self.code, offset + 32, 0x0700A0E1);
            write_long_big_endian(&mut self.code, offset + 36, 0x000050E3);
            write_long_big_endian(&mut self.code, offset + 40, 0x0100A003);
            write_long_big_endian(&mut self.code, offset + 44, 0xF09FBDE8);
            write_long_big_endian(&mut self.code, offset + 208, 0xD2FFFFEA);
        }
    }

    pub(crate) fn patch_forme_reversion(&mut self) -> Result<()> {
        let prefix = constants::save_load_forme_reversion_prefix(self.rom_type);
        if let Some(offset) = find_after_prefix(&self.code, prefix) {
            let offset = offset + 0x10;
            self.code[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]);
        }
        if self.is_oras() {
            let battle_path = self.file("Battle");
            let mut battle_cro = self.read_file(battle_path)?;
            if let Some(offset) =
                find_after_prefix(&battle_cro, constants::AFTER_BATTLE_FORME_REVERSION_PREFIX)
            {
                battle_cro[offset] = 0xFF;
                battle_cro[offset + 1] = 0xFF;
                self.write_file(battle_path, &battle_cro)?;
            }
        }
        Ok(())
    }

    pub(crate) fn enable_guaranteed_catching(&self) -> Result<()> {
        let battle_path = self.file("Battle");
        let mut battle_cro = self.read_file(battle_path)?;
        let offset = find(&battle_cro, constants::PERFECT_ODDS_BRANCH_LOCATOR);
        if offset > 0 {
            let offset = offset as usize;
            battle_cro[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]);
            self.write_file(battle_path, &battle_cro)?;
        }
        Ok(())
    }

    fn pick_intro_pokemon(&mut self) -> Result<(i32, i32)> {
        let mut intro = random_pokemon_incl_formes(self)?;
        while self.core.data.pk(intro).actually_cosmetic {
            intro = random_pokemon_incl_formes(self)?;
        }
        let pokemon = self.core.data.pk(intro).clone();
        let mut number = pokemon.number;
        let mut forme = 0;
        let mut check_cosmetics = true;
        if pokemon.forme_number > 0 {
            forme = pokemon.forme_number;
            number = self
                .core
                .data
                .pk(pokemon
                    .base_forme
                    .ok_or(RandomizationError::NullPointer("baseForme"))?)
                .number;
            check_cosmetics = false;
        }
        if pokemon.cosmetic_forms > 0 {
            let mut random = self.core.engine.random.clone();
            let cosmetic =
                pokemon.cosmetic_form_number(random.next_int_bounded(pokemon.cosmetic_forms));
            if check_cosmetics {
                forme = cosmetic;
            } else {
                forme += cosmetic;
            }
        }
        Ok((number, forme))
    }

    fn intro_cries(&self, number: i32, forme: i32) -> (i32, i32) {
        let base = (find(&self.code, constants::CRIES_TABLE_PREFIX_XY)
            + constants::CRIES_TABLE_PREFIX_XY.len() as i32 / 2) as usize;
        let mut key = number;
        if forme != 0 {
            let extra_offset = read_long(&self.code, base + key as usize * 0x14);
            key = extra_offset + (forme - 1);
        }
        let initial = read_long(&self.code, base + key as usize * 0x14 + 0x4);
        let repeated = read_long(&self.code, base + key as usize * 0x14 + 0x10);
        (initial, repeated)
    }

    fn offset_after(data: &[u8], prefix: &str) -> usize {
        (find(data, prefix) + prefix.len() as i32 / 2) as usize
    }

    pub(crate) fn randomize_intro(&mut self) -> Result<()> {
        if !self.is_xy() {
            return Ok(());
        }
        let (number, forme) = self.pick_intro_pokemon()?;
        let (initial_cry, repeated_cry) = self.intro_cries(number, forme);
        let intro_path = self.file("Intro");
        let mut intro = self.read_file(intro_path)?;
        let model = Self::offset_after(&intro, constants::INTRO_POKEMON_MODEL_OFFSET_XY);
        write_word(&mut intro, model, number);
        intro[model + 2] = forme as u8;
        let mut random = self.core.engine.random.clone();
        if random.next_int_bounded(256) == 0 {
            intro[model + 4] = 1;
        }
        let first = Self::offset_after(&intro, constants::INTRO_INITIAL_CRY_OFFSET1_XY);
        intro[first] = 0x5E;
        let second = Self::offset_after(&intro, constants::INTRO_INITIAL_CRY_OFFSET2_XY);
        intro[second] = 0x2F;
        let third = Self::offset_after(&intro, constants::INTRO_INITIAL_CRY_OFFSET3_XY);
        write_long(&mut intro, third, 0xE1A02000_u32 as i32);
        write_long(&mut intro, third + 0x4, 0xE59F100C_u32 as i32);
        write_long(&mut intro, third + 0x8, 0xE58D0000_u32 as i32);
        write_long(&mut intro, third + 0xC, 0xEBFFFDE9_u32 as i32);
        let moved_first = read_long(&intro, third + 0x14);
        write_long(&mut intro, third + 0x10, moved_first);
        let moved_second = read_long(&intro, third + 0x18);
        write_long(&mut intro, third + 0x14, moved_second);
        write_long(&mut intro, third + 0x18, initial_cry);
        let repeated = Self::offset_after(&intro, constants::INTRO_REPEATED_CRY_OFFSET_XY);
        write_long(&mut intro, repeated, repeated_cry);
        self.write_file(intro_path, &intro)
    }
}
