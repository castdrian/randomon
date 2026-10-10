use std::collections::HashMap;
use std::io::{Read, Seek};

use super::container::align_int;
use crate::bytes::{read_full_long, read_long, write_full_long, write_long};
use crate::error::{FormatError, Result};
use crate::io::{non_negative_offset, read_exact_at};
use crate::java::arrays::{new_array, slice_at};
use crate::java::strings::{decode_utf16le, encode_utf16le, java_trim, sanitize_path};

const CONTEXT: &str = "romfs";
pub const METADATA_UNUSED: i32 = -1;

pub type RomfsScratch = HashMap<String, Vec<u8>>;

#[derive(Debug, Clone)]
pub struct RomfsFile {
    pub offset: i64,
    pub size: i32,
    pub full_path: String,
    pub file_changed: bool,
    pub original_crc: u32,
    extracted: bool,
}

impl RomfsFile {
    pub fn new(offset: i64, size: i32, full_path: String) -> Self {
        RomfsFile {
            offset,
            size,
            full_path,
            file_changed: false,
            original_crc: 0,
            extracted: false,
        }
    }

    pub fn is_extracted(&self) -> bool {
        self.extracted
    }

    fn scratch_key(&self) -> String {
        sanitize_path(&self.full_path, true)
    }

    pub fn contents<R: Read + Seek>(
        &mut self,
        source: &mut R,
        scratch: &mut RomfsScratch,
    ) -> Result<Vec<u8>> {
        let key = self.scratch_key();
        if self.extracted {
            return Ok(scratch.get(&key).cloned().unwrap_or_default());
        }
        new_array(self.size as i64, "RomfsFile")?;
        let buffer = read_exact_at(
            source,
            non_negative_offset(self.offset, "RomfsFile")?,
            self.size as usize,
        )?;
        self.original_crc = crate::bytes::crc32(&buffer);
        scratch.insert(key, buffer.clone());
        self.extracted = true;
        Ok(buffer)
    }

    pub fn write_override<R: Read + Seek>(
        &mut self,
        data: &[u8],
        source: &mut R,
        scratch: &mut RomfsScratch,
    ) -> Result<()> {
        if !self.extracted {
            self.contents(source, scratch)?;
        }
        self.file_changed = true;
        self.size = data.len() as i32;
        scratch.insert(self.scratch_key(), data.to_vec());
        Ok(())
    }

    pub fn override_contents<R: Read + Seek>(
        &mut self,
        source: &mut R,
        scratch: &mut RomfsScratch,
    ) -> Result<Option<Vec<u8>>> {
        if !self.extracted {
            return Ok(None);
        }
        self.contents(source, scratch).map(Some)
    }
}

fn read_int(block: &[u8], offset: i64) -> Result<i32> {
    slice_at(block, offset, 4, CONTEXT).map(|bytes| read_long(bytes, 0))
}

fn read_long_field(block: &[u8], offset: i64) -> Result<i64> {
    slice_at(block, offset, 8, CONTEXT).map(|bytes| read_full_long(bytes, 0))
}

fn read_name(block: &[u8], offset: i64, name_length: i32) -> Result<String> {
    if name_length == METADATA_UNUSED {
        return Ok(String::new());
    }
    new_array(name_length as i64, CONTEXT)?;
    let bytes = slice_at(block, offset, name_length as i64, CONTEXT)?;
    Ok(java_trim(&decode_utf16le(bytes)).to_owned())
}

#[derive(Debug, Clone)]
pub struct DirectoryMetadata {
    pub parent_directory_offset: i32,
    pub sibling_directory_offset: i32,
    pub first_child_directory_offset: i32,
    pub first_file_offset: i32,
    pub next_directory_in_hash_bucket_offset: i32,
    pub name_length: i32,
    pub name: String,
}

impl DirectoryMetadata {
    pub fn parse(block: &[u8], offset: i32) -> Result<Self> {
        let at = offset as i64;
        let name_length = read_int(block, at + 0x14)?;
        Ok(DirectoryMetadata {
            parent_directory_offset: read_int(block, at)?,
            sibling_directory_offset: read_int(block, at + 0x04)?,
            first_child_directory_offset: read_int(block, at + 0x08)?,
            first_file_offset: read_int(block, at + 0x0C)?,
            next_directory_in_hash_bucket_offset: read_int(block, at + 0x10)?,
            name_length,
            name: read_name(block, at + 0x18, name_length)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub offset: i32,
    pub parent_directory_offset: i32,
    pub sibling_file_offset: i32,
    pub file_data_offset: i64,
    pub file_data_length: i64,
    pub next_file_in_hash_bucket_offset: i32,
    pub name_length: i32,
    pub name: String,
    pub file_index: usize,
}

impl FileMetadata {
    pub fn parse(block: &[u8], offset: i32) -> Result<Self> {
        let at = offset as i64;
        let name_length = read_int(block, at + 0x1C)?;
        Ok(FileMetadata {
            offset,
            parent_directory_offset: read_int(block, at)?,
            sibling_file_offset: read_int(block, at + 0x04)?,
            file_data_offset: read_long_field(block, at + 0x08)?,
            file_data_length: read_long_field(block, at + 0x10)?,
            next_file_in_hash_bucket_offset: read_int(block, at + 0x18)?,
            name_length,
            name: read_name(block, at + 0x20, name_length)?,
            file_index: 0,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut length: i64 = 0x20;
        if self.name_length != METADATA_UNUSED {
            length += align_int(self.name_length, 4) as i64;
        }
        let mut output = new_array(length, CONTEXT)?;
        if output.len() < 0x20 {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        write_long(&mut output, 0x00, self.parent_directory_offset);
        write_long(&mut output, 0x04, self.sibling_file_offset);
        write_full_long(&mut output, 0x08, self.file_data_offset);
        write_full_long(&mut output, 0x10, self.file_data_length);
        write_long(&mut output, 0x18, self.next_file_in_hash_bucket_offset);
        write_long(&mut output, 0x1C, self.name_length);
        if !self.name.is_empty() {
            let name = encode_utf16le(&self.name);
            output
                .get_mut(0x20..0x20 + name.len())
                .ok_or(FormatError::out_of_bounds(CONTEXT))?
                .copy_from_slice(&name);
        }
        Ok(output)
    }
}
