use std::collections::BTreeMap;

use crate::bytes::{read_long, read_word, write_long, write_word};
use crate::error::{FormatError, Result};
use crate::java::arrays::{array_copy, new_array};
use crate::java::strings::{decode_us_ascii, encode_us_ascii};

const CONTEXT: &str = "NARCArchive";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NarcArchive {
    pub files: Vec<Vec<u8>>,
    filenames: Vec<Option<String>>,
    has_filenames: bool,
}

impl NarcArchive {
    pub fn new() -> Self {
        NarcArchive::default()
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        let frames = read_nitro_frames(data)?;
        let (Some(fatb), Some(fntb), Some(fimg)) =
            (frames.get("FATB"), frames.get("FNTB"), frames.get("FIMG"))
        else {
            return Err(FormatError::invalid(CONTEXT, "Not a valid narc file"));
        };
        let file_count = checked_long(fatb, 0)?;
        let mut files = Vec::new();
        for index in 0..file_count.max(0) as i64 {
            let start = checked_long(fatb, (4 + index * 8) as usize)? as i64;
            let end = checked_long(fatb, (8 + index * 8) as usize)? as i64;
            let mut file = new_array(end - start, CONTEXT)?;
            array_copy(fimg, start, &mut file, 0, end - start, CONTEXT)?;
            files.push(file);
        }
        let has_filenames = checked_long(fntb, 0)? == 8;
        let filenames = if has_filenames {
            read_filenames_like_upstream(fntb, file_count.max(0) as usize)?
        } else {
            vec![None; file_count.max(0) as usize]
        };
        Ok(NarcArchive {
            files,
            filenames,
            has_filenames,
        })
    }

    pub fn has_filenames(&self) -> bool {
        self.has_filenames
    }

    pub fn filenames(&self) -> &[Option<String>] {
        &self.filenames
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let fatb = self.build_fatb();
        let fimg = self.build_fimg();
        let fntb = self.build_fntb();
        let length = 16 + fatb.len() + fntb.len() + fimg.len();
        let mut nitro = vec![0u8; length];
        nitro[..4].copy_from_slice(b"NARC");
        write_word(&mut nitro, 4, 0xFFFE);
        write_word(&mut nitro, 6, 0x0100);
        write_long(&mut nitro, 8, length as i32);
        write_word(&mut nitro, 12, 0x10);
        write_word(&mut nitro, 14, 3);
        let mut cursor = 16;
        for frame in [&fatb, &fntb, &fimg] {
            nitro[cursor..cursor + frame.len()].copy_from_slice(frame);
            cursor += frame.len();
        }
        nitro
    }

    fn build_fatb(&self) -> Vec<u8> {
        let mut fatb = vec![0u8; 4 + self.files.len() * 8 + 8];
        fatb[..4].copy_from_slice(b"BTAF");
        let length = fatb.len() as i32;
        write_long(&mut fatb, 4, length);
        write_long(&mut fatb, 8, self.files.len() as i32);
        let mut offset = 0usize;
        for (index, file) in self.files.iter().enumerate() {
            write_long(&mut fatb, 12 + index * 8, offset as i32);
            write_long(&mut fatb, 16 + index * 8, (offset + file.len()) as i32);
            offset += padded_length(file.len());
        }
        fatb
    }

    fn build_fimg(&self) -> Vec<u8> {
        let required: usize = self
            .files
            .iter()
            .map(|file| padded_length(file.len()))
            .sum();
        let mut fimg = vec![0u8; required + 8];
        fimg[..4].copy_from_slice(b"GMIF");
        let length = fimg.len() as i32;
        write_long(&mut fimg, 4, length);
        let mut offset = 8;
        for file in &self.files {
            let padded = padded_length(file.len());
            fimg[offset..offset + file.len()].copy_from_slice(file);
            fimg[offset + file.len()..offset + padded].fill(0xFF);
            offset += padded;
        }
        fimg
    }

    fn build_fntb(&self) -> Vec<u8> {
        let encoded: Vec<Vec<u8>> = if self.has_filenames {
            self.filenames
                .iter()
                .map(|name| encode_us_ascii(name.as_deref().unwrap_or("")))
                .collect()
        } else {
            Vec::new()
        };
        let length = 16 + encoded.iter().map(|name| name.len() + 1).sum::<usize>();
        let mut fntb = vec![0u8; length];
        fntb[..4].copy_from_slice(b"BTNF");
        write_long(&mut fntb, 4, length as i32);
        write_long(&mut fntb, 8, if self.has_filenames { 8 } else { 4 });
        write_long(&mut fntb, 12, 0x10000);
        let mut offset = 16;
        for name in &encoded {
            fntb[offset] = name.len() as u8;
            fntb[offset + 1..offset + 1 + name.len()].copy_from_slice(name);
            offset += 1 + name.len();
        }
        fntb
    }
}

fn padded_length(length: usize) -> usize {
    length.div_ceil(4) * 4
}

fn checked_long(data: &[u8], offset: usize) -> Result<i32> {
    if offset + 4 > data.len() {
        return Err(FormatError::out_of_bounds(CONTEXT));
    }
    Ok(read_long(data, offset))
}

fn read_filenames_like_upstream(fntb: &[u8], count: usize) -> Result<Vec<Option<String>>> {
    let mut names = Vec::with_capacity(count);
    let mut offset = 8usize;
    for _ in 0..count {
        let length = *fntb
            .get(offset)
            .ok_or(FormatError::out_of_bounds(CONTEXT))? as usize;
        offset += 1;
        let name = fntb
            .get(offset..offset + length)
            .ok_or(FormatError::out_of_bounds(CONTEXT))?;
        names.push(Some(decode_us_ascii(name)));
    }
    Ok(names)
}

fn read_nitro_frames(data: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    if data.len() < 0x10 {
        return Err(FormatError::out_of_bounds(CONTEXT));
    }
    let frame_count = read_word(data, 0x0E);
    let mut offset: i64 = 0x10;
    let mut frames = BTreeMap::new();
    for index in 0..frame_count {
        if offset < 0 || offset + 8 > data.len() as i64 {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        let start = offset as usize;
        let magic = [
            data[start + 3],
            data[start + 2],
            data[start + 1],
            data[start],
        ];
        let mut frame_size = read_long(data, start + 4) as i64;
        if index == frame_count - 1 && offset + frame_size < data.len() as i64 {
            frame_size = data.len() as i64 - offset;
        }
        let mut frame = new_array(frame_size - 8, CONTEXT)?;
        array_copy(data, offset + 8, &mut frame, 0, frame_size - 8, CONTEXT)?;
        frames.insert(decode_us_ascii(&magic), frame);
        offset += frame_size;
    }
    Ok(frames)
}
