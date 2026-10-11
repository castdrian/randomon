use std::io::{Read, Seek, SeekFrom};

use crate::error::Result;
use crate::io::{read_i32_be_at, read_i32_le_at, read_u16_be_at};
use crate::java::strings::{decode_utf8, java_trim};

pub const CIA_HEADER_SIZE: i32 = 0x2020;
pub const NCCH_MAGIC: i32 = 0x4E434348;
pub const NCSD_MAGIC: i32 = 0x4E435344;
pub const MAGIC_OFFSET: u64 = 0x100;
pub const CCI_PARTITION_OFFSET: u64 = 0x4000;

pub fn align_long(value: i64, alignment: i64) -> i64 {
    let mask = !(alignment - 1);
    (value + (alignment - 1)) & mask
}

pub fn align_int(value: i32, alignment: i32) -> i32 {
    let mask = !(alignment - 1);
    value.wrapping_add(alignment - 1) & mask
}

pub(crate) struct CiaLayout {
    pub ticket_offset: i64,
    pub tmd_offset: i64,
    pub content_offset: i64,
}

pub(crate) fn cia_layout<R: Read + Seek>(source: &mut R) -> Result<CiaLayout> {
    let cert_chain_size = read_i32_le_at(source, 0x08)? as i64;
    let ticket_size = read_i32_le_at(source, 0x0C)? as i64;
    let tmd_file_size = read_i32_le_at(source, 0x10)? as i64;
    let cert_chain_offset = align_long(CIA_HEADER_SIZE as i64, 64);
    let ticket_offset = align_long(cert_chain_offset + cert_chain_size, 64);
    let tmd_offset = align_long(ticket_offset + ticket_size, 64);
    let content_offset = align_long(tmd_offset + tmd_file_size, 64);
    Ok(CiaLayout {
        ticket_offset,
        tmd_offset,
        content_offset,
    })
}

fn offset_from(value: i64) -> Result<u64> {
    crate::io::non_negative_offset(value, "CIA")
}

pub fn cxi_offset<R: Read + Seek>(source: &mut R) -> Result<Option<u64>> {
    if read_i32_le_at(source, 0)? == CIA_HEADER_SIZE {
        let layout = cia_layout(source)?;
        let magic = read_i32_be_at(source, offset_from(layout.content_offset)? + MAGIC_OFFSET)?;
        if magic == NCCH_MAGIC {
            return Ok(Some(layout.content_offset as u64));
        }
    }
    match read_i32_be_at(source, MAGIC_OFFSET)? {
        NCCH_MAGIC => Ok(Some(0)),
        NCSD_MAGIC => Ok(Some(CCI_PARTITION_OFFSET)),
        _ => Ok(None),
    }
}

pub fn read_cia_version<R: Read + Seek>(source: &mut R) -> Result<i32> {
    let magic = read_i32_be_at(source, MAGIC_OFFSET)?;
    if magic == NCCH_MAGIC || magic == NCSD_MAGIC {
        return Ok(0);
    }
    let layout = cia_layout(source)?;
    let Some(tmd_header_offset) = signed_body_offset(source, layout.tmd_offset)? else {
        return Ok(0);
    };
    read_u16_be_at(source, offset_from(tmd_header_offset + 0x9C)?)
}

pub(crate) fn signed_body_offset<R: Read + Seek>(
    source: &mut R,
    signed_offset: i64,
) -> Result<Option<i64>> {
    let signature_type = read_i32_be_at(source, offset_from(signed_offset)?)?;
    let (signature_size, padding_size) = match signature_type {
        0x010003 => (0x200, 0x3C),
        0x010004 => (0x100, 0x3C),
        0x010005 => (0x3C, 0x40),
        _ => return Ok(None),
    };
    Ok(Some(signed_offset + 4 + signature_size + padding_size))
}

fn read_lenient<R: Read + Seek>(source: &mut R, offset: u64, length: usize) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; length];
    source.seek(SeekFrom::Start(offset))?;
    let mut filled = 0;
    while filled < length {
        let read = source.read(&mut buffer[filled..])?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    Ok(buffer)
}

pub fn product_code<R: Read + Seek>(source: &mut R) -> Result<Option<String>> {
    let Some(offset) = cxi_offset(source)? else {
        return Ok(None);
    };
    let bytes = read_lenient(source, offset + 0x150, 0x10)?;
    Ok(Some(java_trim(&decode_utf8(&bytes)).to_owned()))
}

pub fn title_id<R: Read + Seek>(source: &mut R) -> Result<Option<String>> {
    let Some(offset) = cxi_offset(source)? else {
        return Ok(None);
    };
    let mut program_id = read_lenient(source, offset + 0x118, 8)?;
    program_id.reverse();
    Ok(Some(
        program_id
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect(),
    ))
}
