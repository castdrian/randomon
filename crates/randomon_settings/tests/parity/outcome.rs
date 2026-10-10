use randomon_settings::{Settings, SettingsError};

use crate::fields::assert_matches_java;
use crate::json::Json;

pub fn assert_decode_outcome(
    result: Result<Settings, SettingsError>,
    expected: &Json,
    context: &str,
) {
    if let Some(error) = expected.get("error") {
        assert!(
            result.is_err(),
            "{context}: Java failed with {} but Rust decoded {result:?}",
            error.as_string()
        );
        return;
    }
    let settings = result.unwrap_or_else(|error| panic!("{context}: Rust failed with {error}"));
    assert_matches_java(&settings, expected.field("fields"), context);
    let reencoded = settings.to_settings_string();
    if let Some(java_reencoded) = expected.get("reencoded") {
        assert_eq!(
            reencoded.as_ref().ok(),
            Some(&java_reencoded.as_string()),
            "{context}: re-encoding"
        );
    } else {
        assert!(reencoded.is_err(), "{context}: Java could not re-encode");
    }
}
