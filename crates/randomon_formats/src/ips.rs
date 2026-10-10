use crate::error::{FormatError, Result};

const CONTEXT: &str = "IPS patch";
const EOF_MARKER: usize = 0x454F46;

pub fn apply_patch(rom: &mut [u8], patch: &[u8]) -> Result<()> {
    if patch.len() < 8 || &patch[..5] != b"PATCH" {
        return Err(FormatError::invalid(CONTEXT, "not a valid IPS file"));
    }
    let length = patch.len();
    let mut offset = 5;
    while offset + 2 < length {
        let write_offset = read_offset(patch, offset);
        if write_offset == EOF_MARKER {
            return Ok(());
        }
        offset += 3;
        if offset + 1 >= length {
            return Err(abrupt("entry cut off before size"));
        }
        let size = read_size(patch, offset);
        offset += 2;
        if size == 0 {
            offset = apply_rle_record(rom, patch, offset, write_offset)?;
        } else {
            if offset + size > length {
                return Err(abrupt("entry cut off before end of data block"));
            }
            if write_offset + size > rom.len() {
                return Err(past_end());
            }
            rom[write_offset..write_offset + size].copy_from_slice(&patch[offset..offset + size]);
            offset += size;
        }
    }
    Err(FormatError::invalid(
        CONTEXT,
        "improperly terminated IPS file",
    ))
}

fn apply_rle_record(
    rom: &mut [u8],
    patch: &[u8],
    offset: usize,
    write_offset: usize,
) -> Result<usize> {
    if offset + 1 >= patch.len() {
        return Err(abrupt("entry cut off before RLE size"));
    }
    let rle_size = read_size(patch, offset);
    if write_offset + rle_size > rom.len() {
        return Err(past_end());
    }
    let offset = offset + 2;
    if offset >= patch.len() {
        return Err(abrupt("entry cut off before RLE byte"));
    }
    rom[write_offset..write_offset + rle_size].fill(patch[offset]);
    Ok(offset + 1)
}

fn read_offset(data: &[u8], offset: usize) -> usize {
    (data[offset] as usize) << 16 | (data[offset + 1] as usize) << 8 | data[offset + 2] as usize
}

fn read_size(data: &[u8], offset: usize) -> usize {
    (data[offset] as usize) << 8 | data[offset + 1] as usize
}

fn abrupt(detail: &str) -> FormatError {
    FormatError::invalid(CONTEXT, format!("abrupt ending to IPS file, {detail}"))
}

fn past_end() -> FormatError {
    FormatError::invalid(CONTEXT, "trying to patch data past the end of the ROM file")
}
