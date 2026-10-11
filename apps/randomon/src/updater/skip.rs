use semver::Version;

use super::release::parse_version;
use crate::config::ConfigFile;

const SKIPPED_VERSION_ENTRY: &str = "skipped_update";

pub fn skipped_version(file: &ConfigFile) -> Option<Version> {
    file.read(SKIPPED_VERSION_ENTRY)
        .as_deref()
        .and_then(parse_version)
}

pub fn skip_version(file: &ConfigFile, version: &Version) -> Result<(), String> {
    file.write(SKIPPED_VERSION_ENTRY, &version.to_string())
        .map_err(|error| format!("Couldn't remember the skipped version: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_the_skipped_version_alongside_other_settings() {
        let directory = std::env::temp_dir().join(format!("randomon-skip-{}", std::process::id()));
        let path = directory.join("config.txt");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(&path, "aes_keys=/keys/aes_keys.txt\n").unwrap();
        let file = ConfigFile::at(path.clone());
        assert_eq!(skipped_version(&file), None);
        skip_version(&file, &Version::new(1, 4, 0)).unwrap();
        skip_version(&file, &Version::new(1, 5, 0)).unwrap();
        assert_eq!(skipped_version(&file), Some(Version::new(1, 5, 0)));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "aes_keys=/keys/aes_keys.txt\nskipped_update=1.5.0\n"
        );
        let _ = std::fs::remove_dir_all(&directory);
    }
}
