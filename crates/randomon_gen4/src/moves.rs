use std::collections::{BTreeMap, BTreeSet};

use randomon_core::{Compatibility, EggMoves, Movesets, RandomizationError, Result};
use randomon_data::constants::{gen4, global, moves};
use randomon_data::{
    CriticalChance, Move, MoveLearnt, StatChangeMoveType, StatChangeType, StatusMoveType,
    StatusType,
};
use randomon_java::math::round_double;
use randomon_nds::type_tm_palette_number;

use crate::bytes::{after_prefix, file, file_mut, found_at, i8_at, set_u8, set_u16, u8_at, u16_at};
use crate::handler::Gen4RomHandler;
use crate::text::rewrite_description_for_new_line_size;

const EGG_MOVE_SPECIES_MARKER: i32 = 20000;
const EGG_MOVE_TERMINATOR: i32 = 0xFFFF;
const TM_COMPAT_BYTES: i32 = 13;

pub(crate) fn read_flags(
    data: &[u8],
    flags: &mut [bool],
    offset_into_flags: usize,
    offset: i32,
) -> Result<()> {
    let byte = u8_at(data, offset)?;
    for bit in 0..8 {
        if let Some(flag) = flags.get_mut(offset_into_flags + bit) {
            *flag = (byte >> bit) & 1 == 1;
        }
    }
    Ok(())
}

pub(crate) fn byte_from_flags(flags: &[bool], offset_into_flags: usize) -> i32 {
    (0..8).fold(0, |byte, bit| match flags.get(offset_into_flags + bit) {
        Some(true) => byte | (1 << bit),
        _ => byte,
    })
}

fn hit_count(number: i32) -> f64 {
    if global::NORMAL_MULTIHIT_MOVES.contains(&number) {
        3.0
    } else if global::DOUBLE_HIT_MOVES.contains(&number) {
        2.0
    } else if number == moves::TRIPLE_KICK {
        2.71
    } else {
        1.0
    }
}

fn stat_change_move_type(effect: i32, target: i32) -> Option<StatChangeMoveType> {
    use gen4::*;
    match effect {
        NO_DAMAGE_ATK_PLUS_ONE_EFFECT
        | NO_DAMAGE_DEF_PLUS_ONE_EFFECT
        | NO_DAMAGE_SP_ATK_PLUS_ONE_EFFECT
        | NO_DAMAGE_EVASION_PLUS_ONE_EFFECT
        | NO_DAMAGE_ATK_MINUS_ONE_EFFECT
        | NO_DAMAGE_DEF_MINUS_ONE_EFFECT
        | NO_DAMAGE_SPE_MINUS_ONE_EFFECT
        | NO_DAMAGE_ACCURACY_MINUS_ONE_EFFECT
        | NO_DAMAGE_EVASION_MINUS_ONE_EFFECT
        | NO_DAMAGE_ATK_PLUS_TWO_EFFECT
        | NO_DAMAGE_DEF_PLUS_TWO_EFFECT
        | NO_DAMAGE_SPE_PLUS_TWO_EFFECT
        | NO_DAMAGE_SP_ATK_PLUS_TWO_EFFECT
        | NO_DAMAGE_SP_DEF_PLUS_TWO_EFFECT
        | NO_DAMAGE_ATK_MINUS_TWO_EFFECT
        | NO_DAMAGE_DEF_MINUS_TWO_EFFECT
        | NO_DAMAGE_SPE_MINUS_TWO_EFFECT
        | NO_DAMAGE_SP_DEF_MINUS_TWO_EFFECT
        | MINIMIZE_EFFECT
        | SWAGGER_EFFECT
        | DEFENSE_CURL_EFFECT
        | FLATTER_EFFECT
        | CHARGE_EFFECT
        | NO_DAMAGE_ATK_AND_DEF_MINUS_ONE_EFFECT
        | NO_DAMAGE_DEF_AND_SP_DEF_PLUS_ONE_EFFECT
        | NO_DAMAGE_ATK_AND_DEF_PLUS_ONE_EFFECT
        | NO_DAMAGE_SP_ATK_AND_SP_DEF_PLUS_ONE_EFFECT
        | NO_DAMAGE_ATK_AND_SPE_PLUS_ONE_EFFECT
        | NO_DAMAGE_SP_ATK_MINUS_TWO_EFFECT => Some(if target == 16 {
            StatChangeMoveType::NoDamageUser
        } else {
            StatChangeMoveType::NoDamageTarget
        }),
        DAMAGE_ATK_MINUS_ONE_EFFECT
        | DAMAGE_DEF_MINUS_ONE_EFFECT
        | DAMAGE_SPE_MINUS_ONE_EFFECT
        | DAMAGE_SP_ATK_MINUS_ONE_EFFECT
        | DAMAGE_SP_DEF_MINUS_ONE_EFFECT
        | DAMAGE_ACCURACY_MINUS_ONE_EFFECT
        | DAMAGE_SP_DEF_MINUS_TWO_EFFECT => Some(StatChangeMoveType::DamageTarget),
        DAMAGE_USER_DEF_PLUS_ONE_EFFECT
        | DAMAGE_USER_ATK_PLUS_ONE_EFFECT
        | DAMAGE_USER_ALL_PLUS_ONE_EFFECT
        | DAMAGE_USER_ATK_AND_DEF_MINUS_ONE_EFFECT
        | DAMAGE_USER_SP_ATK_MINUS_TWO_EFFECT
        | DAMAGE_USER_SPE_MINUS_ONE_EFFECT
        | DAMAGE_USER_DEF_AND_SP_DEF_MINUS_ONE_EFFECT
        | DAMAGE_USER_SP_ATK_PLUS_ONE_EFFECT => Some(StatChangeMoveType::DamageUser),
        _ => None,
    }
}

fn stat_changes_of(effect: i32) -> &'static [(StatChangeType, i32)] {
    use StatChangeType::*;
    use gen4::*;
    match effect {
        NO_DAMAGE_ATK_PLUS_ONE_EFFECT | DAMAGE_USER_ATK_PLUS_ONE_EFFECT => &[(Attack, 1)],
        NO_DAMAGE_DEF_PLUS_ONE_EFFECT | DAMAGE_USER_DEF_PLUS_ONE_EFFECT | DEFENSE_CURL_EFFECT => {
            &[(Defense, 1)]
        }
        NO_DAMAGE_SP_ATK_PLUS_ONE_EFFECT | FLATTER_EFFECT | DAMAGE_USER_SP_ATK_PLUS_ONE_EFFECT => {
            &[(SpecialAttack, 1)]
        }
        NO_DAMAGE_EVASION_PLUS_ONE_EFFECT | MINIMIZE_EFFECT => &[(Evasion, 1)],
        NO_DAMAGE_ATK_MINUS_ONE_EFFECT | DAMAGE_ATK_MINUS_ONE_EFFECT => &[(Attack, -1)],
        NO_DAMAGE_DEF_MINUS_ONE_EFFECT | DAMAGE_DEF_MINUS_ONE_EFFECT => &[(Defense, -1)],
        NO_DAMAGE_SPE_MINUS_ONE_EFFECT
        | DAMAGE_SPE_MINUS_ONE_EFFECT
        | DAMAGE_USER_SPE_MINUS_ONE_EFFECT => &[(Speed, -1)],
        NO_DAMAGE_ACCURACY_MINUS_ONE_EFFECT | DAMAGE_ACCURACY_MINUS_ONE_EFFECT => &[(Accuracy, -1)],
        NO_DAMAGE_EVASION_MINUS_ONE_EFFECT => &[(Evasion, -1)],
        NO_DAMAGE_ATK_PLUS_TWO_EFFECT | SWAGGER_EFFECT => &[(Attack, 2)],
        NO_DAMAGE_DEF_PLUS_TWO_EFFECT => &[(Defense, 2)],
        NO_DAMAGE_SPE_PLUS_TWO_EFFECT => &[(Speed, 2)],
        NO_DAMAGE_SP_ATK_PLUS_TWO_EFFECT => &[(SpecialAttack, 2)],
        NO_DAMAGE_SP_DEF_PLUS_TWO_EFFECT => &[(SpecialDefense, 2)],
        NO_DAMAGE_ATK_MINUS_TWO_EFFECT => &[(Attack, -2)],
        NO_DAMAGE_DEF_MINUS_TWO_EFFECT => &[(Defense, -2)],
        NO_DAMAGE_SPE_MINUS_TWO_EFFECT => &[(Speed, -2)],
        NO_DAMAGE_SP_DEF_MINUS_TWO_EFFECT | DAMAGE_SP_DEF_MINUS_TWO_EFFECT => {
            &[(SpecialDefense, -2)]
        }
        DAMAGE_SP_ATK_MINUS_ONE_EFFECT => &[(SpecialAttack, -1)],
        DAMAGE_SP_DEF_MINUS_ONE_EFFECT => &[(SpecialDefense, -1)],
        DAMAGE_USER_ALL_PLUS_ONE_EFFECT => &[(All, 1)],
        CHARGE_EFFECT => &[(SpecialDefense, 1)],
        DAMAGE_USER_ATK_AND_DEF_MINUS_ONE_EFFECT | NO_DAMAGE_ATK_AND_DEF_MINUS_ONE_EFFECT => {
            &[(Attack, -1), (Defense, -1)]
        }
        DAMAGE_USER_SP_ATK_MINUS_TWO_EFFECT | NO_DAMAGE_SP_ATK_MINUS_TWO_EFFECT => {
            &[(SpecialAttack, -2)]
        }
        NO_DAMAGE_DEF_AND_SP_DEF_PLUS_ONE_EFFECT => &[(Defense, 1), (SpecialDefense, 1)],
        NO_DAMAGE_ATK_AND_DEF_PLUS_ONE_EFFECT => &[(Attack, 1), (Defense, 1)],
        NO_DAMAGE_SP_ATK_AND_SP_DEF_PLUS_ONE_EFFECT => &[(SpecialAttack, 1), (SpecialDefense, 1)],
        NO_DAMAGE_ATK_AND_SPE_PLUS_ONE_EFFECT => &[(Attack, 1), (Speed, 1)],
        DAMAGE_USER_DEF_AND_SP_DEF_MINUS_ONE_EFFECT => &[(Defense, -1), (SpecialDefense, -1)],
        _ => &[],
    }
}

fn load_stat_changes(mv: &mut Move, secondary_effect_chance: i32) {
    let Some(kind) = stat_change_move_type(mv.effect_index, mv.target) else {
        return;
    };
    mv.stat_change_move_type = kind;
    for (slot, &(stat, stages)) in stat_changes_of(mv.effect_index).iter().enumerate() {
        mv.stat_changes[slot].kind = stat;
        mv.stat_changes[slot].stages = stages;
    }
    if matches!(
        kind,
        StatChangeMoveType::DamageTarget | StatChangeMoveType::DamageUser
    ) {
        for change in &mut mv.stat_changes {
            if change.kind != StatChangeType::None {
                change.percent_chance = f64::from(secondary_effect_chance);
                if change.percent_chance == 0.0 {
                    change.percent_chance = 100.0;
                }
            }
        }
    }
}

fn status_move_type(effect: i32) -> Option<StatusMoveType> {
    use gen4::*;
    match effect {
        NO_DAMAGE_SLEEP_EFFECT
        | TOXIC_EFFECT
        | NO_DAMAGE_CONFUSION_EFFECT
        | NO_DAMAGE_POISON_EFFECT
        | NO_DAMAGE_PARALYZE_EFFECT
        | NO_DAMAGE_BURN_EFFECT
        | SWAGGER_EFFECT
        | FLATTER_EFFECT
        | TEETER_DANCE_EFFECT => Some(StatusMoveType::NoDamage),
        DAMAGE_POISON_EFFECT
        | DAMAGE_BURN_EFFECT
        | DAMAGE_FREEZE_EFFECT
        | DAMAGE_PARALYZE_EFFECT
        | DAMAGE_CONFUSION_EFFECT
        | TWINEEDLE_EFFECT
        | DAMAGE_BURN_AND_THAW_USER_EFFECT
        | THUNDER_EFFECT
        | BLAZE_KICK_EFFECT
        | POISON_FANG_EFFECT
        | DAMAGE_POISON_WITH_INCREASED_CRIT_EFFECT
        | FLARE_BLITZ_EFFECT
        | BLIZZARD_EFFECT
        | VOLT_TACKLE_EFFECT
        | BOUNCE_EFFECT
        | CHATTER_EFFECT
        | FIRE_FANG_EFFECT
        | ICE_FANG_EFFECT
        | THUNDER_FANG_EFFECT => Some(StatusMoveType::Damage),
        _ => None,
    }
}

fn status_type(effect: i32) -> Option<StatusType> {
    use gen4::*;
    match effect {
        NO_DAMAGE_SLEEP_EFFECT => Some(StatusType::Sleep),
        DAMAGE_POISON_EFFECT
        | NO_DAMAGE_POISON_EFFECT
        | TWINEEDLE_EFFECT
        | DAMAGE_POISON_WITH_INCREASED_CRIT_EFFECT => Some(StatusType::Poison),
        DAMAGE_BURN_EFFECT
        | DAMAGE_BURN_AND_THAW_USER_EFFECT
        | NO_DAMAGE_BURN_EFFECT
        | BLAZE_KICK_EFFECT
        | FLARE_BLITZ_EFFECT
        | FIRE_FANG_EFFECT => Some(StatusType::Burn),
        DAMAGE_FREEZE_EFFECT | BLIZZARD_EFFECT | ICE_FANG_EFFECT => Some(StatusType::Freeze),
        DAMAGE_PARALYZE_EFFECT
        | NO_DAMAGE_PARALYZE_EFFECT
        | THUNDER_EFFECT
        | VOLT_TACKLE_EFFECT
        | BOUNCE_EFFECT
        | THUNDER_FANG_EFFECT => Some(StatusType::Paralyze),
        TOXIC_EFFECT | POISON_FANG_EFFECT => Some(StatusType::ToxicPoison),
        NO_DAMAGE_CONFUSION_EFFECT
        | DAMAGE_CONFUSION_EFFECT
        | SWAGGER_EFFECT
        | FLATTER_EFFECT
        | TEETER_DANCE_EFFECT
        | CHATTER_EFFECT => Some(StatusType::Confusion),
        _ => None,
    }
}

fn load_status(mv: &mut Move, secondary_effect_chance: i32) {
    let Some(kind) = status_move_type(mv.effect_index) else {
        return;
    };
    mv.status_move_type = kind;
    if let Some(status) = status_type(mv.effect_index) {
        mv.status_type = status;
    }
    if kind == StatusMoveType::Damage {
        mv.status_percent_chance = f64::from(secondary_effect_chance);
        if mv.status_percent_chance == 0.0 {
            mv.status_percent_chance = if mv.number == moves::CHATTER {
                1.0
            } else {
                100.0
            };
        }
    }
}

fn load_misc_move_info(mv: &mut Move, secondary_effect_chance: i32) {
    use gen4::*;
    let chance = f64::from(secondary_effect_chance);
    match mv.effect_index {
        INCREASED_CRIT_EFFECT | BLAZE_KICK_EFFECT | DAMAGE_POISON_WITH_INCREASED_CRIT_EFFECT => {
            mv.critical_chance = CriticalChance::Increased
        }
        FUTURE_SIGHT_AND_DOOM_DESIRE_EFFECT => {
            mv.critical_chance = CriticalChance::None;
            mv.flinch_percent_chance = chance;
        }
        FLINCH_EFFECT | SNORE_EFFECT | TWISTER_EFFECT | STOMP_EFFECT | FAKE_OUT_EFFECT
        | FIRE_FANG_EFFECT | ICE_FANG_EFFECT | THUNDER_FANG_EFFECT => {
            mv.flinch_percent_chance = chance
        }
        DAMAGE_ABSORB_EFFECT | DREAM_EATER_EFFECT => mv.absorb_percent = 50,
        DAMAGE_RECOIL_25_PERCENT_EFFECT => mv.recoil_percent = 25,
        DAMAGE_RECOIL_33_PERCENT_EFFECT | FLARE_BLITZ_EFFECT | VOLT_TACKLE_EFFECT => {
            mv.recoil_percent = 33
        }
        DAMAGE_RECOIL_50_PERCENT_EFFECT => mv.recoil_percent = 50,
        BINDING_EFFECT | TRAPPING_EFFECT => mv.is_trap_move = true,
        SKULL_BASH_EFFECT | SOLARBEAM_EFFECT | FLY_EFFECT | DIVE_EFFECT | DIG_EFFECT
        | BOUNCE_EFFECT | SHADOW_FORCE_EFFECT => mv.is_charge_move = true,
        RECHARGE_EFFECT => mv.is_recharge_move = true,
        RAZOR_WIND_EFFECT => {
            mv.critical_chance = CriticalChance::Increased;
            mv.is_charge_move = true;
        }
        SKY_ATTACK_EFFECT => {
            mv.critical_chance = CriticalChance::Increased;
            mv.flinch_percent_chance = chance;
            mv.is_charge_move = true;
        }
        _ => {}
    }
}

pub(crate) fn parse_move(number: i32, name: String, data: &[u8]) -> Result<Move> {
    let flags = u8_at(data, 11)?;
    let mut mv = Move {
        name,
        number,
        internal_id: number,
        effect_index: u16_at(data, 0)?,
        hitratio: f64::from(u8_at(data, 5)?),
        power: u8_at(data, 3)?,
        pp: u8_at(data, 6)?,
        kind: gen4::type_table(u8_at(data, 4)?),
        target: u16_at(data, 8)?,
        category: gen4::move_category_from_index(u8_at(data, 2)?)
            .ok_or(RandomizationError::IndexOutOfBounds("moveCategoryIndices"))?,
        priority: i8_at(data, 10)?,
        makes_contact: flags & 1 != 0,
        is_punch_move: gen4::PUNCH_MOVES.contains(&number),
        is_sound_move: gen4::SOUND_MOVES.contains(&number),
        hit_count: hit_count(number),
        ..Move::default()
    };
    let secondary_effect_chance = u8_at(data, 7)?;
    load_stat_changes(&mut mv, secondary_effect_chance);
    load_status(&mut mv, secondary_effect_chance);
    load_misc_move_info(&mut mv, secondary_effect_chance);
    Ok(mv)
}

fn clamped_hitratio(hitratio: f64) -> i32 {
    (round_double(hitratio) as i32).clamp(0, 100)
}

impl Gen4RomHandler {
    pub(crate) fn load_moves(&mut self) -> Result<()> {
        self.move_narc = self.read_narc("MoveData")?;
        let names = self.get_strings(self.entry.int("MoveNamesTextOffset"))?;
        self.core.data.reserve_moves(gen4::MOVE_COUNT as usize + 1);
        for number in 1..=gen4::MOVE_COUNT {
            let data = file(&self.move_narc, number)?;
            let name = names
                .get(number as usize)
                .cloned()
                .ok_or(RandomizationError::IndexOutOfBounds("moveNames"))?;
            let mv = parse_move(number, name, data)?;
            if number == moves::SWIFT {
                self.core.engine.perfect_accuracy = mv.hitratio as i32;
            }
            self.core.data.create_move(mv);
        }
        Ok(())
    }

    pub(crate) fn save_moves(&mut self) -> Result<()> {
        for number in 1..=gen4::MOVE_COUNT {
            let mv = self
                .core
                .data
                .move_by_number(number)
                .ok_or(RandomizationError::NullPointer("moves[i]"))?;
            let data = file_mut(&mut self.move_narc, number)?;
            set_u16(data, 0, mv.effect_index)?;
            set_u8(data, 2, i32::from(gen4::move_category_to_byte(mv.category)))?;
            set_u8(data, 3, mv.power)?;
            set_u8(data, 4, i32::from(gen4::type_to_byte(mv.kind)))?;
            set_u8(data, 5, clamped_hitratio(mv.hitratio))?;
            set_u8(data, 6, mv.pp)?;
        }
        self.write_narc("MoveData", &self.move_narc)
    }

    fn tm_data_prefix(&self) -> &'static str {
        if self.entry.is_hgss() {
            gen4::HGSS_TM_DATA_PREFIX
        } else {
            gen4::DPPT_TM_DATA_PREFIX
        }
    }

    fn tm_data_offset(&self) -> Option<i32> {
        after_prefix(&self.arm9, self.tm_data_prefix())
    }

    fn read_words(data: &[u8], start: i32, count: i32, stride: i32) -> Result<Vec<i32>> {
        (0..count)
            .map(|index| u16_at(data, start + index * stride))
            .collect()
    }

    pub(crate) fn tm_moves(&self) -> Result<Vec<i32>> {
        match self.tm_data_offset() {
            Some(offset) => Self::read_words(&self.arm9, offset, gen4::TM_COUNT, 2),
            None => Ok(Vec::new()),
        }
    }

    pub(crate) fn hm_moves(&self) -> Result<Vec<i32>> {
        match self.tm_data_offset() {
            Some(offset) => {
                Self::read_words(&self.arm9, offset + gen4::TM_COUNT * 2, gen4::HM_COUNT, 2)
            }
            None => Ok(Vec::new()),
        }
    }

    fn move_name(&self, number: i32) -> Result<String> {
        self.core
            .data
            .move_name(number)
            .map(str::to_owned)
            .ok_or(RandomizationError::NullPointer("moves[index]"))
    }

    pub(crate) fn set_tm_moves_impl(&mut self, move_indexes: &[i32]) -> Result<()> {
        let old_move_indexes = self.tm_moves()?;
        let Some(offset) = self.tm_data_offset() else {
            return Ok(());
        };
        if move_indexes.len() < gen4::TM_COUNT as usize
            || old_move_indexes.len() < gen4::TM_COUNT as usize
        {
            return Err(RandomizationError::IndexOutOfBounds("moveIndexes"));
        }
        for (index, &mv) in move_indexes
            .iter()
            .take(gen4::TM_COUNT as usize)
            .enumerate()
        {
            set_u16(&mut self.arm9, offset + index as i32 * 2, mv)?;
        }
        self.update_tm_descriptions(move_indexes)?;
        self.update_tm_palettes(move_indexes)?;
        for index in 0..gen4::TM_COUNT as usize {
            self.update_tm_text(
                index as i32 + 1,
                old_move_indexes[index],
                move_indexes[index],
            )?;
        }
        Ok(())
    }

    fn update_tm_descriptions(&mut self, move_indexes: &[i32]) -> Result<()> {
        let descriptions_index = self.entry.int("ItemDescriptionsTextOffset");
        let mut item_descriptions = self.get_strings(descriptions_index)?;
        let move_descriptions = self.get_strings(self.entry.int("MoveDescriptionsTextOffset"))?;
        let line_size = self.text_chars_per_line();
        for (index, &mv) in move_indexes
            .iter()
            .take(gen4::TM_COUNT as usize)
            .enumerate()
        {
            let description = usize::try_from(mv)
                .ok()
                .and_then(|mv| move_descriptions.get(mv))
                .ok_or(RandomizationError::IndexOutOfBounds("moveDescriptions"))?;
            let slot = item_descriptions
                .get_mut(index + gen4::TM_ITEM_OFFSET as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("itemDescriptions"))?;
            *slot = rewrite_description_for_new_line_size(description, line_size);
        }
        self.set_strings(descriptions_index, &item_descriptions)
    }

    fn update_tm_palettes(&mut self, move_indexes: &[i32]) -> Result<()> {
        let prefix = if self.entry.is_dp() {
            gen4::DP_ITEM_PALETTES_PREFIX
        } else {
            gen4::PTHGSS_ITEM_PALETTES_PREFIX
        };
        let Some(palettes) = found_at(&self.arm9, prefix) else {
            return Ok(());
        };
        for (index, &mv) in move_indexes
            .iter()
            .take(gen4::TM_COUNT as usize)
            .enumerate()
        {
            let kind = self
                .core
                .data
                .move_by_number(mv)
                .ok_or(RandomizationError::NullPointer("moves[moveIndexes.get(i)]"))?
                .kind;
            set_u16(
                &mut self.arm9,
                palettes + index as i32 * 8 + 2,
                type_tm_palette_number(kind),
            )?;
        }
        Ok(())
    }

    fn update_tm_text(&mut self, tm_number: i32, old_move: i32, new_move: i32) -> Result<()> {
        if let Some(entries) = self.entry.tm_texts.get(&tm_number) {
            let text_files: BTreeSet<i32> = entries.iter().map(|entry| entry.text_index).collect();
            let mut old_name = self.move_name(old_move)?;
            let mut new_name = self.move_name(new_move)?;
            if self.entry.is_hgss() && old_move == moves::ROAR {
                old_name = old_name.to_uppercase();
                new_name = new_name.to_uppercase();
            }
            let replacements = BTreeMap::from([(old_name, new_name)]);
            for text_file in text_files {
                self.replace_all_strings_in_entry(text_file, &replacements)?;
            }
        }
        if let Some(entry) = self.entry.tm_texts_game_corner.get(&tm_number).copied() {
            self.set_bottom_screen_tm_text(entry.text_index, entry.string_number, new_move)?;
        }
        if let Some(&script_offset) = self.entry.tm_script_offsets_frontier.get(&tm_number) {
            let script_file = self.entry.int("FrontierScriptNumber");
            set_u16(
                file_mut(&mut self.script_narc, script_file)?,
                script_offset,
                new_move,
            )?;
        }
        if let Some(&string_number) = self.entry.tm_texts_frontier.get(&tm_number) {
            let text_offset = self.entry.int("MiscUITextOffset");
            self.set_bottom_screen_tm_text(text_offset, string_number, new_move)?;
        }
        Ok(())
    }

    fn set_bottom_screen_tm_text(
        &mut self,
        text_offset: i32,
        string_number: i32,
        new_move: i32,
    ) -> Result<()> {
        let mut strings = self.get_strings(text_offset)?;
        let Some(original) = usize::try_from(string_number)
            .ok()
            .and_then(|index| strings.get(index))
        else {
            return Ok(());
        };
        let Some(post_name_index) = original.find('\\') else {
            return Ok(());
        };
        let original_name = &original[..post_name_index];
        let is_all_caps = original_name == original_name.to_uppercase();
        let mut new_name = self.move_name(new_move)?;
        if is_all_caps {
            new_name = new_name.to_uppercase();
        }
        let new_string = format!("{new_name}{}", &original[post_name_index..]);
        strings[string_number as usize] = new_string;
        self.set_strings(text_offset, &strings)
    }

    pub(crate) fn tmhm_compatibility(&self) -> Result<Compatibility> {
        let mut compatibility = Compatibility::new();
        for number in 1..=self.total_pokemon() {
            let data = file(&self.poke_narc, Self::personal_index(number))?;
            let mut flags = vec![false; (gen4::TM_COUNT + gen4::HM_COUNT + 1) as usize];
            for byte in 0..TM_COMPAT_BYTES {
                read_flags(
                    data,
                    &mut flags,
                    byte as usize * 8 + 1,
                    gen4::BS_TMHM_COMPAT_OFFSET + byte,
                )?;
            }
            compatibility.insert(self.pk(number), flags);
        }
        Ok(compatibility)
    }

    pub(crate) fn set_tmhm_compatibility_impl(
        &mut self,
        compatibility: &Compatibility,
    ) -> Result<()> {
        for (&pokemon, flags) in compatibility {
            let index = Self::personal_index(self.number_of(pokemon));
            let data = file_mut(&mut self.poke_narc, index)?;
            for byte in 0..TM_COMPAT_BYTES {
                set_u8(
                    data,
                    gen4::BS_TMHM_COMPAT_OFFSET + byte,
                    byte_from_flags(flags, byte as usize * 8 + 1),
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn has_tutors(&self) -> bool {
        !self.entry.is_dp()
    }

    pub(crate) fn move_tutor_moves(&self) -> Result<Vec<i32>> {
        if !self.has_tutors() {
            return Ok(Vec::new());
        }
        let overlay = self.read_overlay(self.entry.int("FieldOvlNumber"))?;
        Self::read_words(
            &overlay,
            self.entry.int("MoveTutorMovesOffset"),
            self.entry.int("MoveTutorCount"),
            self.entry.int("MoveTutorBytesCount"),
        )
    }

    pub(crate) fn set_move_tutor_moves_impl(&mut self, tutor_moves: &[i32]) -> Result<()> {
        let amount = self.entry.int("MoveTutorCount");
        if !self.has_tutors() || tutor_moves.len() != amount as usize {
            return Ok(());
        }
        let base = self.entry.int("MoveTutorMovesOffset");
        let bytes_per = self.entry.int("MoveTutorBytesCount");
        let number = self.entry.int("FieldOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        for (index, &mv) in tutor_moves.iter().enumerate() {
            set_u16(&mut overlay, base + index as i32 * bytes_per, mv)?;
        }
        self.write_overlay(number, &overlay)?;
        if self.entry.is_hgss()
            && let Some(&headbutt_replacement) = tutor_moves.last()
        {
            self.set_hgss_headbutt_tutor(headbutt_replacement)?;
        }
        Ok(())
    }

    fn set_hgss_headbutt_tutor(&mut self, replacement: i32) -> Result<()> {
        let scripts = file_mut(&mut self.script_narc, gen4::ILEX_FOREST_SCRIPT_FILE)?;
        for &offset in gen4::HEADBUTT_TUTOR_SCRIPT_OFFSETS {
            set_u16(scripts, offset, replacement)?;
        }
        let replacements = BTreeMap::from([(
            self.move_name(moves::HEADBUTT)?,
            self.move_name(replacement)?,
        )]);
        self.replace_all_strings_in_entry(gen4::ILEX_FOREST_STRINGS_FILE, &replacements)
    }

    fn read_tutor_compat_data(&self) -> Result<Vec<u8>> {
        if self.entry.is_hgss() {
            self.read_file("MoveTutorCompat")
        } else {
            self.read_overlay(self.entry.int("MoveTutorCompatOvlNumber"))
        }
    }

    fn write_tutor_compat_data(&self, data: &[u8]) -> Result<()> {
        if self.entry.is_hgss() {
            self.write_file("MoveTutorCompat", data)
        } else {
            self.write_overlay(self.entry.int("MoveTutorCompatOvlNumber"), data)
        }
    }

    pub(crate) fn move_tutor_compatibility(&self) -> Result<Compatibility> {
        let mut compatibility = Compatibility::new();
        if !self.has_tutors() {
            return Ok(compatibility);
        }
        let amount = self.entry.int("MoveTutorCount");
        let base = self.entry.int("MoveTutorCompatOffset");
        let bytes_per = self.entry.int("MoveTutorCompatBytesCount");
        let data = self.read_tutor_compat_data()?;
        for number in 1..=self.total_pokemon() {
            let mut flags = vec![false; amount.max(0) as usize + 1];
            for byte in 0..bytes_per {
                read_flags(
                    &data,
                    &mut flags,
                    byte as usize * 8 + 1,
                    base + (number - 1) * bytes_per + byte,
                )?;
            }
            compatibility.insert(self.pk(number), flags);
        }
        Ok(compatibility)
    }

    pub(crate) fn set_move_tutor_compatibility_impl(
        &mut self,
        compatibility: &Compatibility,
    ) -> Result<()> {
        if !self.has_tutors() {
            return Ok(());
        }
        let amount = self.entry.int("MoveTutorCount");
        let base = self.entry.int("MoveTutorCompatOffset");
        let bytes_per = self.entry.int("MoveTutorCompatBytesCount");
        let mut data = self.read_tutor_compat_data()?;
        for (&pokemon, flags) in compatibility {
            let number = self.number_of(pokemon);
            for byte in 0..bytes_per {
                let offset = base + (number - 1) * bytes_per + byte;
                let first_flag = byte * 8;
                let new_byte = byte_from_flags(flags, first_flag as usize + 1);
                if first_flag + 8 <= amount {
                    set_u8(&mut data, offset, new_byte)?;
                } else if first_flag < amount {
                    let used_bits = amount - first_flag;
                    let kept_bits = (u8_at(&data, offset)? >> used_bits) << used_bits;
                    let used_mask = (1 << used_bits) - 1;
                    set_u8(&mut data, offset, (new_byte & used_mask) | kept_bits)?;
                }
            }
        }
        self.write_tutor_compat_data(&data)
    }

    fn moveset_file_index(number: i32) -> i32 {
        Self::personal_index(number)
    }

    pub(crate) fn moves_learnt(&self) -> Result<Movesets> {
        let narc = self.read_narc("PokemonMovesets")?;
        let mut movesets = Movesets::new();
        for number in 1..=self.total_pokemon() {
            let data = file(&narc, Self::moveset_file_index(number))?;
            let mut learnt = Vec::new();
            let mut location = 0;
            loop {
                let low = u8_at(data, location)?;
                let high = u8_at(data, location + 1)?;
                if low == 0xFF && high == 0xFF {
                    break;
                }
                learnt.push(MoveLearnt {
                    move_number: low + if high & 1 == 1 { 256 } else { 0 },
                    level: (high & 0xFE) >> 1,
                });
                location += 2;
            }
            movesets.insert(number, learnt);
        }
        Ok(movesets)
    }

    pub(crate) fn set_moves_learnt_impl(&mut self, movesets: &Movesets) -> Result<()> {
        let blank_set = vec![0xFF, 0xFF, 0, 0];
        let mut narc = randomon_formats::nds::NarcArchive::new();
        narc.files.push(blank_set.clone());
        for number in 1..=self.total_pokemon() {
            if number == gen4::POKEMON_COUNT + 1 {
                for _ in 0..gen4::FORME_OFFSET {
                    narc.files.push(blank_set.clone());
                }
            }
            let learnt = movesets.get(&number).map_or(&[][..], Vec::as_slice);
            narc.files.push(encode_moveset(learnt));
        }
        self.write_narc("PokemonMovesets", &narc)
    }

    fn egg_move_location(&self) -> Result<Option<(Vec<u8>, i32)>> {
        if self.entry.is_hgss() {
            let narc = self.read_narc("EggMoves")?;
            return Ok(Some((file(&narc, 0)?.clone(), 0)));
        }
        let overlay = self.read_overlay(self.entry.int("FieldOvlNumber"))?;
        Ok(
            after_prefix(&overlay, gen4::DPPT_EGG_MOVE_TABLE_PREFIX)
                .map(|offset| (overlay, offset)),
        )
    }

    pub(crate) fn egg_moves(&self) -> Result<EggMoves> {
        match self.egg_move_location()? {
            Some((data, offset)) => read_egg_moves(&data, offset),
            None => Ok(EggMoves::new()),
        }
    }

    pub(crate) fn set_egg_moves_impl(&mut self, egg_moves: &EggMoves) -> Result<()> {
        let Some((mut data, offset)) = self.egg_move_location()? else {
            return Ok(());
        };
        write_egg_moves(egg_moves, &mut data, offset)?;
        if self.entry.is_hgss() {
            let mut narc = self.read_narc("EggMoves")?;
            *file_mut(&mut narc, 0)? = data;
            self.write_narc("EggMoves", &narc)
        } else {
            self.write_overlay(self.entry.int("FieldOvlNumber"), &data)
        }
    }
}

fn encode_moveset(learnt: &[MoveLearnt]) -> Vec<u8> {
    let mut size = learnt.len() * 2 + 2;
    if !size.is_multiple_of(4) {
        size += 2;
    }
    let mut moveset = vec![0u8; size];
    for (index, entry) in learnt.iter().enumerate() {
        moveset[index * 2] = (entry.move_number & 0xFF) as u8;
        let mut level_part = (entry.level << 1) & 0xFE;
        if entry.move_number > 255 {
            level_part += 1;
        }
        moveset[index * 2 + 1] = level_part as u8;
    }
    moveset[learnt.len() * 2] = 0xFF;
    moveset[learnt.len() * 2 + 1] = 0xFF;
    moveset
}

pub(crate) fn read_egg_moves(data: &[u8], start: i32) -> Result<EggMoves> {
    let mut egg_moves = EggMoves::new();
    let mut offset = start;
    let mut species = 0;
    let mut current: Vec<i32> = Vec::new();
    let mut value = u16_at(data, offset)?;
    while value != EGG_MOVE_TERMINATOR {
        if value > EGG_MOVE_SPECIES_MARKER {
            if !current.is_empty() {
                egg_moves.insert(species, std::mem::take(&mut current));
            }
            species = value - EGG_MOVE_SPECIES_MARKER;
            current = Vec::new();
        } else {
            current.push(value);
        }
        offset += 2;
        value = u16_at(data, offset)?;
    }
    if !current.is_empty() {
        egg_moves.insert(species, current);
    }
    Ok(egg_moves)
}

pub(crate) fn write_egg_moves(egg_moves: &EggMoves, data: &mut [u8], start: i32) -> Result<()> {
    let mut offset = start;
    for (&species, moves) in egg_moves {
        set_u16(data, offset, species + EGG_MOVE_SPECIES_MARKER)?;
        offset += 2;
        for &mv in moves {
            set_u16(data, offset, mv)?;
            offset += 2;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movesets_encode_like_upstream() {
        let learnt = [
            MoveLearnt {
                move_number: 33,
                level: 1,
            },
            MoveLearnt {
                move_number: 300,
                level: 13,
            },
        ];
        assert_eq!(
            encode_moveset(&learnt),
            vec![33, 2, 44, 27, 0xFF, 0xFF, 0, 0]
        );
        assert_eq!(encode_moveset(&[]), vec![0xFF, 0xFF, 0, 0]);
        assert_eq!(encode_moveset(&learnt[..1]), vec![33, 2, 0xFF, 0xFF]);
    }

    #[test]
    fn egg_moves_round_trip() {
        let mut data = vec![0u8; 16];
        let words = [20001, 5, 6, 20004, 7, 0xFFFF];
        for (index, word) in words.iter().enumerate() {
            set_u16(&mut data, index as i32 * 2, *word).unwrap();
        }
        let moves = read_egg_moves(&data, 0).unwrap();
        assert_eq!(moves[&1], vec![5, 6]);
        assert_eq!(moves[&4], vec![7]);
        let mut written = vec![0u8; 16];
        set_u16(&mut written, 10, 0xFFFF).unwrap();
        write_egg_moves(&moves, &mut written, 0).unwrap();
        assert_eq!(written[..12], data[..12]);
        assert!(read_egg_moves(&[0, 0], 0).is_err());
    }

    #[test]
    fn move_effects_derive_upstream_details() {
        let mut data = vec![0u8; 16];
        set_u16(&mut data, 0, gen4::FLARE_BLITZ_EFFECT).unwrap();
        data[2] = 0;
        data[3] = 120;
        data[4] = 0x0A;
        data[5] = 100;
        data[6] = 15;
        data[7] = 10;
        data[11] = 1;
        let mv = parse_move(moves::FLARE_BLITZ, "Flare Blitz".to_owned(), &data).unwrap();
        assert_eq!(mv.recoil_percent, 33);
        assert_eq!(mv.status_type, StatusType::Burn);
        assert_eq!(mv.status_percent_chance, 10.0);
        assert!(mv.makes_contact);
        data[4] = 0x09;
        data[2] = 3;
        assert!(parse_move(1, String::new(), &data).is_err());
        data[2] = 2;
        assert_eq!(parse_move(1, String::new(), &data).unwrap().kind, None);
    }
}
