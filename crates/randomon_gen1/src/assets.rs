pub fn table_text(name: &str) -> Option<&'static str> {
    match name {
        "gameboy_jpn" => Some(randomon_gb::GAMEBOY_JPN_TABLE),
        "green_translation" => Some(include_str!("../assets/config/green_translation.tbl")),
        "rby_english" => Some(include_str!("../assets/config/rby_english.tbl")),
        "rby_espita" => Some(include_str!("../assets/config/rby_espita.tbl")),
        "rby_freger" => Some(include_str!("../assets/config/rby_freger.tbl")),
        _ => None,
    }
}

pub fn patch_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "bwexp/rb_en_bwxp" => Some(include_bytes!("../assets/patches/bwexp/rb_en_bwxp.ips")),
        "bwexp/yellow_en_bwxp" => {
            Some(include_bytes!("../assets/patches/bwexp/yellow_en_bwxp.ips"))
        }
        "rb_en_critrate" => Some(include_bytes!("../assets/patches/rb_en_critrate.ips")),
        "rb_en_xaccnerf" => Some(include_bytes!("../assets/patches/rb_en_xaccnerf.ips")),
        "yellow_en_critrate" => Some(include_bytes!("../assets/patches/yellow_en_critrate.ips")),
        "yellow_en_xaccnerf" => Some(include_bytes!("../assets/patches/yellow_en_xaccnerf.ips")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use randomon_data::constants::gb;
    use randomon_gb::{TextTables, length_of_string_at};

    fn english_tables() -> TextTables {
        let mut tables = TextTables::default();
        for name in ["gameboy_jpn", "rby_english"] {
            tables.read_table(table_text(name).expect("table exists"));
        }
        tables
    }

    #[test]
    fn english_text_round_trips() {
        let tables = english_tables();
        let encoded = tables.translate("PIKACHU");
        assert_eq!(encoded, vec![0x8F, 0x88, 0x8A, 0x80, 0x82, 0x87, 0x94]);
        let mut rom = encoded.clone();
        rom.push(gb::STRING_TERMINATOR);
        assert_eq!(tables.read_string(&rom, 0, usize::MAX, false), "PIKACHU");
        assert_eq!(length_of_string_at(&rom, 0, false), 7);
    }

    #[test]
    fn every_offsets_patch_is_embedded() {
        for entry in crate::rom_entry::rom_entries() {
            for patch in entry.tweak_files.values() {
                assert!(patch_bytes(patch).is_some(), "missing {patch}");
            }
            if let Some(table) = &entry.extra_table_file {
                assert!(table_text(table).is_some(), "missing {table}");
            }
        }
    }
}
