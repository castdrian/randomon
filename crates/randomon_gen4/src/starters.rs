use randomon_core::Result;
use randomon_data::constants::{gen4, species};
use randomon_data::{PokemonId, Type};

use crate::bytes::{
    after_prefix, file, file_mut, i32_at, search_all, set_i32, set_u8, set_u16, u8_at, u16_at,
};
use crate::handler::Gen4RomHandler;

const HGSS_STARTER_SUFFIX_DISTANCE: i32 = 13;
const RIVAL_JUMP_MARKER: i32 = 0x11;
const TAG_BATTLE_RETURN: i32 = 0x1B;
const HGSS_INTROS: [&str; 3] = ["So, you like", "You’ll take", "Do you want"];
const DP_RIVAL_LAKE_TEXT: &str = "\\v0103\\z0000: Fwaaah!\\nYour Pokémon totally rocked!\\pBut mine was way tougher\\nthan yours!\\p...They were other people’s\\nPokémon, though...\\pBut we had to use them...\\nThey won’t mind, will they?\\p";
const PT_RIVAL_ROUTE_201_TEXT: &str =
    "\\v0103\\z0000\\z0000: Then, I choose you!\\nI’m picking this one!\\p";

fn starter_color(index: usize) -> usize {
    if index == 0 { 3 } else { index }
}

fn type_name(kind: Option<Type>) -> String {
    kind.map(Type::camel_case).unwrap_or_default()
}

impl Gen4RomHandler {
    fn hgss_starter_offset(&self) -> Option<i32> {
        match search_all(&self.arm9, gen4::HGSS_STARTER_CODE_SUFFIX).as_slice() {
            [only] => Some(only - HGSS_STARTER_SUFFIX_DISTANCE),
            _ => None,
        }
    }

    fn starters_at(&self, data: &[u8], offset: i32) -> Result<Vec<PokemonId>> {
        (0..3)
            .map(|index| self.pokemon_by_number(u16_at(data, offset + index * 4)?))
            .collect()
    }

    pub(crate) fn starters(&self) -> Result<Vec<PokemonId>> {
        if self.entry.is_hgss() {
            return match self.hgss_starter_offset() {
                Some(offset) => self.starters_at(&self.arm9, offset),
                None => Ok(vec![
                    self.pk(species::CHIKORITA),
                    self.pk(species::CYNDAQUIL),
                    self.pk(species::TOTODILE),
                ]),
            };
        }
        let overlay = self.read_overlay(self.entry.int("StarterPokemonOvlNumber"))?;
        self.starters_at(&overlay, self.entry.int("StarterPokemonOffset"))
    }

    pub(crate) fn set_starters_impl(&mut self, starters: &[PokemonId]) -> Result<bool> {
        if starters.len() != 3 {
            return Ok(false);
        }
        let numbers: Vec<i32> = starters.iter().map(|&pk| self.number_of(pk)).collect();
        if self.entry.is_hgss() {
            self.set_hgss_starters(starters, &numbers)
        } else {
            self.set_dppt_starters(starters, &numbers)?;
            Ok(true)
        }
    }

    fn set_hgss_starters(&mut self, starters: &[PokemonId], numbers: &[i32]) -> Result<bool> {
        let Some(offset) = self.hgss_starter_offset() else {
            return Ok(false);
        };
        for (index, &number) in numbers.iter().enumerate() {
            set_u16(&mut self.arm9, offset + index as i32 * 4, number)?;
        }
        self.fix_hgss_rival_scripts(numbers)?;
        self.fix_hgss_starter_text(starters)?;
        self.fix_hgss_starter_cries(numbers)?;
        Ok(true)
    }

    fn fix_hgss_rival_scripts(&mut self, numbers: &[i32]) -> Result<()> {
        for &file_number in gen4::HGSS_FILES_WITH_RIVAL_SCRIPT {
            let Ok(script) = file_mut(&mut self.script_narc, file_number) else {
                continue;
            };
            let [base] = search_all(script, gen4::HGSS_RIVAL_SCRIPT_MAGIC)[..] else {
                continue;
            };
            set_u16(script, base + 8, numbers[0])?;
            let jump = i32_at(script, base + 13)?;
            let second = jump.wrapping_add(base).wrapping_add(17);
            let is_expected = u8_at(script, second).ok() == Some(RIVAL_JUMP_MARKER)
                && u8_at(script, second + 4).ok() == Some(species::CYNDAQUIL);
            if is_expected {
                set_u16(script, second + 4, numbers[1])?;
            }
        }
        Ok(())
    }

    fn fix_hgss_starter_text(&mut self, starters: &[PokemonId]) -> Result<()> {
        let index = self.entry.int("StarterScreenTextOffset");
        let mut strings = self.get_strings(index)?;
        for (slot, &starter) in starters.iter().enumerate() {
            let pk = self.core.data.pk(starter);
            let color = starter_color(slot);
            let kind = type_name(pk.primary_type);
            let description = format!(
                "Professor Elm: {} \\vFF00\\z000{color}{}\\vFF00\\z0000,\\nthe {kind}-type Pokémon?",
                HGSS_INTROS[slot], pk.name
            );
            let alternative = format!(
                "\\vFF00\\z000{color}{}\\vFF00\\z0000, the {kind}-type Pokémon, is\\nin this Poké Ball!",
                pk.name
            );
            if let Some(entry) = strings.get_mut(slot + 1) {
                *entry = description;
            }
            if let Some(entry) = strings.get_mut(slot + 4) {
                *entry = alternative;
            }
        }
        self.set_strings(index, &strings)
    }

    fn fix_hgss_starter_cries(&mut self, numbers: &[i32]) -> Result<()> {
        let overlay_number = self.entry.int("StarterPokemonOvlNumber");
        let mut overlay = self.read_overlay(overlay_number)?;
        if let Some(mut offset) = after_prefix(&overlay, gen4::STARTER_CRIES_PREFIX) {
            for &number in numbers {
                set_i32(&mut overlay, offset, number)?;
                offset += 4;
            }
        }
        self.write_overlay(overlay_number, &overlay)
    }

    fn set_dppt_starters(&mut self, starters: &[PokemonId], numbers: &[i32]) -> Result<()> {
        let overlay_number = self.entry.int("StarterPokemonOvlNumber");
        let mut overlay = self.read_overlay(overlay_number)?;
        let offset = self.entry.int("StarterPokemonOffset");
        for (index, &number) in numbers.iter().enumerate() {
            set_u16(&mut overlay, offset + index as i32 * 4, number)?;
        }
        self.patch_starter_graphics(&mut overlay, numbers)?;
        self.write_overlay(overlay_number, &overlay)?;
        self.fix_dppt_rival_scripts(numbers)?;
        self.fix_dppt_tag_battles(numbers)?;
        self.fix_dppt_starter_text(starters)
    }

    fn move_word(data: &mut [u8], to: i32, from: i32) -> Result<()> {
        let value = u16_at(data, from)?;
        set_u16(data, to, value)
    }

    fn patch_starter_graphics(&self, data: &mut [u8], numbers: &[i32]) -> Result<()> {
        let prefix = self.entry.string("StarterPokemonGraphicsPrefix");
        let Some(mut offset) = after_prefix(data, prefix) else {
            return Ok(());
        };
        let mut patched = data.to_vec();
        Self::move_word(&mut patched, offset + 0xC, offset + 0xA)?;
        if offset % 4 == 0 {
            let value = u8_at(&patched, offset + 0xC)?;
            set_u8(&mut patched, offset + 0xC, value - 1)?;
        }
        Self::move_word(&mut patched, offset + 0xA, offset + 0x8)?;
        let value = u8_at(&patched, offset + 0xA)?;
        set_u8(&mut patched, offset + 0xA, value - 1)?;
        Self::move_word(&mut patched, offset + 0x8, offset + 0x6)?;
        Self::move_word(&mut patched, offset + 0x6, offset + 0x4)?;
        Self::move_word(&mut patched, offset + 0x4, offset + 0x2)?;
        set_u16(&mut patched, offset + 0x2, 0x6828)?;
        set_u16(&mut patched, offset, 0x182D)?;
        offset += 0x16;
        set_u16(&mut patched, offset, 0x6828)?;
        offset += 0xA;
        for (index, &number) in numbers.iter().enumerate() {
            let base = 4 * (index as i32 + 1);
            let mut difference = number - base;
            let mut first = 0x3200;
            let mut second = 0x3200;
            if difference < 0 {
                first |= 0x800;
                difference = difference.abs();
            } else if difference > 255 {
                second |= 0xFF;
                difference -= 255;
            }
            first |= difference & 0xFF;
            set_u8(&mut patched, offset, base)?;
            Self::move_word(&mut patched, offset + 2, offset + 4)?;
            set_u16(&mut patched, offset + 4, first)?;
            set_u16(&mut patched, offset + 8, second)?;
            offset += 0xE;
        }
        set_u8(&mut patched, offset, 1)?;
        let inner_prefix = self.entry.string("StarterPokemonGraphicsPrefixInner");
        if let Some(inner) = after_prefix(&patched, inner_prefix) {
            set_u8(&mut patched, inner + 1, 0x68)?;
        }
        data.copy_from_slice(&patched);
        Ok(())
    }

    fn fix_dppt_rival_scripts(&mut self, numbers: &[i32]) -> Result<()> {
        let files = if self.entry.is_plat() {
            gen4::PT_FILES_WITH_RIVAL_SCRIPT
        } else {
            gen4::DP_FILES_WITH_RIVAL_SCRIPT
        };
        let is_plat = self.entry.is_plat();
        let magic = gen4::DPPT_RIVAL_SCRIPT_MAGIC;
        for &file_number in files {
            let Ok(script) = file_mut(&mut self.script_narc, file_number) else {
                continue;
            };
            for base in search_all(script, magic) {
                let jump_location = base + magic.len() as i32;
                let Ok(jump) = i32_at(script, jump_location) else {
                    continue;
                };
                let target = jump.wrapping_add(jump_location).wrapping_add(4);
                let command = u16_at(script, target).ok();
                let is_rival_script =
                    matches!(command, Some(0xE5 | 0x28F)) || (command == Some(0x125) && is_plat);
                if !is_rival_script {
                    continue;
                }
                set_u16(script, base + 0x8, numbers[0])?;
                set_u16(script, base + 0x15, numbers[1])?;
            }
        }
        Ok(())
    }

    fn fix_dppt_tag_battles(&mut self, numbers: &[i32]) -> Result<()> {
        let files = if self.entry.is_plat() {
            gen4::PT_FILES_WITH_TAG_SCRIPT
        } else {
            gen4::DP_FILES_WITH_TAG_SCRIPT
        };
        let first_magic = gen4::DPPT_TAG_BATTLE_SCRIPT_MAGIC1;
        let second_magic = gen4::DPPT_TAG_BATTLE_SCRIPT_MAGIC2;
        for &file_number in files {
            let Ok(script) = file_mut(&mut self.script_narc, file_number) else {
                continue;
            };
            for base in search_all(script, first_magic) {
                let second_start = base + first_magic.len() as i32 + 2;
                let second_end = second_start as usize + second_magic.len();
                if second_end > script.len()
                    || script[second_start as usize..second_end] != *second_magic
                {
                    continue;
                }
                let jump_location = second_end as i32;
                let Ok(jump) = i32_at(script, jump_location) else {
                    continue;
                };
                let target = jump.wrapping_add(jump_location).wrapping_add(4);
                if u16_at(script, target).ok() != Some(TAG_BATTLE_RETURN) {
                    continue;
                }
                let replaced = if u16_at(script, base + 0x21)? == species::TURTWIG {
                    numbers[0]
                } else {
                    numbers[2]
                };
                set_u16(script, base + 0x21, replaced)?;
                set_u16(script, base + 0xE, numbers[1])?;
            }
        }
        Ok(())
    }

    fn fix_dppt_starter_text(&mut self, starters: &[PokemonId]) -> Result<()> {
        let screen_index = self.entry.int("StarterScreenTextOffset");
        let mut strings = self.get_strings(screen_index)?;
        let species_strings = self.get_strings(self.entry.int("PokedexSpeciesTextOffset"))?;
        for (slot, &starter) in starters.iter().enumerate() {
            let pk = self.core.data.pk(starter);
            let category = species_strings
                .get(pk.number as usize)
                .cloned()
                .unwrap_or_default();
            let description = format!(
                "\\vFF00\\z000{}{category} {}\\vFF00\\z0000!\\nWill you take this Pokémon?",
                starter_color(slot),
                pk.name
            );
            if let Some(entry) = strings.get_mut(slot + 1) {
                *entry = description;
            }
        }
        self.set_strings(screen_index, &strings)?;
        let (string_index, text) = if self.entry.is_dp() {
            (gen4::DP_STARTER_STRING_INDEX, DP_RIVAL_LAKE_TEXT)
        } else {
            (gen4::PT_STARTER_STRING_INDEX, PT_RIVAL_ROUTE_201_TEXT)
        };
        let location_index = self.entry.int("StarterLocationTextOffset");
        let mut location_strings = self.get_strings(location_index)?;
        if let Some(entry) = location_strings.get_mut(string_index as usize) {
            *entry = text.to_owned();
        }
        self.set_strings(location_index, &location_strings)
    }

    pub(crate) fn starter_held_items(&self) -> Result<Vec<i32>> {
        let script = file(
            &self.script_narc,
            self.entry.int("StarterPokemonScriptOffset"),
        )?;
        Ok(vec![u16_at(
            script,
            self.entry.int("StarterPokemonHeldItemOffset"),
        )?])
    }

    pub(crate) fn set_starter_held_items_impl(&mut self, items: &[i32]) -> Result<()> {
        let Some(&item) = items.first() else {
            return Ok(());
        };
        let offset = self.entry.int("StarterPokemonHeldItemOffset");
        let script = file_mut(
            &mut self.script_narc,
            self.entry.int("StarterPokemonScriptOffset"),
        )?;
        set_u16(script, offset, item)
    }
}
