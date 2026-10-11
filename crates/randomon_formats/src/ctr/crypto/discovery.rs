use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{Duration, Instant};

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
    if store.is_empty() {
        for root in cloud_drive_roots() {
            merge_cloud_drive(&mut store, &root);
        }
    }
    store
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CloudKeyFiles {
    pub available: Vec<PathBuf>,
    pub evicted: Vec<PathBuf>,
}

const CLOUD_SEARCH_DEPTH: usize = 4;
const CLOUD_SEARCH_ENTRY_BUDGET: usize = 20_000;
const CLOUD_DOWNLOAD_WAIT: Duration = Duration::from_secs(3);
const CLOUD_DOWNLOAD_POLL: Duration = Duration::from_millis(100);
const EVICTED_SUFFIX: &str = ".icloud";
const SKIPPED_CLOUD_DIRECTORIES: [&str; 10] = [
    "Photos",
    "Pictures",
    "Movies",
    "Music",
    "Videos",
    "Backups",
    "Downloads",
    "node_modules",
    "target",
    "Library",
];
const SKIPPED_CLOUD_EXTENSIONS: [&str; 6] = [
    "app",
    "photoslibrary",
    "musiclibrary",
    "fcpbundle",
    "imovielibrary",
    "bundle",
];

fn key_file_name(name: &str) -> bool {
    name == AES_KEYS_FILE || name == BOOT9_FILE
}

fn evicted_key_file(name: &str) -> Option<&str> {
    name.strip_prefix('.')?
        .strip_suffix(EVICTED_SUFFIX)
        .filter(|original| key_file_name(original))
}

fn skips_cloud_directory(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    let skipped_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            SKIPPED_CLOUD_EXTENSIONS
                .iter()
                .any(|skipped| extension.eq_ignore_ascii_case(skipped))
        });
    name.starts_with('.') || SKIPPED_CLOUD_DIRECTORIES.contains(&name) || skipped_extension
}

pub fn find_cloud_key_files(root: &Path, max_depth: usize) -> CloudKeyFiles {
    let mut found = CloudKeyFiles::default();
    let mut budget = CLOUD_SEARCH_ENTRY_BUDGET;
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    while let Some((directory, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if budget == 0 {
                return found;
            }
            budget -= 1;
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if file_type.is_dir() {
                if depth < max_depth && !skips_cloud_directory(&path) {
                    pending.push((path, depth + 1));
                }
            } else if key_file_name(&name) {
                found.available.push(path);
            } else if let Some(original) = evicted_key_file(&name) {
                found.evicted.push(directory.join(original));
            }
        }
    }
    found.available.sort();
    found.evicted.sort();
    found
}

fn cloud_drive_roots() -> Vec<PathBuf> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    environment_directory("HOME")
        .map(|home| {
            home.join("Library")
                .join("Mobile Documents")
                .join("com~apple~CloudDocs")
        })
        .into_iter()
        .filter(|root| root.is_dir())
        .collect()
}

fn request_cloud_download(path: &Path) -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    let requested = std::process::Command::new("brctl")
        .arg("download")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !requested {
        return false;
    }
    let started = Instant::now();
    while started.elapsed() < CLOUD_DOWNLOAD_WAIT {
        if path.is_file() {
            return true;
        }
        std::thread::sleep(CLOUD_DOWNLOAD_POLL);
    }
    path.is_file()
}

fn merge_cloud_drive(store: &mut KeyStore, root: &Path) {
    let files = find_cloud_key_files(root, CLOUD_SEARCH_DEPTH);
    for path in &files.available {
        store.merge_file(path);
    }
    for path in files.evicted {
        if request_cloud_download(&path) {
            store.merge_file(&path);
        } else {
            store.add_pending_download(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "randomon-cloud-search-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn finds_nested_and_evicted_key_files_within_bounds() {
        let root = scratch_directory("bounds");
        let nested = root.join("Emulation").join("3DS").join("sysdata");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join(AES_KEYS_FILE), "").unwrap();
        let evicted = root.join("Backup Keys");
        std::fs::create_dir_all(&evicted).unwrap();
        std::fs::write(evicted.join(".boot9.bin.icloud"), "").unwrap();
        let photos = root.join("Photos");
        std::fs::create_dir_all(&photos).unwrap();
        std::fs::write(photos.join(AES_KEYS_FILE), "").unwrap();
        let too_deep = root.join("a").join("b").join("c").join("d").join("e");
        std::fs::create_dir_all(&too_deep).unwrap();
        std::fs::write(too_deep.join(AES_KEYS_FILE), "").unwrap();
        let found = find_cloud_key_files(&root, CLOUD_SEARCH_DEPTH);
        assert_eq!(found.available, vec![nested.join(AES_KEYS_FILE)]);
        assert_eq!(found.evicted, vec![evicted.join(BOOT9_FILE)]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn evicted_names_map_back_to_key_files() {
        assert_eq!(
            evicted_key_file(".aes_keys.txt.icloud"),
            Some(AES_KEYS_FILE)
        );
        assert_eq!(evicted_key_file(".boot9.bin.icloud"), Some(BOOT9_FILE));
        assert_eq!(evicted_key_file(".notes.txt.icloud"), None);
        assert_eq!(evicted_key_file("aes_keys.txt"), None);
    }
}
