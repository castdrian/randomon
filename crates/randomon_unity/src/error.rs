#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnityError {
    #[error("unexpected end of data at offset {offset} (needed {needed} bytes)")]
    UnexpectedEof { offset: usize, needed: usize },
    #[error("unsupported bundle signature {0:?}")]
    UnsupportedSignature(String),
    #[error("unsupported UnityFS format version {0}")]
    UnsupportedBundleVersion(u32),
    #[error("unsupported serialized file version {0}")]
    UnsupportedSerializedVersion(u32),
    #[error("unsupported compression type {0}")]
    UnsupportedCompression(u32),
    #[error("decompression failed: {0}")]
    Decompression(String),
    #[error("invalid bundle layout: {0}")]
    InvalidBundle(String),
    #[error("invalid type tree: {0}")]
    InvalidTypeTree(String),
    #[error("object {0} has no type tree")]
    MissingTypeTree(i64),
    #[error("object {0} not found")]
    ObjectNotFound(i64),
    #[error("object read consumed {read} bytes but the object is {expected} bytes")]
    SizeMismatch { read: usize, expected: usize },
    #[error("value does not match the type tree: {0}")]
    ValueMismatch(String),
    #[error("field {0:?} not found")]
    FieldNotFound(String),
    #[error("index {index} out of range for length {length}")]
    IndexOutOfRange { index: usize, length: usize },
    #[error("value {value} does not fit in {kind}")]
    OutOfRange { value: String, kind: &'static str },
    #[error("string is not valid UTF-8 at offset {0}")]
    InvalidString(usize),
}

pub type Result<T> = std::result::Result<T, UnityError>;
