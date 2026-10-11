pub fn table_text(name: &str) -> Option<&'static str> {
    match name {
        "gameboy_jpn" => Some(randomon_gb::GAMEBOY_JPN_TABLE),
        "gsc_english" => Some(include_str!("../assets/config/gsc_english.tbl")),
        "gsc_espita" => Some(include_str!("../assets/config/gsc_espita.tbl")),
        "gsc_freger" => Some(include_str!("../assets/config/gsc_freger.tbl")),
        "vietcrystal" => Some(include_str!("../assets/config/vietcrystal.tbl")),
        _ => None,
    }
}

pub fn patch_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "bwexp/gs_en_bwxp" => Some(include_bytes!("../assets/patches/bwexp/gs_en_bwxp.ips")),
        "bwexp/crystal_en_bwxp" => Some(include_bytes!(
            "../assets/patches/bwexp/crystal_en_bwxp.ips"
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use randomon_data::constants::gb;
    use randomon_gb::TextTables;

    #[test]
    fn every_offsets_asset_is_embedded() {
        for entry in crate::rom_entry::rom_entries() {
            for patch in entry.code_tweaks.values() {
                assert!(patch_bytes(patch).is_some(), "missing {patch}");
            }
            if let Some(table) = &entry.extra_table_file {
                assert!(table_text(table).is_some(), "missing {table}");
            }
        }
    }

    #[test]
    fn english_text_round_trips() {
        let mut tables = TextTables::default();
        for name in ["gameboy_jpn", "gsc_english"] {
            tables.read_table(table_text(name).expect("table exists"));
        }
        let mut encoded = tables.translate("CYNDAQUIL");
        assert_eq!(encoded.len(), 9);
        encoded.push(gb::STRING_TERMINATOR);
        assert_eq!(
            tables.read_string(&encoded, 0, usize::MAX, false),
            "CYNDAQUIL"
        );
    }
}
