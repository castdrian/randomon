use crate::bytes::{read_long, read_long_be, read_word};
use crate::error::{FormatError, Result};
use crate::java::arrays::slice_at;

const CONTEXT: &str = "BFLIM";
const FLIM_SIGNATURE: i32 = 0x464C494D;
const IMAG_SIGNATURE: i32 = 0x696D6167;

const SWIZZLE_LUT: [usize; 64] = [
    0, 1, 8, 9, 2, 3, 10, 11, 16, 17, 24, 25, 18, 19, 26, 27, 4, 5, 12, 13, 6, 7, 14, 15, 20, 21,
    28, 29, 22, 23, 30, 31, 32, 33, 40, 41, 34, 35, 42, 43, 48, 49, 56, 57, 50, 51, 58, 59, 36, 37,
    44, 45, 38, 39, 46, 47, 52, 53, 60, 61, 54, 55, 62, 63,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bflim {
    pub version: i32,
    pub width: i16,
    pub height: i16,
    pub alignment: i16,
    pub format: i8,
    pub flags: i8,
    pub size: i32,
    image_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgbImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl Bflim {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 0x28 {
            return Err(FormatError::invalid(
                CONTEXT,
                "Invalid BFLIM: not long enough to contain a header",
            ));
        }
        let header_offset = bytes.len() - 0x28;
        if read_long_be(bytes, header_offset) != FLIM_SIGNATURE {
            return Err(FormatError::invalid(
                CONTEXT,
                "Invalid BFLIM: cannot find FLIM header",
            ));
        }
        if read_word(bytes, header_offset + 4) == 0xFFFE {
            return Err(FormatError::invalid(
                CONTEXT,
                "Unsupported BFLIM: this is a big endian BFLIM",
            ));
        }
        if read_word(bytes, header_offset + 6) != 0x14 {
            return Err(FormatError::invalid(
                CONTEXT,
                "Invalid BFLIM: header length does not equal 0x14",
            ));
        }
        let version = read_long(bytes, header_offset + 8);
        let image_offset = bytes.len() - 0x14;
        if read_long_be(bytes, image_offset) != IMAG_SIGNATURE {
            return Err(FormatError::invalid(
                CONTEXT,
                "Invalid BFLIM: cannot find imag header",
            ));
        }
        let image_size = read_long(bytes, image_offset + 16);
        let image_data = slice_at(bytes, 0, image_size as i64, CONTEXT)?.to_vec();
        Ok(Bflim {
            version,
            size: read_long(bytes, image_offset + 4),
            width: read_word(bytes, image_offset + 8) as i16,
            height: read_word(bytes, image_offset + 10) as i16,
            alignment: read_word(bytes, image_offset + 12) as i16,
            format: bytes[image_offset + 14] as i8,
            flags: bytes[image_offset + 15] as i8,
            image_data,
        })
    }

    pub fn image(&self) -> Result<ArgbImage> {
        let width = usize::try_from(self.width)
            .map_err(|_| FormatError::invalid(CONTEXT, "negative width"))?;
        let height = usize::try_from(self.height)
            .map_err(|_| FormatError::invalid(CONTEXT, "negative height"))?;
        let decoded = self.decode_block(height, width)?;
        let colors = convert_to_color_data(&decoded);
        let pixels = rearrange_image(&colors, width, height);
        Ok(ArgbImage {
            width,
            height,
            pixels,
        })
    }

    fn decode_block(&self, width: usize, height: usize) -> Result<Vec<i32>> {
        let mut output = vec![0i32; width * height * 4];
        let mut input_offset = 0usize;
        for tile_y in (0..height).step_by(8) {
            for tile_x in (0..width).step_by(8) {
                for &swizzled in &SWIZZLE_LUT {
                    let x = swizzled & 7;
                    let y = (swizzled - x) >> 3;
                    let row = height as i64 - 1 - (tile_y + y) as i64;
                    let output_offset = (tile_x as i64 + x as i64 + row * width as i64) * 4;
                    if input_offset + 2 > self.image_data.len() {
                        return Err(FormatError::out_of_bounds(CONTEXT));
                    }
                    let value = read_word(&self.image_data, input_offset);
                    let channels = match self.format {
                        7 => decode_rgba5551(value),
                        8 => decode_rgba4(value),
                        _ => {
                            return Err(FormatError::invalid(
                                CONTEXT,
                                "Unsupported BFLIM: unsupported image format",
                            ));
                        }
                    };
                    let target = slice_index(output_offset, output.len())?;
                    output[target..target + 4].copy_from_slice(&channels);
                    input_offset += 2;
                }
            }
        }
        Ok(output)
    }
}

fn slice_index(offset: i64, length: usize) -> Result<usize> {
    if offset < 0 || offset as usize + 4 > length {
        return Err(FormatError::out_of_bounds(CONTEXT));
    }
    Ok(offset as usize)
}

fn decode_rgba5551(value: i32) -> [i32; 4] {
    let mut red = ((value >> 1) & 0x1F) << 3;
    let mut green = ((value >> 6) & 0x1F) << 3;
    let mut blue = ((value >> 11) & 0x1F) << 3;
    let alpha = (value & 1) * 0xFF;
    red |= red >> 5;
    green |= green >> 5;
    blue |= blue >> 5;
    [alpha, blue, green, red]
}

fn decode_rgba4(value: i32) -> [i32; 4] {
    let mut red = (value >> 4) & 0xF;
    let mut green = (value >> 8) & 0xF;
    let mut blue = (value >> 12) & 0xF;
    let alpha = (value & 1) | (value << 4);
    red |= red << 4;
    green |= green << 4;
    blue |= blue << 4;
    [alpha, blue, green, red]
}

fn convert_to_color_data(decoded: &[i32]) -> Vec<u32> {
    decoded
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[a, b, g, r]| ((a << 24) | (b << 16) | (g << 8) | r) as u32)
        .collect()
}

fn rearrange_image(colors: &[u32], width: usize, height: usize) -> Vec<u32> {
    let mut output = vec![0u32; colors.len()];
    for destination_y in 0..height {
        for destination_x in 0..width {
            let source_x = height - destination_y - 1;
            let source_y = width - destination_x - 1;
            output[destination_x + destination_y * width] = colors[source_x + source_y * height];
        }
    }
    output
}
