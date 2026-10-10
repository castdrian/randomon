mod common;

use std::io::Cursor;

use common::{Json, assert_bytes_eq, outcome, vector};
use randomon_formats::ctr::Ncch;
use randomon_formats::ctr::container;
use randomon_formats::ctr::ncch::diagnostics_lines;
use randomon_formats::nds::NdsRom;

fn check_optional_bytes(
    actual: randomon_formats::Result<Option<Vec<u8>>>,
    expected: &Json,
    context: &str,
) {
    let expected = outcome(expected);
    match (expected.result, actual) {
        ("ok", Ok(Some(data))) => assert_bytes_eq(&data, &expected.value.bytes(), context),
        ("null", Ok(None)) => {}
        ("error", Err(_)) => {}
        (expected, actual) => panic!(
            "{context}: expected {expected}, got {:?}",
            actual.map(|data| data.map(|d| d.len()))
        ),
    }
}

fn check_unit(actual: randomon_formats::Result<()>, expected: &Json, context: &str) {
    match outcome(expected).result {
        "ok" => actual.unwrap_or_else(|error| panic!("{context}: {error}")),
        _ => assert!(actual.is_err(), "{context}: expected an error"),
    }
}

#[test]
fn nds_rom_matches_upstream() {
    for case in vector("nds_rom").get("cases").array() {
        let name = case.get("name").str();
        let mut rom = NdsRom::open(Cursor::new(case.get("input").bytes())).unwrap();
        for (index, operation) in case.get("operations").array().iter().enumerate() {
            let context = format!("{name} operation {index} ({})", operation.get("op").str());
            let expected = operation.get("outcome");
            match operation.get("op").str() {
                "code" => assert_eq!(rom.code(), outcome(expected).value.str(), "{context}"),
                "version" => assert_eq!(
                    rom.version() as i64,
                    outcome(expected).value.i64(),
                    "{context}"
                ),
                "read_file" => check_optional_bytes(
                    rom.get_file(operation.get("path").str()),
                    expected,
                    &context,
                ),
                "write_file" => check_unit(
                    rom.write_file(operation.get("path").str(), &operation.get("data").bytes()),
                    expected,
                    &context,
                ),
                "read_overlay" => check_optional_bytes(
                    rom.get_overlay(operation.get("number").i64() as i32),
                    expected,
                    &context,
                ),
                "write_overlay" => check_unit(
                    rom.write_overlay(
                        operation.get("number").i64() as i32,
                        &operation.get("data").bytes(),
                    ),
                    expected,
                    &context,
                ),
                "overlay_address" => assert_eq!(
                    rom.overlay_address(operation.get("number").i64() as i32) as i64,
                    outcome(expected).value.i64(),
                    "{context}"
                ),
                "read_arm9" => check_optional_bytes(rom.get_arm9().map(Some), expected, &context),
                "write_arm9" => check_unit(
                    rom.write_arm9(&operation.get("data").bytes()),
                    expected,
                    &context,
                ),
                other => panic!("unknown operation {other}"),
            }
        }
        let saved = rom.save_to(Cursor::new(Vec::new())).map(Cursor::into_inner);
        let expected = outcome(case.get("saved"));
        assert_eq!(expected.result, "ok", "{name}");
        assert_bytes_eq(
            &saved.unwrap(),
            &expected.value.bytes(),
            &format!("{name} saved ROM"),
        );
        let diagnostics: Vec<String> = case
            .get("diagnostics")
            .array()
            .iter()
            .map(|line| line.str().to_owned())
            .collect();
        assert_eq!(
            rom.diagnostics(case.get("file_name").str()),
            diagnostics,
            "{name} diagnostics"
        );
    }
}

#[test]
fn ncch_matches_upstream() {
    for case in vector("ncch").get("cases").array() {
        let name = case.get("name").str();
        let input = case.get("input").bytes();
        let mut cursor = Cursor::new(input.clone());
        let cxi = container::cxi_offset(&mut cursor).unwrap();
        assert_eq!(
            cxi.map_or(-1, |offset| offset as i64),
            case.get("cxi_offset").i64(),
            "{name}"
        );
        let product_code = container::product_code(&mut cursor).unwrap();
        let title_id = container::title_id(&mut cursor).unwrap();
        assert_eq!(
            product_code.as_deref(),
            Some(case.get("product_code").str()),
            "{name}"
        );
        assert_eq!(
            title_id.as_deref(),
            Some(case.get("title_id").str()),
            "{name}"
        );
        let opened = Ncch::open(Cursor::new(input), product_code, title_id.clone());
        if case.get("open").str() != "ok" {
            assert!(opened.is_err(), "{name}: expected open failure");
            continue;
        }
        let mut ncch = opened.unwrap_or_else(|error| panic!("{name}: {error}"));
        for (index, operation) in case.get("operations").array().iter().enumerate() {
            let context = format!("{name} operation {index} ({})", operation.get("op").str());
            let expected = operation.get("outcome");
            let expected_outcome = outcome(expected);
            match operation.get("op").str() {
                "version" => assert_eq!(
                    ncch.version() as i64,
                    expected_outcome.value.i64(),
                    "{context}"
                ),
                "is_decrypted" => assert_eq!(
                    ncch.is_decrypted().unwrap(),
                    expected_outcome.value.bool(),
                    "{context}"
                ),
                "has_file" => match ncch.has_file(operation.get("path").str()) {
                    Ok(found) => assert_eq!(found, expected_outcome.value.bool(), "{context}"),
                    Err(_) => assert_eq!(expected_outcome.result, "error", "{context}"),
                },
                "read_file" => check_optional_bytes(
                    ncch.get_file(operation.get("path").str()),
                    expected,
                    &context,
                ),
                "write_file" => check_unit(
                    ncch.write_file(operation.get("path").str(), &operation.get("data").bytes()),
                    expected,
                    &context,
                ),
                "read_code" => check_optional_bytes(ncch.get_code().map(Some), expected, &context),
                "write_code" => check_unit(
                    ncch.write_code(&operation.get("data").bytes()),
                    expected,
                    &context,
                ),
                other => panic!("unknown operation {other}"),
            }
        }
        if case.has("saved") {
            let saved = ncch
                .save_as_ncch(
                    Cursor::new(Vec::new()),
                    case.get("acronym").str(),
                    case.get("seed").i64(),
                )
                .map(Cursor::into_inner);
            let expected = outcome(case.get("saved"));
            match expected.result {
                "ok" => assert_bytes_eq(
                    &saved.unwrap(),
                    &expected.value.bytes(),
                    &format!("{name} saved NCCH"),
                ),
                _ => assert!(saved.is_err(), "{name}: expected save failure"),
            }
        }
        let layered = ncch.layered_fs_entries();
        let expected_layered = outcome(case.get("layered_fs"));
        match expected_layered.result {
            "ok" => {
                let prefix = format!("{}/", title_id.as_deref().unwrap());
                let mut actual: Vec<(String, Vec<u8>)> = layered
                    .unwrap()
                    .into_iter()
                    .map(|(path, data)| {
                        let components: Vec<String> = path
                            .components()
                            .map(|component| component.as_os_str().to_string_lossy().into_owned())
                            .collect();
                        (components.join("/"), data)
                    })
                    .collect();
                actual.sort();
                let expected: Vec<(String, Vec<u8>)> = expected_layered
                    .value
                    .array()
                    .iter()
                    .map(|entry| {
                        let path = entry.get("path").str();
                        (
                            path.strip_prefix(&prefix).unwrap_or(path).to_owned(),
                            entry.get("data").bytes(),
                        )
                    })
                    .collect();
                assert_eq!(actual, expected, "{name} layeredfs");
            }
            _ => assert!(layered.is_err(), "{name}: expected layeredfs failure"),
        }
        let expected_diagnostics: Vec<String> = case
            .get("diagnostics")
            .array()
            .iter()
            .map(|line| line.str().to_owned())
            .filter(|line| !line.is_empty())
            .collect();
        match ncch.diagnostics() {
            Ok(snapshot) => {
                let lines = diagnostics_lines(case.get("file_name").str(), &snapshot, None);
                assert_eq!(lines, expected_diagnostics, "{name} diagnostics");
            }
            Err(_) => assert_eq!(
                outcome(case.get("diagnostics_outcome")).result,
                "error",
                "{name}"
            ),
        }
    }
}
