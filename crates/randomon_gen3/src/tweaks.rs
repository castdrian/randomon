use randomon_core::Result;
use randomon_core::engine::pool::{check_pokemon_restrictions, random_pokemon};
use randomon_data::constants::species;
use randomon_data::{PokemonId, Type};
use randomon_java::JavaRng;
use randomon_settings::MiscTweak;

use crate::constants as c;
use crate::handler::Gen3RomHandler;
use crate::items;
use crate::rom::pointer_to_hex;
use crate::rom_entry::RomType;

const LARGEST_BYTE_SPECIES: i32 = 255;
const LARGEST_TWO_INSTRUCTION_SPECIES: i32 = 510;
const LARGEST_GENDERED_RATIO: i32 = 0xFD;
const INTRO_RETRY_LIMIT: usize = 10_000;
const TYPE_TABLE_FORESIGHT_MARKER: u8 = 0xFE;
const FOSSIL_LEVEL: i32 = 30;
const NEUTRAL: u8 = 10;
const RS_NAT_DEX_ROUTINE_SIZE: i32 = 44;
const FRLG_NAT_DEX_ROUTINE_SIZE: i32 = 10;
const E_NAT_DEX_ROUTINE_SIZE: i32 = 27;

fn set_register(register: u8) -> u8 {
    c::GBA_SET_RX_OPCODE | register
}

fn add_register(register: u8) -> u8 {
    c::GBA_ADD_RX_OPCODE | register
}

impl Gen3RomHandler {
    pub(crate) fn attempt_obedience_evolution_patches(&mut self) {
        if self.have_patched_obedience {
            return;
        }
        self.have_patched_obedience = true;
        let deoxys = self.rom.find(c::DEOXYS_OBEY_CODE);
        if deoxys > 0 {
            let movs_r1_zero = [0x00, set_register(c::GBA_R1)];
            self.rom.write_bytes(deoxys, &movs_r1_zero);
            self.rom.write_bytes(deoxys + 2, &movs_r1_zero);
            let mew_check = deoxys + c::MEW_OBEY_OFFSET_FROM_DEOXYS_OBEY;
            let cmp_r0 = (c::GBA_CMP_RX_OPCODE | i32::from(c::GBA_R0)) << 8;
            if self.rom.word(mew_check) == cmp_r0 | species::MEW {
                self.rom.set_word(mew_check, cmp_r0);
            }
        }
        if self.rom_type().is_frlg() {
            self.remove_kanto_dex_evolution_check(
                c::LEVEL_EVO_KANTO_DEX_CHECK_CODE,
                c::LEVEL_EVO_KANTO_DEX_JUMP_AMOUNT,
            );
            self.remove_kanto_dex_evolution_check(
                c::STONE_EVO_KANTO_DEX_CHECK_CODE,
                c::STONE_EVO_KANTO_DEX_JUMP_AMOUNT,
            );
        }
    }

    fn remove_kanto_dex_evolution_check(&mut self, code: &str, jump: i32) {
        let offset = self.rom.find(code);
        if offset > 0 {
            self.rom.set_word(offset, c::GBA_NOP_OPCODE);
            self.rom
                .set_word(offset + 2, (c::GBA_UNCONDITIONAL_JUMP_OPCODE << 8) | jump);
        }
    }

    pub(crate) fn random_pokemon_limited(
        &mut self,
        max_internal: i32,
        block_non_males: bool,
    ) -> Option<PokemonId> {
        check_pokemon_restrictions(self);
        let valid: Vec<PokemonId> = self
            .core
            .engine
            .pool
            .main
            .iter()
            .copied()
            .filter(|&id| {
                self.internal_of(id) <= max_internal
                    && (!block_non_males
                        || self.core.data.pk(id).gender_ratio <= LARGEST_GENDERED_RATIO)
            })
            .collect();
        if valid.is_empty() {
            return None;
        }
        let index = self.core.engine.random.next_int_bounded(valid.len() as i32);
        Some(valid[index as usize])
    }

    fn random_intro_internal(&mut self) -> Option<i32> {
        let pokemon = random_pokemon(self).ok()?;
        Some(self.internal_of(pokemon))
    }

    fn random_ruby_sapphire_intro(&mut self) -> Option<i32> {
        let usable = |internal: i32| internal != 1 && internal <= LARGEST_TWO_INSTRUCTION_SPECIES;
        let mut internal = self.random_intro_internal()?;
        for _ in 0..INTRO_RETRY_LIMIT {
            if usable(internal) {
                return Some(internal);
            }
            internal = self.random_intro_internal()?;
        }
        usable(internal).then_some(internal)
    }

    pub(crate) fn randomize_intro(&mut self) {
        match self.rom_type() {
            RomType::FireRedLeafGreen => self.randomize_frlg_intro(),
            RomType::Ruby | RomType::Sapphire => self.randomize_ruby_sapphire_intro(),
            RomType::Emerald => {
                if let Some(internal) = self.random_intro_internal() {
                    self.rom.set_word(self.value("IntroSpriteOffset"), internal);
                    self.rom.set_word(self.value("IntroCryOffset"), internal);
                }
            }
        }
    }

    fn randomize_frlg_intro(&mut self) {
        let Some(pokemon) = self.random_pokemon_limited(LARGEST_BYTE_SPECIES, false) else {
            return;
        };
        let internal = self.internal_of(pokemon);
        let sprites = self.value("FrontSprites");
        let palettes = self.value("PokemonPalettes");
        self.rom
            .set_byte(self.value("IntroCryOffset"), internal as u8);
        self.rom
            .set_byte(self.value("IntroOtherOffset"), internal as u8);
        let sprite_base = self.value("IntroSpriteOffset");
        self.rom.set_pointer(sprite_base, sprites + internal * 8);
        self.rom
            .set_pointer(sprite_base + 4, palettes + internal * 8);
    }

    fn write_species_load(&mut self, offset: i32, register: u8, internal: i32) {
        if internal > LARGEST_BYTE_SPECIES {
            self.rom
                .write_bytes(offset, &[0xFF, set_register(register)]);
            self.rom.write_bytes(
                offset + 2,
                &[(internal - 0xFF) as u8, add_register(register)],
            );
        } else {
            self.rom
                .write_bytes(offset, &[internal as u8, set_register(register)]);
            self.rom.set_word(offset + 2, c::GBA_NOP_OPCODE);
        }
    }

    fn randomize_ruby_sapphire_intro(&mut self) {
        let Some(internal) = self.random_ruby_sapphire_intro() else {
            return;
        };
        let sprites = self.value("PokemonFrontSprites");
        let palettes = self.value("PokemonNormalPalettes");
        self.write_species_load(self.value("IntroCryOffset"), c::GBA_R0, internal);
        self.write_species_load(self.value("IntroOtherOffset"), c::GBA_R4, internal);
        self.rom
            .set_pointer(self.value("IntroSpriteOffset"), sprites + internal * 8);
        self.rom
            .set_pointer(self.value("IntroPaletteOffset"), palettes + internal * 8);
    }

    fn randomize_catching_tutorial(&mut self) {
        let opponent_offset = self.value("CatchingTutorialOpponentMonOffset");
        if opponent_offset > 0 {
            if self.rom_type().is_frlg() {
                if let Some(opponent) = self.random_pokemon_limited(LARGEST_BYTE_SPECIES, true) {
                    let internal = self.internal_of(opponent);
                    self.rom
                        .write_bytes(opponent_offset, &[internal as u8, set_register(c::GBA_R1)]);
                }
            } else if let Some(opponent) =
                self.random_pokemon_limited(LARGEST_TWO_INSTRUCTION_SPECIES, true)
            {
                let internal = self.internal_of(opponent);
                self.write_species_load(opponent_offset, c::GBA_R1, internal);
            }
        }
        let player_offset = self.value("CatchingTutorialPlayerMonOffset");
        if player_offset > 0
            && let Some(player) =
                self.random_pokemon_limited(LARGEST_TWO_INSTRUCTION_SPECIES, false)
        {
            let internal = self.internal_of(player);
            self.write_species_load(player_offset, c::GBA_R1, internal);
        }
    }

    fn apply_running_shoes_indoors(&mut self) {
        let offset = self.value("RunIndoorsTweakOffset");
        if offset != 0 {
            self.rom.set_byte(offset, 0x00);
        }
    }

    fn apply_fastest_text(&mut self) -> Result<()> {
        if self.apply_tweak_patch("InstantTextTweak")? {
            return Ok(());
        }
        let offset = self.value("TextSpeedValuesOffset");
        if offset > 0 {
            self.rom.write_bytes(offset, &[4, 1, 0]);
        }
        Ok(())
    }

    fn randomize_pc_potion(&mut self) {
        let offset = self.value("PCPotionOffset");
        if offset != 0 {
            let item = self
                .non_bad_items
                .random_non_tm(&mut self.core.engine.random);
            self.rom.set_word(offset, item);
        }
    }

    fn apply_run_without_running_shoes(&mut self) {
        let prefix = c::running_shoes_check_prefix(self.rom_type());
        let offset = self.rom.find(prefix);
        if offset > 0 {
            self.rom
                .set_word(offset + c::RUNNING_SHOES_BRANCH_DISTANCE, 0);
        }
    }

    fn balance_static_levels(&mut self) {
        for offset in self.array("FossilLevelOffsets") {
            self.rom.set_word(offset, FOSSIL_LEVEL);
        }
    }

    fn apply_camel_case_names(&mut self) {
        for id in self.pokemon_list.clone().into_iter().flatten() {
            let name = randomon_data::text::camel_case(&self.core.data.pk(id).name);
            self.core.data.pk_mut(id).name = name;
        }
    }

    fn read_type_effectiveness(&self) -> Vec<(Option<Type>, Option<Type>, u8)> {
        let mut table = Vec::new();
        let mut offset = self.value("TypeEffectivenessOffset");
        while self.rom.contains_range(offset, 3)
            && self.rom.byte(offset) != TYPE_TABLE_FORESIGHT_MARKER
        {
            let attacking = c::type_table(self.rom.int(offset));
            let defending = c::type_table(self.rom.int(offset + 1));
            let effectiveness = self.rom.byte(offset + 2);
            if matches!(effectiveness, 20 | 10 | 5 | 0) {
                table.push((attacking, defending, effectiveness));
            }
            offset += 3;
        }
        table
    }

    fn update_type_effectiveness(&mut self) {
        let mut table = self.read_type_effectiveness();
        self.log("--Updating Type Effectiveness--");
        for (attacker, defender, effectiveness) in &mut table {
            if *attacker == Some(Type::Ghost) && *defender == Some(Type::Steel) {
                *effectiveness = NEUTRAL;
                self.core.engine.handler_log(
                    "Replaced: Ghost not very effective vs Steel => Ghost neutral vs Steel",
                );
            } else if *attacker == Some(Type::Dark) && *defender == Some(Type::Steel) {
                *effectiveness = NEUTRAL;
                self.core.engine.handler_log(
                    "Replaced: Dark not very effective vs Steel => Dark neutral vs Steel",
                );
            }
        }
        self.core.engine.handler_log_blank_line();
        let mut offset = self.value("TypeEffectivenessOffset");
        for (attacker, defender, effectiveness) in table {
            self.rom.write_bytes(
                offset,
                &[
                    c::type_to_byte(attacker),
                    c::type_to_byte(defender),
                    effectiveness,
                ],
            );
            offset += 3;
        }
        self.effectiveness_updated = true;
    }

    pub(crate) fn guarantee_catching(&mut self) {
        let offset = self.rom.find(c::PERFECT_ODDS_BRANCH_LOCATOR);
        if offset > 0 {
            self.rom.write_bytes(offset, &[0; 4]);
        }
    }

    fn log_patch_result(&mut self, successful: bool) {
        let separator = self.core.engine.log.line_separator();
        let message = if successful {
            "Patch successful!"
        } else {
            "Patch unsuccessful."
        };
        self.log(&format!("{message}{separator}"));
    }

    fn patch_script_pointer_dex(
        &mut self,
        identifier: &str,
        size: i32,
        part1: &str,
        part2: &str,
    ) -> bool {
        let dex_script = self.rom.find(identifier);
        if dex_script < 0 {
            return false;
        }
        let text_pointer = self.rom.pointer(dex_script - 4);
        let script = dex_script - 8;
        let pointer_location = self.rom.find(&pointer_to_hex(script));
        if pointer_location < 0 {
            return false;
        }
        let free_space = self.value("FreeSpace");
        let Some(space) = self.rom.allocate(size, free_space, true) else {
            return false;
        };
        self.rom.set_pointer(pointer_location, space);
        self.rom.write_hex(space, part1);
        self.rom.set_pointer(space + 4, text_pointer);
        self.rom.write_hex(space + 8, part2);
        true
    }

    fn patch_frlg_national_dex(&mut self) -> bool {
        let dex_script = self.rom.find(c::FRLG_POKEDEX_SCRIPT_IDENTIFIER);
        if dex_script < 0 {
            return false;
        }
        let free_space = self.value("FreeSpace");
        let Some(space) = self
            .rom
            .allocate(FRLG_NAT_DEX_ROUTINE_SIZE, free_space, true)
        else {
            return false;
        };
        self.rom.set_byte(dex_script, 4);
        self.rom.set_pointer(dex_script + 1, space);
        self.rom.set_byte(dex_script + 5, 0);
        self.rom.write_hex(space, c::FRLG_NAT_DEX_SCRIPT);
        for check in self.rom.find_multiple(c::FRLG_NAT_DEX_FLAG_CHECKER) {
            self.rom.write_hex(check, c::FRLG_E4_FLAG_CHECKER);
        }
        let oak_lab = self.rom.find(c::FRLG_OAKS_LAB_KANTO_DEX_CHECKER);
        if oak_lab > 0 {
            self.rom.write_hex(oak_lab, c::FRLG_OAKS_LAB_FIX);
        }
        let oak_house = self.rom.find(c::FRLG_OAK_OUTSIDE_HOUSE_CHECK);
        if oak_house > 0 {
            self.rom.write_hex(oak_house, c::FRLG_OAK_OUTSIDE_HOUSE_FIX);
        }
        let aide = self.rom.find(c::FRLG_OAK_AIDE_CHECK_PREFIX);
        if aide > 0 {
            let branch = aide + c::FRLG_OAK_AIDE_CHECK_PREFIX.len() as i32 / 2;
            self.rom.set_byte(branch + 1, 0xE0);
        }
        true
    }

    fn patch_for_national_dex(&mut self) {
        self.log("--Patching for National Dex at Start of Game--");
        let successful = match self.rom_type() {
            RomType::Ruby | RomType::Sapphire => self.patch_script_pointer_dex(
                c::RS_POKEDEX_SCRIPT_IDENTIFIER,
                RS_NAT_DEX_ROUTINE_SIZE,
                c::RS_NAT_DEX_SCRIPT_PART1,
                c::RS_NAT_DEX_SCRIPT_PART2,
            ),
            RomType::FireRedLeafGreen => self.patch_frlg_national_dex(),
            RomType::Emerald => self.patch_script_pointer_dex(
                c::E_POKEDEX_SCRIPT_IDENTIFIER,
                E_NAT_DEX_ROUTINE_SIZE,
                c::E_NAT_DEX_SCRIPT_PART1,
                c::E_NAT_DEX_SCRIPT_PART2,
            ),
        };
        self.log_patch_result(successful);
    }

    pub(crate) fn available_misc_tweaks(&self) -> i32 {
        let mut available = MiscTweak::LowerCasePokemonNames.value()
            | MiscTweak::NationalDexAtStart.value()
            | MiscTweak::UpdateTypeEffectiveness.value();
        if self.value("RunIndoorsTweakOffset") > 0 {
            available |= MiscTweak::RunningShoesIndoors.value();
        }
        if self.value("TextSpeedValuesOffset") > 0 || self.entry.tweak("InstantTextTweak").is_some()
        {
            available |= MiscTweak::FastestText.value();
        }
        if self.value("CatchingTutorialOpponentMonOffset") > 0
            || self.value("CatchingTutorialPlayerMonOffset") > 0
        {
            available |= MiscTweak::RandomizeCatchingTutorial.value();
        }
        if self.value("PCPotionOffset") != 0 {
            available |= MiscTweak::RandomizePcPotion.value();
        }
        available |= MiscTweak::BanLuckyEgg.value() | MiscTweak::RunWithoutRunningShoes.value();
        if self.rom_type().is_frlg() {
            available |= MiscTweak::BalanceStaticLevels.value();
        }
        available
    }

    pub(crate) fn apply_tweak(&mut self, tweak: MiscTweak) -> Result<()> {
        match tweak {
            MiscTweak::RunningShoesIndoors => self.apply_running_shoes_indoors(),
            MiscTweak::FastestText => self.apply_fastest_text()?,
            MiscTweak::LowerCasePokemonNames => self.apply_camel_case_names(),
            MiscTweak::NationalDexAtStart => self.patch_for_national_dex(),
            MiscTweak::RandomizeCatchingTutorial => self.randomize_catching_tutorial(),
            MiscTweak::BanLuckyEgg => {
                self.allowed_items.ban_singles(&[items::LUCKY_EGG]);
                self.non_bad_items.ban_singles(&[items::LUCKY_EGG]);
            }
            MiscTweak::RandomizePcPotion => self.randomize_pc_potion(),
            MiscTweak::RunWithoutRunningShoes => self.apply_run_without_running_shoes(),
            MiscTweak::BalanceStaticLevels => self.balance_static_levels(),
            MiscTweak::UpdateTypeEffectiveness => self.update_type_effectiveness(),
            _ => {}
        }
        Ok(())
    }
}
