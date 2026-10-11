use std::collections::BTreeMap;

use crate::compression::lzss::{self, LzssDecoded};
use crate::error::{FormatError, Result};

const CONTEXT: &str = "GARCArchive";
const VER_4: i32 = 0x0400;
const VER_6: i32 = 0x0600;
const GARC_HEADER_SIZE_4: usize = 0x1C;
const GARC_HEADER_SIZE_6: usize = 0x24;

pub type GarcDirectory = BTreeMap<u32, Option<Vec<u8>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decompression {
    Skip,
    All,
    Selected(Vec<bool>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GarcHeader {
    version: i32,
    content_pad_to_nearest: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GarcArchive {
    version: u8,
    header: GarcHeader,
    fato_header_size: i32,
    fato_entry_count: i32,
    fato_padding: i32,
    fatb_header_size: i32,
    fatb_file_count: i32,
    fimb_header_size: i32,
    is_compressed: BTreeMap<usize, bool>,
    pub files: Vec<GarcDirectory>,
}

struct Buffer<'a> {
    data: &'a [u8],
    position: usize,
}

impl Buffer<'_> {
    fn underflow() -> FormatError {
        FormatError::invalid(CONTEXT, "BufferUnderflowException")
    }

    fn bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let slice = self
            .data
            .get(self.position..self.position + N)
            .ok_or_else(Self::underflow)?;
        self.position += N;
        let mut array = [0u8; N];
        array.copy_from_slice(slice);
        Ok(array)
    }

    fn int(&mut self) -> Result<i32> {
        self.bytes::<4>().map(i32::from_le_bytes)
    }

    fn short(&mut self) -> Result<i32> {
        self.bytes::<2>()
            .map(|bytes| i16::from_le_bytes(bytes) as i32)
    }

    fn magic(&mut self) -> Result<[u8; 4]> {
        self.bytes::<4>()
    }
}

struct SubEntry {
    start: i32,
    length: i32,
}

impl GarcArchive {
    pub fn parse(data: &[u8], decompression: Decompression) -> Result<Self> {
        read_frames(data, &decompression)?
            .ok_or_else(|| FormatError::invalid(CONTEXT, "Invalid GARC file"))
    }

    pub fn update_files(&mut self, files: Vec<GarcDirectory>) {
        self.files = files;
    }

    pub fn file(&self, index: usize) -> Option<&[u8]> {
        self.file_at(index, 0)
    }

    pub fn file_at(&self, index: usize, sub_index: u32) -> Option<&[u8]> {
        self.files.get(index)?.get(&sub_index)?.as_deref()
    }

    pub fn set_file(&mut self, index: usize, data: Vec<u8>) -> Result<()> {
        self.files
            .get_mut(index)
            .ok_or(FormatError::out_of_bounds(CONTEXT))?
            .insert(0, Some(data));
        Ok(())
    }

    pub fn directory(&self, index: usize) -> Option<&GarcDirectory> {
        self.files.get(index)
    }

    pub fn directory_mut(&mut self, index: usize) -> Option<&mut GarcDirectory> {
        self.files.get_mut(index)
    }

    pub fn is_compressed(&self, index: usize) -> Option<bool> {
        self.is_compressed.get(&index).copied()
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let garc_header_size = if self.header.version == VER_4 {
            GARC_HEADER_SIZE_4
        } else {
            GARC_HEADER_SIZE_6
        };
        let mut fato = FixedBuffer::new(self.fato_header_size)?;
        fato.put(b"OTAF")?;
        fato.put_int(self.fato_header_size)?;
        fato.put_short(self.fato_entry_count)?;
        fato.put_short(self.fato_padding)?;
        let mut fatb = FixedBuffer::new(self.fatb_header_size)?;
        fatb.put(b"BTAF")?;
        fatb.put_int(self.fatb_header_size)?;
        fatb.put_int(self.fatb_file_count)?;
        let mut fimb_header = FixedBuffer::new(self.fimb_header_size)?;
        fimb_header.put(b"BMIF")?;
        fimb_header.put_int(self.fimb_header_size)?;

        let mut payload: Vec<u8> = Vec::new();
        let mut fimb_offset: i32 = 0;
        let mut largest_size: i32 = 0;
        let mut largest_padded: i32 = 0;
        for (index, directory) in self.files.iter().enumerate() {
            let mut bit_vector: i32 = 0;
            let mut total_length: i32 = 0;
            for (&key, file) in directory {
                bit_vector |= 1i32.wrapping_shl(key);
                let file = file.as_deref().ok_or_else(|| {
                    FormatError::invalid(CONTEXT, "NullPointerException: null GARC file")
                })?;
                let compressed = self.is_compressed.get(&index).copied().ok_or_else(|| {
                    FormatError::invalid(CONTEXT, "NullPointerException: unknown compression")
                })?;
                let written = if compressed {
                    lzss::encode(file)?.ok_or_else(|| {
                        FormatError::invalid(CONTEXT, "NullPointerException: encoding refused")
                    })?
                } else {
                    file.to_vec()
                };
                payload.extend_from_slice(&written);
                total_length = total_length.wrapping_add(written.len() as i32);
            }
            if self.header.content_pad_to_nearest == 0 {
                return Err(FormatError::invalid(
                    CONTEXT,
                    "ArithmeticException: / by zero",
                ));
            }
            let mut padding = total_length % self.header.content_pad_to_nearest;
            if padding != 0 {
                padding = self.header.content_pad_to_nearest - padding;
            }
            largest_size = largest_size.max(total_length);
            largest_padded = largest_padded.max(total_length + padding);
            for _ in 0..padding.max(0) {
                payload.push(self.fato_padding as u8);
            }
            fato.put_int(fatb.position as i32 - 12)?;
            fatb.put_int(bit_vector)?;
            fatb.put_int(fimb_offset)?;
            fimb_offset = payload.len() as i32;
            fatb.put_int(fimb_offset)?;
            fatb.put_int(total_length)?;
        }

        let data_offset = garc_header_size as i32
            + fato.position as i32
            + fatb.position as i32
            + self.fimb_header_size;
        let mut garc = FixedBuffer::new(garc_header_size as i32)?;
        garc.put(b"CRAG")?;
        garc.put_int(garc_header_size as i32)?;
        garc.put_short(0xFEFF)?;
        garc.put_short(if self.version == 4 { VER_4 } else { VER_6 })?;
        garc.put_int(4)?;
        garc.put_int(data_offset)?;
        garc.put_int(data_offset.wrapping_add(fimb_offset))?;
        if self.header.version == VER_4 {
            garc.put_int(largest_size)?;
        } else if self.header.version == VER_6 {
            garc.put_int(largest_padded)?;
            garc.put_int(largest_size)?;
            garc.put_int(self.header.content_pad_to_nearest)?;
        }
        fimb_header.put_int(payload.len() as i32)?;

        let mut output = Vec::new();
        output.extend_from_slice(garc.flipped());
        output.extend_from_slice(fato.flipped());
        output.extend_from_slice(fatb.flipped());
        output.extend_from_slice(fimb_header.flipped());
        output.extend_from_slice(&payload);
        Ok(output)
    }
}

struct FixedBuffer {
    bytes: Vec<u8>,
    position: usize,
}

impl FixedBuffer {
    fn new(capacity: i32) -> Result<Self> {
        let capacity = usize::try_from(capacity).map_err(|_| {
            FormatError::invalid(CONTEXT, "IllegalArgumentException: negative capacity")
        })?;
        Ok(FixedBuffer {
            bytes: vec![0u8; capacity],
            position: 0,
        })
    }

    fn put(&mut self, data: &[u8]) -> Result<()> {
        let end = self.position + data.len();
        if end > self.bytes.len() {
            return Err(FormatError::invalid(CONTEXT, "BufferOverflowException"));
        }
        self.bytes[self.position..end].copy_from_slice(data);
        self.position = end;
        Ok(())
    }

    fn put_int(&mut self, value: i32) -> Result<()> {
        self.put(&value.to_le_bytes())
    }

    fn put_short(&mut self, value: i32) -> Result<()> {
        self.put(&(value as u16).to_le_bytes())
    }

    fn flipped(&self) -> &[u8] {
        &self.bytes[..self.position]
    }
}

fn read_frames(data: &[u8], decompression: &Decompression) -> Result<Option<GarcArchive>> {
    if data.is_empty() {
        return Ok(None);
    }
    let mut buffer = Buffer { data, position: 0 };
    if &buffer.magic()? != b"CRAG" {
        return Ok(None);
    }
    let _garc_header_size = buffer.int()?;
    let _endianness = buffer.short()?;
    let garc_version = buffer.short()?;
    if buffer.int()? != 4 {
        return Ok(None);
    }
    let data_offset = buffer.int()?;
    let _file_size = buffer.int()?;
    let (version, content_pad_to_nearest) = match garc_version {
        VER_4 => {
            let _largest_unpadded = buffer.int()?;
            (4u8, 4)
        }
        VER_6 => {
            let _largest_padded = buffer.int()?;
            let _largest_unpadded = buffer.int()?;
            (6u8, buffer.int()?)
        }
        _ => return Ok(None),
    };

    if &buffer.magic()? != b"OTAF" {
        return Ok(None);
    }
    let fato_header_size = buffer.int()?;
    let fato_entry_count = buffer.short()?;
    let fato_padding = buffer.short()?;
    if fato_entry_count < 0 {
        return Err(FormatError::invalid(CONTEXT, "NegativeArraySizeException"));
    }
    for _ in 0..fato_entry_count {
        buffer.int()?;
    }

    if &buffer.magic()? != b"BTAF" {
        return Ok(None);
    }
    let fatb_header_size = buffer.int()?;
    let fatb_file_count = buffer.int()?;
    if fatb_file_count < 0 {
        return Err(FormatError::invalid(CONTEXT, "NegativeArraySizeException"));
    }
    let mut entries: Vec<BTreeMap<u32, SubEntry>> = Vec::new();
    for _ in 0..fatb_file_count {
        let mut bit_vector = buffer.int()? as u32;
        let mut sub_entries = BTreeMap::new();
        for bit in 0..32u32 {
            let exists = bit_vector & 1 == 1;
            bit_vector >>= 1;
            if !exists {
                continue;
            }
            let start = buffer.int()?;
            let _end = buffer.int()?;
            let length = buffer.int()?;
            sub_entries.insert(bit, SubEntry { start, length });
        }
        entries.push(sub_entries);
    }

    if &buffer.magic()? != b"BMIF" {
        return Ok(None);
    }
    let fimb_header_size = buffer.int()?;
    let _fimb_data_size = buffer.int()?;

    let mut is_compressed = BTreeMap::new();
    let mut files = Vec::with_capacity(entries.len());
    for (index, sub_entries) in entries.iter().enumerate() {
        let mut directory = GarcDirectory::new();
        for (&key, sub_entry) in sub_entries {
            let position = data_offset as i64 + sub_entry.start as i64;
            if position < 0 || position > data.len() as i64 {
                return Err(FormatError::invalid(
                    CONTEXT,
                    "IllegalArgumentException: bad position",
                ));
            }
            let position = position as usize;
            if sub_entry.length < 0 {
                return Err(FormatError::invalid(CONTEXT, "NegativeArraySizeException"));
            }
            let first_byte = *data
                .get(position)
                .ok_or(FormatError::out_of_bounds(CONTEXT))?;
            let compressed = first_byte == 0x11
                && match decompression {
                    Decompression::Skip => false,
                    Decompression::All => true,
                    Decompression::Selected(flags) => *flags
                        .get(index)
                        .ok_or(FormatError::out_of_bounds(CONTEXT))?,
                };
            let file = data
                .get(position..position + sub_entry.length as usize)
                .ok_or_else(Buffer::underflow)?;
            if compressed {
                match lzss::decode(file) {
                    Ok(LzssDecoded::Data(decoded)) => {
                        directory.insert(key, Some(decoded));
                    }
                    Ok(LzssDecoded::Rejected(_)) => {
                        directory.insert(key, None);
                    }
                    Err(_) => return Ok(None),
                }
                is_compressed.insert(index, true);
            } else {
                directory.insert(key, Some(file.to_vec()));
                is_compressed.insert(index, false);
            }
        }
        files.push(directory);
    }

    Ok(Some(GarcArchive {
        version,
        header: GarcHeader {
            version: garc_version,
            content_pad_to_nearest,
        },
        fato_header_size,
        fato_entry_count,
        fato_padding,
        fatb_header_size,
        fatb_file_count,
        fimb_header_size,
        is_compressed,
        files,
    }))
}
