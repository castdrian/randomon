use randomon_settings::{
    LENGTH_OF_SETTINGS_DATA, MiscTweak, OLD_VERSIONS, VERSION, VERSION_STRING,
    is_release_version_newer, release_name,
};

use crate::json::load;

#[test]
fn version_constants_match_java() {
    let vectors = load("constants.json");
    assert_eq!(VERSION, vectors.field("version").as_i32());
    assert_eq!(VERSION_STRING, vectors.field("version_string").as_string());
    assert_eq!(
        LENGTH_OF_SETTINGS_DATA,
        usize::try_from(vectors.field("length_of_settings_data").as_i64()).expect("length")
    );
    let java_versions: Vec<(i32, String)> = vectors
        .field("old_versions")
        .as_array()
        .iter()
        .map(|entry| {
            (
                entry.field("version").as_i32(),
                entry.field("name").as_string(),
            )
        })
        .collect();
    let rust_versions: Vec<(i32, String)> = OLD_VERSIONS
        .iter()
        .map(|(version, name)| (*version, (*name).to_owned()))
        .collect();
    assert_eq!(rust_versions, java_versions);
    assert_eq!(release_name(172), Some("1.7.2"));
    assert_eq!(release_name(173), None);
}

#[test]
fn misc_tweaks_match_java_declarations() {
    let vectors = load("constants.json");
    let tweaks = vectors.field("misc_tweaks");
    let java_tweaks = tweaks.field("tweaks").as_array();
    assert_eq!(MiscTweak::ALL.len(), java_tweaks.len());
    for (tweak, java) in MiscTweak::ALL.iter().zip(java_tweaks) {
        assert_eq!(tweak.constant_name(), java.field("constant").as_string());
        assert_eq!(tweak.value(), java.field("value").as_i32());
        assert_eq!(tweak.name(), java.field("name").as_string());
        assert_eq!(tweak.priority(), java.field("priority").as_i32());
        assert_eq!(MiscTweak::from_value(tweak.value()), Some(*tweak));
    }
    let every_tweak = MiscTweak::ALL
        .iter()
        .fold(0, |mask, tweak| mask | tweak.value());
    let order: Vec<String> = MiscTweak::in_application_order(every_tweak, every_tweak)
        .iter()
        .map(|tweak| tweak.constant_name().to_owned())
        .collect();
    assert_eq!(order, tweaks.field("priority_order").as_strings());
}

#[test]
fn release_version_comparison_matches_java() {
    assert!(is_release_version_newer("v4.6.2"));
    assert!(is_release_version_newer("v5.0.0"));
    assert!(is_release_version_newer("v4.7"));
    assert!(!is_release_version_newer("v4.6.1"));
    assert!(!is_release_version_newer("v4.6"));
    assert!(!is_release_version_newer("v4.5.9"));
    assert!(!is_release_version_newer("vx.y"));
    assert!(!is_release_version_newer(""));
}
