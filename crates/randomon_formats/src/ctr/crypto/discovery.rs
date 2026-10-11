use std::path::{Path, PathBuf};
use std::sync::RwLock;

use super::keys::KeyStore;

pub const AES_KEYS_ENV: &str = "RANDOMON_AES_KEYS";
pub const AES_KEYS_FILE: &str = "aes_keys.txt";
pub const BOOT9_FILE: &str = "boot9.bin";

const EMULATOR_DIRECTORIES: [&str; 16] = [
    "Azahar",
    "azahar-emu",
    "azahar",
    "Lime3DS",
    "lime3ds-emu",
    "lime3ds",
    "Citra",
    "citra-emu",
    "citra",
    "Mandarine",
    "mandarine-emu",
    "mandarine",
    "Citra-PabloMK7",
    "citra-pablomk7",
    "PabloMK7-Citra",
    "Citra Enhanced",
];

const FLATPAK_DATA_DIRECTORIES: [(&str, &str); 4] = [
    ("org.azahar_emu.Azahar", "azahar-emu"),
    ("io.github.lime3ds.Lime3DS", "lime3ds-emu"),
    ("org.citra_emu.citra", "citra-emu"),
    ("io.github.mandarine3ds.mandarine", "mandarine-emu"),
];

static OVERRIDE_PATH: RwLock<Option<PathBuf>> = RwLock::new(None);

pub fn set_aes_keys_path(path: Option<PathBuf>) {
    if let Ok(mut guard) = OVERRIDE_PATH.write() {
        *guard = path;
    }
}

pub fn explicit_key_path() -> Option<PathBuf> {
    let configured = OVERRIDE_PATH.read().ok().and_then(|guard| guard.clone());
    configured.or_else(|| {
        std::env::var_os(AES_KEYS_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    })
}

fn environment_directory(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn emulator_base_directories() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    let home = environment_directory("HOME").or_else(|| environment_directory("USERPROFILE"));
    if let Some(app_data) = environment_directory("APPDATA") {
        bases.push(app_data);
    }
    if let Some(local_app_data) = environment_directory("LOCALAPPDATA") {
        bases.push(local_app_data);
    }
    if let Some(data_home) = environment_directory("XDG_DATA_HOME") {
        bases.push(data_home);
    }
    if let Some(config_home) = environment_directory("XDG_CONFIG_HOME") {
        bases.push(config_home);
    }
    if let Some(home) = &home {
        bases.push(home.join("Library").join("Application Support"));
        bases.push(home.join(".local").join("share"));
        bases.push(home.join(".config"));
        bases.push(home.join("AppData").join("Roaming"));
    }
    bases
}

fn sysdata_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    for base in emulator_base_directories() {
        for name in EMULATOR_DIRECTORIES {
            directories.push(base.join(name).join("sysdata"));
        }
    }
    if let Some(home) = environment_directory("HOME") {
        for (application, data) in FLATPAK_DATA_DIRECTORIES {
            directories.push(
                home.join(".var")
                    .join("app")
                    .join(application)
                    .join("data")
                    .join(data)
                    .join("sysdata"),
            );
        }
    }
    let portable_roots = [
        std::env::current_exe()
            .ok()
            .and_then(|executable| executable.parent().map(Path::to_path_buf)),
        std::env::current_dir().ok(),
    ];
    for root in portable_roots.into_iter().flatten() {
        directories.push(root.join("user").join("sysdata"));
        directories.push(root.join("sysdata"));
    }
    directories
}

pub fn candidate_key_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for directory in sysdata_directories() {
        for name in [AES_KEYS_FILE, BOOT9_FILE] {
            let candidate = directory.join(name);
            if !files.contains(&candidate) {
                files.push(candidate);
            }
        }
    }
    files
}

fn load_explicit(path: &Path) -> KeyStore {
    let mut store = KeyStore::default();
    if path.is_dir() {
        store.merge_file(&path.join(AES_KEYS_FILE));
        store.merge_file(&path.join(BOOT9_FILE));
    } else {
        store.merge_file(path);
        if let Some(sibling) = path.parent().map(|parent| parent.join(BOOT9_FILE))
            && sibling != path
        {
            store.merge_file(&sibling);
        }
    }
    store
}

pub fn discover_keys() -> KeyStore {
    if let Some(path) = explicit_key_path() {
        return load_explicit(&path);
    }
    let mut store = KeyStore::default();
    for candidate in candidate_key_files() {
        if candidate.is_file() {
            store.merge_file(&candidate);
        }
    }
    store
}
