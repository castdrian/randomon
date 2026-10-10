use std::path::PathBuf;

use randomon_settings::{Preset, Settings, release_name};

#[derive(Debug, Clone)]
pub struct LoadedSettings {
    pub settings_string: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct LoadedPreset {
    pub settings_string: String,
    pub seed: i64,
    pub rom_name: String,
}

pub fn describe_settings_string(text: &str) -> Result<String, String> {
    let versioned =
        Settings::from_versioned_settings_string(text.trim()).map_err(|error| error.to_string())?;
    let rom = versioned
        .settings
        .rom_name
        .clone()
        .unwrap_or_else(|| "an unknown game".to_owned());
    let origin = match release_name(versioned.version) {
        Some(name) if versioned.version != randomon_settings::VERSION => {
            format!(" (upgraded from UPR {name})")
        }
        _ => String::new(),
    };
    Ok(format!("Settings for {rom}{origin}"))
}

pub async fn load_quick_settings() -> Option<Result<LoadedSettings, String>> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Load quick settings")
        .add_filter("Randomization quick settings", &["rnqs"])
        .pick_file()
        .await?;
    let bytes = handle.read().await;
    Some(quick_settings_from_bytes(&bytes))
}

fn quick_settings_from_bytes(bytes: &[u8]) -> Result<LoadedSettings, String> {
    let settings = Settings::read_rnqs(bytes).map_err(|error| error.to_string())?;
    let settings_string = settings
        .to_versioned_settings_string()
        .map_err(|error| error.to_string())?;
    let description = describe_settings_string(&settings_string)?;
    Ok(LoadedSettings {
        settings_string,
        description,
    })
}

pub async fn save_quick_settings(settings_string: String) -> Option<Result<PathBuf, String>> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Save quick settings")
        .add_filter("Randomization quick settings", &["rnqs"])
        .set_file_name("settings.rnqs")
        .save_file()
        .await?;
    Some(write_quick_settings(
        &settings_string,
        handle.path().to_path_buf(),
    ))
}

fn write_quick_settings(settings_string: &str, path: PathBuf) -> Result<PathBuf, String> {
    let versioned = Settings::from_versioned_settings_string(settings_string.trim())
        .map_err(|error| error.to_string())?;
    let bytes = versioned
        .settings
        .write_rnqs()
        .map_err(|error| error.to_string())?;
    std::fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

pub async fn load_preset() -> Option<Result<LoadedPreset, String>> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Load a preset")
        .add_filter("Pokemon Randomizer preset", &["rndp"])
        .pick_file()
        .await?;
    let bytes = handle.read().await;
    Some(preset_from_bytes(&bytes))
}

fn preset_from_bytes(bytes: &[u8]) -> Result<LoadedPreset, String> {
    let preset = Preset::read(bytes).map_err(|error| error.to_string())?;
    let rom_name = preset
        .required_rom_name()
        .map_err(|error| error.to_string())?;
    Ok(LoadedPreset {
        settings_string: preset.versioned_config_string(),
        seed: preset.seed,
        rom_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yellow_settings_string() -> String {
        let settings = Settings {
            rom_name: Some("Yellow (U)".to_owned()),
            selected_exp_curve: Some(randomon_settings::ExpCurve::MediumFast),
            ..Settings::default()
        };
        settings.to_versioned_settings_string().expect("encodes")
    }

    #[test]
    fn describes_a_current_settings_string() {
        let description = describe_settings_string(&yellow_settings_string()).expect("valid");
        assert_eq!(description, "Settings for Yellow (U)");
    }

    #[test]
    fn rejects_garbage() {
        assert!(describe_settings_string("322not-base64!!").is_err());
    }

    #[test]
    fn quick_settings_round_trip_through_disk() {
        let directory = std::env::temp_dir().join("randomon-settings-files-test");
        std::fs::create_dir_all(&directory).expect("temp dir");
        let path =
            write_quick_settings(&yellow_settings_string(), directory.join("roundtrip.rnqs"))
                .expect("writes");
        let loaded =
            quick_settings_from_bytes(&std::fs::read(path).expect("reads")).expect("parses");
        assert_eq!(loaded.settings_string, yellow_settings_string());
    }
}
