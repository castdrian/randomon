use crate::error::{FormatError, Result};
use crate::java::arrays::new_array;

const CONTEXT: &str = "DSDecmp";

pub fn decompress(data: &[u8]) -> Result<Option<Vec<u8>>> {
    decompress_at(data, 0)
}

pub fn decompress_at(data: &[u8], offset: usize) -> Result<Option<Vec<u8>>> {
    match data.get(offset) {
        Some(0x10) => decompress_lz10(data, offset).map(Some),
        Some(0x11) => decompress_lz11(data, offset).map(Some),
        Some(_) => Ok(None),
        None => Err(FormatError::out_of_bounds(CONTEXT)),
    }
}

struct Input<'a> {
    data: &'a [u8],
    offset: usize,
}

impl Input<'_> {
    fn next(&mut self) -> Result<u8> {
        let byte = *self
            .data
            .get(self.offset)
            .ok_or(FormatError::out_of_bounds(CONTEXT))?;
        self.offset += 1;
        Ok(byte)
    }
}

fn read_header(data: &[u8], offset: usize) -> Result<(Vec<u8>, Input<'_>)> {
    let mut input = Input {
        data,
        offset: offset + 1,
    };
    let mut length =
        input.next()? as i32 | (input.next()? as i32) << 8 | (input.next()? as i32) << 16;
    if length == 0 {
        length = i32::from_be_bytes([input.next()?, input.next()?, input.next()?, input.next()?]);
    }
    Ok((new_array(length as i64, CONTEXT)?, input))
}

fn copy_back_reference(
    output: &mut [u8],
    size: &mut usize,
    displacement: usize,
    count: usize,
) -> Result<()> {
    if displacement >= *size {
        return Err(FormatError::invalid(
            CONTEXT,
            "Cannot go back more than already written",
        ));
    }
    let start = *size - displacement - 1;
    for index in 0..count {
        if *size >= output.len() {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        output[*size] = output[start + index];
        *size += 1;
    }
    Ok(())
}

fn decompress_lz10(data: &[u8], offset: usize) -> Result<Vec<u8>> {
    let (mut output, mut input) = read_header(data, offset)?;
    let mut size = 0usize;
    while size < output.len() {
        let flags = input.next()?;
        for bit in 0..8 {
            if flags & (0x80 >> bit) != 0 {
                let first = input.next()?;
                let count = (first >> 4) as usize + 3;
                let displacement = ((first as usize & 0x0F) << 8) | input.next()? as usize;
                copy_back_reference(&mut output, &mut size, displacement, count)?;
            } else {
                let literal = input.next()?;
                if size >= output.len() {
                    break;
                }
                output[size] = literal;
                size += 1;
            }
        }
    }
    Ok(output)
}

fn decompress_lz11(data: &[u8], offset: usize) -> Result<Vec<u8>> {
    let (mut output, mut input) = read_header(data, offset)?;
    let mut size = 0usize;
    while size < output.len() {
        let flags = input.next()?;
        let mut bit = 0;
        while bit < 8 && size < output.len() {
            if flags & (0x80 >> bit) != 0 {
                let (count, displacement) = read_lz11_reference(&mut input)?;
                copy_lz11_reference(&mut output, &mut size, displacement, count)?;
            } else {
                output[size] = input.next()?;
                size += 1;
            }
            bit += 1;
        }
    }
    Ok(output)
}

fn read_lz11_reference(input: &mut Input<'_>) -> Result<(usize, usize)> {
    let first = input.next()? as usize;
    match first >> 4 {
        0 => {
            let second = input.next()? as usize;
            let third = input.next()? as usize;
            let count = ((first << 4) | (second >> 4)) + 0x11;
            let displacement = ((second & 0x0F) << 8) | third;
            Ok((count, displacement))
        }
        1 => {
            let second = input.next()? as usize;
            let third = input.next()? as usize;
            let fourth = input.next()? as usize;
            let count = (((first & 0x0F) << 12) | (second << 4) | (third >> 4)) + 0x111;
            let displacement = ((third & 0x0F) << 8) | fourth;
            Ok((count, displacement))
        }
        _ => {
            let second = input.next()? as usize;
            Ok(((first >> 4) + 1, ((first & 0x0F) << 8) | second))
        }
    }
}

fn copy_lz11_reference(
    output: &mut [u8],
    size: &mut usize,
    displacement: usize,
    count: usize,
) -> Result<()> {
    if displacement >= *size {
        return Err(FormatError::invalid(
            CONTEXT,
            "Cannot go back more than already written",
        ));
    }
    let start = *size - displacement - 1;
    let mut index = 0;
    while index < count && *size < output.len() {
        output[*size] = output[start + index];
        *size += 1;
        index += 1;
    }
    Ok(())
}
