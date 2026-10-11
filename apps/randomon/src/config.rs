use std::path::PathBuf;

const CONFIG_DIRECTORY: &str = "randomon";
const CONFIG_FILE: &str = "config.txt";

pub fn environment_directory(name: &str) -> Option<PathBuf> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFile {
    path: PathBuf,
}

impl ConfigFile {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn standard() -> Option<Self> {
        config_directory().map(|directory| Self::at(directory.join(CONFIG_FILE)))
    }

    pub fn read(&self, name: &str) -> Option<String> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        parse_entry(&text, name)
    }

    pub fn write(&self, name: &str, value: &str) -> std::io::Result<()> {
        if let Some(directory) = self.path.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let existing = std::fs::read_to_string(&self.path).unwrap_or_default();
        std::fs::write(&self.path, with_entry(&existing, name, value))
    }
}

fn is_entry(line: &str, name: &str) -> bool {
    line.split_once('=')
        .is_some_and(|(key, _)| key.trim() == name)
}

fn parse_entry(config: &str, name: &str) -> Option<String> {
    config.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == name && !value.trim().is_empty()).then(|| value.trim().to_owned())
    })
}

fn with_entry(config: &str, name: &str, value: &str) -> String {
    let mut lines: Vec<String> = config
        .lines()
        .filter(|line| !is_entry(line, name))
        .map(str::to_owned)
        .collect();
    lines.push(format!("{name}={value}"));
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_round_trip_and_keep_other_settings() {
        let config = with_entry(
            "theme=dark\naes_keys=/old/aes_keys.txt\n",
            "aes_keys",
            "/keys/aes_keys.txt",
        );
        assert_eq!(config, "theme=dark\naes_keys=/keys/aes_keys.txt\n");
        assert_eq!(
            parse_entry(&config, "aes_keys").as_deref(),
            Some("/keys/aes_keys.txt")
        );
        assert_eq!(parse_entry("theme=dark\n", "aes_keys"), None);
    }

    #[test]
    fn file_writes_preserve_unrelated_entries() {
        let directory =
            std::env::temp_dir().join(format!("randomon-config-{}", std::process::id()));
        let file = ConfigFile::at(directory.join("config.txt"));
        file.write("aes_keys", "/keys.txt").unwrap();
        file.write("skipped_version", "2.0.0").unwrap();
        assert_eq!(file.read("aes_keys").as_deref(), Some("/keys.txt"));
        assert_eq!(file.read("skipped_version").as_deref(), Some("2.0.0"));
        let _ = std::fs::remove_dir_all(directory);
    }
}
