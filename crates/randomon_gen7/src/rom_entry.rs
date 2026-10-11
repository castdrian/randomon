use std::sync::OnceLock;

use randomon_ctr::{
    CtrIdentity, CtrRomEntry, KeyPrecedence, TYPE_SM, TYPE_USUM, find_entry, parse_rom_entries,
};

const OFFSETS_INI: &str = include_str!("../assets/config/gen7_offsets.ini");

pub fn rom_entries() -> &'static [CtrRomEntry] {
    static ENTRIES: OnceLock<Vec<CtrRomEntry>> = OnceLock::new();
    ENTRIES.get_or_init(|| parse_rom_entries(OFFSETS_INI, KeyPrecedence::NumbersBeforeArrays))
}

pub fn entry_for(identity: &CtrIdentity) -> Option<&'static CtrRomEntry> {
    find_entry(
        rom_entries(),
        identity.product_code.as_deref(),
        identity.title_id.as_deref(),
    )
}

pub fn rom_type(entry: &CtrRomEntry) -> i32 {
    if entry.type_name.eq_ignore_ascii_case("USUM") {
        TYPE_USUM
    } else {
        TYPE_SM
    }
}
