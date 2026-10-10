mod common;

use common::{Json, assert_bytes_eq, outcome, vector};
use randomon_formats::ctr::{Decompression, GarcArchive};

fn compare_bytes_outcome(
    actual: randomon_formats::Result<Vec<u8>>,
    expected: &Json,
    context: &str,
) {
    let expected = outcome(expected);
    match expected.result {
        "ok" => assert_bytes_eq(
            &actual.unwrap_or_else(|error| panic!("{context}: {error}")),
            &expected.value.bytes(),
            context,
        ),
        other => assert!(actual.is_err(), "{context}: expected {other}"),
    }
}

#[test]
fn garc_matches_upstream() {
    for (case_index, case) in vector("garc").get("cases").array().iter().enumerate() {
        let input = case.get("input").bytes();
        let selection: Vec<bool> = case
            .get("selection")
            .array()
            .iter()
            .map(Json::bool)
            .collect();
        let replacement = case.get("replacement").bytes();
        for mode in case.get("modes").array() {
            let mode_name = mode.get("mode").str();
            let decompression = match mode_name {
                "skip" => Decompression::Skip,
                "all" => Decompression::All,
                _ => Decompression::Selected(selection.clone()),
            };
            let context = format!("garc case {case_index} mode {mode_name}");
            let parsed = GarcArchive::parse(&input, decompression);
            let expected = outcome(mode.get("outcome"));
            if expected.result != "ok" {
                assert!(parsed.is_err(), "{context}: expected failure");
                continue;
            }
            let mut garc = parsed.unwrap_or_else(|error| panic!("{context}: {error}"));
            let expected_files = expected.value.get("files").array();
            assert_eq!(garc.files.len(), expected_files.len(), "{context}");
            for (directory, expected_directory) in garc.files.iter().zip(expected_files) {
                let entries = expected_directory.array();
                assert_eq!(directory.len(), entries.len(), "{context}");
                for ((key, data), entry) in directory.iter().zip(entries) {
                    assert_eq!(*key as i64, entry.get("key").i64(), "{context}");
                    match entry.get("data") {
                        Json::Null => assert!(data.is_none(), "{context}"),
                        value => {
                            assert_eq!(data.as_deref(), Some(value.bytes().as_slice()), "{context}")
                        }
                    }
                }
            }
            compare_bytes_outcome(
                garc.to_bytes(),
                expected.value.get("bytes"),
                &format!("{context} bytes"),
            );
            garc.set_file(0, replacement.clone()).unwrap();
            compare_bytes_outcome(
                garc.to_bytes(),
                expected.value.get("bytes_after_set"),
                &format!("{context} bytes after set"),
            );
        }
    }
}
