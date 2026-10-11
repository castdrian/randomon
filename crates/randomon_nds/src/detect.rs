use randomon_formats::java::strings::decode_us_ascii;

const ROM_CODE_OFFSET: usize = 0x0C;
const ROM_CODE_LENGTH: usize = 4;
const VERSION_OFFSET: usize = 0x1E;

pub fn rom_code_from_bytes(rom: &[u8]) -> Option<String> {
    rom.get(ROM_CODE_OFFSET..ROM_CODE_OFFSET + ROM_CODE_LENGTH)
        .map(decode_us_ascii)
}

pub fn version_from_bytes(rom: &[u8]) -> Option<i32> {
    rom.get(VERSION_OFFSET).map(|&byte| i32::from(byte as i8))
}
