use crate::codec;
use crate::custom_names::default_custom_names_checksum;
use crate::error::SettingsError;
use crate::java_base64;
use crate::java_data::DataReader;
use crate::java_text::{decode_utf8, java_trim, parse_java_int};
use crate::settings::Settings;
use crate::updater;
use crate::version::VERSION;

const VERSION_PREFIX_LENGTH: usize = 3;
const OLDEST_UPDATABLE_HEADER_VERSION: i32 = 172;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionedSettings {
    pub version: i32,
    pub settings: Settings,
}

impl Settings {
    pub fn to_settings_string(&self) -> Result<String, SettingsError> {
        self.to_settings_string_with_checksum(default_custom_names_checksum())
    }

    pub fn to_settings_string_with_checksum(
        &self,
        custom_names_checksum: i32,
    ) -> Result<String, SettingsError> {
        Ok(java_base64::encode(&codec::encode(
            self,
            custom_names_checksum,
        )?))
    }

    pub fn to_versioned_settings_string(&self) -> Result<String, SettingsError> {
        self.to_versioned_settings_string_with_checksum(default_custom_names_checksum())
    }

    pub fn to_versioned_settings_string_with_checksum(
        &self,
        custom_names_checksum: i32,
    ) -> Result<String, SettingsError> {
        Ok(format!(
            "{VERSION}{}",
            self.to_settings_string_with_checksum(custom_names_checksum)?
        ))
    }

    pub fn from_settings_string(settings_string: &str) -> Result<Self, SettingsError> {
        codec::decode(&java_base64::decode(settings_string)?)
    }

    pub fn from_versioned_settings_string(text: &str) -> Result<VersionedSettings, SettingsError> {
        let config_string = java_trim(text);
        if config_string.is_empty() {
            return Err(SettingsError::EmptySettingsString);
        }
        let units: Vec<u16> = config_string.encode_utf16().collect();
        if units.len() < VERSION_PREFIX_LENGTH {
            return Err(SettingsError::InvalidVersionPrefix);
        }
        let version = parse_java_int(&units[..VERSION_PREFIX_LENGTH])
            .ok_or(SettingsError::InvalidVersionPrefix)?;
        let body = String::from_utf16(&units[VERSION_PREFIX_LENGTH..])
            .map_err(|_| SettingsError::InvalidVersionPrefix)?;
        let settings = match version.cmp(&VERSION) {
            std::cmp::Ordering::Less => {
                Self::from_settings_string(&updater::update(version, &body)?)?
            }
            std::cmp::Ordering::Greater => {
                return Err(SettingsError::SettingsStringTooNew { version });
            }
            std::cmp::Ordering::Equal => Self::from_settings_string(&body)?,
        };
        Ok(VersionedSettings { version, settings })
    }

    pub fn write_rnqs(&self) -> Result<Vec<u8>, SettingsError> {
        self.write_rnqs_with_checksum(default_custom_names_checksum())
    }

    pub fn write_rnqs_with_checksum(
        &self,
        custom_names_checksum: i32,
    ) -> Result<Vec<u8>, SettingsError> {
        let settings_string = self.to_settings_string_with_checksum(custom_names_checksum)?;
        let mut out = Vec::with_capacity(settings_string.len() + 8);
        out.extend_from_slice(&VERSION.to_be_bytes());
        out.extend_from_slice(&(settings_string.len() as i32).to_be_bytes());
        out.extend_from_slice(settings_string.as_bytes());
        Ok(out)
    }

    pub fn read_rnqs(bytes: &[u8]) -> Result<Self, SettingsError> {
        let mut reader = DataReader::new(bytes);
        let version = reader
            .read_i32()
            .map_err(|_| SettingsError::RnqsMissingVersion)?;
        let header_version = (version >> 24) & 0xFF;
        if header_version > 0 && header_version <= OLDEST_UPDATABLE_HEADER_VERSION {
            return Err(SettingsError::RnqsTooOld);
        }
        if version > VERSION {
            return Err(SettingsError::RnqsTooNew);
        }
        let length = reader
            .read_i32()
            .map_err(|_| SettingsError::RnqsMissingLength)?;
        let length = usize::try_from(length).map_err(|_| SettingsError::NegativeLength(length))?;
        if length > reader.remaining().len() {
            return Err(SettingsError::InvalidBase64(
                "settings string is padded with zero bytes",
            ));
        }
        let settings_string = decode_utf8(reader.read_exact(length)?);
        let mut settings = if version < VERSION {
            Self::from_settings_string(&updater::update(version, &settings_string)?)?
        } else {
            Self::from_settings_string(&settings_string)?
        };
        settings.updated_from_old_version = version < VERSION;
        Ok(settings)
    }
}

pub fn update_settings_string(
    old_version: i32,
    settings_string: &str,
) -> Result<String, SettingsError> {
    updater::update(old_version, settings_string)
}
