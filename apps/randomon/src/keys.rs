use std::path::{Path, PathBuf};

use randomon_formats::ctr::crypto::{self, AES_KEYS_ENV, KeyStore};

const CONFIG_DIRECTORY: &str = "randomon";
const CONFIG_FILE: &str = "config.txt";
const AES_KEYS_ENTRY: &str = "aes_keys";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeysRetry {
    Game(PathBuf),
    Update(PathBuf),
}

pub fn needs_keys(message: &str) -> bool {
    crypto::is_key_problem_message(message)
}

fn environment_directory(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn config_directory() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        environment_directory("APPDATA")
    } else if cfg!(target_os = "macos") {
        environment_directory("HOME").map(|home| home.join("Library").join("Application Support"))
    } else {
        environment_directory("XDG_CONFIG_HOME")
            .or_else(|| environment_directory("HOME").map(|home| home.join(".config")))
    };
    base.map(|base| base.join(CONFIG_DIRECTORY))
}

fn config_path() -> Option<PathBuf> {
    config_directory().map(|directory| directory.join(CONFIG_FILE))
}

fn parse_keys_entry(config: &str) -> Option<PathBuf> {
    config.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == AES_KEYS_ENTRY && !value.trim().is_empty())
            .then(|| PathBuf::from(value.trim()))
    })
}

fn with_keys_entry(config: &str, keys: &Path) -> String {
    let mut lines: Vec<String> = config
        .lines()
        .filter(|line| {
            line.split_once('=')
                .is_none_or(|(key, _)| key.trim() != AES_KEYS_ENTRY)
        })
        .map(str::to_owned)
        .collect();
    lines.push(format!("{AES_KEYS_ENTRY}={}", keys.display()));
    lines.join("\n") + "\n"
}

fn remembered_keys_path() -> Option<PathBuf> {
    let text = std::fs::read_to_string(config_path()?).ok()?;
    parse_keys_entry(&text)
}

fn remember(keys: &Path) -> Result<(), String> {
    let path = config_path().ok_or("randomon couldn't find a folder for its settings.")?;
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("Couldn't save the keys location: {error}"))?;
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    std::fs::write(&path, with_keys_entry(&existing, keys))
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
    fn keys_entry_round_trips_and_keeps_other_settings() {
        let keys = PathBuf::from("/keys/aes_keys.txt");
        let config = with_keys_entry("theme=dark\naes_keys=/old/aes_keys.txt\n", &keys);
        assert_eq!(config, "theme=dark\naes_keys=/keys/aes_keys.txt\n");
        assert_eq!(parse_keys_entry(&config), Some(keys));
        assert_eq!(parse_keys_entry("theme=dark\n"), None);
    }

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
