use std::io::{Read, Seek, SeekFrom, Write};

use crate::error::{FormatError, Result};

pub struct RandomAccessFile<F> {
    inner: F,
    pointer: u64,
}

impl<F: Read + Write + Seek> RandomAccessFile<F> {
    pub fn new(inner: F) -> Self {
        RandomAccessFile { inner, pointer: 0 }
    }

    pub fn seek(&mut self, position: i64) -> Result<()> {
        if position < 0 {
            return Err(FormatError::invalid(
                "RandomAccessFile.seek",
                "negative seek offset",
            ));
        }
        self.pointer = position as u64;
        Ok(())
    }

    pub fn pointer(&self) -> u64 {
        self.pointer
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.inner.seek(SeekFrom::Start(self.pointer))?;
        self.inner.write_all(data)?;
        self.pointer += data.len() as u64;
        Ok(())
    }

    pub fn write_byte(&mut self, value: u8) -> Result<()> {
        self.write(&[value])
    }

    pub fn read_fully(&mut self, buffer: &mut [u8]) -> Result<()> {
        self.inner.seek(SeekFrom::Start(self.pointer))?;
        self.inner.read_exact(buffer)?;
        self.pointer += buffer.len() as u64;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.inner.flush()?;
        Ok(())
    }

    pub fn into_inner(self) -> F {
        self.inner
    }
}

pub fn read_exact_at<R: Read + Seek>(
    source: &mut R,
    offset: u64,
    length: usize,
) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; length];
    source.seek(SeekFrom::Start(offset))?;
    source.read_exact(&mut buffer)?;
    Ok(buffer)
}

pub fn read_i32_le_at<R: Read + Seek>(source: &mut R, offset: u64) -> Result<i32> {
    let bytes = read_exact_at(source, offset, 4)?;
    Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub fn read_i32_be_at<R: Read + Seek>(source: &mut R, offset: u64) -> Result<i32> {
    let bytes = read_exact_at(source, offset, 4)?;
    Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub fn read_u16_be_at<R: Read + Seek>(source: &mut R, offset: u64) -> Result<i32> {
    let bytes = read_exact_at(source, offset, 2)?;
    Ok((bytes[0] as i32) << 8 | bytes[1] as i32)
}

pub fn read_le_sized_at<R: Read + Seek>(source: &mut R, offset: u64, size: usize) -> Result<i32> {
    let bytes = read_exact_at(source, offset, size)?;
    Ok(crate::bytes::read_le_sized(&bytes, 0, size))
}

pub fn non_negative_offset(value: i64, context: &'static str) -> Result<u64> {
    u64::try_from(value)
        .map_err(|_| FormatError::invalid(context, format!("negative offset {value}")))
}
