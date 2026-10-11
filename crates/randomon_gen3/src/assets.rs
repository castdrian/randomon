pub const OFFSETS_INI: &str = include_str!("../assets/config/gen3_offsets.ini");

pub fn table_text(name: &str) -> Option<&'static str> {
    match name {
        "gba_english" => Some(include_str!("../assets/config/gba_english.tbl")),
        "gba_jpn" => Some(include_str!("../assets/config/gba_jpn.tbl")),
        _ => None,
    }
}

macro_rules! patches {
    ($($name:literal),+ $(,)?) => {
        pub fn patch_bytes(name: &str) -> Option<&'static [u8]> {
            match name {
                $($name => Some(include_bytes!(concat!("../assets/patches/", $name, ".ips"))),)+
                _ => None,
            }
        }
    };
}

patches!(
    "hardcoded_statics/em_firstbattle",
    "hardcoded_statics/rs_firstbattle",
    "hardcoded_statics/fr_marowak_10",
    "hardcoded_statics/fr_marowak_11",
    "hardcoded_statics/lg_marowak_10",
    "hardcoded_statics/lg_marowak_11",
    "hardcoded_statics/roamers/fr_roamers_10",
    "hardcoded_statics/roamers/fr_roamers_11",
    "hardcoded_statics/roamers/lg_roamers_10",
    "hardcoded_statics/roamers/lg_roamers_11",
    "instant_text/em_instant_text",
    "instant_text/fr_10_instant_text",
    "instant_text/fr_11_instant_text",
    "instant_text/lg_10_instant_text",
    "instant_text/lg_11_instant_text",
    "instant_text/ruby_10_instant_text",
    "instant_text/ruby_11_instant_text",
    "instant_text/ruby_12_instant_text",
    "instant_text/sapphire_10_instant_text",
    "instant_text/sapphire_11_instant_text",
    "instant_text/sapphire_12_instant_text",
    "musicfix/em_musicfix",
    "musicfix/fr_musicfix_10",
    "musicfix/fr_musicfix_11",
    "musicfix/lg_musicfix_10",
    "musicfix/lg_musicfix_11",
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom_entry::rom_entries;

    #[test]
    fn every_referenced_asset_is_embedded() {
        for entry in rom_entries() {
            for patch in entry.code_tweaks.values() {
                let bytes = patch_bytes(patch).unwrap_or_else(|| panic!("missing {patch}"));
                assert!(bytes.starts_with(b"PATCH"), "{patch} is an IPS file");
                assert!(bytes.ends_with(b"EOF"), "{patch} is terminated");
            }
            let table = entry
                .table_file
                .as_deref()
                .expect("every entry has a table");
            assert!(table_text(table).is_some(), "missing {table}");
        }
    }
}
