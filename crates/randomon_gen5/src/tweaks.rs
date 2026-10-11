use randomon_core::{RandomizationError, Result};
use randomon_data::Type;
use randomon_data::constants::{gen5, items};
use randomon_java::JavaRng;
use randomon_nds::{find, read_byte, read_relative_pointer, read_word, write_full_int, write_word};
use randomon_settings::MiscTweak;

use crate::handler::Gen5RomHandler;

const TYPE_TABLE_SIZE: i32 = Type::Dark as i32 + 1;
const MOV_R0_2_BX_LR: [u8; 4] = [0x02, 0x20, 0x70, 0x47];
const SCRIPT_BLOCK_START: i32 = 0x2E;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TableEffectiveness {
    Double,
    Neutral,
    Half,
    Zero,
}

impl TableEffectiveness {
    fn from_internal(value: i32) -> Option<Self> {
        match value {
            8 => Some(Self::Double),
            4 => Some(Self::Neutral),
            2 => Some(Self::Half),
            0 => Some(Self::Zero),
            _ => None,
        }
    }

    fn internal(self) -> u8 {
        match self {
            Self::Double => 8,
            Self::Neutral => 4,
            Self::Half => 2,
            Self::Zero => 0,
        }
    }
}

impl Gen5RomHandler {
    pub(crate) fn tweaks_available(&self) -> i32 {
        let mut available = 0;
        if self.entry.tweak("FastestTextTweak").is_some() {
            available |= MiscTweak::FastestText.value();
        }
        available |= MiscTweak::BanLuckyEgg.value();
        available |= MiscTweak::NoFreeLuckyEgg.value();
        available |= MiscTweak::BanBigManiacItems.value();
        available |= MiscTweak::UpdateTypeEffectiveness.value();
        if !self.is_bw2() {
            available |= MiscTweak::BalanceStaticLevels.value();
        }
        if self.entry.tweak("NationalDexAtStartTweak").is_some() {
            available |= MiscTweak::NationalDexAtStart.value();
        }
        available |= MiscTweak::RunWithoutRunningShoes.value();
        if self.is_bw2() {
            available |= MiscTweak::ForceChallengeMode.value();
        }
        available |= MiscTweak::DisableLowHpMusic.value();
        available
    }

    pub(crate) fn apply_tweak(&mut self, tweak: MiscTweak) -> Result<()> {
        match tweak {
            MiscTweak::FastestText => {
                let mut arm9 = std::mem::take(&mut self.arm9);
                let patched = self.generic_ips_patch(&mut arm9, "FastestTextTweak");
                self.arm9 = arm9;
                patched?;
            }
            MiscTweak::BanLuckyEgg => {
                self.allowed_items.ban_singles(&[items::LUCKY_EGG]);
                self.non_bad_items.ban_singles(&[items::LUCKY_EGG]);
            }
            MiscTweak::NoFreeLuckyEgg => self.remove_free_lucky_egg()?,
            MiscTweak::BanBigManiacItems => {
                for (start, length) in [
                    (items::BALM_MUSHROOM, 4),
                    (items::RELIC_VASE, 4),
                    (items::LANSAT_BERRY, 7),
                ] {
                    self.allowed_items.ban_range(start, length);
                    self.non_bad_items.ban_range(start, length);
                }
            }
            MiscTweak::BalanceStaticLevels => {
                let fossil = self
                    .script_narc
                    .files
                    .get_mut(gen5::FOSSIL_POKEMON_FILE as usize)
                    .ok_or(RandomizationError::IndexOutOfBounds("scriptNarc.files"))?;
                write_word(fossil, gen5::FOSSIL_POKEMON_LEVEL_OFFSET, 20);
            }
            MiscTweak::NationalDexAtStart => self.patch_for_national_dex()?,
            MiscTweak::RunWithoutRunningShoes => self.apply_run_without_running_shoes_patch()?,
            MiscTweak::UpdateTypeEffectiveness => self.update_type_effectiveness()?,
            MiscTweak::ForceChallengeMode => self.force_challenge_mode(),
            MiscTweak::DisableLowHpMusic => self.disable_low_hp_music()?,
            _ => {}
        }
        Ok(())
    }

    fn remove_free_lucky_egg(&mut self) -> Result<()> {
        let file = self.entry.int("LuckyEggScriptOffset");
        let mulch_index = self.core.engine.random.clone().next_int_bounded(4);
        let mut looking_for_eggs = if self.is_bw2() { 2 } else { 1 };
        let scripts = self
            .script_narc
            .files
            .get_mut(file as usize)
            .ok_or(RandomizationError::IndexOutOfBounds("scriptNarc.files"))?;
        let mut offset = 0;
        while looking_for_eggs > 0 {
            if read_word(scripts, offset) == gen5::SCRIPT_LIST_TERMINATOR {
                break;
            }
            let mut offset_in_file = read_relative_pointer(scripts, offset);
            offset += 4;
            if offset_in_file > scripts.len() as i32 {
                break;
            }
            loop {
                offset_in_file += 1;
                let byte = read_byte(scripts, offset_in_file);
                if byte == gen5::HIDDEN_ITEM_SET_VAR_COMMAND {
                    let command = read_word(scripts, offset_in_file);
                    let variable = read_word(scripts, offset_in_file + 2);
                    let item = read_word(scripts, offset_in_file + 4);
                    if command == gen5::HIDDEN_ITEM_SET_VAR_COMMAND
                        && variable == gen5::HIDDEN_ITEM_VAR_SET
                        && item == items::LUCKY_EGG
                    {
                        write_word(
                            scripts,
                            offset_in_file + 4,
                            gen5::MULCH_INDICES[mulch_index as usize],
                        );
                        looking_for_eggs -= 1;
                    }
                }
                if byte == SCRIPT_BLOCK_START {
                    break;
                }
            }
        }
        Ok(())
    }

    fn patch_for_national_dex(&mut self) -> Result<()> {
        let index = self.entry.int("NationalDexScriptOffset") as usize;
        let script = self
            .script_narc
            .files
            .get(index)
            .ok_or(RandomizationError::IndexOutOfBounds("scriptNarc.files"))?;
        let mut expanded = script.clone();
        expanded.extend_from_slice(&[0; 4]);
        self.generic_ips_patch(&mut expanded, "NationalDexAtStartTweak")?;
        self.script_narc.files[index] = expanded;
        Ok(())
    }

    fn apply_run_without_running_shoes_patch(&mut self) -> Result<()> {
        let number = self.entry.int("FieldOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        let offset = find(&overlay, gen5::RUNNING_SHOES_PREFIX);
        if offset != 0 {
            write_word(&mut overlay, offset, 0);
            self.write_overlay(number, &overlay)?;
        }
        Ok(())
    }

    fn update_type_effectiveness(&mut self) -> Result<()> {
        let number = self.entry.int("BattleOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        let table_offset = find(&overlay, gen5::TYPE_EFFECTIVENESS_TABLE_LOCATOR);
        if table_offset <= 0 {
            return Ok(());
        }
        let position = |attacker: i32, defender: i32| {
            (table_offset + attacker * TYPE_TABLE_SIZE + defender) as usize
        };
        let mut table: Vec<Option<TableEffectiveness>> =
            Vec::with_capacity((TYPE_TABLE_SIZE * TYPE_TABLE_SIZE) as usize);
        for attacker in 0..TYPE_TABLE_SIZE {
            for defender in 0..TYPE_TABLE_SIZE {
                table.push(TableEffectiveness::from_internal(i32::from(
                    overlay[position(attacker, defender)] as i8,
                )));
            }
        }
        self.core
            .engine
            .handler_log("--Updating Type Effectiveness--");
        let steel = i32::from(gen5::type_to_byte(Some(Type::Steel)));
        let dark = i32::from(gen5::type_to_byte(Some(Type::Dark)));
        let ghost = i32::from(gen5::type_to_byte(Some(Type::Ghost)));
        table[(ghost * TYPE_TABLE_SIZE + steel) as usize] = Some(TableEffectiveness::Neutral);
        self.core
            .engine
            .handler_log("Replaced: Ghost not very effective vs Steel => Ghost neutral vs Steel");
        table[(dark * TYPE_TABLE_SIZE + steel) as usize] = Some(TableEffectiveness::Neutral);
        self.core
            .engine
            .handler_log("Replaced: Dark not very effective vs Steel => Dark neutral vs Steel");
        self.core.engine.handler_log_blank_line();
        for attacker in 0..TYPE_TABLE_SIZE {
            for defender in 0..TYPE_TABLE_SIZE {
                let effectiveness = table[(attacker * TYPE_TABLE_SIZE + defender) as usize]
                    .ok_or(RandomizationError::NullPointer("typeEffectivenessTable"))?;
                overlay[position(attacker, defender)] = effectiveness.internal();
            }
        }
        self.write_overlay(number, &overlay)?;
        self.effectiveness_updated = true;
        Ok(())
    }

    fn force_challenge_mode(&mut self) {
        let offset = find(&self.arm9, gen5::FORCE_CHALLENGE_MODE_LOCATOR);
        if offset > 0 {
            let start = offset as usize;
            self.arm9[start..start + 4].copy_from_slice(&MOV_R0_2_BX_LR);
        }
    }

    fn disable_low_hp_music(&mut self) -> Result<()> {
        let number = self.entry.int("LowHealthMusicOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        let offset = find(&overlay, gen5::LOW_HEALTH_MUSIC_LOCATOR);
        if offset > 0 {
            overlay[(offset + 1) as usize] = 0xE0;
            self.write_overlay(number, &overlay)?;
        }
        Ok(())
    }

    pub(crate) fn enable_guaranteed_catching(&mut self) -> Result<()> {
        let number = self.entry.int("BattleOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        let offset = find(&overlay, gen5::PERFECT_ODDS_BRANCH_LOCATOR);
        if offset > 0 {
            overlay[offset as usize] = 0x00;
            overlay[(offset + 1) as usize] = 0x00;
            self.write_overlay(number, &overlay)?;
        }
        Ok(())
    }

    pub(crate) fn randomize_intro(&mut self) -> Result<()> {
        let intro = randomon_core::engine::pool::random_pokemon(self)?;
        let intro_number = self.core.data.pk(intro).number;
        let graphic_number = self.entry.int("IntroGraphicOvlNumber");
        let mut graphic_overlay = self.read_overlay(graphic_number)?;
        let offset = find(&graphic_overlay, gen5::INTRO_GRAPHIC_PREFIX);
        if offset > 0 {
            write_word(
                &mut graphic_overlay,
                offset + gen5::INTRO_GRAPHIC_PREFIX.len() as i32 / 2,
                intro_number,
            );
            self.write_overlay(graphic_number, &graphic_overlay)?;
        }
        let cry_number = self.entry.int("IntroCryOvlNumber");
        let mut cry_overlay = self.read_overlay(cry_number)?;
        if self.is_bw2() {
            let offset = find(&cry_overlay, gen5::BW2_INTRO_CRY_LOCATOR);
            if offset > 0 {
                write_word(&mut cry_overlay, offset, intro_number);
                self.write_overlay(cry_number, &cry_overlay)?;
            }
        } else {
            let offset = find(&cry_overlay, gen5::BW1_INTRO_CRY_PREFIX);
            if offset > 0 {
                let start = (offset + gen5::BW1_INTRO_CRY_PREFIX.len() as i32 / 2) as usize;
                cry_overlay.copy_within(start + 6..start + 40, start + 4);
                cry_overlay[start + 10] = cry_overlay[start + 10].wrapping_add(1);
                write_full_int(&mut cry_overlay, start as i32 + 38, intro_number);
                cry_overlay[start] = 0x9;
                cry_overlay[start + 1] = 0x48;
                self.write_overlay(cry_number, &cry_overlay)?;
            }
        }
        Ok(())
    }
}
