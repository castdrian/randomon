use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SwshError {
    #[error("required file not found: {}", .0.display())]
    MissingFile(PathBuf),
    #[error("{path}: {message}", path = .0.display(), message = .1)]
    Io(PathBuf, String),
    #[error("{context}: unexpected end of data at offset {offset} (needed {needed} bytes)")]
    UnexpectedEof {
        context: &'static str,
        offset: usize,
        needed: usize,
    },
    #[error("{context}: {reason}")]
    Invalid {
        context: &'static str,
        reason: String,
    },
    #[error("unsupported pack compression type {0}")]
    UnsupportedCompression(u8),
    #[error("pack entry not found: {0}")]
    PackEntryNotFound(String),
    #[error("{path}: {source}", path = .path.display())]
    InFile {
        path: PathBuf,
        source: Box<SwshError>,
    },
}

pub type Result<T> = std::result::Result<T, SwshError>;

pub fn invalid(context: &'static str, reason: impl Into<String>) -> SwshError {
    SwshError::Invalid {
        context,
        reason: reason.into(),
    }
}

impl SwshError {
    pub fn in_file(self, path: impl Into<PathBuf>) -> SwshError {
        SwshError::InFile {
            path: path.into(),
            source: Box::new(self),
        }
    }
}
