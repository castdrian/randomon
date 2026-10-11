pub fn apply_ips_patch(rom: &mut [u8], patch: &[u8]) -> Result<(), String> {
    if patch.len() < 8 || &patch[..5] != b"PATCH" {
        return Err("not a valid IPS file".to_owned());
    }
    let read_offset = |position: usize| {
        (usize::from(patch[position]) << 16)
            | (usize::from(patch[position + 1]) << 8)
            | usize::from(patch[position + 2])
    };
    let read_size =
        |position: usize| (usize::from(patch[position]) << 8) | usize::from(patch[position + 1]);
    let mut position = 5;
    while position + 2 < patch.len() {
        let write_offset = read_offset(position);
        if write_offset == 0x454f46 {
            return Ok(());
        }
        position += 3;
        if position + 1 >= patch.len() {
            return Err("abrupt ending to IPS file, entry cut off before size".to_owned());
        }
        let size = read_size(position);
        position += 2;
        if size == 0 {
            if position + 1 >= patch.len() {
                return Err("abrupt ending to IPS file, entry cut off before RLE size".to_owned());
            }
            let rle_size = read_size(position);
            if write_offset + rle_size > rom.len() {
                return Err("trying to patch data past the end of the ROM file".to_owned());
            }
            position += 2;
            if position >= patch.len() {
                return Err("abrupt ending to IPS file, entry cut off before RLE byte".to_owned());
            }
            let value = patch[position];
            position += 1;
            rom[write_offset..write_offset + rle_size].fill(value);
        } else {
            if position + size > patch.len() {
                return Err(
                    "abrupt ending to IPS file, entry cut off before end of data block".to_owned(),
                );
            }
            if write_offset + size > rom.len() {
                return Err("trying to patch data past the end of the ROM file".to_owned());
            }
            rom[write_offset..write_offset + size]
                .copy_from_slice(&patch[position..position + size]);
            position += size;
        }
    }
    Err("improperly terminated IPS file".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ips_patches_apply_records_and_rle() {
        let patch = [
            b'P', b'A', b'T', b'C', b'H', 0, 0, 1, 0, 2, 0xAA, 0xBB, 0, 0, 4, 0, 0, 0, 3, 0xCC,
            b'E', b'O', b'F',
        ];
        let mut rom = vec![0u8; 8];
        apply_ips_patch(&mut rom, &patch).expect("valid patch");
        assert_eq!(rom, vec![0, 0xAA, 0xBB, 0, 0xCC, 0xCC, 0xCC, 0]);
        assert!(apply_ips_patch(&mut rom, b"NOPE....").is_err());
    }
}
