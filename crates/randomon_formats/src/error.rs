use std::io;

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("{context}: out of bounds access")]
    OutOfBounds { context: &'static str },
    #[error("{context}: {reason}")]
    Invalid {
        context: &'static str,
        reason: String,
    },
    #[error("{context}: upstream would never terminate on this input")]
    UpstreamHang { context: &'static str },
    #[error("{context}: upstream would exit the process ({message})")]
    UpstreamExit {
        context: &'static str,
        message: &'static str,
    },
    #[error("the ROM appears to be encrypted: {0}")]
    EncryptedRom(Box<FormatError>),
    #[error("{0}")]
    Crypto(#[from] crate::ctr::crypto::CryptoError),
}

impl FormatError {
    pub fn invalid(context: &'static str, reason: impl Into<String>) -> Self {
        FormatError::Invalid {
            context,
            reason: reason.into(),
        }
    }

    pub fn out_of_bounds(context: &'static str) -> Self {
        FormatError::OutOfBounds { context }
    }
}

pub type Result<T> = std::result::Result<T, FormatError>;
