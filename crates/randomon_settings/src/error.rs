use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SettingsError {
    #[error("invalid base64 settings data: {0}")]
    InvalidBase64(&'static str),
    #[error("settings data is too short: {length} bytes")]
    TruncatedSettings { length: usize },
    #[error("Malformed input string")]
    ChecksumMismatch,
    #[error("Only one value for {0} may be chosen!")]
    ConflictingEnumBits(&'static str),
    #[error("settings have no ROM name")]
    MissingRomName,
    #[error("settings have no selected EXP curve")]
    MissingExpCurve,
    #[error("the settings string is empty")]
    EmptySettingsString,
    #[error("the settings string has no valid version prefix")]
    InvalidVersionPrefix,
    #[error("the settings string is from a newer randomizer version ({version})")]
    SettingsStringTooNew { version: i32 },
    #[error("old settings data could not be updated")]
    UpdaterOutOfBounds,
    #[error("Error reading version number from settings string.")]
    RnqsMissingVersion,
    #[error("The settings file is too old to update and cannot be loaded.")]
    RnqsTooOld,
    #[error("Cannot read settings from a newer version of the randomizer.")]
    RnqsTooNew,
    #[error("Error reading settings length from settings string.")]
    RnqsMissingLength,
    #[error("negative length {0}")]
    NegativeLength(i32),
    #[error("unexpected end of data")]
    UnexpectedEof,
    #[error("malformed modified UTF-8 input")]
    MalformedModifiedUtf8,
    #[error("encoded string too long: {0} bytes")]
    ModifiedUtf8TooLong(usize),
    #[error("string contains an unpaired surrogate")]
    UnpairedSurrogate,
    #[error("Invalid custom names file provided.")]
    InvalidCustomNamesVersion,
    #[error("Invalid size specified.")]
    InvalidCustomNamesSize,
    #[error("preset is from randomizer version {version}, this is version {expected}")]
    PresetVersionMismatch {
        version: i32,
        expected: i32,
        required_release: Option<&'static str>,
    },
    #[error("The preset config is too short to be valid")]
    PresetConfigTooShort,
    #[error("Checksum failure.")]
    PresetChecksumMismatch,
    #[error("the preset config has an invalid length")]
    PresetConfigInvalidLength,
}
