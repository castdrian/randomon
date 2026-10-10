use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use super::container::{align_long, cxi_offset, read_cia_version};
use super::romfs::{DirectoryMetadata, FileMetadata, METADATA_UNUSED, RomfsFile, RomfsScratch};
use super::smdh::Smdh;
use crate::bytes::{read_long, read_long_be, write_full_long, write_long};
use crate::compression::blz::{self, BlzMode};
use crate::error::{FormatError, Result};
use crate::io::{RandomAccessFile, non_negative_offset, read_exact_at, read_i32_le_at};
use crate::java::arrays::new_array;
use crate::java::sort::java_list_sort;
use crate::java::strings::{decode_utf8, java_trim};
use crate::sha256::sha256;

const CONTEXT: &str = "NCCH";
const MEDIA_UNIT_SIZE: i64 = 0x200;
const HEADER_AND_EXHEADER_SIZE: i64 = 0xA00;
const EXEFS_HEADER_SIZE: i64 = 0x200;
const ROMFS_HEADER_SIZE: usize = 0x5C;
const ROMFS_MAGIC_1: i32 = 0x49564643;
const ROMFS_MAGIC_2: i32 = 0x00000100;
const LEVEL3_HEADER_SIZE: usize = 0x28;
const EXEFS_SLOTS: usize = 10;
const ROMFS_ALIGNMENT_WORKAROUND: i64 = 4096;
pub const PUBLISHER: &str = "Universal Pokemon Randomizer ZX";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExefsFileHeader {
    pub filename: String,
    pub offset: i32,
    pub size: i32,
}

impl ExefsFileHeader {
    fn parse(header: &[u8], offset: usize) -> Self {
        ExefsFileHeader {
            filename: java_trim(&decode_utf8(&header[offset..offset + 8])).to_owned(),
            offset: read_long(header, offset + 8),
            size: read_long(header, offset + 12),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.filename.is_empty() && self.size != 0
    }

    fn to_bytes(&self) -> Result<[u8; 16]> {
        let mut output = [0u8; 16];
        let name = self.filename.as_bytes();
        output
            .get_mut(..name.len())
            .ok_or(FormatError::out_of_bounds(CONTEXT))?
            .copy_from_slice(name);
        write_long(&mut output, 8, self.offset);
        write_long(&mut output, 12, self.size);
        Ok(output)
    }
}

#[derive(Debug, Default)]
struct Romfs {
    metadata: Vec<FileMetadata>,
    files: Vec<RomfsFile>,
    by_path: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NcchDiagnostics {
    pub original_code_crc: u32,
    pub original_romfs_header_crc: u32,
    pub files: HashMap<String, String>,
}

pub struct Ncch<R> {
    source: R,
    ncch_offset: i64,
    product_code: Option<String>,
    title_id: Option<String>,
    version: i32,
    exefs_offset: i64,
    romfs_offset: i64,
    file_data_offset: i64,
    code_header: Option<ExefsFileHeader>,
    smdh: Option<Smdh>,
    extra_exefs_files: Vec<ExefsFileHeader>,
    romfs: Option<Romfs>,
    scratch: RomfsScratch,
    code_compressed: bool,
    code_open: bool,
    code_changed: bool,
    code_stored: Option<Vec<u8>>,
    pub original_code_crc: u32,
    pub original_romfs_header_crc: u32,
}

struct Level3Layout {
    directory_hash_table: (i32, i32),
    directory_metadata_table: (i32, i32),
    file_hash_table: (i32, i32),
    file_metadata_table: (i32, i32),
    file_data_offset: i32,
}

impl Level3Layout {
    fn parse(header: &[u8]) -> Self {
        Level3Layout {
            directory_hash_table: (read_long(header, 0x04), read_long(header, 0x08)),
            directory_metadata_table: (read_long(header, 0x0C), read_long(header, 0x10)),
            file_hash_table: (read_long(header, 0x14), read_long(header, 0x18)),
            file_metadata_table: (read_long(header, 0x1C), read_long(header, 0x20)),
            file_data_offset: read_long(header, 0x24),
        }
    }
}

impl<R: Read + Seek> Ncch<R> {
    pub fn open(
        mut source: R,
        product_code: Option<String>,
        title_id: Option<String>,
    ) -> Result<Self> {
        let ncch_offset = cxi_offset(&mut source)?.map_or(-1, |offset| offset as i64);
        let version = if ncch_offset != -1 {
            read_cia_version(&mut source)?
        } else {
            0
        };
        let mut ncch = Ncch {
            source,
            ncch_offset,
            product_code,
            title_id,
            version,
            exefs_offset: 0,
            romfs_offset: 0,
            file_data_offset: 0,
            code_header: None,
            smdh: None,
            extra_exefs_files: Vec::new(),
            romfs: None,
            scratch: RomfsScratch::new(),
            code_compressed: false,
            code_open: false,
            code_changed: false,
            code_stored: None,
            original_code_crc: 0,
            original_romfs_header_crc: 0,
        };
        if let Err(error) = ncch.read_file_system() {
            return match ncch.is_decrypted() {
                Ok(false) => Err(FormatError::EncryptedRom(Box::new(error))),
                Ok(true) => Err(error),
                Err(check_error) => Err(check_error),
            };
        }
        Ok(ncch)
    }

    fn at(&self, relative: i64) -> Result<u64> {
        non_negative_offset(self.ncch_offset + relative, CONTEXT)
    }

    fn media_units_at(&mut self, relative: i64) -> Result<i64> {
        let position = self.at(relative)?;
        Ok(read_i32_le_at(&mut self.source, position)?.wrapping_mul(MEDIA_UNIT_SIZE as i32) as i64)
    }

    fn read_file_system(&mut self) -> Result<()> {
        self.exefs_offset = self.ncch_offset + self.media_units_at(0x1A0)?;
        self.romfs_offset = self.ncch_offset + self.media_units_at(0x1B0)?;
        let flags_position = self.at(0x20D)?;
        let system_control_flags = read_exact_at(&mut self.source, flags_position, 1)?[0];
        self.code_compressed = system_control_flags & 0x01 != 0;
        self.read_exefs()?;
        self.read_romfs()
    }

    fn read_exefs(&mut self) -> Result<()> {
        let exefs_start = non_negative_offset(self.exefs_offset, CONTEXT)?;
        let header = read_exact_at(&mut self.source, exefs_start, EXEFS_HEADER_SIZE as usize)?;
        self.extra_exefs_files.clear();
        for slot in 0..EXEFS_SLOTS {
            let file_header = ExefsFileHeader::parse(&header, slot * 0x10);
            if !file_header.is_valid() {
                continue;
            }
            if file_header.filename == "icon" {
                new_array(file_header.size as i64, CONTEXT)?;
                let position = non_negative_offset(
                    self.exefs_offset + EXEFS_HEADER_SIZE + file_header.offset as i64,
                    CONTEXT,
                )?;
                let smdh_bytes =
                    read_exact_at(&mut self.source, position, file_header.size as usize)?;
                self.smdh = Some(Smdh::parse(smdh_bytes)?);
            }
            if file_header.filename == ".code" {
                self.code_header = Some(file_header);
            } else {
                self.extra_exefs_files.push(file_header);
            }
        }
        Ok(())
    }

    fn read_romfs(&mut self) -> Result<()> {
        let romfs_start = non_negative_offset(self.romfs_offset, CONTEXT)?;
        let header = read_exact_at(&mut self.source, romfs_start, ROMFS_HEADER_SIZE)?;
        self.original_romfs_header_crc = crate::bytes::crc32(&header);
        if read_long_be(&header, 0) != ROMFS_MAGIC_1 || read_long_be(&header, 4) != ROMFS_MAGIC_2 {
            return Ok(());
        }
        let master_hash_size = read_long(&header, 0x08);
        let level3_block_size = java_block_size(read_long(&header, 0x4C));
        let level3_offset = self.romfs_offset
            + align_long(
                0x60i32.wrapping_add(master_hash_size) as i64,
                level3_block_size,
            );
        let level3_header = read_exact_at(
            &mut self.source,
            non_negative_offset(level3_offset, CONTEXT)?,
            LEVEL3_HEADER_SIZE,
        )?;
        if read_long(&level3_header, 0) != LEVEL3_HEADER_SIZE as i32 {
            return Ok(());
        }
        let layout = Level3Layout::parse(&level3_header);
        self.file_data_offset = level3_offset + layout.file_data_offset as i64;
        let directory_block =
            self.read_level3_block(level3_offset, layout.directory_metadata_table)?;
        let file_block = self.read_level3_block(level3_offset, layout.file_metadata_table)?;
        self.romfs = Some(Romfs::default());
        self.visit_directories(&directory_block, &file_block)
    }

    fn read_level3_block(
        &mut self,
        level3_offset: i64,
        (offset, length): (i32, i32),
    ) -> Result<Vec<u8>> {
        new_array(length as i64, CONTEXT)?;
        read_exact_at(
            &mut self.source,
            non_negative_offset(level3_offset + offset as i64, CONTEXT)?,
            length as usize,
        )
    }

    fn visit_directories(&mut self, directory_block: &[u8], file_block: &[u8]) -> Result<()> {
        let mut pending: Vec<(i32, String)> = vec![(0, String::new())];
        while let Some((offset, root_path)) = pending.pop() {
            let metadata = DirectoryMetadata::parse(directory_block, offset)?;
            let current_path = if metadata.name.is_empty() {
                root_path.clone()
            } else {
                format!("{root_path}{}/", metadata.name)
            };
            if metadata.first_file_offset != METADATA_UNUSED {
                self.visit_files(metadata.first_file_offset, &current_path, file_block)?;
            }
            if metadata.sibling_directory_offset != METADATA_UNUSED {
                pending.push((metadata.sibling_directory_offset, root_path));
            }
            if metadata.first_child_directory_offset != METADATA_UNUSED {
                pending.push((metadata.first_child_directory_offset, current_path));
            }
        }
        Ok(())
    }

    fn visit_files(&mut self, first_offset: i32, root_path: &str, file_block: &[u8]) -> Result<()> {
        let file_data_offset = self.file_data_offset;
        let romfs = self
            .romfs
            .as_mut()
            .ok_or(FormatError::invalid(CONTEXT, "romfs missing"))?;
        let mut offset = first_offset;
        loop {
            let mut metadata = FileMetadata::parse(file_block, offset)?;
            let path = format!("{root_path}{}", metadata.name);
            let index = romfs.files.len();
            romfs.files.push(RomfsFile::new(
                file_data_offset.wrapping_add(metadata.file_data_offset),
                metadata.file_data_length as i32,
                path.clone(),
            ));
            metadata.file_index = index;
            let sibling = metadata.sibling_file_offset;
            romfs.metadata.push(metadata);
            romfs.by_path.insert(path, index);
            if sibling == METADATA_UNUSED {
                return Ok(());
            }
            offset = sibling;
        }
    }

    pub fn is_decrypted(&mut self) -> Result<bool> {
        let flags_position = self.at(0x188)?;
        let flags = read_exact_at(&mut self.source, flags_position, 8)?;
        if flags[7] & 0x4 != 0 {
            return Ok(true);
        }
        Ok(self.romfs.as_ref().is_some_and(|romfs| {
            romfs.by_path.contains_key("DllBattle.cro") || romfs.by_path.contains_key("Battle.cro")
        }))
    }

    pub fn product_code(&self) -> Option<&str> {
        self.product_code.as_deref()
    }

    pub fn title_id(&self) -> Option<&str> {
        self.title_id.as_deref()
    }

    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn smdh(&self) -> Option<&Smdh> {
        self.smdh.as_ref()
    }

    pub fn is_code_compressed(&self) -> bool {
        self.code_compressed
    }

    pub fn romfs_paths(&self) -> Vec<&str> {
        self.romfs
            .as_ref()
            .map(|romfs| romfs.by_path.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    pub fn get_code(&mut self) -> Result<Vec<u8>> {
        if self.code_open {
            return self
                .code_stored
                .clone()
                .ok_or_else(|| FormatError::invalid(CONTEXT, "cached .code is missing"));
        }
        self.code_open = true;
        let header = self.code_header.clone().ok_or_else(|| {
            FormatError::invalid(CONTEXT, "NullPointerException: no .code in exefs")
        })?;
        new_array(header.size as i64, CONTEXT)?;
        let position = non_negative_offset(
            self.exefs_offset + EXEFS_HEADER_SIZE + header.offset as i64,
            CONTEXT,
        )?;
        let mut code = read_exact_at(&mut self.source, position, header.size as usize)?;
        self.original_code_crc = crate::bytes::crc32(&code);
        if self.code_compressed {
            code = blz::decode(&code)?;
        }
        self.code_stored = Some(code.clone());
        Ok(code)
    }

    pub fn write_code(&mut self, code: &[u8]) -> Result<()> {
        if !self.code_open {
            self.get_code()?;
        }
        self.code_changed = true;
        self.code_stored = Some(code.to_vec());
        Ok(())
    }

    pub fn is_code_changed(&self) -> bool {
        self.code_changed
    }

    fn romfs_or_null(&self) -> Result<&Romfs> {
        self.romfs.as_ref().ok_or_else(|| {
            FormatError::invalid(CONTEXT, "NullPointerException: romfs was not parsed")
        })
    }

    pub fn has_file(&self, path: &str) -> Result<bool> {
        Ok(self.romfs_or_null()?.by_path.contains_key(path))
    }

    pub fn get_file(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        let Some(index) = self.romfs_or_null()?.by_path.get(path).copied() else {
            return Ok(None);
        };
        let romfs = self
            .romfs
            .as_mut()
            .ok_or(FormatError::invalid(CONTEXT, "romfs missing"))?;
        romfs.files[index]
            .contents(&mut self.source, &mut self.scratch)
            .map(Some)
    }

    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<()> {
        let Some(index) = self.romfs_or_null()?.by_path.get(path).copied() else {
            return Ok(());
        };
        let romfs = self
            .romfs
            .as_mut()
            .ok_or(FormatError::invalid(CONTEXT, "romfs missing"))?;
        romfs.files[index].write_override(data, &mut self.source, &mut self.scratch)
    }

    pub fn diagnostics(&self) -> Result<NcchDiagnostics> {
        let romfs = self.romfs_or_null()?;
        let files = romfs
            .by_path
            .iter()
            .filter(|&(_, &index)| romfs.files[index].original_crc != 0)
            .map(|(path, &index)| {
                (
                    path.clone(),
                    format!("{path}: {:08X}", romfs.files[index].original_crc),
                )
            })
            .collect();
        Ok(NcchDiagnostics {
            original_code_crc: self.original_code_crc,
            original_romfs_header_crc: self.original_romfs_header_crc,
            files,
        })
    }

    pub fn save_as_ncch<W: Read + Write + Seek>(
        &mut self,
        output: W,
        game_acronym: &str,
        seed: i64,
    ) -> Result<W> {
        let mut out = RandomAccessFile::new(output);
        let header_position = self.at(0)?;
        let header = read_exact_at(
            &mut self.source,
            header_position,
            HEADER_AND_EXHEADER_SIZE as usize,
        )?;
        out.write(&header)?;

        let flags_position = self.at(0x188)?;
        let mut flags = read_exact_at(&mut self.source, flags_position, 8)?;
        flags[3] = 0;
        flags[7] = 4;
        out.seek(0x188)?;
        out.write(&flags)?;

        let logo_length =
            self.copy_small_region(&mut out, 0x198, 0x19C, HEADER_AND_EXHEADER_SIZE)?;
        let plain_length = self.copy_small_region(
            &mut out,
            0x190,
            0x194,
            HEADER_AND_EXHEADER_SIZE + logo_length,
        )?;

        let smdh = self.smdh.as_mut().ok_or_else(|| {
            FormatError::invalid(CONTEXT, "NullPointerException: no icon in exefs")
        })?;
        smdh.set_all_descriptions(&format!("{game_acronym} randomizer seed: {seed}"))?;
        smdh.set_all_publishers(PUBLISHER)?;

        let new_exefs_offset = HEADER_AND_EXHEADER_SIZE + logo_length + plain_length;
        let new_exefs_length = self.rebuild_exefs(&mut out, new_exefs_offset)?;
        write_media_unit_byte(&mut out, 0x1A0, new_exefs_offset)?;
        write_media_unit_byte(&mut out, 0x1A4, new_exefs_length)?;

        let new_romfs_offset = align_long(
            HEADER_AND_EXHEADER_SIZE + logo_length + plain_length + new_exefs_length,
            ROMFS_ALIGNMENT_WORKAROUND,
        );
        let new_romfs_length = self.rebuild_romfs(&mut out, new_romfs_offset)?;
        write_media_unit_byte(&mut out, 0x1B0, new_romfs_offset)?;
        write_media_unit_byte(&mut out, 0x1B4, new_romfs_length)?;

        let exefs_hash_region = self.media_units_at(0x1A8)?;
        let exefs_hash = hash_output_region(&mut out, new_exefs_offset, exefs_hash_region)?;
        out.seek(0x1C0)?;
        out.write(&exefs_hash)?;
        let romfs_hash_region = self.media_units_at(0x1B8)?;
        let romfs_hash = hash_output_region(&mut out, new_romfs_offset, romfs_hash_region)?;
        out.seek(0x1E0)?;
        out.write(&romfs_hash)?;

        out.seek(0)?;
        out.write(&[0u8; 0x100])?;
        out.flush()?;
        Ok(out.into_inner())
    }

    fn copy_small_region<W: Read + Write + Seek>(
        &mut self,
        out: &mut RandomAccessFile<W>,
        offset_field: i64,
        length_field: i64,
        new_offset: i64,
    ) -> Result<i64> {
        let region_offset = self.ncch_offset + self.media_units_at(offset_field)?;
        let region_length = self.media_units_at(length_field)?;
        if region_length > 0 {
            let data = read_exact_at(
                &mut self.source,
                non_negative_offset(region_offset, CONTEXT)?,
                region_length as usize,
            )?;
            out.seek(new_offset)?;
            out.write(&data)?;
            write_media_unit_byte(out, offset_field, new_offset)?;
        }
        Ok(region_length)
    }

    fn rebuild_exefs<W: Read + Write + Seek>(
        &mut self,
        out: &mut RandomAccessFile<W>,
        new_exefs_offset: i64,
    ) -> Result<i64> {
        let mut code = self.get_code()?;
        if self.code_compressed {
            code = blz::encode(&code, false, BlzMode::Best)?;
        }
        let code_header = self.code_header.clone().ok_or_else(|| {
            FormatError::invalid(CONTEXT, "NullPointerException: no .code in exefs")
        })?;
        let new_code_header = ExefsFileHeader {
            filename: code_header.filename,
            offset: 0,
            size: code.len() as i32,
        };
        let mut headers: Vec<ExefsFileHeader> = vec![new_code_header];
        headers.extend(self.extra_exefs_files.iter().cloned());
        if headers.len() > EXEFS_SLOTS {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        out.seek(new_exefs_offset)?;
        for header in &headers {
            out.write(&header.to_bytes()?)?;
        }

        let mut ending_offset: i64 = 0;
        for (slot, header) in headers.iter().enumerate() {
            let data = match header.filename.as_str() {
                ".code" => code.clone(),
                "icon" => self
                    .smdh
                    .as_ref()
                    .map(|smdh| smdh.bytes().to_vec())
                    .ok_or_else(|| {
                        FormatError::invalid(CONTEXT, "NullPointerException: no SMDH")
                    })?,
                _ => {
                    new_array(header.size as i64, CONTEXT)?;
                    let position = non_negative_offset(
                        self.exefs_offset + EXEFS_HEADER_SIZE + header.offset as i64,
                        CONTEXT,
                    )?;
                    read_exact_at(&mut self.source, position, header.size as usize)?
                }
            };
            out.seek(new_exefs_offset + EXEFS_HEADER_SIZE + header.offset as i64)?;
            out.write(&data)?;
            out.seek(new_exefs_offset + EXEFS_HEADER_SIZE - (slot as i64 + 1) * 0x20)?;
            out.write(&sha256(&data))?;
            ending_offset =
                new_exefs_offset + EXEFS_HEADER_SIZE + header.offset as i64 + header.size as i64;
        }

        out.seek(ending_offset)?;
        let mut exefs_length = ending_offset - new_exefs_offset;
        while exefs_length % MEDIA_UNIT_SIZE != 0 {
            out.write_byte(0)?;
            exefs_length += 1;
        }
        Ok(exefs_length)
    }

    fn rebuild_romfs<W: Read + Write + Seek>(
        &mut self,
        out: &mut RandomAccessFile<W>,
        new_romfs_offset: i64,
    ) -> Result<i64> {
        let romfs_start = non_negative_offset(self.romfs_offset, CONTEXT)?;
        let mut romfs_header = read_exact_at(&mut self.source, romfs_start, ROMFS_HEADER_SIZE)?;
        out.seek(new_romfs_offset)?;
        out.write(&romfs_header)?;

        let master_hash_size = read_long(&romfs_header, 0x08);
        let level3_block_size = java_block_size(read_long(&romfs_header, 0x4C));
        let master_and_header = 0x60i32.wrapping_add(master_hash_size) as i64;
        let level3_offset = self.romfs_offset + align_long(master_and_header, level3_block_size);
        let new_level3_offset = new_romfs_offset + align_long(master_and_header, level3_block_size);

        let level3_header = read_exact_at(
            &mut self.source,
            non_negative_offset(level3_offset, CONTEXT)?,
            LEVEL3_HEADER_SIZE,
        )?;
        out.seek(new_level3_offset)?;
        out.write(&level3_header)?;
        let layout = Level3Layout::parse(&level3_header);
        for table in [
            layout.directory_hash_table,
            layout.directory_metadata_table,
            layout.file_hash_table,
        ] {
            let data = self.read_level3_block(level3_offset, table)?;
            out.seek(new_level3_offset + table.0 as i64)?;
            out.write(&data)?;
        }

        let (metadata_offset, metadata_length) = layout.file_metadata_table;
        let new_metadata_table = self.update_file_metadata_table(metadata_length)?;
        out.seek(new_level3_offset + metadata_offset as i64)?;
        out.write(&new_metadata_table)?;

        let end_of_file_data =
            self.write_romfs_file_data(out, new_level3_offset, layout.file_data_offset)?;

        let level3_hash_size = end_of_file_data - new_level3_offset;
        let level3_blocks = align_long(level3_hash_size, level3_block_size) / level3_block_size;
        let level2_block_size = java_block_size(read_long(&romfs_header, 0x34));
        let level2_hash_size = level3_blocks * 0x20;
        let level2_blocks = align_long(level2_hash_size, level2_block_size) / level2_block_size;
        let level1_block_size = java_block_size(read_long(&romfs_header, 0x1C));
        let level1_hash_size = level2_blocks * 0x20;
        let new_level1_offset = new_level3_offset + align_long(level3_hash_size, level3_block_size);
        let new_level2_offset = new_level1_offset + align_long(level1_hash_size, level1_block_size);
        let new_file_ending_offset =
            align_long(new_level2_offset + level2_hash_size, level2_block_size);

        hash_blocks(
            out,
            new_level3_offset,
            level3_block_size,
            level3_blocks,
            new_level2_offset,
        )?;
        if out.pointer() as i64 > new_file_ending_offset {
            return Err(FormatError::UpstreamHang { context: CONTEXT });
        }
        while out.pointer() as i64 != new_file_ending_offset {
            out.write_byte(0)?;
        }
        hash_blocks(
            out,
            new_level2_offset,
            level2_block_size,
            level2_blocks,
            new_level1_offset,
        )?;
        let level1_blocks = align_long(level1_hash_size, level1_block_size) / level1_block_size;
        hash_blocks(
            out,
            new_level1_offset,
            level1_block_size,
            level1_blocks,
            new_romfs_offset + 0x60,
        )?;

        let level2_logical_offset = align_long(level1_hash_size, level1_block_size);
        let level3_logical_offset =
            align_long(level2_logical_offset + level2_hash_size, level2_block_size);
        write_long(
            &mut romfs_header,
            0x08,
            (level1_blocks as i32).wrapping_mul(0x20),
        );
        write_full_long(&mut romfs_header, 0x0C, 0);
        write_full_long(&mut romfs_header, 0x14, level1_hash_size);
        write_full_long(&mut romfs_header, 0x24, level2_logical_offset);
        write_full_long(&mut romfs_header, 0x2C, level2_hash_size);
        write_full_long(&mut romfs_header, 0x3C, level3_logical_offset);
        write_full_long(&mut romfs_header, 0x44, level3_hash_size);
        out.seek(new_romfs_offset)?;
        out.write(&romfs_header)?;

        let current_length = new_file_ending_offset - new_romfs_offset;
        let new_romfs_length = align_long(current_length, MEDIA_UNIT_SIZE);
        out.seek(new_file_ending_offset)?;
        while (out.pointer() as i64) < new_romfs_offset + new_romfs_length {
            out.write_byte(0)?;
        }
        Ok(new_romfs_length)
    }

    fn update_file_metadata_table(&mut self, table_length: i32) -> Result<Vec<u8>> {
        let romfs = self.romfs.as_mut().ok_or_else(|| {
            FormatError::invalid(CONTEXT, "NullPointerException: romfs was not parsed")
        })?;
        java_list_sort(&mut romfs.metadata, |left, right| {
            left.file_data_offset.wrapping_sub(right.file_data_offset) as i32
        })
        .map_err(|error| FormatError::invalid(CONTEXT, error.to_string()))?;
        let mut table = new_array(table_length as i64, CONTEXT)?;
        let mut table_offset = 0usize;
        let mut file_data_offset: i64 = 0;
        for metadata in &mut romfs.metadata {
            metadata.file_data_offset = file_data_offset;
            let file = &romfs.files[metadata.file_index];
            if file.file_changed {
                metadata.file_data_length = file.size as i64;
            }
            let bytes = metadata.to_bytes()?;
            table
                .get_mut(table_offset..table_offset + bytes.len())
                .ok_or(FormatError::out_of_bounds(CONTEXT))?
                .copy_from_slice(&bytes);
            table_offset += bytes.len();
            file_data_offset = file_data_offset.wrapping_add(metadata.file_data_length);
        }
        Ok(table)
    }

    fn write_romfs_file_data<W: Read + Write + Seek>(
        &mut self,
        out: &mut RandomAccessFile<W>,
        new_level3_offset: i64,
        file_data_offset: i32,
    ) -> Result<i64> {
        let romfs = self.romfs.as_mut().ok_or_else(|| {
            FormatError::invalid(CONTEXT, "NullPointerException: romfs was not parsed")
        })?;
        let mut end_of_file_data: i64 = 0;
        for metadata in &romfs.metadata {
            let file = &mut romfs.files[metadata.file_index];
            let data = if file.file_changed {
                file.override_contents(&mut self.source, &mut self.scratch)?
                    .ok_or_else(|| {
                        FormatError::invalid(CONTEXT, "changed romfs file has no contents")
                    })?
            } else {
                new_array(file.size as i64, CONTEXT)?;
                read_exact_at(
                    &mut self.source,
                    non_negative_offset(file.offset, CONTEXT)?,
                    file.size as usize,
                )
                .map_err(|error| romfs_write_error(file, metadata, error))?
            };
            let position = new_level3_offset + file_data_offset as i64 + metadata.file_data_offset;
            out.seek(position)?;
            out.write(&data)?;
            end_of_file_data = position + data.len() as i64;
        }
        Ok(end_of_file_data)
    }

    pub fn layered_fs_entries(&mut self) -> Result<Vec<(PathBuf, Vec<u8>)>> {
        let mut entries = Vec::new();
        if self.code_changed {
            entries.push((PathBuf::from("code.bin"), self.get_code()?));
        }
        let romfs = self.romfs.as_mut().ok_or_else(|| {
            FormatError::invalid(CONTEXT, "NullPointerException: romfs was not parsed")
        })?;
        for &index in romfs.by_path.values() {
            let file = &mut romfs.files[index];
            if !file.file_changed {
                continue;
            }
            let contents = file
                .override_contents(&mut self.source, &mut self.scratch)?
                .ok_or_else(|| {
                    FormatError::invalid(CONTEXT, "changed romfs file has no contents")
                })?;
            let mut path = PathBuf::from("romfs");
            for component in java_split_slash(&file.full_path) {
                path.push(component);
            }
            entries.push((path, contents));
        }
        Ok(entries)
    }

    pub fn save_as_layered_fs(&mut self, output_path: &Path) -> Result<()> {
        let title_id = self
            .title_id
            .clone()
            .ok_or_else(|| FormatError::invalid(CONTEXT, "missing title id"))?;
        let root = output_path.join(title_id);
        if !root.exists() {
            fs::create_dir_all(&root)?;
        } else {
            purge_directory(&root)?;
        }
        let romfs_root = root.join("romfs");
        if !romfs_root.exists() {
            fs::create_dir_all(&romfs_root)?;
        }
        for (relative, contents) in self.layered_fs_entries()? {
            let target = root.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(target, contents)?;
        }
        Ok(())
    }
}

fn romfs_write_error(file: &RomfsFile, metadata: &FileMetadata, error: FormatError) -> FormatError {
    FormatError::invalid(
        CONTEXT,
        format!(
            "Error when building romfs: File: {}, offset: {}, size: {} ({error})",
            file.full_path, metadata.offset, file.size
        ),
    )
}

fn java_split_slash(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = path.split('/').collect();
    while parts.len() > 1 && parts.last() == Some(&"") {
        parts.pop();
    }
    parts
}

fn java_block_size(exponent: i32) -> i64 {
    1i32.wrapping_shl(exponent as u32) as i64
}

fn purge_directory(directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            purge_directory(&path)?;
            let _ = fs::remove_dir(&path);
        } else {
            let _ = fs::remove_file(&path);
        }
    }
    Ok(())
}

fn write_media_unit_byte<W: Read + Write + Seek>(
    out: &mut RandomAccessFile<W>,
    field: i64,
    value: i64,
) -> Result<()> {
    out.seek(field)?;
    out.write_byte(((value as i32) / MEDIA_UNIT_SIZE as i32) as u8)
}

fn hash_output_region<W: Read + Write + Seek>(
    out: &mut RandomAccessFile<W>,
    offset: i64,
    length: i64,
) -> Result<[u8; 32]> {
    let mut data = new_array(length, CONTEXT)?;
    out.seek(offset)?;
    out.read_fully(&mut data)?;
    Ok(sha256(&data))
}

fn hash_blocks<W: Read + Write + Seek>(
    out: &mut RandomAccessFile<W>,
    data_offset: i64,
    block_size: i64,
    block_count: i64,
    hash_offset: i64,
) -> Result<()> {
    let mut block = new_array(block_size, CONTEXT)?;
    for index in 0..block_count {
        out.seek(data_offset + index * block_size)?;
        out.read_fully(&mut block)?;
        let hash = sha256(&block);
        out.seek(hash_offset + index * 0x20)?;
        out.write(&hash)?;
    }
    Ok(())
}

pub fn diagnostics_lines(
    file_name: &str,
    base: &NcchDiagnostics,
    update: Option<&NcchDiagnostics>,
) -> Vec<String> {
    let mut lines = vec![format!("File name: {file_name}")];
    let code_crc = update.map_or(base.original_code_crc, |update| update.original_code_crc);
    lines.push(format!(".code: {code_crc:08X}"));
    lines.push(format!(
        "romfs header: {:08X}",
        base.original_romfs_header_crc
    ));
    if let Some(update) = update {
        lines.push(format!(
            "romfs header (game update): {:08X}",
            update.original_romfs_header_crc
        ));
    }
    let mut base_files = base.files.clone();
    let mut file_lines: Vec<String> = Vec::new();
    if let Some(update) = update {
        for (path, line) in &update.files {
            base_files.remove(path);
            file_lines.push(line.clone());
        }
    }
    file_lines.extend(base_files.into_values());
    file_lines.sort();
    lines.extend(file_lines);
    lines
}
