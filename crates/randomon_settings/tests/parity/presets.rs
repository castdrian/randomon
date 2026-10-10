use randomon_settings::java_data::{DataReader, modified_utf8_from_units};
use randomon_settings::{CustomNames, Preset, SettingsError, VERSION};

use crate::custom_names::assert_names_match;
use crate::json::{Json, load};

fn vectors() -> Json {
    load("presets.json")
}

fn modified_utf8_segment(bytes: &[u8]) -> &[u8] {
    let length = usize::from(u16::from_be_bytes([bytes[12], bytes[13]]));
    &bytes[12..14 + length]
}

#[test]
fn preset_files_read_and_write_like_java() {
    let vectors = vectors();
    let cases = vectors.field("cases").as_array();
    assert!(cases.len() > 60);
    for (index, case) in cases.iter().enumerate() {
        let bytes = case.field("bytes").as_base64();
        let read = case.field("read");
        let context = format!("preset case {index}");
        let result = Preset::read(&bytes);
        match read.field("result").as_string().as_str() {
            "version_mismatch" => assert_version_mismatch(&result, read, &context),
            "error" => assert!(result.is_err(), "{context}: {result:?}"),
            "read" => assert_read_preset(result, &bytes, read, &context),
            other => panic!("unknown preset outcome {other}"),
        }
    }
}

fn assert_version_mismatch(result: &Result<Preset, SettingsError>, read: &Json, context: &str) {
    let Err(SettingsError::PresetVersionMismatch {
        version,
        expected,
        required_release,
    }) = result
    else {
        panic!("{context}: expected a version mismatch, got {result:?}");
    };
    assert_eq!(*version, read.field("version").as_i32(), "{context}");
    assert_eq!(*expected, VERSION, "{context}");
    let java_release = read.get("required_release").map(Json::as_string);
    assert_eq!(
        required_release.map(str::to_owned),
        java_release,
        "{context}"
    );
}

fn assert_read_preset(
    result: Result<Preset, SettingsError>,
    bytes: &[u8],
    read: &Json,
    context: &str,
) {
    let units = read.field("config_units").as_unit_array();
    let Ok(config_string) = String::from_utf16(&units) else {
        assert_eq!(result, Err(SettingsError::UnpairedSurrogate), "{context}");
        return;
    };
    let preset = result.unwrap_or_else(|error| panic!("{context}: {error}"));
    assert_eq!(preset.version, VERSION, "{context}");
    assert_eq!(preset.seed, read.field("seed").as_i64(), "{context}");
    assert_eq!(preset.config_string, config_string, "{context}");
    assert_eq!(
        preset.custom_names,
        read.field("custom_names_bytes").as_base64(),
        "{context}"
    );
    let names = preset.parsed_custom_names().expect("custom names parse");
    assert_names_match(&names, read.field("custom_names"), context);
    let required = read.field("required_rom_name");
    match required.field("result").as_string().as_str() {
        "ok" => assert_eq!(
            preset.required_rom_name(),
            Ok(required.field("name").as_string()),
            "{context}"
        ),
        _ => assert!(preset.required_rom_name().is_err(), "{context}"),
    }
    let rewritten = preset.to_bytes().expect("preset re-encodes");
    let canonical = modified_utf8_from_units(&units).expect("config encodes");
    if canonical == modified_utf8_segment(bytes) {
        assert_eq!(rewritten, bytes, "{context}: byte-identical rewrite");
    }
    assert_eq!(Preset::read(&rewritten).as_ref(), Ok(&preset), "{context}");
}

#[test]
fn modified_utf8_writing_matches_data_output_stream() {
    let vectors = vectors();
    for (index, case) in vectors.field("write_utf").as_array().iter().enumerate() {
        let units = match case.get("units") {
            Some(units) => units.as_unit_array(),
            None => vec![
                u16::try_from(case.field("repeat_unit").as_i64()).expect("unit");
                usize::try_from(case.field("repeat_count").as_i64()).expect("count")
            ],
        };
        let encoded = modified_utf8_from_units(&units);
        let context = format!("writeUTF case {index}");
        if case.has("error") {
            assert!(encoded.is_err(), "{context}");
        } else if let Some(expected) = case.get("bytes") {
            assert_eq!(encoded, Ok(expected.as_base64()), "{context}");
        } else {
            let length = usize::try_from(case.field("encoded_length").as_i64()).expect("length");
            assert_eq!(encoded.map(|bytes| bytes.len()), Ok(length), "{context}");
        }
    }
}

#[test]
fn modified_utf8_reading_matches_data_input_stream() {
    let vectors = vectors();
    for (index, case) in vectors.field("read_utf").as_array().iter().enumerate() {
        let bytes = case.field("bytes").as_base64();
        let decoded = DataReader::new(&bytes).read_modified_utf8_units();
        let context = format!("readUTF case {index}");
        match case.get("units") {
            Some(units) => assert_eq!(decoded, Ok(units.as_unit_array()), "{context}"),
            None => assert!(decoded.is_err(), "{context}: {decoded:?}"),
        }
    }
}

#[test]
fn new_presets_use_the_current_version_and_default_names() {
    let preset = Preset::with_default_custom_names(-5, "abc".to_owned());
    let bytes = preset.to_bytes().expect("encodes");
    assert_eq!(&bytes[..4], &VERSION.to_be_bytes());
    assert_eq!(&bytes[4..12], &(-5i64).to_be_bytes());
    assert_eq!(&bytes[12..17], &[0, 3, b'a', b'b', b'c']);
    assert_eq!(
        preset.parsed_custom_names(),
        Ok(CustomNames::default_names())
    );
    assert_eq!(preset.versioned_config_string(), format!("{VERSION}abc"));
}
