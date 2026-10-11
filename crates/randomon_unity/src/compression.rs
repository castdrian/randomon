use crate::error::{Result, UnityError};

pub const COMPRESSION_MASK: u32 = 0x3f;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Lzma,
    Lz4,
    Lz4Hc,
}

impl Compression {
    pub fn from_flags(flags: u32) -> Result<Self> {
        match flags & COMPRESSION_MASK {
            0 => Ok(Self::None),
            1 => Ok(Self::Lzma),
            2 => Ok(Self::Lz4),
            3 => Ok(Self::Lz4Hc),
            other => Err(UnityError::UnsupportedCompression(other)),
        }
    }

    pub fn flag_bits(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Lzma => 1,
            Self::Lz4 => 2,
            Self::Lz4Hc => 3,
        }
    }
}

pub fn decompress(compression: Compression, input: &[u8], output_size: usize) -> Result<Vec<u8>> {
    let output = match compression {
        Compression::None => input.to_vec(),
        Compression::Lz4 | Compression::Lz4Hc => lz4_flex::block::decompress(input, output_size)
            .map_err(|error| UnityError::Decompression(error.to_string()))?,
        Compression::Lzma => decompress_lzma(input, output_size)?,
    };
    if output.len() != output_size {
        return Err(UnityError::Decompression(format!(
            "expected {output_size} bytes, got {}",
            output.len()
        )));
    }
    Ok(output)
}

fn decompress_lzma(input: &[u8], output_size: usize) -> Result<Vec<u8>> {
    let options = lzma_rs::decompress::Options {
        unpacked_size: lzma_rs::decompress::UnpackedSize::UseProvided(Some(output_size as u64)),
        ..Default::default()
    };
    let mut reader = std::io::Cursor::new(input);
    let mut output = Vec::with_capacity(output_size);
    lzma_rs::lzma_decompress_with_options(&mut reader, &mut output, &options)
        .map_err(|error| UnityError::Decompression(error.to_string()))?;
    Ok(output)
}

pub fn compress(compression: Compression, input: &[u8]) -> Result<Vec<u8>> {
    match compression {
        Compression::None => Ok(input.to_vec()),
        Compression::Lz4 | Compression::Lz4Hc => Ok(lz4_flex::block::compress(input)),
        Compression::Lzma => compress_lzma(input),
    }
}

fn compress_lzma(input: &[u8]) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    let options = lzma_rs::compress::Options {
        unpacked_size: lzma_rs::compress::UnpackedSize::SkipWritingToHeader,
    };
    lzma_rs::lzma_compress_with_options(&mut std::io::Cursor::new(input), &mut encoded, &options)
        .map_err(|error| UnityError::Decompression(error.to_string()))?;
    Ok(encoded)
}
