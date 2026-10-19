use std::path::Path;

use randomon_bdsp::{
    BdspFactory, STARTER_OFFSETS, decode_item_argument, encode_item_argument, movz_w,
    starter_patch, starter_patches,
};
use randomon_core::{Ips32Patch, RomHandlerFactory};

#[test]
fn movz_matches_the_assembler() {
    assert_eq!(movz_w(8, 0x1ed), [0xa8, 0x3d, 0x80, 0x52]);
    assert_eq!(movz_w(0, 1), [0x20, 0x00, 0x80, 0x52]);
}

#[test]
fn starter_patches_target_every_supported_build() {
    assert_eq!(STARTER_OFFSETS.len(), 14);
    assert_eq!(starter_patches([1, 4, 7]).len(), 14);
    let patch = starter_patch(&STARTER_OFFSETS[0], [1, 4, 7]);
    let bytes = patch.to_bytes();
    assert!(bytes.starts_with(b"IPS32") && bytes.ends_with(b"EEOF"));
    let parsed = Ips32Patch::parse(&bytes).expect("parses");
    assert_eq!(parsed, patch);
    let (first_offset, first_bytes) = &parsed.records()[0];
    assert_eq!(*first_offset, 0x1BB_BBB0 + 0xC8 + 0x100);
    assert_eq!(first_bytes.as_slice(), movz_w(0, 1));
}

#[test]
fn field_item_arguments_are_float_bits() {
    assert_eq!(decode_item_argument(1_136_033_792), Some(365));
    assert_eq!(decode_item_argument(1_065_353_216), Some(1));
    assert_eq!(encode_item_argument(365), 1_136_033_792);
}

#[test]
fn ordinary_folders_and_files_are_not_bdsp_games() {
    let folder = std::env::temp_dir().join("randomon-bdsp-tests/not-a-romfs");
    std::fs::create_dir_all(&folder).expect("folder");
    assert!(BdspFactory.detect_file(&folder).is_none());
    assert!(
        BdspFactory
            .detect_file(Path::new("/nowhere/game.nds"))
            .is_none()
    );
    assert!(!BdspFactory.is_loadable(&[0; 16]));
}
