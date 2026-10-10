use crate::compression::ds;
use crate::error::Result;

pub const HEADER_CHECKSUM_OFFSET: usize = 0xBD;
pub const ROM_CODE_OFFSET: usize = 0xAC;
pub const ROM_NAME_OFFSET: usize = 0xA0;
pub const ROM_VERSION_OFFSET: usize = 0xBC;

pub fn decompress_lz77(rom: &[u8], offset: usize) -> Result<Option<Vec<u8>>> {
    ds::decompress_at(rom, offset)
}
