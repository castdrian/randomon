use crate::error::{Result, UnityError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

pub struct ByteReader<'a> {
    data: &'a [u8],
    position: usize,
    endian: Endian,
}

macro_rules! read_number {
    ($name:ident, $type:ty) => {
        pub fn $name(&mut self) -> Result<$type> {
            let bytes = self.take_array::<{ std::mem::size_of::<$type>() }>()?;
            Ok(match self.endian {
                Endian::Little => <$type>::from_le_bytes(bytes),
                Endian::Big => <$type>::from_be_bytes(bytes),
            })
        }
    };
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8], endian: Endian) -> Self {
        Self {
            data,
            position: 0,
            endian,
        }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn set_position(&mut self, position: usize) {
        self.position = position;
    }

    pub fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    pub fn align(&mut self, alignment: usize) {
        self.position = self.position.next_multiple_of(alignment);
    }

    pub fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .filter(|&end| end <= self.data.len())
            .ok_or(UnityError::UnexpectedEof {
                offset: self.position,
                needed: length,
            })?;
        let slice = &self.data[self.position..end];
        self.position = end;
        Ok(slice)
    }

    pub fn take_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let slice = self.take(N)?;
        let mut array = [0u8; N];
        array.copy_from_slice(slice);
        Ok(array)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }

    read_number!(u16, u16);
    read_number!(i16, i16);
    read_number!(u32, u32);
    read_number!(i32, i32);
    read_number!(u64, u64);
    read_number!(i64, i64);
    read_number!(f32, f32);
    read_number!(f64, f64);

    pub fn cstring(&mut self) -> Result<String> {
        let rest = &self.data[self.position.min(self.data.len())..];
        let length = rest
            .iter()
            .position(|&byte| byte == 0)
            .ok_or(UnityError::UnexpectedEof {
                offset: self.position,
                needed: 1,
            })?;
        let start = self.position;
        let text = std::str::from_utf8(&rest[..length])
            .map_err(|_| UnityError::InvalidString(start))?
            .to_owned();
        self.position += length + 1;
        Ok(text)
    }
}

pub struct ByteWriter {
    buffer: Vec<u8>,
    endian: Endian,
}

macro_rules! write_number {
    ($name:ident, $type:ty) => {
        pub fn $name(&mut self, value: $type) {
            match self.endian {
                Endian::Little => self.buffer.extend_from_slice(&value.to_le_bytes()),
                Endian::Big => self.buffer.extend_from_slice(&value.to_be_bytes()),
            }
        }
    };
}

impl ByteWriter {
    pub fn new(endian: Endian) -> Self {
        Self {
            buffer: Vec::new(),
            endian,
        }
    }

    pub fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn align(&mut self, alignment: usize) {
        let target = self.buffer.len().next_multiple_of(alignment);
        self.buffer.resize(target, 0);
    }

    pub fn bytes(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn u8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    pub fn i8(&mut self, value: i8) {
        self.buffer.push(value as u8);
    }

    write_number!(u16, u16);
    write_number!(i16, i16);
    write_number!(u32, u32);
    write_number!(i32, i32);
    write_number!(u64, u64);
    write_number!(i64, i64);
    write_number!(f32, f32);
    write_number!(f64, f64);

    pub fn cstring(&mut self, text: &str) {
        self.buffer.extend_from_slice(text.as_bytes());
        self.buffer.push(0);
    }

    pub fn patch_u32_be(&mut self, offset: usize, value: u32) {
        self.buffer[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }
}
