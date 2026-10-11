use randomon_data::Type;
use randomon_nds::{
    extend_arm9, find, read_relative_pointer, rom_code_from_bytes, signed_byte,
    type_tm_palette_number, version_from_bytes, write_relative_pointer,
};

#[test]
fn find_follows_upstream_result_codes() {
    let data = [0x00, 0x12, 0x34, 0x56, 0x12, 0x34];
    assert_eq!(find(&data, "3456"), 2);
    assert_eq!(find(&data, "1234"), -2);
    assert_eq!(find(&data, "7777"), -1);
    assert_eq!(find(&data, "123"), -3);
}

#[test]
fn relative_pointers_round_trip() {
    let mut data = vec![0u8; 16];
    write_relative_pointer(&mut data, 4, 2);
    assert_eq!(read_relative_pointer(&data, 4), 2);
    assert_eq!(signed_byte(&[0xFF], 0), -1);
}

#[test]
fn rom_header_fields_are_detected() {
    let mut header = vec![0u8; 0x20];
    header[0x0C..0x10].copy_from_slice(b"IREO");
    header[0x1E] = 1;
    assert_eq!(rom_code_from_bytes(&header).as_deref(), Some("IREO"));
    assert_eq!(version_from_bytes(&header), Some(1));
    assert_eq!(rom_code_from_bytes(&header[..8]), None);
}

#[test]
fn palettes_match_upstream_table() {
    assert_eq!(type_tm_palette_number(Some(Type::Bug)), 610);
    assert_eq!(type_tm_palette_number(Some(Type::Ghost)), 411);
    assert_eq!(type_tm_palette_number(None), 411);
}

#[test]
fn arm9_extension_grows_the_itcm_section() {
    let arm9_offset: i32 = 0x0200_0000;
    let prefix = "AABBCCDD";
    let mut arm9 = vec![0u8; 0x80];
    arm9[0x10..0x14].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    let pointers = 0x14;
    let destination_pointers: i32 = 0x40;
    arm9[pointers..pointers + 4]
        .copy_from_slice(&(arm9_offset + destination_pointers).to_le_bytes());
    arm9[pointers + 8..pointers + 12].copy_from_slice(&(arm9_offset + 0x50).to_le_bytes());
    arm9[0x44..0x48].copy_from_slice(&0x10i32.to_le_bytes());
    arm9[0x60] = 0x99;
    let extended = extend_arm9(&arm9, 8, prefix, arm9_offset);
    assert_eq!(extended.len(), 0x88);
    assert_eq!(extended[0x68], 0x99);
    assert_eq!(
        i32::from_le_bytes(extended[0x44..0x48].try_into().unwrap()),
        0x18
    );
    assert_eq!(
        i32::from_le_bytes(extended[pointers + 4..pointers + 8].try_into().unwrap()),
        arm9_offset + 0x88
    );
}
