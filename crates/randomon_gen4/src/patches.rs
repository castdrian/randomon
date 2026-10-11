macro_rules! patch_table {
    ($($name:literal),+ $(,)?) => {
        #[cfg(test)]
        pub const PATCH_NAMES: &[&str] = &[$($name,)+];

        pub fn patch_bytes(name: &str) -> Option<&'static [u8]> {
            match name {
                $($name => Some(include_bytes!(concat!("../assets/patches/", $name, ".ips"))),)+
                _ => None,
            }
        }
    };
}

patch_table!(
    "hardcoded_statics/roamers/hgss_roamers",
    "hardcoded_statics/roamers/plat_roamers",
    "hgss_catching_tutorialfix",
    "instant_text/dp_instant_text",
    "instant_text/hgss_instant_text",
    "instant_text/plat_instant_text",
    "musicfix/diamond_musicfix",
    "musicfix/plat_musicfix",
    "national_dex/dp_national_dex",
    "national_dex/hgss_national_dex",
    "national_dex/plat_national_dex",
    "pt_fast_distortion_world",
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom_entry::rom_entries;

    #[test]
    fn every_tweak_the_offsets_reference_is_embedded() {
        for entry in rom_entries() {
            for name in entry.tweak_files.values() {
                assert!(patch_bytes(name).is_some(), "{}: {name}", entry.name);
            }
        }
    }

    #[test]
    fn embedded_patches_are_ips_files() {
        for name in PATCH_NAMES {
            let bytes = patch_bytes(name).expect("embedded");
            assert!(bytes.starts_with(b"PATCH"), "{name}");
            assert!(bytes.windows(3).any(|window| window == b"EOF"), "{name}");
        }
    }
}
