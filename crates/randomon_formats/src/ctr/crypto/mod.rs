pub mod aes;
mod cia;
pub mod discovery;
pub mod keys;
mod ncch;
pub mod source;

use std::borrow::Cow;
use std::io::{Read, Seek};

pub use aes::Aes128;
pub use discovery::{AES_KEYS_ENV, discover_keys, set_aes_keys_path};
pub use keys::{KeyStore, scramble_normal_key};
pub use ncch::{FIXED_KEY, NO_CRYPTO, SEED_CRYPTO, secondary_slot};
pub use source::CryptoSource;

use super::container::{NCCH_MAGIC, cxi_offset};
use crate::error::Result;
use crate::io::read_i32_be_at;

const KEYS_REMEDY: &str = "Point randomon to an emulator's aes_keys.txt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    MissingKeys,
    MissingKeySlot(u8),
    WrongKeys,
    SeedCrypto,
    UnknownCryptoMethod(u8),
    UnsupportedNcchVersion(u16),
    MissingCommonKeys,
    WrongCommonKeys,
}

impl CryptoError {
    pub fn message_for(&self, subject: &str, noun: &str) -> String {
        match self {
            CryptoError::MissingKeys => format!(
                "{subject} is encrypted and no 3DS keys were found. {KEYS_REMEDY} or use a decrypted {noun}."
            ),
            CryptoError::MissingKeySlot(slot) => format!(
                "{subject} is encrypted with keyslot 0x{slot:02X}, but the 3DS keys found have no slot0x{slot:02X}KeyX. Point randomon to a complete aes_keys.txt or use a decrypted {noun}."
            ),
            CryptoError::WrongKeys => format!(
                "{subject} is encrypted and the 3DS keys found did not decrypt it. Check your aes_keys.txt or use a decrypted {noun}."
            ),
            CryptoError::SeedCrypto => format!(
                "{subject} uses seed encryption, which randomon cannot remove. Use a decrypted {noun}, for example one made with GodMode9."
            ),
            CryptoError::UnknownCryptoMethod(method) => format!(
                "{subject} uses an unknown encryption method (0x{method:02X}). Use a decrypted {noun}."
            ),
            CryptoError::UnsupportedNcchVersion(version) => format!(
                "{subject} has an NCCH version ({version}) randomon cannot decrypt. Use a decrypted {noun}."
            ),
            CryptoError::MissingCommonKeys => format!(
                "{subject} is encrypted with a title key and no 3DS common keys were found. {KEYS_REMEDY} (with slot0x3DKeyX and common0-5) or use a decrypted {noun}."
            ),
            CryptoError::WrongCommonKeys => format!(
                "{subject} is encrypted with a title key and the 3DS common keys found did not decrypt it. Check your aes_keys.txt or use a decrypted {noun}."
            ),
        }
    }
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message_for("This ROM", "ROM"))
    }
}

impl std::error::Error for CryptoError {}

#[derive(Clone, Copy, Debug, Default)]
pub enum KeyLookup<'a> {
    #[default]
    Discover,
    Using(&'a KeyStore),
}

struct LazyKeys<'a> {
    lookup: KeyLookup<'a>,
    loaded: Option<Cow<'a, KeyStore>>,
}

impl<'a> LazyKeys<'a> {
    fn new(lookup: KeyLookup<'a>) -> Self {
        LazyKeys {
            lookup,
            loaded: None,
        }
    }

    fn get(&mut self) -> &KeyStore {
        let lookup = self.lookup;
        self.loaded.get_or_insert_with(|| match lookup {
            KeyLookup::Discover => Cow::Owned(discover_keys()),
            KeyLookup::Using(store) => Cow::Borrowed(store),
        })
    }
}

pub fn prepare<R: Read + Seek>(source: &mut CryptoSource<R>, lookup: KeyLookup<'_>) -> Result<()> {
    let mut keys = LazyKeys::new(lookup);
    if let Ok(Some(content)) = cia::encrypted_first_content(source) {
        let layer = cia::cia_layer(source, &content, keys.get())?;
        source.push_layer(layer);
        let magic = read_i32_be_at(source, content.start + 0x100).ok();
        if magic != Some(NCCH_MAGIC) {
            return Err(CryptoError::WrongCommonKeys.into());
        }
    }
    let Ok(Some(offset)) = cxi_offset(source) else {
        return Ok(());
    };
    let Ok((header, geometry)) = ncch::read_geometry(source, offset) else {
        return Ok(());
    };
    if ncch::is_marked_plaintext(&header)
        || ncch::has_plaintext_evidence(source, &geometry).unwrap_or(true)
    {
        return Ok(());
    }
    let layer = ncch::ncch_layer(source, &header, &geometry, keys.get())?;
    source.push_layer(layer);
    if !ncch::looks_decrypted(source, &geometry).unwrap_or(false) {
        return Err(CryptoError::WrongKeys.into());
    }
    Ok(())
}
