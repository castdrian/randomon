use crate::error::SettingsError;
use crate::java_base64;
use crate::misc_tweak::MiscTweak;

const DATA_BLOCK_SIZE: usize = 200;

struct DataBlock {
    bytes: [u8; DATA_BLOCK_SIZE],
    length: i32,
}

fn out_of_bounds() -> SettingsError {
    SettingsError::UpdaterOutOfBounds
}

fn index(position: i32) -> Result<usize, SettingsError> {
    usize::try_from(position)
        .ok()
        .filter(|&position| position < DATA_BLOCK_SIZE)
        .ok_or_else(out_of_bounds)
}

impl DataBlock {
    fn new(data: &[u8]) -> Result<Self, SettingsError> {
        if data.len() > DATA_BLOCK_SIZE {
            return Err(out_of_bounds());
        }
        let mut bytes = [0u8; DATA_BLOCK_SIZE];
        bytes[..data.len()].copy_from_slice(data);
        Ok(Self {
            bytes,
            length: data.len() as i32,
        })
    }

    fn get(&self, position: usize) -> u8 {
        self.bytes[position]
    }

    fn set(&mut self, position: usize, value: u8) {
        self.bytes[position] = value;
    }

    fn or(&mut self, position: usize, value: u8) {
        self.bytes[position] |= value;
    }

    fn read_i32_be(&self, offset: usize) -> i32 {
        let mut array = [0u8; 4];
        array.copy_from_slice(&self.bytes[offset..offset + 4]);
        i32::from_be_bytes(array)
    }

    fn write_i32_be(&mut self, offset: usize, value: i32) {
        self.bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn read_u16_le(&self, offset: usize) -> i32 {
        i32::from(self.bytes[offset]) | (i32::from(self.bytes[offset + 1]) << 8)
    }

    fn write_u16_le(&mut self, offset: usize, value: i32) {
        self.bytes[offset] = value as u8;
        self.bytes[offset + 1] = (value >> 8) as u8;
    }

    fn insert_extra_byte(&mut self, position: i32, value: u8) -> Result<(), SettingsError> {
        if self.length == DATA_BLOCK_SIZE as i32 {
            return Ok(());
        }
        let mut cursor = self.length;
        while cursor > position {
            let previous = self.bytes[index(cursor - 1)?];
            self.bytes[index(cursor)?] = previous;
            cursor -= 1;
        }
        self.bytes[index(position)?] = value;
        self.length += 1;
        Ok(())
    }

    fn insert_int_field(&mut self, position: i32, value: i32) -> Result<(), SettingsError> {
        if self.length + 4 > DATA_BLOCK_SIZE as i32 {
            return Ok(());
        }
        let mut cursor = self.length;
        while cursor > position + 3 {
            let source = self.bytes[index(cursor - 4)?];
            self.bytes[index(cursor)?] = source;
            cursor -= 1;
        }
        self.write_i32_be(index(position)?, value);
        self.length += 4;
        Ok(())
    }

    fn finish(mut self) -> Result<String, SettingsError> {
        let length = usize::try_from(self.length)
            .ok()
            .filter(|&length| (8..=DATA_BLOCK_SIZE).contains(&length))
            .ok_or_else(out_of_bounds)?;
        let checksum = crc32fast::hash(&self.bytes[..length - 8]) as i32;
        self.write_i32_be(length - 8, checksum);
        Ok(java_base64::encode(&self.bytes[..length]))
    }
}

fn remapped_byte(old: u8, old_indexes: &[u32]) -> u8 {
    old_indexes
        .iter()
        .enumerate()
        .filter(|&(_, &old_index)| old & (1 << old_index) != 0)
        .fold(0, |remapped, (new_index, _)| remapped | (1 << new_index))
}

type UpgradeStep = fn(&mut DataBlock) -> Result<(), SettingsError>;

const UPGRADE_STEPS: [(i32, UpgradeStep); 15] = [
    (102, before_102),
    (110, before_110),
    (150, before_150),
    (160, before_160),
    (162, before_162),
    (170, before_170),
    (171, before_171),
    (172, before_172),
    (300, before_300),
    (311, before_311),
    (314, before_314),
    (315, before_315),
    (317, before_317),
    (319, before_319),
    (321, before_321),
];

pub fn update(old_version: i32, config_string: &str) -> Result<String, SettingsError> {
    let data = java_base64::decode(config_string)?;
    let mut block = DataBlock::new(&data)?;
    for (threshold, step) in UPGRADE_STEPS {
        if old_version < threshold {
            step(&mut block)?;
        }
    }
    block.finish()
}

fn before_102(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.or(1, 0x10);
    Ok(())
}

fn before_110(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.insert_extra_byte(15, 0x04 | 0x10)
}

fn before_150(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.insert_extra_byte(16, 0x40)?;
    block.insert_extra_byte(17, 0x04)?;
    block.length += 4;
    Ok(())
}

fn before_160(block: &mut DataBlock) -> Result<(), SettingsError> {
    let first_byte = block.get(0);
    let update_moves = first_byte & 0x08;
    let later_fields = first_byte & (0x10 | 0x20 | 0x40);
    block.set(
        0,
        (first_byte & 0x0F) | (update_moves << 1) | (later_fields << 1),
    );

    let has_bw_patch = (block.get(2) & 0x08) >> 3;
    let has_held_items = (block.get(2) & 0x80) >> 7;
    block.set(2, block.get(2) & 0x7F);

    if has_held_items > 0 {
        block.or(3, 0x10);
    }

    let wild_no_legendaries = (block.get(11) & 0x80) >> 7;
    let trainer_no_early_shedinja = (block.get(13) & 0x10) >> 4;
    let wild_catch_rate = (block.get(13) & 0x08) >> 3;
    block.set(
        11,
        (block.get(11) & 0x7F) | (trainer_no_early_shedinja << 7),
    );

    block.insert_extra_byte(
        13,
        wild_catch_rate | (wild_no_legendaries << 1) | (has_held_items << 3),
    )?;

    block.set(14, block.get(14) & 0x07);

    block.insert_int_field(19, 0)?;
    block.insert_int_field(23, i32::from(has_bw_patch))
}

fn before_162(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.insert_extra_byte(3, 0)
}

fn before_170(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.insert_extra_byte(17, 0)?;
    block.insert_extra_byte(21, 0)?;
    block.insert_extra_byte(22, 1)?;

    let mut old_tweaks = block.read_i32_be(27);
    if block.get(0) & 1 != 0 {
        old_tweaks |= MiscTweak::LowerCasePokemonNames.value();
    }
    if block.get(0) & (1 << 1) != 0 {
        old_tweaks |= MiscTweak::NationalDexAtStart.value();
    }
    if block.get(0) & (1 << 5) != 0 {
        old_tweaks |= MiscTweak::UpdateTypeEffectiveness.value();
    }
    if block.get(2) & (1 << 5) != 0 {
        old_tweaks |= MiscTweak::ForceChallengeMode.value();
    }
    block.write_i32_be(27, old_tweaks);

    block.set(0, remapped_byte(block.get(0), &[2, 3, 4, 6, 7]));
    block.set(2, remapped_byte(block.get(2), &[0, 1, 2, 4, 6, 7]));
    Ok(())
}

fn before_171(block: &mut DataBlock) -> Result<(), SettingsError> {
    if block.get(1) & 1 != 0 {
        block.or(1, 1 << 1);
    }
    if block.get(3) & 1 != 0 {
        block.or(0, 1 << 5);
    }
    block.set(3, (block.get(1) & 0x70) >> 4);
    block.set(1, (block.get(1) & 0x0F) | ((block.get(1) & 0x80) >> 3));

    block.insert_extra_byte(13, 30)?;
    block.insert_extra_byte(12, 0)?;
    block.insert_extra_byte(20, 0)?;
    block.insert_extra_byte(22, 0)
}

fn before_172(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.length -= 8;
    block.set(16, block.get(16) ^ (1 << 1));
    block.insert_extra_byte(35, 50)
}

fn before_300(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.insert_extra_byte(38, 50)?;
    block.insert_extra_byte(39, 1)
}

fn before_311(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.insert_extra_byte(40, 0)?;
    block.insert_extra_byte(41, 8)?;
    block.insert_extra_byte(42, 9)?;
    block.insert_extra_byte(43, 50)?;
    block.insert_extra_byte(44, 0)?;
    block.insert_extra_byte(45, 0)
}

fn before_314(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.insert_extra_byte(46, 0)?;
    block.insert_extra_byte(47, 50)
}

fn before_315(block: &mut DataBlock) -> Result<(), SettingsError> {
    let old_tweaks = block.read_i32_be(32) & !MiscTweak::ForceChallengeMode.value();
    block.write_i32_be(32, old_tweaks);
    block.insert_extra_byte(48, 0)
}

fn before_317(block: &mut DataBlock) -> Result<(), SettingsError> {
    block.insert_extra_byte(49, 0)?;
    let restrictions = block.read_i32_be(28) & 127;
    block.write_i32_be(28, restrictions);
    Ok(())
}

fn before_319(block: &mut DataBlock) -> Result<(), SettingsError> {
    for offset in [5, 7, 9] {
        let starter = block.read_u16_le(offset) + 1;
        block.write_u16_le(offset, starter);
    }
    block.insert_extra_byte(50, 0)
}

fn before_321(block: &mut DataBlock) -> Result<(), SettingsError> {
    let old_minimum_catch_rate = i32::from((block.get(16) & 0x60) >> 5) + 1;
    block.set(16, block.get(16) & !0x60);
    block.or(50, ((old_minimum_catch_rate - 1) << 3) as u8);
    Ok(())
}
