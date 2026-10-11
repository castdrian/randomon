use randomon_core::{Compatibility, EggMoves, Movesets, RandomizationError, Result};
use randomon_data::constants::{gen5, global, moves};
use randomon_data::{
    CriticalChance, Move, MoveLearnt, MoveStatChange, StatChangeMoveType, StatChangeType,
    StatusMoveType, StatusType,
};
use randomon_java::math::round_double;
use randomon_nds::{
    find, get_byte_from_flags, read_byte_into_flags, read_full_int, read_word, signed_byte,
    type_tm_palette_number, write_word,
};

use crate::handler::Gen5RomHandler;

const MOVESET_TERMINATOR: i32 = 0xFFFF;
const TUTOR_COUNTS_PERSONAL_ORDER: [usize; 4] = [15, 17, 13, 15];
const TUTOR_COUNTS_MOVE_ORDER: [usize; 4] = [13, 15, 15, 17];
const TUTOR_PERSONAL_TO_MOVE_ORDER: [usize; 4] = [1, 3, 0, 2];

fn unsigned(data: &[u8], offset: usize) -> i32 {
    i32::from(data[offset])
}

fn hit_count(number: i32) -> f64 {
    if global::NORMAL_MULTIHIT_MOVES.contains(&number) {
        19.0 / 6.0
    } else if global::DOUBLE_HIT_MOVES.contains(&number) {
        2.0
    } else if number == moves::TRIPLE_KICK {
        2.71
    } else {
        1.0
    }
}

fn stat_change_move_type(qualities: i32, target: i32) -> StatChangeMoveType {
    match qualities {
        gen5::NO_DAMAGE_STAT_CHANGE_QUALITY | gen5::NO_DAMAGE_STATUS_AND_STAT_CHANGE_QUALITY => {
            if target == 6 || target == 7 {
                StatChangeMoveType::NoDamageUser
            } else {
                StatChangeMoveType::NoDamageTarget
            }
        }
        gen5::DAMAGE_TARGET_DEBUFF_QUALITY => StatChangeMoveType::DamageTarget,
        gen5::DAMAGE_USER_BUFF_QUALITY => StatChangeMoveType::DamageUser,
        _ => StatChangeMoveType::NoneOrUnknown,
    }
}

fn apply_status(
    mv: &mut Move,
    data: &[u8],
    internal_status_type: i32,
    qualities: i32,
) -> Result<()> {
    if internal_status_type >= 7 {
        return Ok(());
    }
    mv.status_type = StatusType::from_ordinal(internal_status_type as usize)
        .ok_or(RandomizationError::IndexOutOfBounds("StatusType.values()"))?;
    if mv.status_type == StatusType::Poison
        && (mv.number == moves::TOXIC || mv.number == moves::POISON_FANG)
    {
        mv.status_type = StatusType::ToxicPoison;
    }
    mv.status_percent_chance = f64::from(unsigned(data, 10));
    if mv.number == moves::CHATTER {
        mv.status_percent_chance = 1.0;
    }
    match qualities {
        gen5::NO_DAMAGE_STATUS_QUALITY | gen5::NO_DAMAGE_STATUS_AND_STAT_CHANGE_QUALITY => {
            mv.status_move_type = StatusMoveType::NoDamage
        }
        gen5::DAMAGE_STATUS_QUALITY => mv.status_move_type = StatusMoveType::Damage,
        _ => {}
    }
    Ok(())
}

fn stat_changes(data: &[u8]) -> Result<[MoveStatChange; 3]> {
    let mut changes = [MoveStatChange::default(); 3];
    for (index, change) in changes.iter_mut().enumerate() {
        let kind = signed_byte(data, 21 + index as i32);
        change.kind = usize::try_from(kind)
            .ok()
            .and_then(StatChangeType::from_ordinal)
            .ok_or(RandomizationError::IndexOutOfBounds(
                "StatChangeType.values()",
            ))?;
        change.stages = signed_byte(data, 24 + index as i32);
        change.percent_chance = f64::from(signed_byte(data, 27 + index as i32));
    }
    Ok(changes)
}

fn parse_move(number: i32, name: String, data: &[u8]) -> Result<Move> {
    let effect_index = read_word(data, 16);
    let internal_status_type = read_word(data, 8);
    let flags = read_full_int(data, 32);
    let qualities = signed_byte(data, 1);
    let recoil_or_absorb = signed_byte(data, 18);
    let target = unsigned(data, 20);
    let critical_stages = unsigned(data, 14);
    let mut mv = Move {
        name,
        number,
        internal_id: number,
        effect_index,
        hitratio: f64::from(unsigned(data, 4)),
        power: unsigned(data, 3),
        pp: unsigned(data, 5),
        kind: gen5::type_table(unsigned(data, 0)),
        flinch_percent_chance: f64::from(unsigned(data, 15)),
        target,
        category: gen5::move_category_from_index(unsigned(data, 2))
            .ok_or(RandomizationError::IndexOutOfBounds("moveCategoryIndices"))?,
        priority: signed_byte(data, 6),
        makes_contact: flags & 0x001 != 0,
        is_charge_move: flags & 0x002 != 0,
        is_recharge_move: flags & 0x004 != 0,
        is_punch_move: flags & 0x080 != 0,
        is_sound_move: flags & 0x100 != 0,
        is_trap_move: effect_index == gen5::TRAPPING_EFFECT || internal_status_type == 8,
        hit_count: hit_count(number),
        stat_change_move_type: stat_change_move_type(qualities, target),
        stat_changes: stat_changes(data)?,
        ..Move::default()
    };
    if critical_stages == 6 {
        mv.critical_chance = CriticalChance::Guaranteed;
    } else if critical_stages > 0 {
        mv.critical_chance = CriticalChance::Increased;
    }
    if qualities == gen5::DAMAGE_ABSORB_QUALITY {
        mv.absorb_percent = recoil_or_absorb;
    } else {
        mv.recoil_percent = -recoil_or_absorb;
    }
    apply_status(&mut mv, data, internal_status_type, qualities)?;
    Ok(mv)
}

fn file_mut(files: &mut [Vec<u8>], index: i32) -> Result<&mut Vec<u8>> {
    files
        .get_mut(index as usize)
        .ok_or(RandomizationError::IndexOutOfBounds("NARC files"))
}

fn file_ref(files: &[Vec<u8>], index: i32) -> Result<&Vec<u8>> {
    files
        .get(index as usize)
        .ok_or(RandomizationError::IndexOutOfBounds("NARC files"))
}

fn tutor_offset_of(personal_block: usize) -> usize {
    TUTOR_COUNTS_MOVE_ORDER[..TUTOR_PERSONAL_TO_MOVE_ORDER[personal_block]]
        .iter()
        .sum()
}

impl Gen5RomHandler {
    pub(crate) fn load_moves(&mut self) -> Result<()> {
        self.move_narc = self.read_narc("MoveData")?;
        let names = self.get_strings(false, self.entry.int("MoveNamesTextOffset"))?;
        self.core.data.reserve_moves(gen5::MOVE_COUNT as usize + 1);
        for number in 1..=gen5::MOVE_COUNT {
            let data = file_ref(&self.move_narc.files, number)?;
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
        for number in 1..=gen5::MOVE_COUNT {
            let mv = self
                .core
                .data
                .move_by_number(number)
                .ok_or(RandomizationError::NullPointer("moves[i]"))?;
            let data = file_mut(&mut self.move_narc.files, number)?;
            data[2] = gen5::move_category_to_byte(mv.category);
            data[3] = mv.power as u8;
            data[0] = gen5::type_to_byte(mv.kind);
            let mut hitratio = round_double(mv.hitratio) as i32;
            if hitratio < 0 {
                hitratio = 0;
            }
            if hitratio > 101 {
                hitratio = 100;
            }
            data[4] = hitratio as u8;
            data[5] = mv.pp as u8;
        }
        self.write_narc("MoveData", &self.move_narc)
    }

    fn tm_data_offset(&self) -> Option<i32> {
        let offset = find(&self.arm9, gen5::TM_DATA_PREFIX);
        (offset > 0).then(|| offset + gen5::TM_DATA_PREFIX.len() as i32 / 2)
    }

    pub(crate) fn tm_moves(&self) -> Vec<i32> {
        let Some(offset) = self.tm_data_offset() else {
            return Vec::new();
        };
        let mut tms: Vec<i32> = (0..gen5::TM_BLOCK_ONE_COUNT)
            .map(|index| read_word(&self.arm9, offset + index * 2))
            .collect();
        let second_block = offset + (gen5::TM_BLOCK_ONE_COUNT + gen5::HM_COUNT) * 2;
        tms.extend(
            (0..gen5::TM_COUNT - gen5::TM_BLOCK_ONE_COUNT)
                .map(|index| read_word(&self.arm9, second_block + index * 2)),
        );
        tms
    }

    pub(crate) fn hm_moves(&self) -> Vec<i32> {
        let Some(offset) = self.tm_data_offset() else {
            return Vec::new();
        };
        let start = offset + gen5::TM_BLOCK_ONE_COUNT * 2;
        (0..gen5::HM_COUNT)
            .map(|index| read_word(&self.arm9, start + index * 2))
            .collect()
    }

    fn move_at(&self, list: &[i32], index: i32) -> Result<i32> {
        list.get(index as usize)
            .copied()
            .ok_or(RandomizationError::IndexOutOfBounds("moveIndexes"))
    }

    fn string_at(strings: &[String], index: i32) -> Result<String> {
        strings
            .get(index as usize)
            .cloned()
            .ok_or(RandomizationError::IndexOutOfBounds("strings"))
    }

    pub(crate) fn set_tm_moves_impl(&mut self, move_indexes: &[i32]) -> Result<()> {
        let Some(offset) = self.tm_data_offset() else {
            return Ok(());
        };
        let block_two_count = gen5::TM_COUNT - gen5::TM_BLOCK_ONE_COUNT;
        for index in 0..gen5::TM_BLOCK_ONE_COUNT {
            let mv = self.move_at(move_indexes, index)?;
            write_word(&mut self.arm9, offset + index * 2, mv);
        }
        let second_block = offset + (gen5::TM_BLOCK_ONE_COUNT + gen5::HM_COUNT) * 2;
        for index in 0..block_two_count {
            let mv = self.move_at(move_indexes, index + gen5::TM_BLOCK_ONE_COUNT)?;
            write_word(&mut self.arm9, second_block + index * 2, mv);
        }
        let descriptions_index = self.entry.int("ItemDescriptionsTextOffset");
        let mut item_descriptions = self.get_strings(false, descriptions_index)?;
        let move_descriptions =
            self.get_strings(false, self.entry.int("MoveDescriptionsTextOffset"))?;
        for index in 0..gen5::TM_BLOCK_ONE_COUNT {
            let description =
                Self::string_at(&move_descriptions, self.move_at(move_indexes, index)?)?;
            *item_descriptions
                .get_mut((index + gen5::TM_BLOCK_ONE_OFFSET) as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("itemDescriptions"))? = description;
        }
        for index in 0..block_two_count {
            let mv = self.move_at(move_indexes, index + gen5::TM_BLOCK_ONE_COUNT)?;
            let description = Self::string_at(&move_descriptions, mv)?;
            *item_descriptions
                .get_mut((index + gen5::TM_BLOCK_TWO_OFFSET) as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("itemDescriptions"))? = description;
        }
        self.set_strings(false, descriptions_index, &item_descriptions)?;
        let palettes_prefix = if self.is_bw2() {
            gen5::BW2_ITEM_PALETTES_PREFIX
        } else {
            gen5::BW1_ITEM_PALETTES_PREFIX
        };
        let palettes = find(&self.arm9, palettes_prefix);
        if palettes > 0 {
            for index in 0..gen5::TM_COUNT {
                let item = if index < gen5::TM_BLOCK_ONE_COUNT {
                    gen5::TM_BLOCK_ONE_OFFSET + index
                } else {
                    gen5::TM_BLOCK_TWO_OFFSET + index - gen5::TM_BLOCK_ONE_COUNT
                };
                let mv = self.move_at(move_indexes, index)?;
                let kind = self
                    .core
                    .data
                    .move_by_number(mv)
                    .ok_or(RandomizationError::NullPointer("moves[moveIndexes.get(i)]"))?
                    .kind;
                write_word(
                    &mut self.arm9,
                    palettes + item * 4 + 2,
                    type_tm_palette_number(kind),
                );
            }
        }
        Ok(())
    }

    pub(crate) fn tmhm_compatibility(&self) -> Compatibility {
        let mut compatibility = Compatibility::new();
        let total = gen5::POKEMON_COUNT + self.forme_count();
        for number in 1..=total {
            let data = &self.poke_narc.files[self.personal_index(number) as usize];
            let mut flags = vec![false; (gen5::TM_COUNT + gen5::HM_COUNT + 1) as usize];
            for byte in 0..13 {
                read_byte_into_flags(
                    data,
                    &mut flags,
                    byte * 8 + 1,
                    gen5::BS_TMHM_COMPAT_OFFSET + byte as i32,
                );
            }
            compatibility.insert(self.pk(number), flags);
        }
        compatibility
    }

    pub(crate) fn set_tmhm_compatibility_impl(&mut self, compatibility: &Compatibility) {
        for (&pokemon, flags) in compatibility {
            let number = self.core.data.pk(pokemon).number;
            let index = self.personal_index(number) as usize;
            let data = &mut self.poke_narc.files[index];
            for byte in 0..13 {
                data[gen5::BS_TMHM_COMPAT_OFFSET as usize + byte] =
                    get_byte_from_flags(flags, byte * 8 + 1);
            }
        }
    }

    pub(crate) fn move_tutor_moves(&self) -> Result<Vec<i32>> {
        if !self.is_bw2() {
            return Ok(Vec::new());
        }
        let base = self.entry.int("MoveTutorDataOffset");
        let overlay = self.read_overlay(self.entry.int("MoveTutorOvlNumber"))?;
        Ok((0..gen5::BW2_MOVE_TUTOR_COUNT)
            .map(|index| {
                read_word(
                    &overlay,
                    base + index * gen5::BW2_MOVE_TUTOR_BYTES_PER_ENTRY,
                )
            })
            .collect())
    }

    pub(crate) fn set_move_tutor_moves_impl(&mut self, tutor_moves: &[i32]) -> Result<()> {
        if !self.is_bw2() || tutor_moves.len() != gen5::BW2_MOVE_TUTOR_COUNT as usize {
            return Ok(());
        }
        let base = self.entry.int("MoveTutorDataOffset");
        let number = self.entry.int("MoveTutorOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        for (index, &mv) in tutor_moves.iter().enumerate() {
            write_word(
                &mut overlay,
                base + index as i32 * gen5::BW2_MOVE_TUTOR_BYTES_PER_ENTRY,
                mv,
            );
        }
        self.write_overlay(number, &overlay)
    }

    pub(crate) fn move_tutor_compatibility(&self) -> Compatibility {
        let mut compatibility = Compatibility::new();
        if !self.is_bw2() {
            return compatibility;
        }
        let total = gen5::POKEMON_COUNT + self.forme_count();
        for number in 1..=total {
            let data = &self.poke_narc.files[self.personal_index(number) as usize];
            let mut flags = vec![false; gen5::BW2_MOVE_TUTOR_COUNT as usize + 1];
            for (block, &count) in TUTOR_COUNTS_PERSONAL_ORDER.iter().enumerate() {
                let mut block_flags = vec![false; count + 1];
                for byte in 0..4 {
                    read_byte_into_flags(
                        data,
                        &mut block_flags,
                        byte * 8 + 1,
                        gen5::BS_MT_COMPAT_OFFSET + (block * 4 + byte) as i32,
                    );
                }
                let offset = tutor_offset_of(block);
                flags[offset + 1..offset + 1 + count].copy_from_slice(&block_flags[1..=count]);
            }
            compatibility.insert(self.pk(number), flags);
        }
        compatibility
    }

    pub(crate) fn set_move_tutor_compatibility_impl(&mut self, compatibility: &Compatibility) {
        if !self.is_bw2() {
            return;
        }
        for (&pokemon, flags) in compatibility {
            let number = self.core.data.pk(pokemon).number;
            let index = self.personal_index(number) as usize;
            let data = &mut self.poke_narc.files[index];
            for (block, &count) in TUTOR_COUNTS_PERSONAL_ORDER.iter().enumerate() {
                let offset = tutor_offset_of(block);
                let mut block_flags = vec![false; count + 1];
                block_flags[1..=count].copy_from_slice(&flags[offset + 1..offset + 1 + count]);
                for byte in 0..4 {
                    data[gen5::BS_MT_COMPAT_OFFSET as usize + block * 4 + byte] =
                        get_byte_from_flags(&block_flags, byte * 8 + 1);
                }
            }
        }
    }

    pub(crate) fn moves_learnt(&self) -> Result<Movesets> {
        let narc = self.read_narc("PokemonMovesets")?;
        let mut movesets = Movesets::new();
        let total = gen5::POKEMON_COUNT + self.forme_count();
        for number in 1..=total {
            let data = file_ref(&narc.files, self.personal_index(number))?;
            let mut learnt = Vec::new();
            let mut location = 0;
            while read_word(data, location) != MOVESET_TERMINATOR
                || read_word(data, location + 2) != MOVESET_TERMINATOR
            {
                learnt.push(MoveLearnt {
                    move_number: read_word(data, location),
                    level: read_word(data, location + 2),
                });
                location += 4;
            }
            movesets.insert(number, learnt);
        }
        Ok(movesets)
    }

    pub(crate) fn set_moves_learnt_impl(&mut self, movesets: &Movesets) -> Result<()> {
        let mut narc = self.read_narc("PokemonMovesets")?;
        let total = gen5::POKEMON_COUNT + self.forme_count();
        for number in 1..=total {
            let learnt = movesets
                .get(&number)
                .ok_or(RandomizationError::NullPointer("movesets.get"))?;
            let mut moveset = vec![0u8; learnt.len() * 4 + 4];
            for (index, entry) in learnt.iter().enumerate() {
                write_word(&mut moveset, index as i32 * 4, entry.move_number);
                write_word(&mut moveset, index as i32 * 4 + 2, entry.level);
            }
            let end = learnt.len() as i32 * 4;
            write_word(&mut moveset, end, MOVESET_TERMINATOR);
            write_word(&mut moveset, end + 2, MOVESET_TERMINATOR);
            *file_mut(&mut narc.files, self.personal_index(number))? = moveset;
        }
        self.write_narc("PokemonMovesets", &narc)
    }

    pub(crate) fn egg_moves(&self) -> Result<EggMoves> {
        let narc = self.read_narc("EggMoves")?;
        let mut egg_moves = EggMoves::new();
        for number in 1..=gen5::POKEMON_COUNT {
            let data = file_ref(&narc.files, number)?;
            let count = read_word(data, 0);
            egg_moves.insert(
                number,
                (0..count)
                    .map(|index| read_word(data, 2 + index * 2))
                    .collect(),
            );
        }
        Ok(egg_moves)
    }

    pub(crate) fn set_egg_moves_impl(&mut self, egg_moves: &EggMoves) -> Result<()> {
        let mut narc = self.read_narc("EggMoves")?;
        for number in 1..=gen5::POKEMON_COUNT {
            let moves = egg_moves
                .get(&number)
                .ok_or(RandomizationError::NullPointer("eggMoves.get"))?;
            let data = file_mut(&mut narc.files, number)?;
            for (index, &mv) in moves.iter().enumerate() {
                let offset = 2 + index * 2;
                if offset + 2 > data.len() {
                    return Err(RandomizationError::IndexOutOfBounds("egg move data"));
                }
                write_word(data, offset as i32, mv);
            }
        }
        self.write_narc("EggMoves", &narc)
    }
}
