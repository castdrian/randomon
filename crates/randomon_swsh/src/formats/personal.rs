use crate::bytes::{array_at, put_bytes, put_u8, put_u16, put_u32, u8_at, u16_at, u32_at};
use crate::error::{Result, invalid};
use crate::formats::common::{BitFlags, Stats};
use crate::version::DataVersion;

pub const BASE_RECORD_SIZE: usize = 0xA8;
pub const EXPANSION_RECORD_SIZE: usize = 0xB0;
pub const BASE_RECORD_COUNT: usize = 1171;
pub const EXPANSION_RECORD_COUNT: usize = 1192;
pub const TM_COUNT: usize = 100;
pub const TR_COUNT: usize = 100;
const CONTEXT: &str = "personal_total.bin";

const COLOR_MASK: u8 = 0x3F;
const PRESENT_IN_GAME_BIT: u8 = 0x40;
const SPRITE_FORM_BIT: u8 = 0x80;
const REGIONAL_FORM_BIT: u16 = 0x0001;
const CANNOT_DYNAMAX_BIT: u16 = 0x0004;
const EV_YIELD_MASK: u16 = 0x0FFF;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonalInfo {
    pub base_stats: Stats<u8>,
    pub types: [u8; 2],
    pub catch_rate: u8,
    pub evolution_stage: u8,
    pub ev_yield: Stats<u8>,
    pub held_items: [u16; 3],
    pub gender_ratio: u8,
    pub hatch_cycles: u8,
    pub base_friendship: u8,
    pub growth_rate: u8,
    pub egg_groups: [u8; 2],
    pub abilities: [u16; 3],
    pub form_stats_index: u16,
    pub form_count: u8,
    pub color: u8,
    pub is_present_in_game: bool,
    pub sprite_form: bool,
    pub base_experience: u16,
    pub height: u16,
    pub weight: u16,
    pub tm: BitFlags<16>,
    pub type_tutors: BitFlags<4>,
    pub tr: BitFlags<16>,
    pub national_dex: u32,
    pub hatched_species: u16,
    pub local_form_index: u16,
    pub regional_flags: u16,
    pub galar_dex: u16,
    pub form: u16,
    pub armor_tutors: Option<BitFlags<4>>,
    pub armor_dex: Option<u16>,
    pub crown_dex: Option<u16>,
    pub raw: Vec<u8>,
}

fn read_ev_yield(packed: u16) -> Stats<u8> {
    let field = |shift: u16| ((packed >> shift) & 0x3) as u8;
    Stats {
        hp: field(0),
        attack: field(2),
        defense: field(4),
        speed: field(6),
        special_attack: field(8),
        special_defense: field(10),
    }
}

fn pack_ev_yield(stats: &Stats<u8>, previous: u16) -> u16 {
    let field = |value: u8, shift: u16| (u16::from(value) & 0x3) << shift;
    (previous & !EV_YIELD_MASK)
        | field(stats.hp, 0)
        | field(stats.attack, 2)
        | field(stats.defense, 4)
        | field(stats.speed, 6)
        | field(stats.special_attack, 8)
        | field(stats.special_defense, 10)
}

fn read_pair<const N: usize>(record: &[u8], offset: usize) -> Result<[u16; N]> {
    let mut values = [0u16; N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = u16_at(record, offset + index * 2, CONTEXT)?;
    }
    Ok(values)
}

fn write_pair(record: &mut [u8], offset: usize, values: &[u16]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        put_u16(record, offset + index * 2, *value, CONTEXT)?;
    }
    Ok(())
}

impl PersonalInfo {
    pub fn parse(record: &[u8]) -> Result<Self> {
        if record.len() != BASE_RECORD_SIZE && record.len() != EXPANSION_RECORD_SIZE {
            return Err(invalid(
                CONTEXT,
                format!("record of {} bytes", record.len()),
            ));
        }
        let expansion = record.len() == EXPANSION_RECORD_SIZE;
        let stats = array_at::<6>(record, 0, CONTEXT)?;
        let appearance = u8_at(record, 0x21, CONTEXT)?;
        Ok(PersonalInfo {
            base_stats: Stats::from_hp_atk_def_spe_spa_spd(stats),
            types: array_at(record, 0x06, CONTEXT)?,
            catch_rate: u8_at(record, 0x08, CONTEXT)?,
            evolution_stage: u8_at(record, 0x09, CONTEXT)?,
            ev_yield: read_ev_yield(u16_at(record, 0x0A, CONTEXT)?),
            held_items: read_pair(record, 0x0C)?,
            gender_ratio: u8_at(record, 0x12, CONTEXT)?,
            hatch_cycles: u8_at(record, 0x13, CONTEXT)?,
            base_friendship: u8_at(record, 0x14, CONTEXT)?,
            growth_rate: u8_at(record, 0x15, CONTEXT)?,
            egg_groups: array_at(record, 0x16, CONTEXT)?,
            abilities: read_pair(record, 0x18)?,
            form_stats_index: u16_at(record, 0x1E, CONTEXT)?,
            form_count: u8_at(record, 0x20, CONTEXT)?,
            color: appearance & COLOR_MASK,
            is_present_in_game: appearance & PRESENT_IN_GAME_BIT != 0,
            sprite_form: appearance & SPRITE_FORM_BIT != 0,
            base_experience: u16_at(record, 0x22, CONTEXT)?,
            height: u16_at(record, 0x24, CONTEXT)?,
            weight: u16_at(record, 0x26, CONTEXT)?,
            tm: BitFlags(array_at(record, 0x28, CONTEXT)?),
            type_tutors: BitFlags(array_at(record, 0x38, CONTEXT)?),
            tr: BitFlags(array_at(record, 0x3C, CONTEXT)?),
            national_dex: u32_at(record, 0x4C, CONTEXT)?,
            hatched_species: u16_at(record, 0x56, CONTEXT)?,
            local_form_index: u16_at(record, 0x58, CONTEXT)?,
            regional_flags: u16_at(record, 0x5A, CONTEXT)?,
            galar_dex: u16_at(record, 0x5C, CONTEXT)?,
            form: u16_at(record, 0x5E, CONTEXT)?,
            armor_tutors: expansion
                .then(|| array_at(record, 0xA8, CONTEXT).map(BitFlags))
                .transpose()?,
            armor_dex: expansion
                .then(|| u16_at(record, 0xAC, CONTEXT))
                .transpose()?,
            crown_dex: expansion
                .then(|| u16_at(record, 0xAE, CONTEXT))
                .transpose()?,
            raw: record.to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut record = self.raw.clone();
        let expansion = record.len() == EXPANSION_RECORD_SIZE;
        if record.len() != BASE_RECORD_SIZE && !expansion {
            return Err(invalid(
                CONTEXT,
                format!("record of {} bytes", record.len()),
            ));
        }
        if !expansion
            && (self.armor_tutors.is_some() || self.armor_dex.is_some() || self.crown_dex.is_some())
        {
            return Err(invalid(
                CONTEXT,
                "base game records have no armor or crown fields",
            ));
        }
        put_bytes(
            &mut record,
            0,
            &self.base_stats.to_hp_atk_def_spe_spa_spd(),
            CONTEXT,
        )?;
        put_bytes(&mut record, 0x06, &self.types, CONTEXT)?;
        put_u8(&mut record, 0x08, self.catch_rate, CONTEXT)?;
        put_u8(&mut record, 0x09, self.evolution_stage, CONTEXT)?;
        let previous_yield = u16_at(&record, 0x0A, CONTEXT)?;
        put_u16(
            &mut record,
            0x0A,
            pack_ev_yield(&self.ev_yield, previous_yield),
            CONTEXT,
        )?;
        write_pair(&mut record, 0x0C, &self.held_items)?;
        put_u8(&mut record, 0x12, self.gender_ratio, CONTEXT)?;
        put_u8(&mut record, 0x13, self.hatch_cycles, CONTEXT)?;
        put_u8(&mut record, 0x14, self.base_friendship, CONTEXT)?;
        put_u8(&mut record, 0x15, self.growth_rate, CONTEXT)?;
        put_bytes(&mut record, 0x16, &self.egg_groups, CONTEXT)?;
        write_pair(&mut record, 0x18, &self.abilities)?;
        put_u16(&mut record, 0x1E, self.form_stats_index, CONTEXT)?;
        put_u8(&mut record, 0x20, self.form_count, CONTEXT)?;
        put_u8(&mut record, 0x21, self.appearance_byte(), CONTEXT)?;
        put_u16(&mut record, 0x22, self.base_experience, CONTEXT)?;
        put_u16(&mut record, 0x24, self.height, CONTEXT)?;
        put_u16(&mut record, 0x26, self.weight, CONTEXT)?;
        put_bytes(&mut record, 0x28, &self.tm.0, CONTEXT)?;
        put_bytes(&mut record, 0x38, &self.type_tutors.0, CONTEXT)?;
        put_bytes(&mut record, 0x3C, &self.tr.0, CONTEXT)?;
        put_u32(&mut record, 0x4C, self.national_dex, CONTEXT)?;
        put_u16(&mut record, 0x56, self.hatched_species, CONTEXT)?;
        put_u16(&mut record, 0x58, self.local_form_index, CONTEXT)?;
        put_u16(&mut record, 0x5A, self.regional_flags, CONTEXT)?;
        put_u16(&mut record, 0x5C, self.galar_dex, CONTEXT)?;
        put_u16(&mut record, 0x5E, self.form, CONTEXT)?;
        if let Some(armor_tutors) = self.armor_tutors {
            put_bytes(&mut record, 0xA8, &armor_tutors.0, CONTEXT)?;
        }
        if let Some(armor_dex) = self.armor_dex {
            put_u16(&mut record, 0xAC, armor_dex, CONTEXT)?;
        }
        if let Some(crown_dex) = self.crown_dex {
            put_u16(&mut record, 0xAE, crown_dex, CONTEXT)?;
        }
        Ok(record)
    }

    fn appearance_byte(&self) -> u8 {
        let present = if self.is_present_in_game {
            PRESENT_IN_GAME_BIT
        } else {
            0
        };
        let sprite = if self.sprite_form { SPRITE_FORM_BIT } else { 0 };
        (self.color & COLOR_MASK) | present | sprite
    }

    pub fn is_regional_form(&self) -> bool {
        self.regional_flags & REGIONAL_FORM_BIT != 0
    }

    pub fn cannot_dynamax(&self) -> bool {
        self.regional_flags & CANNOT_DYNAMAX_BIT != 0
    }

    pub fn set_cannot_dynamax(&mut self, value: bool) {
        if value {
            self.regional_flags |= CANNOT_DYNAMAX_BIT;
        } else {
            self.regional_flags &= !CANNOT_DYNAMAX_BIT;
        }
    }

    pub fn can_learn_tm(&self, index: usize) -> bool {
        index < TM_COUNT && self.tm.get(index)
    }

    pub fn can_learn_tr(&self, index: usize) -> bool {
        index < TR_COUNT && self.tr.get(index)
    }

    pub fn is_in_any_dex(&self) -> bool {
        self.galar_dex != 0 || self.armor_dex.unwrap_or(0) != 0 || self.crown_dex.unwrap_or(0) != 0
    }

    pub fn alternate_form_index(&self, form: u16) -> Option<usize> {
        (form != 0 && form < u16::from(self.form_count) && self.form_stats_index != 0)
            .then(|| usize::from(self.form_stats_index) + usize::from(form) - 1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonalTable {
    pub entries: Vec<PersonalInfo>,
}

pub fn record_size_for(length: usize) -> Result<usize> {
    let fits = |size: usize| length > 0 && length.is_multiple_of(size);
    match (fits(BASE_RECORD_SIZE), fits(EXPANSION_RECORD_SIZE)) {
        (true, false) => Ok(BASE_RECORD_SIZE),
        (false, true) => Ok(EXPANSION_RECORD_SIZE),
        (true, true) if length / BASE_RECORD_SIZE == BASE_RECORD_COUNT => Ok(BASE_RECORD_SIZE),
        (true, true) if length / EXPANSION_RECORD_SIZE == EXPANSION_RECORD_COUNT => {
            Ok(EXPANSION_RECORD_SIZE)
        }
        _ => Err(invalid(
            CONTEXT,
            format!("{length} bytes is not a whole number of base or expansion records"),
        )),
    }
}

impl PersonalTable {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let size = record_size_for(data.len())?;
        let entries = data
            .chunks_exact(size)
            .map(PersonalInfo::parse)
            .collect::<Result<Vec<_>>>()?;
        Ok(PersonalTable { entries })
    }

    pub fn record_size(&self) -> usize {
        self.entries
            .first()
            .map_or(BASE_RECORD_SIZE, |entry| entry.raw.len())
    }

    pub fn data_version(&self) -> DataVersion {
        if self.record_size() == EXPANSION_RECORD_SIZE {
            DataVersion::Expansion
        } else {
            DataVersion::Base
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&PersonalInfo> {
        self.entries.get(index)
    }

    pub fn form_entry(&self, species: u16, form: u16) -> Option<&PersonalInfo> {
        let base = self.entries.get(usize::from(species))?;
        match base.alternate_form_index(form) {
            Some(index) => self.entries.get(index),
            None => Some(base),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let size = self.record_size();
        let mut output = Vec::with_capacity(size * self.entries.len());
        for entry in &self.entries {
            let record = entry.to_bytes()?;
            if record.len() != size {
                return Err(invalid(CONTEXT, "records of mixed sizes"));
            }
            output.extend_from_slice(&record);
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(size: usize) -> Vec<u8> {
        let mut data = vec![0u8; size];
        data[..6].copy_from_slice(&[45, 49, 49, 45, 65, 65]);
        data[0x06] = 11;
        data[0x07] = 3;
        data[0x0A] = 0b0100_0000;
        data[0x0B] = 0xF0 | 0b0000_0001;
        data[0x18] = 65;
        data[0x1C] = 34;
        data[0x20] = 1;
        data[0x21] = 0x40 | 5;
        data[0x28] = 0b0000_0101;
        data[0x3C + 12] = 0x08;
        data[0x4C] = 1;
        data[0x5A] = 0x04;
        data[0x5C] = 7;
        data[0x60] = 0xAB;
        if size == EXPANSION_RECORD_SIZE {
            data[0xA8] = 0x01;
            data[0xAC] = 3;
            data[0xAE] = 9;
        }
        data
    }

    #[test]
    fn parses_typed_fields() {
        let info = PersonalInfo::parse(&record(BASE_RECORD_SIZE)).expect("parse");
        assert_eq!(info.base_stats.speed, 45);
        assert_eq!(info.base_stats.special_attack, 65);
        assert_eq!(info.types, [11, 3]);
        assert_eq!(info.ev_yield.speed, 1);
        assert_eq!(info.ev_yield.special_attack, 1);
        assert_eq!(info.abilities, [65, 0, 34]);
        assert_eq!(info.color, 5);
        assert!(info.is_present_in_game);
        assert!(info.can_learn_tm(0) && !info.can_learn_tm(1) && info.can_learn_tm(2));
        assert!(info.can_learn_tr(99) && !info.can_learn_tr(103));
        assert!(info.cannot_dynamax());
        assert_eq!(info.armor_dex, None);
        assert!(info.is_in_any_dex());
    }

    #[test]
    fn base_and_expansion_tables_round_trip() {
        for size in [BASE_RECORD_SIZE, EXPANSION_RECORD_SIZE] {
            let data: Vec<u8> = (0..3).flat_map(|_| record(size)).collect();
            let table = PersonalTable::parse(&data).expect("parse");
            assert_eq!(table.len(), 3);
            assert_eq!(table.record_size(), size);
            assert_eq!(table.to_bytes().expect("write"), data);
        }
        let expansion = PersonalTable::parse(&record(EXPANSION_RECORD_SIZE)).expect("parse");
        assert_eq!(expansion.data_version(), DataVersion::Expansion);
        assert_eq!(expansion.entries[0].crown_dex, Some(9));
        assert_eq!(
            expansion.entries[0].armor_tutors.map(|flags| flags.get(0)),
            Some(true)
        );
    }

    #[test]
    fn modified_fields_are_written_without_touching_unknown_bits() {
        let mut table = PersonalTable::parse(&record(BASE_RECORD_SIZE)).expect("parse");
        let entry = &mut table.entries[0];
        entry.base_stats.hp = 200;
        entry.ev_yield.hp = 3;
        entry.tm.set(1, true);
        entry.set_cannot_dynamax(false);
        entry.is_present_in_game = false;
        let bytes = table.to_bytes().expect("write");
        assert_eq!(bytes[0], 200);
        assert_eq!(bytes[0x0B] & 0xF0, 0xF0);
        assert_eq!(bytes[0x28], 0b0000_0111);
        assert_eq!(bytes[0x21], 5);
        assert_eq!(bytes[0x5A], 0);
        assert_eq!(bytes[0x60], 0xAB);
        let reparsed = PersonalTable::parse(&bytes).expect("parse");
        assert_eq!(reparsed.entries[0].ev_yield.hp, 3);
        assert_eq!(reparsed.entries[0].base_stats.hp, 200);
    }

    #[test]
    fn rejects_lengths_that_are_not_whole_records() {
        assert!(PersonalTable::parse(&[0u8; 0xA9]).is_err());
        assert!(PersonalTable::parse(&[]).is_err());
        assert_eq!(record_size_for(0xA8 * 1171), Ok(BASE_RECORD_SIZE));
        assert_eq!(record_size_for(0xB0 * 1192), Ok(EXPANSION_RECORD_SIZE));
        assert!(record_size_for(3696).is_err());
    }

    #[test]
    fn base_records_reject_expansion_fields() {
        let mut info = PersonalInfo::parse(&record(BASE_RECORD_SIZE)).expect("parse");
        info.crown_dex = Some(1);
        assert!(info.to_bytes().is_err());
    }

    #[test]
    fn alternate_forms_point_into_the_form_table() {
        let mut data: Vec<u8> = (0..3).flat_map(|_| record(BASE_RECORD_SIZE)).collect();
        data[0x1E] = 2;
        data[0x20] = 2;
        let table = PersonalTable::parse(&data).expect("parse");
        assert_eq!(table.entries[0].alternate_form_index(1), Some(2));
        assert_eq!(table.entries[0].alternate_form_index(2), None);
        assert!(table.form_entry(0, 1).is_some());
        assert!(table.form_entry(9, 0).is_none());
    }
}
