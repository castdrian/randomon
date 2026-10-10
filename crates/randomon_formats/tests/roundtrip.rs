mod common;

use std::io::Cursor;

use common::{outcome, vector};
use randomon_formats::compression::blz::{self, BlzMode};
use randomon_formats::compression::lzss::{self, LzssDecoded};
use randomon_formats::ctr::{Amx, Decompression, GarcArchive, Ncch, container, mini};
use randomon_formats::ips::apply_patch;
use randomon_formats::nds::{NarcArchive, NdsRom};
use randomon_formats::search::{HexFind, find_hex, find_hex_multiple, write_hex_string};

fn pseudo_random(seed: u64, length: usize, alphabet: u64) -> Vec<u8> {
    let mut state = seed;
    (0..length)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) % alphabet) as u8
        })
        .collect()
}

#[test]
fn narc_round_trips_files() {
    let mut narc = NarcArchive::new();
    for index in 0..9 {
        narc.files
            .push(pseudo_random(index, index as usize * 5, 256));
    }
    let bytes = narc.to_bytes();
    let parsed = NarcArchive::parse(&bytes).unwrap();
    assert_eq!(parsed.files, narc.files);
    assert_eq!(parsed.to_bytes(), bytes);
}

#[test]
fn mini_round_trips_files() {
    let files: Vec<Vec<u8>> = (0..6)
        .map(|index| pseudo_random(index, index as usize * 3 + 1, 256))
        .collect();
    let packed = mini::pack(&files, "EA").unwrap();
    let unpacked = mini::unpack(&packed, "EA").unwrap().unwrap();
    for (original, restored) in files.iter().zip(&unpacked) {
        assert_eq!(&restored[..original.len()], original.as_slice());
        assert!(restored[original.len()..].iter().all(|&byte| byte == 0));
    }
    assert_eq!(mini::unpack(&packed, "WD").unwrap(), None);
}

#[test]
fn lzss_and_blz_round_trip_compressible_data() {
    for seed in 0..4 {
        let data = pseudo_random(seed, 5000 + seed as usize * 777, 3);
        let encoded = lzss::encode(&data).unwrap().unwrap();
        assert_eq!(
            lzss::decode(&encoded).unwrap(),
            LzssDecoded::Data(data.clone())
        );
        for mode in [BlzMode::Normal, BlzMode::Best] {
            let packed = blz::encode(&data, false, mode).unwrap();
            assert_eq!(blz::decode(&packed).unwrap(), data);
        }
    }
    let arm9 = pseudo_random(9, 0x4000 + 4000, 4);
    let packed = blz::encode(&arm9, true, BlzMode::Normal).unwrap();
    assert_eq!(&packed[..0x100], &arm9[..0x100]);
    assert_eq!(blz::decode(&packed).unwrap(), arm9);
}

#[test]
fn garc_round_trips_single_file_directories() {
    let input = vector("garc").get("cases").array()[2].get("input").bytes();
    let garc = GarcArchive::parse(&input, Decompression::All).unwrap();
    let rebuilt = garc.to_bytes().unwrap();
    let reparsed = GarcArchive::parse(&rebuilt, Decompression::All).unwrap();
    assert_eq!(reparsed.files, garc.files);
    assert_eq!(reparsed.to_bytes().unwrap(), rebuilt);
}

#[test]
fn amx_round_trips_scripts() {
    for case in vector("amx").get("cases").array() {
        if outcome(case.get("outcome")).result != "ok" {
            continue;
        }
        let amx = Amx::parse(&case.get("input").bytes()).unwrap();
        let reparsed = Amx::parse(&amx.to_bytes().unwrap()).unwrap();
        assert_eq!(reparsed.dec_data, amx.dec_data);
        assert_eq!(reparsed.to_bytes().unwrap(), amx.to_bytes().unwrap());
    }
}

#[test]
fn ips_applies_records_and_rle() {
    let mut rom = vec![0u8; 16];
    let patch = b"PATCH\x00\x00\x02\x00\x02\xAA\xBB\x00\x00\x08\x00\x00\x00\x03\x77EOF";
    apply_patch(&mut rom, patch).unwrap();
    assert_eq!(
        &rom[..12],
        &[0, 0, 0xAA, 0xBB, 0, 0, 0, 0, 0x77, 0x77, 0x77, 0]
    );
}

#[test]
fn hex_find_reports_upstream_codes() {
    let data = [1u8, 2, 3, 4, 1, 2, 9, 9];
    assert_eq!(find_hex(&data, "0304").unwrap(), HexFind::Unique(2));
    assert_eq!(find_hex(&data, "0102").unwrap().upr_code(), -2);
    assert_eq!(find_hex(&data, "0505").unwrap().upr_code(), -1);
    assert_eq!(find_hex(&data, "050").unwrap().upr_code(), -3);
    assert_eq!(find_hex_multiple(&data, "0102").unwrap(), vec![0, 4]);
    let mut target = [0u8; 4];
    write_hex_string(&mut target, "A0B1", 1).unwrap();
    assert_eq!(target, [0, 0xA0, 0xB1, 0]);
}

#[test]
fn saved_nds_rom_reopens_with_written_contents() {
    let vectors = vector("nds_rom");
    let case = &vectors.get("cases").array()[0];
    let mut rom = NdsRom::open(Cursor::new(case.get("input").bytes())).unwrap();
    let replacement = pseudo_random(3, 123, 256);
    rom.write_file("data/foo.bin", &replacement).unwrap();
    let overlay = pseudo_random(4, 61, 256);
    rom.write_overlay(1, &overlay).unwrap();
    let arm9 = rom.get_arm9().unwrap();
    let compressed_overlay = rom.get_overlay(0).unwrap().unwrap();
    let saved = rom.save_to(Cursor::new(Vec::new())).unwrap().into_inner();
    let mut reopened = NdsRom::open(Cursor::new(saved)).unwrap();
    assert_eq!(
        reopened.get_file("data/foo.bin").unwrap().unwrap(),
        replacement
    );
    assert_eq!(reopened.get_overlay(1).unwrap().unwrap(), overlay);
    assert_eq!(
        reopened.get_overlay(0).unwrap().unwrap(),
        compressed_overlay
    );
    assert_eq!(reopened.get_arm9().unwrap(), arm9);
}

#[test]
fn saved_ncch_reopens_with_written_contents() {
    let vectors = vector("ncch");
    let case = &vectors.get("cases").array()[3];
    let input = case.get("input").bytes();
    let mut cursor = Cursor::new(input.clone());
    let product_code = container::product_code(&mut cursor).unwrap();
    let title_id = container::title_id(&mut cursor).unwrap();
    let mut ncch = Ncch::open(Cursor::new(input), product_code.clone(), title_id.clone()).unwrap();
    let replacement = pseudo_random(5, 0x777, 256);
    ncch.write_file("a/0/1", &replacement).unwrap();
    let mut code = ncch.get_code().unwrap();
    code[0x40] ^= 0xFF;
    ncch.write_code(&code).unwrap();
    let untouched = ncch.get_file("sound/bgm.bcsar").unwrap().unwrap();
    let saved = ncch
        .save_as_ncch(Cursor::new(Vec::new()), "XY", 7)
        .unwrap()
        .into_inner();
    let mut reopened = Ncch::open(Cursor::new(saved), product_code, title_id).unwrap();
    assert_eq!(reopened.get_file("a/0/1").unwrap().unwrap(), replacement);
    assert_eq!(
        reopened.get_file("sound/bgm.bcsar").unwrap().unwrap(),
        untouched
    );
    assert_eq!(reopened.get_code().unwrap(), code);
    let descriptions = reopened.smdh().unwrap().short_descriptions().unwrap();
    assert!(
        descriptions
            .iter()
            .all(|text| text == "XY randomizer seed: 7")
    );
}
