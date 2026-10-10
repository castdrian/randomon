use std::collections::HashMap;
use std::io::{Read, Seek};

use crate::compression::blz::{self, BlzMode};
use crate::error::Result;
use crate::io::{non_negative_offset, read_exact_at};
use crate::java::arrays::new_array;
use crate::java::strings::sanitize_path;

pub type ScratchFiles = HashMap<String, Vec<u8>>;

#[derive(Debug, Clone)]
pub struct NdsFile {
    pub offset: i32,
    pub size: i32,
    pub file_id: i32,
    pub full_path: String,
    pub original_crc: u32,
    extracted: bool,
}

impl NdsFile {
    pub fn new(offset: i32, size: i32, file_id: i32, full_path: String) -> Self {
        NdsFile {
            offset,
            size,
            file_id,
            full_path,
            original_crc: 0,
            extracted: false,
        }
    }

    pub fn is_extracted(&self) -> bool {
        self.extracted
    }

    fn scratch_key(&self) -> String {
        sanitize_path(&self.full_path, false)
    }

    pub fn contents<R: Read + Seek>(
        &mut self,
        source: &mut R,
        scratch: &mut ScratchFiles,
    ) -> Result<Vec<u8>> {
        let key = self.scratch_key();
        if self.extracted {
            return Ok(scratch.get(&key).cloned().unwrap_or_default());
        }
        new_array(self.size as i64, "NDSFile")?;
        let offset = non_negative_offset(self.offset as i64, "NDSFile")?;
        let buffer = read_exact_at(source, offset, self.size as usize)?;
        self.original_crc = crate::bytes::crc32(&buffer);
        scratch.insert(key, buffer.clone());
        self.extracted = true;
        Ok(buffer)
    }

    pub fn write_override<R: Read + Seek>(
        &mut self,
        data: &[u8],
        source: &mut R,
        scratch: &mut ScratchFiles,
    ) -> Result<()> {
        if !self.extracted {
            self.contents(source, scratch)?;
        }
        scratch.insert(self.scratch_key(), data.to_vec());
        Ok(())
    }

    pub fn override_contents<R: Read + Seek>(
        &mut self,
        source: &mut R,
        scratch: &mut ScratchFiles,
    ) -> Result<Option<Vec<u8>>> {
        if !self.extracted {
            return Ok(None);
        }
        self.contents(source, scratch).map(Some)
    }
}

#[derive(Debug, Clone)]
pub struct NdsOverlay {
    pub offset: i32,
    pub size: i32,
    pub original_size: i32,
    pub file_id: i32,
    pub overlay_id: i32,
    pub ram_address: i32,
    pub ram_size: i32,
    pub bss_size: i32,
    pub static_start: i32,
    pub static_end: i32,
    pub compressed_size: i32,
    pub compress_flag: i32,
    pub original_crc: u32,
    extracted: bool,
    decompressed_data: bool,
}

pub struct OverlayTableEntry {
    pub file_id: i32,
    pub start: i32,
    pub end: i32,
    pub overlay_id: i32,
    pub ram_address: i32,
    pub ram_size: i32,
    pub bss_size: i32,
    pub static_start: i32,
    pub static_end: i32,
    pub compressed_size: i32,
    pub compress_flag: i32,
}

impl NdsOverlay {
    pub fn new(entry: OverlayTableEntry) -> Self {
        NdsOverlay {
            offset: entry.start,
            size: entry.end.wrapping_sub(entry.start),
            original_size: entry.end.wrapping_sub(entry.start),
            file_id: entry.file_id,
            overlay_id: entry.overlay_id,
            ram_address: entry.ram_address,
            ram_size: entry.ram_size,
            bss_size: entry.bss_size,
            static_start: entry.static_start,
            static_end: entry.static_end,
            compressed_size: entry.compressed_size,
            compress_flag: entry.compress_flag,
            original_crc: 0,
            extracted: false,
            decompressed_data: false,
        }
    }

    pub fn is_extracted(&self) -> bool {
        self.extracted
    }

    fn scratch_key(&self) -> String {
        sanitize_path(&format!("overlay_{:04}", self.overlay_id), false)
    }

    fn is_stored_compressed(&self) -> bool {
        self.compress_flag != 0
            && self.original_size == self.compressed_size
            && self.compressed_size != 0
    }

    pub fn contents<R: Read + Seek>(
        &mut self,
        source: &mut R,
        scratch: &mut ScratchFiles,
    ) -> Result<Vec<u8>> {
        let key = self.scratch_key();
        if self.extracted {
            return Ok(scratch.get(&key).cloned().unwrap_or_default());
        }
        new_array(self.original_size as i64, "NDSY9Entry")?;
        let offset = non_negative_offset(self.offset as i64, "NDSY9Entry")?;
        let mut buffer = read_exact_at(source, offset, self.original_size as usize)?;
        self.original_crc = crate::bytes::crc32(&buffer);
        if self.is_stored_compressed() {
            buffer = blz::decode(&buffer)?;
            self.decompressed_data = true;
        }
        scratch.insert(key, buffer.clone());
        self.extracted = true;
        Ok(buffer)
    }

    pub fn write_override<R: Read + Seek>(
        &mut self,
        data: &[u8],
        source: &mut R,
        scratch: &mut ScratchFiles,
    ) -> Result<()> {
        if !self.extracted {
            self.contents(source, scratch)?;
        }
        self.size = data.len() as i32;
        scratch.insert(self.scratch_key(), data.to_vec());
        Ok(())
    }

    pub fn override_contents<R: Read + Seek>(
        &mut self,
        source: &mut R,
        scratch: &mut ScratchFiles,
    ) -> Result<Option<Vec<u8>>> {
        if !self.extracted {
            return Ok(None);
        }
        let mut buffer = self.contents(source, scratch)?;
        if self.decompressed_data {
            buffer = blz::encode(&buffer, false, BlzMode::Normal)?;
            self.compressed_size = buffer.len() as i32;
        }
        Ok(Some(buffer))
    }
}
