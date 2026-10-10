use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RandomizationError {
    #[error("{0}")]
    Randomization(String),
    #[error("bound must be positive")]
    EmptyChoice,
    #[error("java.lang.NullPointerException: {0}")]
    NullPointer(&'static str),
    #[error("java.lang.IndexOutOfBoundsException: {0}")]
    IndexOutOfBounds(&'static str),
    #[error("unsupported ROM: {0}")]
    UnsupportedRom(String),
    #[error("{0}")]
    Io(String),
}

impl RandomizationError {
    pub fn randomization(message: impl Into<String>) -> Self {
        Self::Randomization(message.into())
    }
}

pub type Result<T> = std::result::Result<T, RandomizationError>;
