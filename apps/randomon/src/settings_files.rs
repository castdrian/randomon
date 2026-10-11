use std::path::{Path, PathBuf};

use randomon_settings::{Preset, Settings, release_name};

#[derive(Debug, Clone)]
pub struct ParsedSettings {
    pub settings: Settings,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct LoadedPreset {
    pub settings: Settings,
    pub seed: i64,
    pub rom_name: String,
}

pub fn parse_settings_string(text: &str) -> Result<ParsedSettings, String> {
    let versioned =
        Settings::from_versioned_settings_string(text.trim()).map_err(|error| error.to_string())?;
    let origin = match release_name(versioned.version) {
        Some(name) if versioned.version != randomon_settings::VERSION => {
            format!(" (upgraded from UPR {name})")
        }
        _ => String::new(),
    };
    let description = format!("{}{origin}", describe(&versioned.settings));
    Ok(ParsedSettings {
        settings: versioned.settings,
        description,
    })
}

fn describe(settings: &Settings) -> String {
    let rom = settings.rom_name.as_deref().unwrap_or("an unknown game");
    format!("Settings for {rom}")
}

#[derive(Debug, Clone)]
pub enum ImportedSettings {
    QuickSettings(ParsedSettings),
    Preset(LoadedPreset),
}

pub async fn import_settings_file() -> Option<Result<ImportedSettings, String>> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Import settings or a preset")
        .add_filter("Settings files and presets", &["rnqs", "rndp"])
        .pick_file()
        .await?;
    let bytes = handle.read().await;
    Some(imported_from_bytes(handle.path(), &bytes))
}

fn imported_from_bytes(path: &Path, bytes: &[u8]) -> Result<ImportedSettings, String> {
    let is_preset = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rndp"));
    if is_preset {
        preset_from_bytes(bytes).map(ImportedSettings::Preset)
    } else {
        quick_settings_from_bytes(bytes).map(ImportedSettings::QuickSettings)
    }
}

fn quick_settings_from_bytes(bytes: &[u8]) -> Result<ParsedSettings, String> {
    let settings = Settings::read_rnqs(bytes).map_err(|error| error.to_string())?;
    let suffix = if settings.updated_from_old_version {
        " (upgraded from an older version)"
    } else {
        ""
    };
    Ok(ParsedSettings {
        description: format!("{}{suffix}", describe(&settings)),
        settings,
    })
}

pub async fn export_settings_file(settings: Settings) -> Option<Result<PathBuf, String>> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Export settings")
        .add_filter("Randomization quick settings", &["rnqs"])
        .set_file_name("settings.rnqs")
        .save_file()
        .await?;
    Some(write_quick_settings(&settings, handle.path().to_path_buf()))
}

fn write_quick_settings(settings: &Settings, path: PathBuf) -> Result<PathBuf, String> {
    let bytes = settings.write_rnqs().map_err(|error| error.to_string())?;
    std::fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

fn preset_from_bytes(bytes: &[u8]) -> Result<LoadedPreset, String> {
    let preset = Preset::read(bytes).map_err(|error| error.to_string())?;
    let rom_name = preset
        .required_rom_name()
        .map_err(|error| error.to_string())?;
    let parsed = parse_settings_string(&preset.versioned_config_string())?;
    Ok(LoadedPreset {
        settings: parsed.settings,
        seed: preset.seed,
        rom_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(settings: &Settings) -> String {
        settings.to_versioned_settings_string().expect("encodes")
    }

    fn yellow_settings() -> Settings {
        Settings {
            rom_name: Some("Yellow (U)".to_owned()),
            selected_exp_curve: Some(randomon_settings::ExpCurve::MediumFast),
            ..Settings::default()
        }
    }

    #[test]
    fn describes_a_current_settings_string() {
        let text = yellow_settings()
            .to_versioned_settings_string()
            .expect("encodes");
        let parsed = parse_settings_string(&text).expect("valid");
        assert_eq!(parsed.description, "Settings for Yellow (U)");
        assert_eq!(encoded(&parsed.settings), text);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_settings_string("322not-base64!!").is_err());
    }

    #[test]
    fn quick_settings_round_trip_through_disk() {
        let directory = std::env::temp_dir().join("randomon-settings-files-test");
        std::fs::create_dir_all(&directory).expect("temp dir");
        let path = write_quick_settings(&yellow_settings(), directory.join("roundtrip.rnqs"))
            .expect("writes");
        let loaded =
            quick_settings_from_bytes(&std::fs::read(path).expect("reads")).expect("parses");
        assert_eq!(encoded(&loaded.settings), encoded(&yellow_settings()));
    }

    #[test]
    fn presets_round_trip() {
        let config = yellow_settings().to_settings_string().expect("encodes");
        let bytes = Preset::with_default_custom_names(1234, config)
            .to_bytes()
            .expect("writes");
        let preset = preset_from_bytes(&bytes).expect("reads");
        assert_eq!(preset.seed, 1234);
        assert_eq!(preset.rom_name, "Yellow (U)");
        assert_eq!(encoded(&preset.settings), encoded(&yellow_settings()));
    }

    #[test]
    fn imports_pick_the_format_from_the_extension() {
        let config = yellow_settings().to_settings_string().expect("encodes");
        let preset_bytes = Preset::with_default_custom_names(99, config)
            .to_bytes()
            .expect("writes");
        let quick_bytes = yellow_settings().write_rnqs().expect("writes");
        assert!(matches!(
            imported_from_bytes(Path::new("shared.RNDP"), &preset_bytes),
            Ok(ImportedSettings::Preset(preset)) if preset.seed == 99
        ));
        assert!(matches!(
            imported_from_bytes(Path::new("shared.rnqs"), &quick_bytes),
            Ok(ImportedSettings::QuickSettings(_))
        ));
    }
}
