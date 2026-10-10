use randomon_settings::{Settings, VERSION, default_custom_names_checksum};

use crate::fields::{assert_matches_java, settings_from_java};
use crate::json::{Json, load};
use crate::outcome::assert_decode_outcome;

fn vectors() -> Json {
    load("settings_strings.json")
}

#[test]
fn default_custom_names_checksum_matches_java() {
    let java_checksum = vectors().field("custom_names_checksum").as_i32();
    assert_eq!(default_custom_names_checksum(), java_checksum);
}

#[test]
fn encoding_every_settings_object_matches_java() {
    let vectors = vectors();
    let cases = vectors.field("cases").as_array();
    assert!(cases.len() > 300);
    for case in cases {
        let name = case.field("name").as_string();
        let settings = settings_from_java(case.field("fields"));
        assert_matches_java(&settings, case.field("fields"), &name);
        let expected = case.field("string").as_string();
        assert_eq!(
            settings.to_settings_string().as_ref(),
            Ok(&expected),
            "{name}"
        );
        assert_eq!(
            settings.to_versioned_settings_string(),
            Ok(format!("{VERSION}{expected}")),
            "{name}"
        );
    }
}

#[test]
fn rnqs_files_match_java_bytes() {
    let vectors = vectors();
    for case in vectors.field("cases").as_array() {
        let name = case.field("name").as_string();
        let settings = settings_from_java(case.field("fields"));
        let expected = case.field("rnqs").as_base64();
        assert_eq!(settings.write_rnqs().as_ref(), Ok(&expected), "{name}");
    }
}

#[test]
fn decoding_every_settings_string_matches_java() {
    let vectors = vectors();
    for case in vectors.field("cases").as_array() {
        let name = case.field("name").as_string();
        let string = case.field("string").as_string();
        assert_decode_outcome(
            Settings::from_settings_string(&string),
            case.field("decoded"),
            &name,
        );
    }
}

#[test]
fn reading_written_rnqs_matches_java_decode() {
    let vectors = vectors();
    for case in vectors.field("cases").as_array() {
        let name = case.field("name").as_string();
        let bytes = case.field("rnqs").as_base64();
        assert_decode_outcome(Settings::read_rnqs(&bytes), case.field("decoded"), &name);
    }
}

#[test]
fn versioned_strings_round_trip() {
    let vectors = vectors();
    for case in vectors.field("cases").as_array() {
        let string = case.field("string").as_string();
        let decoded = case.field("decoded");
        let parsed = Settings::from_versioned_settings_string(&format!("{VERSION}{string}"));
        if decoded.has("error") {
            assert!(parsed.is_err());
            continue;
        }
        let parsed = parsed.expect("current version strings load");
        assert_eq!(parsed.version, VERSION);
        assert_matches_java(&parsed.settings, decoded.field("fields"), "versioned");
    }
}
