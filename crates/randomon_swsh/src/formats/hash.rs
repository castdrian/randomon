pub const FNV_OFFSET_BASIS: u64 = 0xCBF2_9CE4_8422_2645;
pub const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;
pub const EMPTY_HASH: u64 = FNV_OFFSET_BASIS;

pub fn fnv1a_64_units(units: impl IntoIterator<Item = u64>) -> u64 {
    units.into_iter().fold(FNV_OFFSET_BASIS, |hash, unit| {
        (hash ^ unit).wrapping_mul(FNV_PRIME)
    })
}

pub fn fnv1a_64_bytes(bytes: &[u8]) -> u64 {
    fnv1a_64_units(bytes.iter().map(|&byte| u64::from(byte)))
}

pub fn fnv1a_64(text: &str) -> u64 {
    fnv1a_64_units(text.encode_utf16().map(u64::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_hashes_to_the_offset_basis() {
        assert_eq!(fnv1a_64(""), EMPTY_HASH);
        assert_eq!(fnv1a_64_bytes(&[]), EMPTY_HASH);
    }

    #[test]
    fn uses_the_game_freak_offset_basis() {
        assert_eq!(fnv1a_64("a"), 0xAF66_BC4C_8606_CF2C);
        assert_eq!(fnv1a_64("foobar"), 0x09D3_072C_DA4F_5808);
    }

    #[test]
    fn ascii_text_hashes_like_its_bytes() {
        assert_eq!(fnv1a_64("encount_t.bin"), fnv1a_64_bytes(b"encount_t.bin"));
        assert_eq!(fnv1a_64("encount_t.bin"), 15_989_386_484_782_987_925);
    }

    #[test]
    fn non_ascii_text_hashes_utf16_code_units() {
        let units = "é".encode_utf16().map(u64::from);
        assert_eq!(fnv1a_64("é"), fnv1a_64_units(units));
        assert_ne!(fnv1a_64("é"), fnv1a_64_bytes("é".as_bytes()));
    }
}
