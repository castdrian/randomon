use std::collections::BTreeMap;

const HEADER_SIZE: usize = 0x4000;
const ARM9_RAM_ADDRESS: u32 = 0x0200_0000;
const ARM7_SIZE: usize = 0x200;
const BANNER_SIZE: usize = 0x840;
const ALIGNMENT: usize = 0x200;

pub struct NdsImage {
    pub code: String,
    pub version: u8,
    pub arm9: Vec<u8>,
    pub overlays: Vec<Vec<u8>>,
    pub files: BTreeMap<String, Vec<u8>>,
}

#[derive(Default)]
struct Directory {
    name: String,
    parent: usize,
    subdirectories: BTreeMap<String, usize>,
    files: Vec<String>,
    first_file_id: u16,
}

fn put_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn align(value: usize) -> usize {
    value.div_ceil(ALIGNMENT) * ALIGNMENT
}

fn directory_tree(paths: &[&String]) -> Vec<Directory> {
    let mut directories = vec![Directory::default()];
    for path in paths {
        let mut parts: Vec<&str> = path.split('/').collect();
        let file_name = parts.pop().unwrap_or_default();
        let mut current = 0;
        for part in parts {
            current = match directories[current].subdirectories.get(part) {
                Some(&existing) => existing,
                None => {
                    directories.push(Directory {
                        name: part.to_owned(),
                        parent: current,
                        ..Directory::default()
                    });
                    let created = directories.len() - 1;
                    directories[current]
                        .subdirectories
                        .insert(part.to_owned(), created);
                    created
                }
            };
        }
        directories[current].files.push(file_name.to_owned());
    }
    directories
}

fn assign_file_ids(directories: &mut [Directory], first_id: u16) -> Vec<(usize, String)> {
    let mut order = Vec::new();
    let mut next_id = first_id;
    for (index, directory) in directories.iter_mut().enumerate() {
        directory.first_file_id = next_id;
        for file in &directory.files {
            order.push((index, file.clone()));
            next_id += 1;
        }
    }
    order
}

fn full_path(directories: &[Directory], index: usize, file: &str) -> String {
    let mut parts = vec![file.to_owned()];
    let mut current = index;
    while current != 0 {
        parts.insert(0, directories[current].name.clone());
        current = directories[current].parent;
    }
    parts.join("/")
}

fn file_name_table(directories: &[Directory]) -> Vec<u8> {
    let mut subtables: Vec<Vec<u8>> = Vec::new();
    for directory in directories {
        let mut table = Vec::new();
        for file in &directory.files {
            table.push(file.len() as u8);
            table.extend_from_slice(file.as_bytes());
        }
        for (name, &id) in &directory.subdirectories {
            table.push(name.len() as u8 | 0x80);
            table.extend_from_slice(name.as_bytes());
            table.extend_from_slice(&(0xF000 + id as u16).to_le_bytes());
        }
        table.push(0);
        subtables.push(table);
    }
    let mut fnt = vec![0u8; directories.len() * 8];
    let mut offset = fnt.len();
    for (index, (directory, table)) in directories.iter().zip(&subtables).enumerate() {
        put_u32(&mut fnt, index * 8, offset as u32);
        put_u16(&mut fnt, index * 8 + 4, directory.first_file_id);
        let parent = if index == 0 {
            directories.len() as u16
        } else {
            0xF000 + directory.parent as u16
        };
        put_u16(&mut fnt, index * 8 + 6, parent);
        offset += table.len();
    }
    for table in subtables {
        fnt.extend_from_slice(&table);
    }
    fnt
}

impl NdsImage {
    pub fn build(&self) -> Vec<u8> {
        let paths: Vec<&String> = self.files.keys().collect();
        let mut directories = directory_tree(&paths);
        let order = assign_file_ids(&mut directories, self.overlays.len() as u16);
        let fnt = file_name_table(&directories);
        let file_count = self.overlays.len() + order.len();
        let mut rom = vec![0u8; HEADER_SIZE];
        rom[..12].copy_from_slice(b"SYNTHETICGEN");
        rom[0x0C..0x10].copy_from_slice(self.code.as_bytes());
        rom[0x1E] = self.version;
        let arm9_offset = rom.len();
        rom.extend_from_slice(&self.arm9);
        rom.extend_from_slice(&[0u8; 12]);
        let overlay_table_offset = align(rom.len());
        rom.resize(overlay_table_offset + self.overlays.len() * 32, 0);
        let arm7_offset = align(rom.len());
        rom.resize(arm7_offset + ARM7_SIZE, 0x7);
        let banner_offset = align(rom.len());
        rom.resize(banner_offset + BANNER_SIZE, 0);
        let fnt_offset = align(rom.len());
        rom.resize(fnt_offset, 0);
        rom.extend_from_slice(&fnt);
        let fat_offset = align(rom.len());
        rom.resize(fat_offset + file_count * 8, 0);
        let contents: Vec<&Vec<u8>> =
            self.overlays
                .iter()
                .chain(order.iter().map(|(directory, file)| {
                    &self.files[&full_path(&directories, *directory, file)]
                }))
                .collect();
        for (file_id, data) in contents.iter().enumerate() {
            let start = align(rom.len());
            rom.resize(start, 0);
            rom.extend_from_slice(data);
            put_u32(&mut rom, fat_offset + file_id * 8, start as u32);
            put_u32(
                &mut rom,
                fat_offset + file_id * 8 + 4,
                (start + data.len()) as u32,
            );
        }
        for (overlay_id, data) in self.overlays.iter().enumerate() {
            let entry = overlay_table_offset + overlay_id * 32;
            put_u32(&mut rom, entry, overlay_id as u32);
            put_u32(
                &mut rom,
                entry + 4,
                0x0210_0000 + overlay_id as u32 * 0x1000,
            );
            put_u32(&mut rom, entry + 8, data.len() as u32);
            put_u32(&mut rom, entry + 24, overlay_id as u32);
        }
        let end = align(rom.len());
        rom.resize(end, 0);
        put_u32(&mut rom, 0x20, arm9_offset as u32);
        put_u32(&mut rom, 0x24, ARM9_RAM_ADDRESS);
        put_u32(&mut rom, 0x28, ARM9_RAM_ADDRESS);
        put_u32(&mut rom, 0x2C, self.arm9.len() as u32);
        put_u32(&mut rom, 0x30, arm7_offset as u32);
        put_u32(&mut rom, 0x38, 0x0238_0000);
        put_u32(&mut rom, 0x3C, ARM7_SIZE as u32);
        put_u32(&mut rom, 0x40, fnt_offset as u32);
        put_u32(&mut rom, 0x44, fnt.len() as u32);
        put_u32(&mut rom, 0x48, fat_offset as u32);
        put_u32(&mut rom, 0x4C, (file_count * 8) as u32);
        put_u32(&mut rom, 0x50, overlay_table_offset as u32);
        put_u32(&mut rom, 0x54, (self.overlays.len() * 32) as u32);
        put_u32(&mut rom, 0x58, 0);
        put_u32(&mut rom, 0x5C, 0);
        put_u32(&mut rom, 0x68, banner_offset as u32);
        put_u32(&mut rom, 0x80, end as u32);
        put_u32(&mut rom, 0x84, HEADER_SIZE as u32);
        rom
    }
}
