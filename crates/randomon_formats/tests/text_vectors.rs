mod common;

use common::{Json, assert_bytes_eq, hex_sha256, outcome, vector};
use randomon_formats::FormatError;
use randomon_formats::text::poketext::{
    DecodedText, PokeTextData, encode_utf16, generation4_table, make_file_utf16, read_strings,
    write_strings, write_strings_utf16,
};

fn units(json: &Json) -> Vec<u16> {
    json.bytes()
        .chunks(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect()
}

fn case_strings(case: &Json) -> Vec<Vec<u16>> {
    if case.has("empty_count") {
        return vec![Vec::new(); case.get("empty_count").i64() as usize];
    }
    case.get("strings").array().iter().map(units).collect()
}

fn expect_error_kind(error: &FormatError, java_error: Option<&str>, context: &str) {
    let matches = match java_error {
        Some("java.lang.NumberFormatException") => matches!(error, FormatError::Invalid { .. }),
        Some(
            "java.lang.ArrayIndexOutOfBoundsException"
            | "java.lang.IndexOutOfBoundsException"
            | "java.lang.StringIndexOutOfBoundsException",
        ) => matches!(error, FormatError::OutOfBounds { .. }),
        _ => false,
    };
    assert!(
        matches,
        "{context}: java threw {java_error:?}, rust returned {error:?}"
    );
}

fn compare_blob(actual: &randomon_formats::Result<Vec<u8>>, expected: &Json, context: &str) {
    let expected = outcome(expected);
    match (expected.result, actual) {
        ("ok", Ok(bytes)) => {
            assert_eq!(
                bytes.len() as i64,
                expected.value.get("length").i64(),
                "{context}: length"
            );
            assert_eq!(
                hex_sha256(bytes),
                expected.value.get("sha256").str(),
                "{context}: sha256"
            );
            if expected.value.has("bytes") {
                assert_bytes_eq(bytes, &expected.value.get("bytes").bytes(), context);
            }
        }
        ("error", Err(error)) => expect_error_kind(error, expected.error, context),
        (result, actual) => panic!("{context}: java {result}, rust {actual:?}"),
    }
}

fn compare_decoded(actual: randomon_formats::Result<DecodedText>, expected: &Json, context: &str) {
    let expected = outcome(expected);
    match (expected.result, actual) {
        ("ok", Ok(decoded)) => {
            let actual_units: Vec<Vec<u16>> = decoded
                .strings
                .iter()
                .map(|text| text.encode_utf16().collect())
                .collect();
            let expected_units: Vec<Vec<u16>> = expected
                .value
                .get("strings")
                .array()
                .iter()
                .map(units)
                .collect();
            assert_eq!(actual_units, expected_units, "{context}: strings");
            assert_eq!(
                decoded.compressed,
                expected.value.get("compressed").bool(),
                "{context}: compress flag"
            );
        }
        ("error", Err(error)) => expect_error_kind(&error, expected.error, context),
        (result, actual) => panic!("{context}: java {result}, rust {actual:?}"),
    }
}

#[test]
fn character_table_matches_unicode_parser() {
    let vectors = vector("poketext");
    let table = generation4_table().unwrap();
    let expected_by_code: Vec<(i64, Vec<u16>)> = vectors
        .get("table")
        .get("by_code")
        .array()
        .iter()
        .map(|pair| (pair.array()[0].i64(), units(&pair.array()[1])))
        .collect();
    let actual_by_code: Vec<(i64, Vec<u16>)> = table
        .texts_by_code()
        .map(|(code, text)| (i64::from(code), text.encode_utf16().collect()))
        .collect();
    assert_eq!(actual_by_code, expected_by_code);
    let mut expected_by_text: Vec<(Vec<u16>, i64)> = vectors
        .get("table")
        .get("by_text")
        .array()
        .iter()
        .map(|pair| (units(&pair.array()[0]), pair.array()[1].i64()))
        .collect();
    let mut actual_by_text: Vec<(Vec<u16>, i64)> = table
        .codes_by_text()
        .map(|(text, code)| (text.to_vec(), i64::from(code)))
        .collect();
    expected_by_text.sort();
    actual_by_text.sort();
    assert_eq!(actual_by_text, expected_by_text);
}

#[test]
fn hex_escapes_accept_exactly_the_character_digit_set_for_the_whole_bmp() {
    let vectors = vector("poketext");
    let mut expected = vec![None; 0x10000];
    for pair in vectors.get("hex_digits").array() {
        expected[pair.array()[0].i64() as usize] = Some(pair.array()[1].i64() as i32);
    }
    let prefix = "\\x000".encode_utf16();
    for (unit, digit) in (0..=u16::MAX).zip(expected) {
        let escape: Vec<u16> = prefix.clone().chain([unit]).collect();
        let encoded = encode_utf16(&escape, false);
        match digit {
            Some(value) => assert_eq!(encoded.unwrap(), vec![value, 0xFFFF], "unit {unit:#06x}"),
            None => assert!(
                matches!(encoded, Err(FormatError::Invalid { .. })),
                "unit {unit:#06x}: {encoded:?}"
            ),
        }
    }
}

#[test]
fn make_file_write_and_read_back_match_upstream() {
    let vectors = vector("poketext");
    for (index, case) in vectors.get("encodes").array().iter().enumerate() {
        let strings = case_strings(case);
        let compressed = case.get("compressed").bool();
        let context = format!("encode case {index}");
        compare_blob(
            &make_file_utf16(&strings, compressed),
            case.get("make_file"),
            &context,
        );
        let written = write_strings_utf16(&strings, compressed);
        compare_blob(&written, case.get("write"), &context);
        let read_back = written.and_then(|bytes| read_strings(&bytes));
        compare_decoded(read_back, case.get("read_back"), &context);
        let valid: Option<Vec<String>> = strings
            .iter()
            .map(|text| String::from_utf16(text).ok())
            .collect();
        if let Some(valid) = valid {
            compare_blob(
                &write_strings(&valid, compressed),
                case.get("write"),
                &context,
            );
        }
    }
}

#[test]
fn decoding_synthetic_files_matches_upstream() {
    let vectors = vector("poketext");
    for (index, case) in vectors.get("decodes").array().iter().enumerate() {
        let file = case.get("file").bytes();
        compare_decoded(
            read_strings(&file),
            case.get("decoded"),
            &format!("decode case {index}"),
        );
    }
}

#[test]
fn encrypting_raw_files_matches_upstream() {
    let vectors = vector("poketext");
    for (index, case) in vectors.get("encrypts").array().iter().enumerate() {
        let raw = case.get("raw").bytes();
        let key = case.get("key").i64() as i32;
        let encrypted = (|| {
            let mut data = PokeTextData::new(&raw);
            data.set_key(key)?;
            data.encrypt()?;
            Ok(data.into_bytes())
        })();
        compare_blob(
            &encrypted,
            case.get("encrypted"),
            &format!("encrypt case {index}"),
        );
    }
}
