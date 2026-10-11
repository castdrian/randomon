use std::collections::BTreeMap;

use randomon_core::engine::pool::{check_pokemon_restrictions, random_pokemon};
use randomon_core::engine::util::random_index;
use randomon_core::{RandomizationError, Result};
use randomon_data::constants::species::gen4_formes;
use randomon_data::constants::{gen4, items, species};
use randomon_data::{PokemonId, Type};
use randomon_settings::MiscTweak;

use crate::bytes::{
    after_prefix, file, file_mut, found_at, set_bytes, set_i32, set_u8, set_u16, u8_at,
};
use crate::handler::Gen4RomHandler;

const TYPE_TABLE_END: i32 = 0xFE;
const NEUTRAL_EFFECTIVENESS: i32 = 10;
const FEMALE_ONLY: i32 = 0xFE;
const MAX_REROLLS: usize = 10_000;
const MARILL_SPRITE_RANGE: i32 = 548;
const FIRST_FOLLOWER_SPRITE: i32 = 297;
const BIG_FOLLOWER_SIZE: i32 = 0x5208;
const NORMAL_FOLLOWER_SIZE: i32 = 0x4E27;

impl Gen4RomHandler {
    pub(crate) fn tweaks_available(&self) -> i32 {
        let mut available = MiscTweak::LowerCasePokemonNames.value()
            | MiscTweak::RandomizeCatchingTutorial.value()
            | MiscTweak::UpdateTypeEffectiveness.value()
            | MiscTweak::BanLuckyEgg.value()
            | MiscTweak::RunWithoutRunningShoes.value()
            | MiscTweak::FasterHpAndExpBars.value();
        if self.entry.tweak("FastestTextTweak").is_some() {
            available |= MiscTweak::FastestText.value();
        }
        if self.entry.tweak("NationalDexAtStartTweak").is_some() {
            available |= MiscTweak::NationalDexAtStart.value();
        }
        if self.entry.tweak("FastDistortionWorldTweak").is_some() {
            available |= MiscTweak::FastDistortionWorld.value();
        }
        if !self.entry.is_dp() {
            available |= MiscTweak::UpdateRotomFormeTyping.value();
        }
        available
    }

    pub(crate) fn apply_tweak(&mut self, tweak: MiscTweak) -> Result<()> {
        match tweak {
            MiscTweak::LowerCasePokemonNames => self.apply_camel_case_names(),
            MiscTweak::RandomizeCatchingTutorial => self.randomize_catching_tutorial()?,
            MiscTweak::FastestText => {
                self.patch_arm9("FastestTextTweak")?;
            }
            MiscTweak::BanLuckyEgg => {
                self.allowed_items.ban_singles(&[items::LUCKY_EGG]);
                self.non_bad_items.ban_singles(&[items::LUCKY_EGG]);
            }
            MiscTweak::NationalDexAtStart => self.patch_for_national_dex()?,
            MiscTweak::RunWithoutRunningShoes => self.apply_run_without_running_shoes_patch()?,
            MiscTweak::FasterHpAndExpBars => self.patch_faster_bars()?,
            MiscTweak::UpdateTypeEffectiveness => self.update_type_effectiveness()?,
            MiscTweak::FastDistortionWorld => self.apply_fast_distortion_world()?,
            MiscTweak::UpdateRotomFormeTyping => self.update_rotom_forme_typing(),
            _ => {}
        }
        Ok(())
    }

    fn apply_camel_case_names(&mut self) {
        for pokemon in self.pokemon_list.clone().into_iter().flatten() {
            let name = randomon_data::text::camel_case(&self.core.data.pk(pokemon).name);
            self.core.data.pk_mut(pokemon).name = name;
        }
    }

    fn random_pokemon_limited(&mut self, max_value: i32) -> Result<Option<PokemonId>> {
        check_pokemon_restrictions(self);
        let valid: Vec<PokemonId> = self
            .core
            .engine
            .pool
            .main
            .iter()
            .copied()
            .filter(|&pokemon| self.number_of(pokemon) <= max_value)
            .collect();
        if valid.is_empty() {
            return Ok(None);
        }
        let index = random_index(&mut self.core.engine.random, valid.len())?;
        Ok(Some(valid[index]))
    }

    fn catching_tutorial_table(&self) -> Option<i32> {
        if !self.has_catching_tutorial_fix() {
            return None;
        }
        after_prefix(
            &self.arm9,
            self.entry.string("CatchingTutorialMonTablePrefix"),
        )
    }

    fn randomize_catching_tutorial(&mut self) -> Result<()> {
        if self.has_catching_tutorial_fix() {
            if let Some(offset) = self.catching_tutorial_table()
                && let Some(opponent) = self.random_pokemon_limited(i32::MAX)?
            {
                let number = self.number_of(opponent);
                set_u16(&mut self.arm9, offset + 4, number)?;
            }
            return Ok(());
        }
        let opponent_offset = self.entry.int("CatchingTutorialOpponentMonOffset");
        if self.entry.is_hgss() {
            let player_offset = self.entry.int("CatchingTutorialPlayerMonOffset");
            let level_offset = self.entry.int("CatchingTutorialPlayerLevelOffset");
            let opponent = self.random_pokemon_limited(255)?;
            let player = self.random_pokemon_limited(255)?;
            if let (Some(opponent), Some(player)) = (opponent, player) {
                let mut arm9 = self.arm9.clone();
                set_u8(&mut arm9, opponent_offset, self.number_of(opponent))?;
                set_u8(&mut arm9, player_offset, self.number_of(player))?;
                set_u8(&mut arm9, level_offset, 10)?;
                self.arm9 = arm9;
            }
        } else if let Some(opponent) = self.random_pokemon_limited(i32::MAX)? {
            let number = self.number_of(opponent);
            set_i32(&mut self.arm9, opponent_offset, number)?;
        }
        Ok(())
    }

    fn patch_for_national_dex(&mut self) -> Result<()> {
        let index = self.entry.int("NationalDexScriptOffset");
        let mut script = file(&self.script_narc, index)?.clone();
        if self.entry.is_hgss() {
            script.extend_from_slice(&[0; 4]);
        }
        self.ips_patch(&mut script, "NationalDexAtStartTweak")?;
        *file_mut(&mut self.script_narc, index)? = script;
        Ok(())
    }

    fn apply_run_without_running_shoes_patch(&mut self) -> Result<()> {
        let prefix = gen4::run_without_running_shoes_prefix(self.rom_type());
        if let Some(offset) = found_at(&self.arm9, prefix) {
            set_u16(&mut self.arm9, offset + 0xE, 0)?;
        }
        Ok(())
    }

    fn patch_faster_bars(&mut self) -> Result<()> {
        let number = self.entry.int("BattleOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        if let Some(offset) = after_prefix(&overlay, gen4::HP_BAR_SPEED_PREFIX) {
            set_u8(&mut overlay, offset, 0x02)?;
        }
        if let Some(offset) = after_prefix(&overlay, gen4::EXP_BAR_SPEED_PREFIX) {
            set_bytes(&mut overlay, offset, &[0xB0, 0x19])?;
        }
        if let Some(offset) = after_prefix(&overlay, gen4::BOTH_BARS_SPEED_PREFIX) {
            set_u8(&mut overlay, offset, 0x40)?;
        }
        self.write_overlay(number, &overlay)
    }

    fn update_type_effectiveness(&mut self) -> Result<()> {
        let number = self.entry.int("BattleOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        let Some(table) = found_at(&overlay, gen4::TYPE_EFFECTIVENESS_TABLE_LOCATOR) else {
            return Ok(());
        };
        let ghost = i32::from(gen4::type_to_byte(Some(Type::Ghost)));
        let dark = i32::from(gen4::type_to_byte(Some(Type::Dark)));
        let steel = i32::from(gen4::type_to_byte(Some(Type::Steel)));
        self.core
            .engine
            .handler_log("--Updating Type Effectiveness--");
        let mut offset = table;
        while u8_at(&overlay, offset)? != TYPE_TABLE_END {
            let attacker = u8_at(&overlay, offset)?;
            let defender = u8_at(&overlay, offset + 1)?;
            let known = matches!(u8_at(&overlay, offset + 2)?, 0 | 5 | 10 | 20);
            if known && defender == steel && (attacker == ghost || attacker == dark) {
                set_u8(&mut overlay, offset + 2, NEUTRAL_EFFECTIVENESS)?;
                let line = if attacker == ghost {
                    "Replaced: Ghost not very effective vs Steel => Ghost neutral vs Steel"
                } else {
                    "Replaced: Dark not very effective vs Steel => Dark neutral vs Steel"
                };
                self.core.engine.handler_log(line);
            }
            offset += 3;
        }
        self.core.engine.handler_log_blank_line();
        self.write_overlay(number, &overlay)?;
        self.effectiveness_updated = true;
        Ok(())
    }

    fn apply_fast_distortion_world(&mut self) -> Result<()> {
        let index = gen4::PT_SPEAR_PILLAR_PORTAL_SCRIPT_FILE;
        let mut script = file(&self.script_narc, index)?.clone();
        script.extend_from_slice(&[0; 12]);
        self.ips_patch(&mut script, "FastDistortionWorldTweak")?;
        *file_mut(&mut self.script_narc, index)? = script;
        Ok(())
    }

    fn update_rotom_forme_typing(&mut self) {
        for (forme, kind) in [
            (gen4_formes::ROTOM_H, Type::Fire),
            (gen4_formes::ROTOM_W, Type::Water),
            (gen4_formes::ROTOM_FR, Type::Ice),
            (gen4_formes::ROTOM_FA, Type::Flying),
            (gen4_formes::ROTOM_M, Type::Grass),
        ] {
            if let Ok(pokemon) = self.pokemon_by_number(forme) {
                self.core.data.pk_mut(pokemon).secondary_type = Some(kind);
            }
        }
    }

    pub(crate) fn enable_guaranteed_catching(&mut self) -> Result<()> {
        let number = self.entry.int("BattleOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        if let Some(offset) = found_at(&overlay, gen4::PERFECT_ODDS_BRANCH_LOCATOR) {
            set_bytes(&mut overlay, offset, &[0; 4])?;
            self.write_overlay(number, &overlay)?;
        }
        Ok(())
    }

    fn random_non_female_only_pokemon(&mut self) -> Result<Option<PokemonId>> {
        for _ in 0..MAX_REROLLS {
            let pokemon = random_pokemon(self)?;
            if self.core.data.pk(pokemon).gender_ratio != FEMALE_ONLY {
                return Ok(Some(pokemon));
            }
        }
        Ok(None)
    }

    fn randomize_dppt_intro(&mut self) -> Result<()> {
        let Some(intro) = self.random_non_female_only_pokemon()? else {
            return Ok(());
        };
        let number = self.entry.int("IntroOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        for prefix in gen4::DPPT_INTRO_PREFIXES {
            if let Some(offset) = after_prefix(&overlay, prefix) {
                set_u16(&mut overlay, offset, self.number_of(intro))?;
            }
        }
        self.write_overlay(number, &overlay)
    }

    fn random_marill_sprite(&mut self) -> Result<i32> {
        let mut sprite = FIRST_FOLLOWER_SPRITE;
        for _ in 0..MAX_REROLLS {
            sprite = random_index(&mut self.core.engine.random, MARILL_SPRITE_RANGE as usize)?
                as i32
                + FIRST_FOLLOWER_SPRITE;
            if !gen4::HGSS_BANNED_OVERWORLD_POKEMON.contains(&sprite) {
                break;
            }
        }
        Ok(sprite)
    }

    fn randomize_hgss_intro(&mut self) -> Result<()> {
        let sprite = self.random_marill_sprite()?;
        let field_number = self.entry.int("FieldOvlNumber");
        let mut overlay = self.read_overlay(field_number)?;
        if let Some(offset) = after_prefix(&overlay, gen4::LYRA_ETHAN_MARILL_SPRITE_PREFIX) {
            set_u16(&mut overlay, offset, sprite)?;
            let size = if gen4::HGSS_BIG_OVERWORLD_POKEMON.contains(&sprite) {
                BIG_FOLLOWER_SIZE
            } else {
                NORMAL_FOLLOWER_SIZE
            };
            set_u16(&mut overlay, offset + 2, size)?;
        }
        self.write_overlay(field_number, &overlay)?;
        let replacement_number = gen4::convert_overworld_sprite_to_species(sprite);
        let replacement = self.pokemon_by_number(replacement_number)?;
        for entry in &self.entry.marill_cry_script_entries {
            set_u16(
                file_mut(&mut self.script_narc, entry.file)?,
                entry.offset,
                replacement_number,
            )?;
        }
        let original = self
            .core
            .data
            .pk(self.pk(species::MARILL))
            .name
            .to_uppercase();
        let replacements =
            BTreeMap::from([(original, self.core.data.pk(replacement).name.clone())]);
        for &text in self.entry.array_or_empty("MarillTextFiles") {
            self.replace_all_strings_in_entry(text, &replacements)?;
        }
        if let Some(offset) = self.catching_tutorial_table() {
            set_u16(&mut self.arm9, offset, replacement_number)?;
        }
        Ok(())
    }

    pub(crate) fn randomize_intro(&mut self) -> Result<()> {
        match self.rom_type() {
            gen4::TYPE_DP | gen4::TYPE_PLAT => self.randomize_dppt_intro(),
            gen4::TYPE_HGSS => self.randomize_hgss_intro(),
            _ => Err(RandomizationError::randomization("unknown Gen 4 game")),
        }
    }
}
