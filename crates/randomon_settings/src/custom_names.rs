use crate::error::SettingsError;
use crate::java_data::DataReader;
use crate::java_text::{decode_utf8, java_trim, scanner_lines, scanner_names};

pub const DEFAULT_CUSTOM_NAMES_FILE: &[u8] = include_bytes!("../assets/customnames.rncn");

const CUSTOM_NAMES_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineSeparator {
    Lf,
    CrLf,
}

impl LineSeparator {
    pub fn platform() -> Self {
        if cfg!(windows) { Self::CrLf } else { Self::Lf }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomNames {
    pub trainer_names: Vec<String>,
    pub trainer_classes: Vec<String>,
    pub doubles_trainer_names: Vec<String>,
    pub doubles_trainer_classes: Vec<String>,
    pub pokemon_nicknames: Vec<String>,
}

impl CustomNames {
    pub fn default_names() -> Self {
        Self::from_bytes(DEFAULT_CUSTOM_NAMES_FILE)
            .unwrap_or_else(|error| unreachable!("bundled custom names are valid: {error}"))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SettingsError> {
        let mut reader = DataReader::new(bytes);
        let version = reader
            .read_exact(1)
            .map_err(|_| SettingsError::InvalidCustomNamesVersion)?;
        if version[0] != CUSTOM_NAMES_VERSION {
            return Err(SettingsError::InvalidCustomNamesVersion);
        }
        Ok(Self {
            trainer_names: read_names_block(&mut reader)?,
            trainer_classes: read_names_block(&mut reader)?,
            doubles_trainer_names: read_names_block(&mut reader)?,
            doubles_trainer_classes: read_names_block(&mut reader)?,
            pokemon_nicknames: read_names_block(&mut reader)?,
        })
    }

    pub fn to_bytes(&self, separator: LineSeparator) -> Vec<u8> {
        let mut out = vec![CUSTOM_NAMES_VERSION];
        for names in self.blocks() {
            let joined = names.join(separator.as_str()).into_bytes();
            out.extend_from_slice(&(joined.len() as i32).to_be_bytes());
            out.extend_from_slice(&joined);
        }
        out
    }

    fn blocks(&self) -> [&Vec<String>; 5] {
        [
            &self.trainer_names,
            &self.trainer_classes,
            &self.doubles_trainer_names,
            &self.doubles_trainer_classes,
            &self.pokemon_nicknames,
        ]
    }
}

fn read_names_block(reader: &mut DataReader<'_>) -> Result<Vec<String>, SettingsError> {
    let size_bytes = reader.read_zero_padded(4);
    let size = i32::from_be_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]]);
    let available = reader.remaining().len();
    if i64::from(size) > available as i64 {
        return Err(SettingsError::InvalidCustomNamesSize);
    }
    let size = usize::try_from(size).map_err(|_| SettingsError::NegativeLength(size))?;
    Ok(scanner_names(reader.read_exact(size)?))
}

pub fn custom_names_checksum(file_bytes: &[u8]) -> i32 {
    let text = decode_utf8(file_bytes);
    let mut hasher = crc32fast::Hasher::new();
    for line in scanner_lines(&text) {
        let trimmed = java_trim(line);
        if !trimmed.is_empty() {
            hasher.update(trimmed.as_bytes());
        }
    }
    hasher.finalize() as i32
}

pub fn default_custom_names_checksum() -> i32 {
    custom_names_checksum(DEFAULT_CUSTOM_NAMES_FILE)
}
