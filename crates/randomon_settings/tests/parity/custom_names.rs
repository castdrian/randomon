use randomon_settings::{
    CustomNames, DEFAULT_CUSTOM_NAMES_FILE, LineSeparator, custom_names_checksum,
    default_custom_names_checksum,
};

use crate::json::{Json, load};

pub fn names_from_java(object: &Json) -> CustomNames {
    CustomNames {
        trainer_names: object.field("trainer_names").as_strings(),
        trainer_classes: object.field("trainer_classes").as_strings(),
        doubles_trainer_names: object.field("doubles_trainer_names").as_strings(),
        doubles_trainer_classes: object.field("doubles_trainer_classes").as_strings(),
        pokemon_nicknames: object.field("pokemon_nicknames").as_strings(),
    }
}

pub fn assert_names_match(names: &CustomNames, object: &Json, context: &str) {
    assert_eq!(names, &names_from_java(object), "{context}");
}

#[test]
fn default_custom_names_parse_and_checksum_like_java() {
    let vectors = load("custom_names.json");
    let default = vectors.field("default");
    assert_eq!(
        default.field("bytes").as_base64(),
        DEFAULT_CUSTOM_NAMES_FILE
    );
    let names = CustomNames::from_bytes(DEFAULT_CUSTOM_NAMES_FILE).expect("default names parse");
    assert_names_match(&names, default.field("parsed"), "default names");
    assert_eq!(names, CustomNames::default_names());
    assert_eq!(
        default_custom_names_checksum(),
        default.field("checksum").as_i32()
    );
    assert_eq!(
        names.to_bytes(LineSeparator::Lf),
        default.field("reserialized").as_base64()
    );
}

#[test]
fn default_custom_names_rewrite_byte_identically_with_crlf() {
    let names = CustomNames::default_names();
    assert_eq!(
        names.to_bytes(LineSeparator::CrLf),
        DEFAULT_CUSTOM_NAMES_FILE
    );
}

#[test]
fn custom_name_sets_serialize_like_java() {
    let vectors = load("custom_names.json");
    for (index, case) in vectors.field("sets").as_array().iter().enumerate() {
        let context = format!("names set {index}");
        let names = names_from_java(case.field("lists"));
        let bytes = case.field("bytes").as_base64();
        assert_eq!(names.to_bytes(LineSeparator::Lf), bytes, "{context}");
        let parsed = CustomNames::from_bytes(&bytes).expect("parses");
        assert_names_match(&parsed, case.field("parsed"), &context);
        assert_eq!(
            custom_names_checksum(&bytes),
            case.field("checksum").as_i32(),
            "{context}"
        );
    }
}

#[test]
fn custom_name_sets_serialize_with_each_line_separator() {
    for (file, separator) in [
        ("custom_names_lf.json", LineSeparator::Lf),
        ("custom_names_crlf.json", LineSeparator::CrLf),
    ] {
        let vectors = load(file);
        assert_eq!(
            vectors.field("line_separator").as_string(),
            separator.as_str()
        );
        assert_eq!(
            CustomNames::default_names().to_bytes(separator),
            vectors.field("default_reserialized").as_base64(),
            "{file}"
        );
        for (index, case) in vectors.field("sets").as_array().iter().enumerate() {
            let names = names_from_java(case.field("lists"));
            assert_eq!(
                names.to_bytes(separator),
                case.field("bytes").as_base64(),
                "{file} set {index}"
            );
        }
    }
}

#[test]
fn malformed_custom_names_files_fail_like_java() {
    let vectors = load("custom_names.json");
    for (index, case) in vectors.field("parse_fuzz").as_array().iter().enumerate() {
        let bytes = case.field("bytes").as_base64();
        let parsed = CustomNames::from_bytes(&bytes);
        let context = format!("names fuzz {index}: {bytes:02x?}");
        match case.get("parsed") {
            Some(expected) => {
                let names = parsed.unwrap_or_else(|error| panic!("{context}: {error}"));
                assert_names_match(&names, expected, &context);
            }
            None => assert!(parsed.is_err(), "{context}: {parsed:?}"),
        }
    }
}

#[test]
fn custom_names_checksum_matches_java_scanner_semantics() {
    let vectors = load("custom_names.json");
    for (index, case) in vectors.field("checksums").as_array().iter().enumerate() {
        let bytes = case.field("bytes").as_base64();
        assert_eq!(
            custom_names_checksum(&bytes),
            case.field("checksum").as_i32(),
            "checksum case {index}: {bytes:02x?}"
        );
    }
}
