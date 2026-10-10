use crate::codec::verify_checksum;
use crate::custom_names::{CustomNames, DEFAULT_CUSTOM_NAMES_FILE};
use crate::error::SettingsError;
use crate::java_base64;
use crate::java_data::{DataReader, modified_utf8_from_str};
use crate::java_text::decode_us_ascii;
use crate::settings::LENGTH_OF_SETTINGS_DATA;
use crate::version::{VERSION, release_name};

const MINIMUM_CONFIG_LENGTH: usize = LENGTH_OF_SETTINGS_DATA + 9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preset {
    pub version: i32,
    pub seed: i64,
    pub config_string: String,
    pub custom_names: Vec<u8>,
}

impl Preset {
    pub fn new(seed: i64, config_string: String, custom_names: Vec<u8>) -> Self {
        Self {
            version: VERSION,
            seed,
            config_string,
            custom_names,
        }
    }

    pub fn with_default_custom_names(seed: i64, config_string: String) -> Self {
        Self::new(seed, config_string, DEFAULT_CUSTOM_NAMES_FILE.to_vec())
    }

    pub fn read(bytes: &[u8]) -> Result<Self, SettingsError> {
        let mut reader = DataReader::new(bytes);
        let version = reader.read_i32()?;
        if version != VERSION {
            return Err(SettingsError::PresetVersionMismatch {
                version,
                expected: VERSION,
                required_release: (version < VERSION)
                    .then(|| release_name(version).unwrap_or("Unknown")),
            });
        }
        let seed = reader.read_i64()?;
        let config_string = reader.read_modified_utf8()?;
        let custom_names = reader.remaining().to_vec();
        CustomNames::from_bytes(&custom_names)?;
        Ok(Self {
            version,
            seed,
            config_string,
            custom_names,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, SettingsError> {
        let mut out = Vec::with_capacity(14 + self.config_string.len() + self.custom_names.len());
        out.extend_from_slice(&self.version.to_be_bytes());
        out.extend_from_slice(&self.seed.to_be_bytes());
        out.extend_from_slice(&modified_utf8_from_str(&self.config_string)?);
        out.extend_from_slice(&self.custom_names);
        Ok(out)
    }

    pub fn parsed_custom_names(&self) -> Result<CustomNames, SettingsError> {
        CustomNames::from_bytes(&self.custom_names)
    }

    pub fn versioned_config_string(&self) -> String {
        format!("{}{}", self.version, self.config_string)
    }

    pub fn required_rom_name(&self) -> Result<String, SettingsError> {
        let data = java_base64::decode(&self.config_string)?;
        if data.len() < MINIMUM_CONFIG_LENGTH {
            return Err(SettingsError::PresetConfigTooShort);
        }
        verify_checksum(&data).map_err(|_| SettingsError::PresetChecksumMismatch)?;
        let name_length = usize::from(data[LENGTH_OF_SETTINGS_DATA]);
        if data.len() != MINIMUM_CONFIG_LENGTH + name_length {
            return Err(SettingsError::PresetConfigInvalidLength);
        }
        let start = LENGTH_OF_SETTINGS_DATA + 1;
        Ok(decode_us_ascii(&data[start..start + name_length]))
    }
}
