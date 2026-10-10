use randomon_settings::java_text::{decimal_digit, decode_utf8, parse_java_int};

use crate::json::load;

#[test]
fn utf8_decoding_matches_java_string_and_reader() {
    let vectors = load("java_text.json");
    let cases = vectors.field("utf8_decodes").as_array();
    assert!(cases.len() > 1000);
    for (index, case) in cases.iter().enumerate() {
        let bytes = case.field("bytes").as_base64();
        let decoded: Vec<u16> = decode_utf8(&bytes).encode_utf16().collect();
        let context = format!("decode case {index}: {bytes:02x?}");
        assert_eq!(
            decoded,
            case.field("string_units").as_unit_array(),
            "{context}"
        );
        assert_eq!(
            decoded,
            case.field("reader_units").as_unit_array(),
            "{context}"
        );
    }
}

#[test]
fn decimal_digits_match_character_digit_for_the_whole_bmp() {
    let vectors = load("java_text.json");
    let mut expected = vec![None; 0x10000];
    for entry in vectors.field("decimal_digits").as_array() {
        let pair = entry.as_array();
        let unit = usize::try_from(pair[0].as_i64()).expect("unit");
        expected[unit] = Some(u32::try_from(pair[1].as_i64()).expect("digit"));
    }
    for (unit, digit) in expected.iter().enumerate() {
        let unit = u16::try_from(unit).expect("BMP unit");
        assert_eq!(decimal_digit(unit), *digit, "unit {unit:#06x}");
    }
}

#[test]
fn integer_parsing_matches_java_parse_int() {
    let vectors = load("java_text.json");
    for case in vectors.field("parse_int").as_array() {
        let units = case.field("units").as_unit_array();
        let expected = case.get("value").map(|value| value.as_i32());
        assert_eq!(parse_java_int(&units), expected, "{units:?}");
    }
}
