use randomon_core::{RandomizationError, Result};
use randomon_data::PokemonId;
use randomon_data::constants::gen5;
use randomon_formats::compression::ds;
use randomon_formats::nds::NarcArchive;
use randomon_nds::{find, format_error, read_word, write_relative_pointer, write_word};

use crate::handler::Gen5RomHandler;
use crate::rom_entry::OffsetWithinEntry;

const STARTER_COUNT: usize = 3;
const SPRITES_PER_POKEMON: usize = 20;
const PALETTE_SPRITE_INDEX: usize = 18;
const STARTER_PICTURE_BASE: usize = 12;

fn starter_offsets(
    handler: &Gen5RomHandler,
    starter: usize,
) -> Result<&'static [OffsetWithinEntry]> {
    handler
        .entry
        .offset_array_entries
        .get(&format!("StarterOffsets{}", starter + 1))
        .map(Vec::as_slice)
        .ok_or(RandomizationError::NullPointer("offsetArrayEntries.get"))
}

fn starter_text(type_name: &str, name: &str) -> String {
    format!(
        "\\xF000\\xBD02\\x0000The {type_name}-type Pok\\x00E9mon\\xFFFE\\xF000\\xBD02\\x0000{name}"
    )
}

const BW1_CHEREN_TEXT1: &str = "Cheren: Hey, how come you get to pick\\xFFFEout my Pok\\x00E9mon?\\xF000\\xBE01\\x0000\\xFFFEOh, never mind. I wanted this one\\xFFFEfrom the start, anyway.\\xF000\\xBE01\\x0000";
const BW1_CHEREN_TEXT2: &str = "It's decided. You'll be my opponent...\\xFFFEin our first Pok\\x00E9mon battle!\\xF000\\xBE01\\x0000\\xFFFELet's see what you can do, \\xFFFEmy Pok\\x00E9mon!\\xF000\\xBE01\\x0000";
const BW2_RIVAL_TEXT: &str = "\\xF000\\x0100\\x0001\\x0001: Let's see how good\\xFFFEa Trainer you are!\\xF000\\xBE01\\x0000\\xFFFEI'll use my Pok\\x00E9mon\\xFFFEthat I raised from an Egg!\\xF000\\xBE01\\x0000";

impl Gen5RomHandler {
    pub(crate) fn starters(&self) -> Vec<PokemonId> {
        (0..STARTER_COUNT)
            .map(|starter| {
                let offsets = starter_offsets(self, starter).expect("starter offsets exist");
                let first = offsets[0];
                let script = &self.script_narc.files[first.entry as usize];
                self.pk(read_word(script, first.offset))
            })
            .collect()
    }

    fn patch_starter_script(&mut self) -> Result<()> {
        let (new_script, magic, jump_opcode, japanese_adjustments): (&[u8], &str, u8, &[usize]) =
            if self.is_bw2() {
                (
                    gen5::BW2_NEW_STARTER_SCRIPT,
                    gen5::BW2_STARTER_SCRIPT_MAGIC,
                    0x1E,
                    &[0x6],
                )
            } else {
                (
                    gen5::BW1_NEW_STARTER_SCRIPT,
                    gen5::BW1_STARTER_SCRIPT_MAGIC,
                    0x04,
                    &[0x4, 0x8],
                )
            };
        let file_index = self.entry.int("PokedexGivenFileOffset") as usize;
        let old_file = self
            .script_narc
            .files
            .get(file_index)
            .ok_or(RandomizationError::IndexOutOfBounds("scriptNARC.files"))?;
        let offset = find(old_file, magic);
        if offset <= 0 {
            return Ok(());
        }
        let old_length = old_file.len();
        let mut new_file = old_file.clone();
        new_file.extend_from_slice(new_script);
        if self.entry.rom_code.as_bytes().get(3) == Some(&b'J') {
            for &adjustment in japanese_adjustments {
                new_file[old_length + adjustment] =
                    new_file[old_length + adjustment].wrapping_sub(4);
            }
        }
        new_file[offset as usize] = jump_opcode;
        new_file[offset as usize + 1] = 0x0;
        write_relative_pointer(&mut new_file, offset + 2, old_length as i32);
        self.script_narc.files[file_index] = new_file;
        Ok(())
    }

    fn replace_starter_files(
        starters: &mut NarcArchive,
        sprites: &NarcArchive,
        starter: usize,
        number: i32,
    ) -> Result<()> {
        let base = number as usize * SPRITES_PER_POKEMON;
        let sprite = |index: usize| {
            sprites
                .files
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds(
                    "pokespritesNARC.files",
                ))
        };
        let palette = sprite(base + PALETTE_SPRITE_INDEX)?.clone();
        let picture = ds::decompress(sprite(base)?)
            .map_err(format_error)?
            .ok_or(RandomizationError::NullPointer("DSDecmp.Decompress"))?;
        let slot = |index: usize, files: &mut Vec<Vec<u8>>| -> Result<()> {
            if index >= files.len() {
                return Err(RandomizationError::IndexOutOfBounds("starterNARC.files"));
            }
            Ok(())
        };
        slot(starter * 2, &mut starters.files)?;
        starters.files[starter * 2] = palette;
        slot(STARTER_PICTURE_BASE + starter, &mut starters.files)?;
        starters.files[STARTER_PICTURE_BASE + starter] = picture;
        Ok(())
    }

    fn update_starter_graphics(&self, numbers: &[i32]) -> Result<()> {
        let mut starter_graphics = self.read_narc("StarterGraphics")?;
        let sprites = self.read_narc("PokemonGraphics")?;
        for (starter, &number) in numbers.iter().enumerate() {
            Self::replace_starter_files(&mut starter_graphics, &sprites, starter, number)?;
        }
        self.write_narc("StarterGraphics", &starter_graphics)
    }

    fn update_starter_cries(&self, numbers: &[i32]) -> Result<()> {
        let overlay_number = self.entry.int("StarterCryOvlNumber");
        let mut overlay = self.read_overlay(overlay_number)?;
        let prefix = self.entry.string("StarterCryTablePrefix");
        let offset = find(&overlay, prefix);
        if offset > 0 {
            let mut position = offset + prefix.len() as i32 / 2;
            for &number in numbers {
                write_word(&mut overlay, position, number);
                position += 2;
            }
            self.write_overlay(overlay_number, &overlay)?;
        }
        Ok(())
    }

    fn update_starter_text(&mut self, starters: &[PokemonId]) -> Result<()> {
        let text_index = self.entry.int("StarterLocationTextOffset");
        let mut strings = self.get_strings(true, text_index)?;
        let first_text = if self.is_bw2() {
            gen5::BW2_STARTER_TEXT_OFFSET
        } else {
            gen5::BW1_STARTER_TEXT_OFFSET
        };
        for (index, &starter) in starters.iter().enumerate() {
            let pk = self.core.data.pk(starter);
            let type_name = pk
                .primary_type
                .ok_or(RandomizationError::NullPointer("primaryType"))?
                .camel_case();
            let text = starter_text(&type_name, &pk.name);
            *strings
                .get_mut((first_text - index as i32) as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("starter strings"))? = text;
        }
        let mut set = |index: i32, text: &str| -> Result<()> {
            *strings
                .get_mut(index as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("starter strings"))? = text.to_owned();
            Ok(())
        };
        if self.is_bw2() {
            set(gen5::BW2_RIVAL_TEXT_OFFSET, BW2_RIVAL_TEXT)?;
        } else {
            set(gen5::BW1_CHEREN_TEXT1_OFFSET, BW1_CHEREN_TEXT1)?;
            set(gen5::BW1_CHEREN_TEXT2_OFFSET, BW1_CHEREN_TEXT2)?;
        }
        self.set_strings(true, text_index, &strings)
    }

    pub(crate) fn set_starters_impl(&mut self, starters: &[PokemonId]) -> Result<bool> {
        if starters.len() != STARTER_COUNT {
            return Ok(false);
        }
        let numbers: Vec<i32> = starters
            .iter()
            .map(|&starter| self.core.data.pk(starter).number)
            .collect();
        for (starter, &number) in numbers.iter().enumerate() {
            for entry in starter_offsets(self, starter)? {
                let script = self
                    .script_narc
                    .files
                    .get_mut(entry.entry as usize)
                    .ok_or(RandomizationError::IndexOutOfBounds("scriptNARC.files"))?;
                write_word(script, entry.offset, number);
            }
        }
        self.patch_starter_script()?;
        self.update_starter_graphics(&numbers)?;
        self.update_starter_cries(&numbers)?;
        self.update_starter_text(starters)?;
        Ok(true)
    }
}
