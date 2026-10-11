use std::path::{Path, PathBuf};

use randomon_formats::ctr::crypto::{self, AES_KEYS_ENV, KeyStore};

use crate::config::{ConfigFile, environment_directory};

const AES_KEYS_ENTRY: &str = "aes_keys";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeysRetry {
    Game(PathBuf),
    Update(PathBuf),
}

pub fn needs_keys(message: &str) -> bool {
    crypto::is_key_problem_message(message)
}

fn remembered_keys_path() -> Option<PathBuf> {
    ConfigFile::standard()?
        .read(AES_KEYS_ENTRY)
        .map(PathBuf::from)
}

fn remember(keys: &Path) -> Result<(), String> {
    let file = ConfigFile::standard().ok_or("randomon couldn't find a folder for its settings.")?;
    file.write(AES_KEYS_ENTRY, &keys.display().to_string())
        .map_err(|error| format!("Couldn't save the keys location: {error}"))
}

pub fn apply_remembered() {
    if environment_directory(AES_KEYS_ENV).is_some() {
        return;
    }
    if let Some(path) = remembered_keys_path().filter(|path| path.is_file()) {
        crypto::set_aes_keys_path(Some(path));
    }
}

pub fn use_keys_file(path: PathBuf) -> Result<(), String> {
    let mut store = KeyStore::default();
    if !store.merge_file(&path) || store.is_empty() {
        return Err(format!(
            "\u{201C}{}\u{201D} doesn't contain any 3DS keys. Choose an emulator's aes_keys.txt or a boot9.bin.",
            path.file_name().map_or_else(
                || path.display().to_string(),
                |name| name.to_string_lossy().into_owned()
            )
        ));
    }
    crypto::set_aes_keys_path(Some(path.clone()));
    remember(&path)
}

pub async fn pick_keys_file() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Choose aes_keys.txt or boot9.bin")
        .add_filter("3DS keys", &["txt", "bin"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_key_problems_offer_a_keys_file() {
        assert!(needs_keys(
            "The ROM \u{201C}X.3ds\u{201D} is encrypted and no 3DS keys were found. Point randomon to an emulator's aes_keys.txt or use a decrypted ROM."
        ));
        assert!(!needs_keys("That folder isn't a ROM."));
    }

    #[test]
    fn rejects_files_without_keys() {
        let empty =
            std::env::temp_dir().join(format!("randomon-empty-keys-{}.txt", std::process::id()));
        std::fs::write(&empty, "# nothing here\n").unwrap();
        assert!(use_keys_file(empty.clone()).is_err());
        let _ = std::fs::remove_file(&empty);
    }
}
