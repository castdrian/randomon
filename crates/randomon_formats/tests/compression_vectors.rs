mod common;

use common::{assert_bytes_eq, outcome, vector};
use randomon_formats::FormatError;
use randomon_formats::compression::blz::{self, BlzMode};
use randomon_formats::compression::ds;
use randomon_formats::compression::lzss::{self, LzssDecoded};

#[test]
fn blz_encode_matches_upstream() {
    let vectors = vector("blz");
    for (index, case) in vectors.get("encode").array().iter().enumerate() {
        let input = case.get("input").bytes();
        let mode = if case.get("best").bool() {
            BlzMode::Best
        } else {
            BlzMode::Normal
        };
        let actual = blz::encode(&input, case.get("arm9").bool(), mode);
        let expected = outcome(case.get("outcome"));
        let context = format!("blz encode case {index} (len {})", input.len());
        match expected.result {
            "ok" => assert_bytes_eq(&actual.expect(&context), &expected.value.bytes(), &context),
            "hang" => assert!(
                matches!(actual, Err(FormatError::UpstreamHang { .. })),
                "{context}: {actual:?}"
            ),
            other => assert!(
                actual.is_err(),
                "{context}: expected {other}, got {actual:?}"
            ),
        }
    }
}

#[test]
fn blz_decode_matches_upstream() {
    let vectors = vector("blz");
    for (index, case) in vectors.get("decode").array().iter().enumerate() {
        let input = case.get("input").bytes();
        let actual = blz::decode(&input);
        let expected = outcome(case.get("outcome"));
        let context = format!("blz decode case {index}");
        match expected.result {
            "ok" => assert_bytes_eq(&actual.expect(&context), &expected.value.bytes(), &context),
            "exit" => assert!(
                matches!(actual, Err(FormatError::UpstreamExit { .. })),
                "{context}: {actual:?}"
            ),
            other => assert!(
                actual.is_err(),
                "{context}: expected {other}, got {actual:?}"
            ),
        }
    }
}

#[test]
fn lzss_encode_matches_upstream() {
    let vectors = vector("lzss");
    for (index, case) in vectors.get("encode").array().iter().enumerate() {
        let input = case.get("input").bytes();
        let actual = lzss::encode(&input);
        let expected = outcome(case.get("outcome"));
        let context = format!("lzss encode case {index} (len {})", input.len());
        match expected.result {
            "ok" => assert_bytes_eq(
                &actual.expect(&context).expect(&context),
                &expected.value.bytes(),
                &context,
            ),
            "null" => assert!(matches!(actual, Ok(None)), "{context}: {actual:?}"),
            other => assert!(
                actual.is_err(),
                "{context}: expected {other}, got {actual:?}"
            ),
        }
    }
}

#[test]
fn lzss_decode_matches_upstream() {
    let vectors = vector("lzss");
    for (index, case) in vectors.get("decode").array().iter().enumerate() {
        let input = case.get("input").bytes();
        let actual = lzss::decode(&input);
        let expected = outcome(case.get("outcome"));
        let context = format!("lzss decode case {index}");
        match (expected.result, actual) {
            ("ok", Ok(LzssDecoded::Data(data))) => {
                assert_bytes_eq(&data, &expected.value.bytes(), &context)
            }
            ("null", Ok(LzssDecoded::Rejected(_))) => {}
            ("error", Err(_)) => {}
            (expected, actual) => panic!("{context}: expected {expected}, got {actual:?}"),
        }
    }
}

#[test]
fn ds_decompress_matches_upstream() {
    let vectors = vector("ds");
    for (index, case) in vectors.get("cases").array().iter().enumerate() {
        let input = case.get("input").bytes();
        let offset = case.get("offset").i64() as usize;
        let actual = ds::decompress_at(&input, offset);
        let expected = outcome(case.get("outcome"));
        let context = format!("ds case {index}");
        match (expected.result, actual) {
            ("ok", Ok(Some(data))) => assert_bytes_eq(&data, &expected.value.bytes(), &context),
            ("null", Ok(None)) => {}
            ("error", Err(_)) => {}
            (expected, actual) => panic!("{context}: expected {expected}, got {actual:?}"),
        }
    }
}
