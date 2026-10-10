use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek, SeekFrom, Write};

use super::crc16;
use super::entries::{NdsFile, NdsOverlay, OverlayTableEntry, ScratchFiles};
use crate::bytes::{read_le_sized, write_le_sized};
use crate::compression::blz::{self, BlzMode};
use crate::error::{FormatError, Result};
use crate::io::{RandomAccessFile, non_negative_offset, read_exact_at, read_le_sized_at};
use crate::java::strings::decode_us_ascii;
use crate::search::search;

const CONTEXT: &str = "NDSRom";
const SECTION_ALIGN: i64 = 0x1FF;
const BANNER_SIZE: i64 = 0x840;
const NITROCODE: i32 = 0xDEC00621_u32 as i32;
const COPY_CHUNK: usize = 256 * 1024;

#[derive(Debug, Default)]
struct Arm9State {
    open: bool,
    changed: bool,
    has_footer: bool,
    compressed: bool,
    size_offset: usize,
    footer: Vec<u8>,
    stored: Option<Vec<u8>>,
    original_crc: u32,
}

pub struct NdsRom<R> {
    source: R,
    rom_code: String,
    version: u8,
    arm9_ram_offset: i32,
    fat: Vec<u8>,
    files: Vec<NdsFile>,
    files_by_path: HashMap<String, usize>,
    files_by_id: HashMap<i32, usize>,
    overlays: Vec<NdsOverlay>,
    overlays_by_file_id: HashMap<i32, usize>,
    scratch: ScratchFiles,
    arm9: Arm9State,
}

struct DirectoryTable {
    sub_table_offsets: Vec<i32>,
    first_file_ids: Vec<i32>,
    parent_dir_ids: Vec<i32>,
}

struct FirstPass {
    directory_names: Vec<Option<String>>,
    filenames: BTreeMap<i32, String>,
    file_directories: HashMap<i32, usize>,
}

pub fn read_rom_code<R: Read + Seek>(source: &mut R) -> Result<String> {
    Ok(decode_us_ascii(&read_exact_at(source, 0x0C, 4)?))
}

pub fn read_version<R: Read + Seek>(source: &mut R) -> Result<u8> {
    Ok(read_exact_at(source, 0x1E, 1)?[0])
}

impl<R: Read + Seek> NdsRom<R> {
    pub fn open(mut source: R) -> Result<Self> {
        let rom_code = read_rom_code(&mut source)?;
        let version = read_version(&mut source)?;
        let arm9_ram_offset = read_le_sized_at(&mut source, 0x28, 4)?;
        let fnt_offset = read_le_sized_at(&mut source, 0x40, 4)?;
        let fat_offset = read_le_sized_at(&mut source, 0x48, 4)?;
        let fat_size = read_le_sized_at(&mut source, 0x4C, 4)?;
        let fat = read_exact_at(
            &mut source,
            non_negative_offset(fat_offset as i64, CONTEXT)?,
            usize::try_from(fat_size)
                .map_err(|_| FormatError::invalid(CONTEXT, "negative FAT size"))?,
        )?;

        let mut rom = NdsRom {
            source,
            rom_code,
            version,
            arm9_ram_offset,
            fat,
            files: Vec::new(),
            files_by_path: HashMap::new(),
            files_by_id: HashMap::new(),
            overlays: Vec::new(),
            overlays_by_file_id: HashMap::new(),
            scratch: ScratchFiles::new(),
            arm9: Arm9State::default(),
        };
        rom.read_file_system(fnt_offset)?;
        rom.read_overlay_table()?;
        Ok(rom)
    }

    fn read_file_system(&mut self, fnt_offset: i32) -> Result<()> {
        let dir_count =
            read_le_sized_at(&mut self.source, (fnt_offset as i64 + 6) as u64, 2)? as usize;
        let table = self.read_directory_table(fnt_offset, dir_count)?;
        let first_pass = self.first_pass(&table, dir_count)?;
        let directory_paths = directory_paths(&table, &first_pass.directory_names, dir_count)?;
        for (&file_id, filename) in &first_pass.filenames {
            let directory = first_pass.file_directories[&file_id];
            let dir_path = directory_paths
                .get(&(directory as i32 + 0xF000))
                .ok_or(FormatError::invalid(CONTEXT, "file in unknown directory"))?;
            let full_path = if dir_path.is_empty() {
                filename.clone()
            } else {
                format!("{dir_path}/{filename}")
            };
            let start = self.fat_entry(file_id, 0)?;
            let end = self.fat_entry(file_id, 4)?;
            let index = self.files.len();
            self.files.push(NdsFile::new(
                start,
                end.wrapping_sub(start),
                file_id,
                full_path.clone(),
            ));
            self.files_by_path.insert(full_path, index);
            self.files_by_id.insert(file_id, index);
        }
        Ok(())
    }

    fn fat_entry(&self, file_id: i32, field: i64) -> Result<i32> {
        let offset = file_id as i64 * 8 + field;
        if offset < 0 || offset + 4 > self.fat.len() as i64 {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        Ok(read_le_sized(&self.fat, offset as usize, 4))
    }

    fn read_directory_table(
        &mut self,
        fnt_offset: i32,
        dir_count: usize,
    ) -> Result<DirectoryTable> {
        let mut table = DirectoryTable {
            sub_table_offsets: vec![0; dir_count],
            first_file_ids: vec![0; dir_count],
            parent_dir_ids: vec![0; dir_count],
        };
        let mut position = non_negative_offset(fnt_offset as i64, CONTEXT)?;
        for index in 0..dir_count.min(0x1000) {
            let entry = read_exact_at(&mut self.source, position, 8)?;
            position += 8;
            table.sub_table_offsets[index] = read_le_sized(&entry, 0, 4).wrapping_add(fnt_offset);
            table.first_file_ids[index] = read_le_sized(&entry, 4, 2);
            table.parent_dir_ids[index] = read_le_sized(&entry, 6, 2);
        }
        Ok(table)
    }

    fn first_pass(&mut self, table: &DirectoryTable, dir_count: usize) -> Result<FirstPass> {
        let mut pass = FirstPass {
            directory_names: vec![None; dir_count],
            filenames: BTreeMap::new(),
            file_directories: HashMap::new(),
        };
        for directory in 0..dir_count.min(0x1000) {
            self.first_pass_directory(
                directory,
                table.sub_table_offsets[directory],
                table.first_file_ids[directory],
                &mut pass,
            )?;
        }
        Ok(pass)
    }

    fn first_pass_directory(
        &mut self,
        directory: usize,
        sub_table_offset: i32,
        first_file_id: i32,
        pass: &mut FirstPass,
    ) -> Result<()> {
        let mut position = non_negative_offset(sub_table_offset as i64, CONTEXT)?;
        let mut next_file_id = first_file_id;
        loop {
            let control = read_exact_at(&mut self.source, position, 1)?[0];
            position += 1;
            if control == 0 {
                return Ok(());
            }
            let name_length = (control & 0x7F) as usize;
            let name = decode_us_ascii(&read_exact_at(&mut self.source, position, name_length)?);
            position += name_length as u64;
            if control & 0x80 != 0 {
                let sub_directory_id =
                    read_le_sized(&read_exact_at(&mut self.source, position, 2)?, 0, 2);
                position += 2;
                let slot = pass
                    .directory_names
                    .get_mut((sub_directory_id - 0xF000) as usize)
                    .ok_or(FormatError::out_of_bounds(CONTEXT))?;
                *slot = Some(name);
            } else {
                let file_id = next_file_id;
                next_file_id += 1;
                pass.filenames.insert(file_id, name);
                pass.file_directories.insert(file_id, directory);
            }
        }
    }

    fn read_overlay_table(&mut self) -> Result<()> {
        let table_offset = read_le_sized_at(&mut self.source, 0x50, 4)?;
        let table_size = read_le_sized_at(&mut self.source, 0x54, 4)?;
        let count = table_size / 32;
        let table = read_exact_at(
            &mut self.source,
            non_negative_offset(table_offset as i64, CONTEXT)?,
            usize::try_from(table_size)
                .map_err(|_| FormatError::invalid(CONTEXT, "negative overlay table"))?,
        )?;
        for overlay_id in 0..count.max(0) {
            let base = overlay_id as usize * 32;
            let field = |offset: usize, size: usize| read_le_sized(&table, base + offset, size);
            let file_id = field(24, 4);
            let entry = OverlayTableEntry {
                file_id,
                start: self.fat_entry(file_id, 0)?,
                end: self.fat_entry(file_id, 4)?,
                overlay_id,
                ram_address: field(4, 4),
                ram_size: field(8, 4),
                bss_size: field(12, 4),
                static_start: field(16, 4),
                static_end: field(20, 4),
                compressed_size: field(28, 3),
                compress_flag: table[base + 31] as i32,
            };
            self.overlays_by_file_id
                .insert(file_id, self.overlays.len());
            self.overlays.push(NdsOverlay::new(entry));
        }
        Ok(())
    }

    pub fn code(&self) -> &str {
        &self.rom_code
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn files(&self) -> &[NdsFile] {
        &self.files
    }

    pub fn overlays(&self) -> &[NdsOverlay] {
        &self.overlays
    }

    pub fn has_file(&self, path: &str) -> bool {
        self.files_by_path.contains_key(path)
    }

    pub fn get_file(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        match self.files_by_path.get(path) {
            Some(&index) => self.files[index]
                .contents(&mut self.source, &mut self.scratch)
                .map(Some),
            None => Ok(None),
        }
    }

    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<()> {
        if let Some(&index) = self.files_by_path.get(path) {
            self.files[index].write_override(data, &mut self.source, &mut self.scratch)?;
        }
        Ok(())
    }

    pub fn get_overlay(&mut self, number: i32) -> Result<Option<Vec<u8>>> {
        match self.overlay_index(number) {
            Some(index) => self.overlays[index]
                .contents(&mut self.source, &mut self.scratch)
                .map(Some),
            None => Ok(None),
        }
    }

    pub fn overlay_address(&self, number: i32) -> i32 {
        self.overlay_index(number)
            .map_or(-1, |index| self.overlays[index].ram_address)
    }

    pub fn write_overlay(&mut self, number: i32, data: &[u8]) -> Result<()> {
        if number < 0 || number as usize > self.overlays.len() {
            return Ok(());
        }
        let overlay = self
            .overlays
            .get_mut(number as usize)
            .ok_or(FormatError::out_of_bounds(CONTEXT))?;
        overlay.write_override(data, &mut self.source, &mut self.scratch)
    }

    fn overlay_index(&self, number: i32) -> Option<usize> {
        (number >= 0 && (number as usize) < self.overlays.len()).then_some(number as usize)
    }

    pub fn get_arm9(&mut self) -> Result<Vec<u8>> {
        if self.arm9.open {
            return self
                .arm9
                .stored
                .clone()
                .ok_or_else(|| FormatError::invalid(CONTEXT, "cached arm9 is missing"));
        }
        self.arm9.open = true;
        let arm9_offset = read_le_sized_at(&mut self.source, 0x20, 4)?;
        let arm9_size = read_le_sized_at(&mut self.source, 0x2C, 4)?;
        let arm9_start = non_negative_offset(arm9_offset as i64, CONTEXT)?;
        let mut arm9 = read_exact_at(
            &mut self.source,
            arm9_start,
            usize::try_from(arm9_size)
                .map_err(|_| FormatError::invalid(CONTEXT, "negative arm9 size"))?,
        )?;
        self.arm9.original_crc = crate::bytes::crc32(&arm9);
        let after_arm9 = arm9_start + arm9.len() as u64;
        let nitrocode = read_le_sized_at(&mut self.source, after_arm9, 4)?;
        if nitrocode == NITROCODE {
            let mut footer = vec![0u8; 12];
            write_le_sized(&mut footer, 0, 4, NITROCODE);
            footer[4..].copy_from_slice(&read_exact_at(&mut self.source, after_arm9 + 4, 8)?);
            self.arm9.footer = footer;
            self.arm9.has_footer = true;
        } else {
            self.arm9.has_footer = false;
        }
        arm9 = self.strip_trailing_footers(arm9)?;
        self.detect_arm9_compression(&arm9)?;
        if self.arm9.compressed {
            arm9 = blz::decode(&arm9)?;
        }
        self.arm9.stored = Some(arm9.clone());
        Ok(arm9)
    }

    fn strip_trailing_footers(&mut self, mut arm9: Vec<u8>) -> Result<Vec<u8>> {
        loop {
            if arm9.len() < 12 {
                return Err(FormatError::out_of_bounds(CONTEXT));
            }
            let tail = arm9.len() - 12;
            let first = read_le_sized(&arm9, tail, 4);
            let is_footer = first == NITROCODE
                || (first == 0
                    && read_le_sized(&arm9, tail + 4, 4) == 0
                    && read_le_sized(&arm9, tail + 8, 4) == 0);
            if !is_footer {
                return Ok(arm9);
            }
            if !self.arm9.has_footer {
                self.arm9.has_footer = true;
                self.arm9.footer = Vec::new();
            }
            let mut new_footer = arm9[tail..].to_vec();
            new_footer.extend_from_slice(&self.arm9.footer);
            self.arm9.footer = new_footer;
            arm9.truncate(tail);
        }
    }

    fn detect_arm9_compression(&mut self, arm9: &[u8]) -> Result<()> {
        self.arm9.compressed = false;
        self.arm9.size_offset = 0;
        if arm9.len() < 5 {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        let header_length = arm9[arm9.len() - 5] as i8 as i32;
        if !(0x08..=0x0B).contains(&header_length) {
            return Ok(());
        }
        if arm9.len() < 8 {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        let compressed_size = read_le_sized(arm9, arm9.len() - 8, 3);
        let length = arm9.len() as i32;
        if compressed_size > length * 9 / 10 && compressed_size < length * 11 / 10 {
            self.arm9.compressed = true;
            let mut needle = [0u8; 4];
            write_le_sized(&mut needle, 0, 4, length.wrapping_add(self.arm9_ram_offset));
            let found = search(arm9, &needle)?;
            match found.as_slice() {
                [only] => self.arm9.size_offset = *only,
                _ => {
                    return Err(FormatError::invalid(
                        CONTEXT,
                        "Could not read ARM9 size offset. May be a bad ROM.",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn write_arm9(&mut self, arm9: &[u8]) -> Result<()> {
        if !self.arm9.open {
            self.get_arm9()?;
        }
        self.arm9.changed = true;
        self.arm9.stored = Some(arm9.to_vec());
        Ok(())
    }

    pub fn diagnostics(&self, file_name: &str) -> Vec<String> {
        let mut overlay_lines: Vec<String> = self
            .overlays_by_file_id
            .iter()
            .filter(|&(_, &index)| self.overlays[index].original_crc != 0)
            .map(|(file_id, &index)| {
                format!(
                    "overlay9_{file_id}: {:08X}",
                    self.overlays[index].original_crc
                )
            })
            .collect();
        let mut file_lines: Vec<String> = self
            .files_by_path
            .iter()
            .filter(|&(_, &index)| self.files[index].original_crc != 0)
            .map(|(path, &index)| format!("{path}: {:08X}", self.files[index].original_crc))
            .collect();
        overlay_lines.sort();
        file_lines.sort();
        let mut lines = vec![
            format!("File name: {file_name}"),
            format!("arm9: {:08X}", self.arm9.original_crc),
        ];
        lines.extend(overlay_lines);
        lines.extend(file_lines);
        lines
    }

    pub fn save_to<W: Read + Write + Seek>(&mut self, output: W) -> Result<W> {
        let mut out = RandomAccessFile::new(output);
        let header_size = self.header_field(0x84)?;
        self.copy_region(&mut out, 0, header_size as i64)?;

        let arm9_offset = align(out.pointer() as i64);
        let old_arm9_offset = self.header_field(0x20)?;
        let mut arm9_size = self.header_field(0x2C)?;
        if self.arm9.open && self.arm9.changed {
            let arm9 = self.encoded_arm9()?;
            arm9_size = arm9.len() as i32;
            out.seek(arm9_offset)?;
            out.write(&arm9)?;
            if self.arm9.has_footer {
                out.write(&self.arm9.footer)?;
            }
        } else {
            out.seek(arm9_offset)?;
            self.copy_region(&mut out, old_arm9_offset as i64, arm9_size as i64 + 12)?;
        }

        let arm9_overlay_offset = out.pointer() as i64;
        let arm9_overlay_size = self.overlays.len() as i64 * 32;

        let arm7_offset = align(arm9_overlay_offset + arm9_overlay_size);
        let old_arm7_offset = self.header_field(0x30)?;
        let arm7_size = self.header_field(0x3C)?;
        out.seek(arm7_offset)?;
        self.copy_region(&mut out, old_arm7_offset as i64, arm7_size as i64)?;

        let arm7_overlay_offset = out.pointer() as i64;
        let old_arm7_overlay_offset = self.header_field(0x58)?;
        let arm7_overlay_size = self.header_field(0x5C)?;
        out.seek(arm7_overlay_offset)?;
        self.copy_region(
            &mut out,
            old_arm7_overlay_offset as i64,
            arm7_overlay_size as i64,
        )?;

        let banner_offset = align(out.pointer() as i64);
        let old_banner_offset = self.header_field(0x68)?;
        out.seek(banner_offset)?;
        self.copy_region(&mut out, old_banner_offset as i64, BANNER_SIZE)?;

        let fnt_offset = align(out.pointer() as i64);
        let old_fnt_offset = self.header_field(0x40)?;
        let fnt_size = self.header_field(0x44)?;
        out.seek(fnt_offset)?;
        self.copy_region(&mut out, old_fnt_offset as i64, fnt_size as i64)?;

        let fat_offset = align(out.pointer() as i64);
        let (new_fat, overlay_table, end_of_files) = self.write_files(&mut out, fat_offset)?;
        out.seek(fat_offset)?;
        out.write(&new_fat)?;
        out.seek(arm9_overlay_offset)?;
        out.write(&overlay_table)?;

        let application_end_offset = (end_of_files + 3) & !3;
        if application_end_offset != end_of_files {
            out.seek(application_end_offset - 1)?;
            out.write_byte(0)?;
        }
        let device_capacity = device_capacity(application_end_offset as i32);

        let header_updates: [(i64, usize, i64); 11] = [
            (0x20, 4, arm9_offset),
            (0x2C, 4, arm9_size as i64),
            (0x30, 4, arm7_offset),
            (0x3C, 4, arm7_size as i64),
            (0x40, 4, fnt_offset),
            (0x48, 4, fat_offset),
            (0x50, 4, arm9_overlay_offset),
            (0x58, 4, arm7_overlay_offset),
            (0x68, 4, banner_offset),
            (0x80, 4, application_end_offset),
            (0x14, 1, device_capacity as i64),
        ];
        for (offset, size, value) in header_updates {
            write_sized(&mut out, offset, size, value as i32)?;
        }
        out.seek(0)?;
        let mut header = vec![0u8; 0x15E];
        out.read_fully(&mut header)?;
        let crc = crc16::calculate(&header);
        write_sized(&mut out, 0x15E, 2, crc as i32)?;
        out.flush()?;
        Ok(out.into_inner())
    }

    fn encoded_arm9(&mut self) -> Result<Vec<u8>> {
        let mut arm9 = self.get_arm9()?;
        if self.arm9.compressed {
            arm9 = blz::encode(&arm9, true, BlzMode::Normal)?;
            if self.arm9.size_offset > 0 {
                let value = (arm9.len() as i32).wrapping_add(self.arm9_ram_offset);
                let offset = self.arm9.size_offset;
                if offset + 4 > arm9.len() {
                    return Err(FormatError::out_of_bounds(CONTEXT));
                }
                write_le_sized(&mut arm9, offset, 4, value);
            }
        }
        Ok(arm9)
    }

    fn write_files<W: Read + Write + Seek>(
        &mut self,
        out: &mut RandomAccessFile<W>,
        fat_offset: i64,
    ) -> Result<(Vec<u8>, Vec<u8>, i64)> {
        let mut new_fat = vec![0u8; self.fat.len()];
        let mut overlay_table = vec![0u8; self.overlays.len() * 32];
        let mut base_offset = fat_offset + self.fat.len() as i64;
        let file_count = self.fat.len() / 8;
        for file_id in 0..file_count as i32 {
            let file_offset = align(base_offset);
            let mut file_length: i64 = 0;
            let mut copied_custom = false;
            if let Some(&index) = self.files_by_id.get(&file_id)
                && let Some(contents) =
                    self.files[index].override_contents(&mut self.source, &mut self.scratch)?
            {
                out.seek(file_offset)?;
                out.write(&contents)?;
                copied_custom = true;
                file_length = contents.len() as i64;
            }
            if let Some(&index) = self.overlays_by_file_id.get(&file_id) {
                if let Some(contents) =
                    self.overlays[index].override_contents(&mut self.source, &mut self.scratch)?
                {
                    out.seek(file_offset)?;
                    out.write(&contents)?;
                    copied_custom = true;
                    file_length = contents.len() as i64;
                }
                write_overlay_entry(&mut overlay_table, &self.overlays[index], file_id)?;
            }
            if !copied_custom {
                let start = read_le_sized(&self.fat, file_id as usize * 8, 4);
                let end = read_le_sized(&self.fat, file_id as usize * 8 + 4, 4);
                file_length = end.wrapping_sub(start) as i64;
                out.seek(file_offset)?;
                self.copy_region(out, start as i64, file_length)?;
            }
            write_le_sized(&mut new_fat, file_id as usize * 8, 4, file_offset as i32);
            write_le_sized(
                &mut new_fat,
                file_id as usize * 8 + 4,
                4,
                (file_offset + file_length) as i32,
            );
            base_offset = file_offset + file_length;
        }
        Ok((new_fat, overlay_table, base_offset))
    }

    fn header_field(&mut self, offset: u64) -> Result<i32> {
        read_le_sized_at(&mut self.source, offset, 4)
    }

    fn copy_region<W: Read + Write + Seek>(
        &mut self,
        out: &mut RandomAccessFile<W>,
        from: i64,
        length: i64,
    ) -> Result<()> {
        if length < 0 {
            return Err(FormatError::invalid(CONTEXT, "negative copy length"));
        }
        let mut remaining = length as usize;
        self.source
            .seek(SeekFrom::Start(non_negative_offset(from, CONTEXT)?))?;
        let mut chunk = vec![0u8; COPY_CHUNK.min(remaining)];
        while remaining > 0 {
            let size = remaining.min(chunk.len());
            self.source.read_exact(&mut chunk[..size])?;
            out.write(&chunk[..size])?;
            remaining -= size;
        }
        Ok(())
    }
}

fn write_overlay_entry(table: &mut [u8], overlay: &NdsOverlay, file_id: i32) -> Result<()> {
    let base = overlay.overlay_id as usize * 32;
    if base + 32 > table.len() {
        return Err(FormatError::out_of_bounds(CONTEXT));
    }
    let fields = [
        (0, 4, overlay.overlay_id),
        (4, 4, overlay.ram_address),
        (8, 4, overlay.ram_size),
        (12, 4, overlay.bss_size),
        (16, 4, overlay.static_start),
        (20, 4, overlay.static_end),
        (24, 4, file_id),
        (28, 3, overlay.compressed_size),
        (31, 1, overlay.compress_flag),
    ];
    for (offset, size, value) in fields {
        write_le_sized(table, base + offset, size, value);
    }
    Ok(())
}

fn write_sized<W: Read + Write + Seek>(
    out: &mut RandomAccessFile<W>,
    offset: i64,
    size: usize,
    value: i32,
) -> Result<()> {
    let mut buffer = vec![0u8; size];
    write_le_sized(&mut buffer, 0, size, value);
    out.seek(offset)?;
    out.write(&buffer)
}

fn align(value: i64) -> i64 {
    ((value + SECTION_ALIGN) as i32 & !(SECTION_ALIGN as i32)) as i64
}

fn device_capacity(application_end: i32) -> i32 {
    let mut size = application_end;
    size |= size >> 16;
    size |= size >> 8;
    size |= size >> 4;
    size |= size >> 2;
    size |= size >> 1;
    size = size.wrapping_add(1);
    if size <= 128 * 1024 {
        size = 128 * 1024;
    }
    let mut capacity = -18;
    let mut remaining = size;
    while remaining != 0 {
        remaining >>= 1;
        capacity += 1;
    }
    capacity.max(0)
}

fn directory_paths(
    table: &DirectoryTable,
    directory_names: &[Option<String>],
    dir_count: usize,
) -> Result<HashMap<i32, String>> {
    let mut paths = HashMap::new();
    paths.insert(0xF000, String::new());
    for directory in 1..dir_count.min(0x1000) {
        let Some(first_name) = &directory_names[directory] else {
            paths.insert(directory as i32 + 0xF000, String::new());
            continue;
        };
        let mut components: Vec<String> = Vec::new();
        let mut current = directory;
        let mut name = Some(first_name.clone());
        while let Some(part) = name.take().filter(|part| !part.is_empty()) {
            components.insert(0, part);
            let parent = table.parent_dir_ids[current];
            if (0xF001..=0xFFFF).contains(&parent) {
                current = (parent - 0xF000) as usize;
                name = directory_names
                    .get(current)
                    .ok_or(FormatError::out_of_bounds(CONTEXT))?
                    .clone();
            }
        }
        paths.insert(directory as i32 + 0xF000, components.join("/"));
    }
    Ok(paths)
}
