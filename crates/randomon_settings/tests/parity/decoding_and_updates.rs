use randomon_settings::{Settings, SettingsError, update_settings_string};

use crate::fields::assert_matches_java;
use crate::json::load;
use crate::outcome::assert_decode_outcome;

#[test]
fn fuzzed_settings_strings_decode_like_java() {
    let vectors = load("decode_fuzz.json");
    let cases = vectors.field("cases").as_array();
    assert!(cases.len() > 600);
    for (index, case) in cases.iter().enumerate() {
        let input = case.field("input").as_string();
        assert_decode_outcome(
            Settings::from_settings_string(&input),
            case.field("decoded"),
            &format!("fuzz case {index}: {input:?}"),
        );
    }
}

#[test]
fn settings_updater_matches_java_for_every_version() {
    let vectors = load("updater.json");
    let cases = vectors.field("cases").as_array();
    assert!(cases.len() > 800);
    for (index, case) in cases.iter().enumerate() {
        let version = case.field("version").as_i32();
        let input = case.field("input").as_string();
        let context = format!("updater case {index} from version {version}");
        let updated = update_settings_string(version, &input);
        if case.has("error") {
            assert!(
                updated.is_err(),
                "{context}: Java failed, Rust produced {updated:?}"
            );
            continue;
        }
        let expected = case.field("output").as_string();
        assert_eq!(updated.as_ref(), Ok(&expected), "{context}");
        assert_decode_outcome(
            Settings::from_settings_string(&expected),
            case.field("decoded"),
            &context,
        );
    }
}

#[test]
fn versioned_settings_strings_load_like_the_java_gui() {
    let vectors = load("versioned_strings.json");
    for (index, case) in vectors.field("cases").as_array().iter().enumerate() {
        let input = case.field("input").as_string();
        let outcome = case.field("outcome");
        let context = format!("versioned case {index}");
        let loaded = Settings::from_versioned_settings_string(&input);
        match outcome.field("result").as_string().as_str() {
            "empty" => assert_eq!(loaded, Err(SettingsError::EmptySettingsString), "{context}"),
            "too_new" => assert_eq!(
                loaded,
                Err(SettingsError::SettingsStringTooNew {
                    version: outcome.field("version").as_i32()
                }),
                "{context}"
            ),
            "invalid" | "crash" => assert!(loaded.is_err(), "{context}: {loaded:?}"),
            "updated" | "loaded" => {
                let loaded = loaded.unwrap_or_else(|error| panic!("{context}: {error}"));
                assert_eq!(
                    loaded.version,
                    outcome.field("version").as_i32(),
                    "{context}"
                );
                assert_matches_java(&loaded.settings, outcome.field("fields"), &context);
            }
            other => panic!("unknown outcome {other}"),
        }
    }
}

#[test]
fn rnqs_reading_matches_java_including_errors() {
    let vectors = load("rnqs.json");
    for (index, case) in vectors.field("cases").as_array().iter().enumerate() {
        let bytes = case.field("bytes").as_base64();
        let read = case.field("read");
        let context = format!("rnqs case {index}");
        let result = Settings::read_rnqs(&bytes);
        let java_error = read.get("error").map(|error| error.as_string());
        if java_error.as_deref() == Some("UnsupportedOperationException") {
            let message = read.field("message").as_string();
            let rust_message = result.as_ref().err().map(ToString::to_string);
            assert_eq!(rust_message, Some(message), "{context}");
            continue;
        }
        assert_decode_outcome(result, read, &context);
    }
}
