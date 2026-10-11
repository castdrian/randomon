mod common;

use common::{Json, assert_bytes_eq, hex_sha256, outcome, upstream_dir, vector};
use randomon_formats::ctr::{Amx, Bflim, Smdh, mini};
use randomon_formats::ips::apply_patch;
use randomon_formats::java::sort::java_list_sort;
use randomon_formats::nds::{NarcArchive, crc16};
use randomon_formats::search::{free_space_finder, search_range};

fn synthetic_rom(size: usize) -> Vec<u8> {
    (0..size)
        .map(|index| ((index * 31) ^ (index >> 9)) as u8)
        .collect()
}

#[test]
fn jdk_used_for_vectors_returns_interned_empty_string_from_trim() {
    assert!(vector("jdk").get("trimmed_empty_is_literal").bool());
}

#[test]
fn kmp_search_matches_upstream() {
    let vectors = vector("search");
    for (index, case) in vectors.get("search").array().iter().enumerate() {
        let haystack = case.get("haystack").bytes();
        let needle = case.get("needle").bytes();
        let found = search_range(
            &haystack,
            case.get("begin").i64() as usize,
            case.get("end").i64() as usize,
            &needle,
        )
        .unwrap();
        let expected: Vec<usize> = case
            .get("found")
            .array()
            .iter()
            .map(|value| value.i64() as usize)
            .collect();
        assert_eq!(found, expected, "search case {index}");
    }
}

#[test]
fn free_space_finder_matches_upstream() {
    let vectors = vector("search");
    for (index, case) in vectors.get("free_space").array().iter().enumerate() {
        let rom = case.get("rom").bytes();
        let result = free_space_finder(
            &rom,
            0xFF,
            case.get("amount").i64() as usize,
            case.get("offset").i64() as usize,
            case.get("aligned").bool(),
        )
        .unwrap();
        assert_eq!(
            result as i64,
            case.get("result").i64(),
            "free space case {index}"
        );
    }
}

#[test]
fn upstream_ips_patches_apply_identically() {
    let patches = upstream_dir().join("patches");
    if !patches.exists() {
        eprintln!(
            "upstream source missing, run `go run ./cmd/fetch-upstream` in tools/ to include this test"
        );
        return;
    }
    let vectors = vector("ips");
    for case in vectors.get("upstream").array() {
        let name = case.get("name").str();
        let patch = std::fs::read(patches.join(format!("{name}.ips"))).unwrap();
        let mut rom = synthetic_rom(case.get("rom_size").i64() as usize);
        let result = apply_patch(&mut rom, &patch);
        let expected = outcome(case.get("outcome"));
        assert_eq!(expected.result, "ok", "{name}");
        result.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(hex_sha256(&rom), expected.value.str(), "{name}");
    }
}

#[test]
fn synthetic_ips_patches_match_upstream() {
    let vectors = vector("ips");
    for (index, case) in vectors.get("synthetic").array().iter().enumerate() {
        let mut rom = case.get("rom").bytes();
        let result = apply_patch(&mut rom, &case.get("patch").bytes());
        let expected = outcome(case.get("outcome"));
        match expected.result {
            "ok" => {
                result.unwrap();
                assert_bytes_eq(&rom, &expected.value.bytes(), &format!("ips case {index}"));
            }
            _ => assert!(result.is_err(), "ips case {index}"),
        }
    }
}

#[test]
fn crc16_matches_upstream() {
    for case in vector("crc16").get("cases").array() {
        assert_eq!(
            crc16::calculate(&case.get("data").bytes()) as i64,
            case.get("crc").i64()
        );
    }
}

#[test]
fn timsort_matches_java_list_sort_with_overflowing_comparator() {
    for (index, case) in vector("timsort").get("cases").array().iter().enumerate() {
        let mut items: Vec<(i64, i64)> = case
            .get("values")
            .array()
            .iter()
            .enumerate()
            .map(|(position, value)| (position as i64, value.i64()))
            .collect();
        let result = java_list_sort(&mut items, |left, right| {
            left.1.wrapping_sub(right.1) as i32
        });
        let expected = outcome(case.get("outcome"));
        match expected.result {
            "ok" => {
                result.unwrap_or_else(|error| panic!("timsort case {index}: {error}"));
                let order: Vec<i64> = items.iter().map(|item| item.0).collect();
                let expected_order: Vec<i64> =
                    expected.value.array().iter().map(Json::i64).collect();
                assert_eq!(order, expected_order, "timsort case {index}");
            }
            _ => assert!(
                result.is_err(),
                "timsort case {index} should violate the contract"
            ),
        }
    }
}

#[test]
fn mini_matches_upstream() {
    let vectors = vector("mini");
    for case in vectors.get("pack").array() {
        let files: Vec<Vec<u8>> = case.get("files").array().iter().map(Json::bytes).collect();
        let packed = mini::pack(&files, case.get("identifier").str()).unwrap();
        assert_eq!(packed, outcome(case.get("outcome")).value.bytes());
    }
    for (index, case) in vectors.get("unpack").array().iter().enumerate() {
        let result = mini::unpack(&case.get("input").bytes(), case.get("identifier").str());
        let expected = outcome(case.get("outcome"));
        match (expected.result, result) {
            ("ok", Ok(Some(files))) => {
                let expected_files: Vec<Vec<u8>> =
                    expected.value.array().iter().map(Json::bytes).collect();
                assert_eq!(files, expected_files, "mini unpack {index}");
            }
            ("null", Ok(None)) | ("error", Err(_)) => {}
            (expected, actual) => {
                panic!("mini unpack {index}: expected {expected}, got {actual:?}")
            }
        }
    }
}

#[test]
fn smdh_matches_upstream() {
    for (index, case) in vector("smdh").get("cases").array().iter().enumerate() {
        let result = Smdh::parse(case.get("input").bytes()).and_then(|mut smdh| {
            smdh.set_all_descriptions(case.get("description").str())?;
            smdh.set_all_publishers(case.get("publisher").str())?;
            Ok(smdh.bytes().to_vec())
        });
        let expected = outcome(case.get("outcome"));
        match expected.result {
            "ok" => assert_bytes_eq(
                &result.unwrap(),
                &expected.value.bytes(),
                &format!("smdh case {index}"),
            ),
            _ => assert!(result.is_err(), "smdh case {index}"),
        }
    }
}

fn check_amx(result: randomon_formats::Result<Option<Amx>>, expected: &Json, context: &str) {
    let expected = outcome(expected);
    match expected.result {
        "ok" => {
            let value = expected.value;
            let amx = result.unwrap_or_else(|error| panic!("{context}: {error}"));
            if value.get("dec_data").is_null() {
                assert!(amx.is_none(), "{context}");
                return;
            }
            let amx = amx.unwrap_or_else(|| panic!("{context}: script missing"));
            assert_eq!(amx.dec_data, value.get("dec_data").bytes(), "{context}");
            assert_eq!(
                amx.script_offset as i64,
                value.get("script_offset").i64(),
                "{context}"
            );
            let bytes = outcome(value.get("bytes"));
            match bytes.result {
                "ok" => assert_bytes_eq(&amx.to_bytes().unwrap(), &bytes.value.bytes(), context),
                _ => assert!(amx.to_bytes().is_err(), "{context}"),
            }
        }
        _ => assert!(result.is_err(), "{context}"),
    }
}

#[test]
fn amx_matches_upstream() {
    let vectors = vector("amx");
    for (index, case) in vectors.get("cases").array().iter().enumerate() {
        let result = Amx::parse(&case.get("input").bytes()).map(Some);
        check_amx(result, case.get("outcome"), &format!("amx case {index}"));
    }
    for (index, case) in vectors.get("embedded").array().iter().enumerate() {
        let result = Amx::find_script(
            &case.get("input").bytes(),
            case.get("script_number").i64() as usize,
        );
        check_amx(
            result,
            case.get("outcome"),
            &format!("embedded amx {index}"),
        );
    }
}

#[test]
fn bflim_matches_upstream() {
    for (index, case) in vector("bflim").get("cases").array().iter().enumerate() {
        let result = Bflim::parse(&case.get("input").bytes()).and_then(|bflim| bflim.image());
        let expected = outcome(case.get("outcome"));
        match expected.result {
            "ok" => {
                let image = result.unwrap();
                let pixels: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_be_bytes())
                    .collect();
                assert_bytes_eq(
                    &pixels,
                    &expected.value.bytes(),
                    &format!("bflim case {index}"),
                );
            }
            _ => assert!(result.is_err(), "bflim case {index}"),
        }
    }
}

#[test]
fn narc_matches_upstream() {
    for (index, case) in vector("narc").get("cases").array().iter().enumerate() {
        let result = NarcArchive::parse(&case.get("input").bytes());
        let expected = outcome(case.get("outcome"));
        match expected.result {
            "ok" => {
                let narc = result.unwrap_or_else(|error| panic!("narc case {index}: {error}"));
                let files: Vec<Vec<u8>> = expected
                    .value
                    .get("files")
                    .array()
                    .iter()
                    .map(Json::bytes)
                    .collect();
                assert_eq!(narc.files, files, "narc case {index}");
                assert_bytes_eq(
                    &narc.to_bytes(),
                    &expected.value.get("bytes").bytes(),
                    &format!("narc case {index}"),
                );
            }
            _ => assert!(result.is_err(), "narc case {index}"),
        }
    }
}

#[test]
fn utf16_decoding_matches_java_replacement_rules() {
    use randomon_formats::java::strings::{decode_utf16le, encode_utf16le, java_trim};
    for case in vector("jdk").get("utf16").array() {
        let decoded = decode_utf16le(&case.get("input").bytes());
        assert_eq!(encode_utf16le(&decoded), case.get("decoded").bytes());
        assert_eq!(
            encode_utf16le(java_trim(&decoded)),
            case.get("trimmed").bytes()
        );
    }
}
