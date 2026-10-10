use crate::error::SettingsError;

const MAX_MODIFIED_UTF8_LENGTH: usize = 65535;

pub fn modified_utf8_from_units(units: &[u16]) -> Result<Vec<u8>, SettingsError> {
    let encoded_length: usize = units.iter().map(|&unit| encoded_unit_length(unit)).sum();
    if encoded_length > MAX_MODIFIED_UTF8_LENGTH {
        return Err(SettingsError::ModifiedUtf8TooLong(encoded_length));
    }
    let mut out = Vec::with_capacity(encoded_length + 2);
    out.extend_from_slice(&(encoded_length as u16).to_be_bytes());
    for &unit in units {
        push_encoded_unit(&mut out, unit);
    }
    Ok(out)
}

pub fn modified_utf8_from_str(text: &str) -> Result<Vec<u8>, SettingsError> {
    let units: Vec<u16> = text.encode_utf16().collect();
    modified_utf8_from_units(&units)
}

fn encoded_unit_length(unit: u16) -> usize {
    match unit {
        0x0001..=0x007F => 1,
        0x0000 | 0x0080..=0x07FF => 2,
        _ => 3,
    }
}

fn push_encoded_unit(out: &mut Vec<u8>, unit: u16) {
    match encoded_unit_length(unit) {
        1 => out.push(unit as u8),
        2 => out.extend_from_slice(&[
            0xC0 | ((unit >> 6) & 0x1F) as u8,
            0x80 | (unit & 0x3F) as u8,
        ]),
        _ => out.extend_from_slice(&[
            0xE0 | ((unit >> 12) & 0x0F) as u8,
            0x80 | ((unit >> 6) & 0x3F) as u8,
            0x80 | (unit & 0x3F) as u8,
        ]),
    }
}

pub fn units_from_modified_utf8(bytes: &[u8]) -> Result<Vec<u16>, SettingsError> {
    let malformed = || SettingsError::MalformedModifiedUtf8;
    let mut units = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let lead = u16::from(bytes[index]);
        let width = match lead >> 4 {
            0..=7 => 1,
            12 | 13 => 2,
            14 => 3,
            _ => return Err(malformed()),
        };
        let sequence = bytes.get(index..index + width).ok_or_else(malformed)?;
        if sequence[1..].iter().any(|&byte| byte & 0xC0 != 0x80) {
            return Err(malformed());
        }
        let unit = match width {
            1 => lead,
            2 => ((lead & 0x1F) << 6) | u16::from(sequence[1] & 0x3F),
            _ => {
                ((lead & 0x0F) << 12)
                    | (u16::from(sequence[1] & 0x3F) << 6)
                    | u16::from(sequence[2] & 0x3F)
            }
        };
        units.push(unit);
        index += width;
    }
    Ok(units)
}

pub struct DataReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> DataReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.position..]
    }

    pub fn read_exact(&mut self, length: usize) -> Result<&'a [u8], SettingsError> {
        let end = self
            .position
            .checked_add(length)
            .filter(|&end| end <= self.bytes.len())
            .ok_or(SettingsError::UnexpectedEof)?;
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    pub fn read_i32(&mut self) -> Result<i32, SettingsError> {
        let bytes = self.read_exact(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_i64(&mut self) -> Result<i64, SettingsError> {
        let bytes = self.read_exact(8)?;
        let mut array = [0u8; 8];
        array.copy_from_slice(bytes);
        Ok(i64::from_be_bytes(array))
    }

    pub fn read_modified_utf8_units(&mut self) -> Result<Vec<u16>, SettingsError> {
        let length_bytes = self.read_exact(2)?;
        let length = usize::from(u16::from_be_bytes([length_bytes[0], length_bytes[1]]));
        units_from_modified_utf8(self.read_exact(length)?)
    }

    pub fn read_modified_utf8(&mut self) -> Result<String, SettingsError> {
        let units = self.read_modified_utf8_units()?;
        String::from_utf16(&units).map_err(|_| SettingsError::UnpairedSurrogate)
    }

    pub fn read_zero_padded(&mut self, length: usize) -> Vec<u8> {
        let available = self.remaining().len().min(length);
        let mut out = self.remaining()[..available].to_vec();
        self.position += available;
        out.resize(length, 0);
        out
    }
}
