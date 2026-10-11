use std::path::PathBuf;

use crate::assets::BundleId;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BdspError {
    #[error("required file not found: {}", .0.display())]
    MissingFile(PathBuf),
    #[error("{path}: {message}", path = .0.display(), message = .1)]
    Io(PathBuf, String),
    #[error("{0:?}: {1}")]
    Unity(BundleId, randomon_unity::UnityError),
    #[error("unexpected game data: {0}")]
    Data(String),
    #[error("no valid choice for {0}")]
    EmptyPool(String),
    #[error(transparent)]
    Settings(#[from] crate::settings::SettingsError),
}

impl From<randomon_python_random::PythonError> for BdspError {
    fn from(error: randomon_python_random::PythonError) -> Self {
        BdspError::Data(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, BdspError>;

pub fn data_error(message: impl Into<String>) -> BdspError {
    BdspError::Data(message.into())
}

impl From<randomon_unity::UnityError> for BdspError {
    fn from(error: randomon_unity::UnityError) -> Self {
        BdspError::Data(error.to_string())
    }
}
