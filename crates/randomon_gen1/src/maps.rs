use std::collections::HashMap;

use randomon_settings::java_text::java_trim;

use crate::gb::{TextTables, bank_of, calculate_offset, read_word};
use crate::rom_entry::RomEntry;

pub struct MapData {
    pub item_offsets: Vec<Option<Vec<i32>>>,
    pub names: Vec<Option<String>>,
}

fn preload_map(
    rom: &[u8],
    map_banks: i32,
    map_addresses: i32,
    map_id: usize,
    maps: &mut Vec<Option<Vec<i32>>>,
) {
    if maps[map_id].is_some() || map_id == 0xED || map_id == 0xFF {
        return;
    }
    maps[map_id] = Some(Vec::new());
    let address = calculate_offset(
        i32::from(rom[(map_banks + map_id as i32) as usize]),
        read_word(rom, map_addresses + map_id as i32 * 2),
    );
    let bank = bank_of(address);
    let connect_byte = i32::from(rom[(address + 9) as usize]);
    let connection_count = ((connect_byte & 8) >> 3)
        + ((connect_byte & 4) >> 2)
        + ((connect_byte & 2) >> 1)
        + (connect_byte & 1);
    let connections_offset = address + 10;
    for index in 0..connection_count {
        let connection_offset = connections_offset + index * 11;
        let connected = rom[connection_offset as usize] as usize;
        preload_map(rom, map_banks, map_addresses, connected, maps);
    }
    let objects = calculate_offset(
        bank,
        read_word(rom, connections_offset + connection_count * 11),
    );
    let warp_count = i32::from(rom[(objects + 1) as usize]);
    let mut offset = objects + 2;
    for _ in 0..warp_count {
        let target = rom[(offset + 3) as usize] as usize;
        preload_map(rom, map_banks, map_addresses, target, maps);
        offset += 4;
    }
    let sign_count = i32::from(rom[offset as usize]);
    offset += 1;
    offset += sign_count * 3;
    let entity_count = i32::from(rom[offset as usize]);
    offset += 1;
    let mut items = Vec::new();
    for _ in 0..entity_count {
        let text_id = i32::from(rom[(offset + 5) as usize]);
        if (text_id & (1 << 6)) > 0 {
            offset += 8;
        } else if (text_id & (1 << 7)) > 0 && rom[(offset + 6) as usize] != 0x00 {
            items.push(offset + 6);
            offset += 7;
        } else {
            offset += 6;
        }
    }
    maps[map_id] = Some(items);
}

fn load_map_names(
    rom: &[u8],
    entry: &RomEntry,
    tables: &TextTables,
    maps: &[Option<Vec<i32>>],
) -> Vec<Option<String>> {
    let mut names: Vec<Option<String>> = vec![None; 256];
    let mut table_offset = entry.value("MapNameTableOffset");
    let bank = bank_of(table_offset);
    let mut used_external: Vec<i32> = Vec::new();
    for name in names.iter_mut().take(0x25) {
        let external = calculate_offset(bank, read_word(rom, table_offset + 1));
        used_external.push(external);
        *name = Some(tables.read_string(rom, external, usize::MAX, false));
        table_offset += 3;
    }
    let mut last_max_map = 0x25usize;
    let mut previous_counts: HashMap<i32, i32> = HashMap::new();
    while rom[table_offset as usize] != 0xFF {
        let max_map = rom[table_offset as usize] as usize;
        let name_offset = calculate_offset(bank, read_word(rom, table_offset + 2));
        let actual = java_trim(&tables.read_string(rom, name_offset, usize::MAX, false)).to_owned();
        if used_external.contains(&name_offset) {
            for index in last_max_map..max_map {
                if maps[index].is_some() {
                    names[index] = Some(format!("{actual} (Building)"));
                }
            }
        } else {
            let mut count = previous_counts.get(&name_offset).copied().unwrap_or(0);
            for index in last_max_map..max_map {
                if maps[index].is_some() {
                    count += 1;
                    names[index] = Some(format!("{actual} ({count})"));
                }
            }
            previous_counts.insert(name_offset, count);
        }
        last_max_map = max_map;
        table_offset += 4;
    }
    names
}

pub fn load_maps(rom: &[u8], entry: &RomEntry, tables: &TextTables) -> MapData {
    let mut maps: Vec<Option<Vec<i32>>> = vec![None; 256];
    preload_map(
        rom,
        entry.value("MapBanks"),
        entry.value("MapAddresses"),
        0,
        &mut maps,
    );
    let names = load_map_names(rom, entry, tables, &maps);
    MapData {
        item_offsets: maps,
        names,
    }
}
