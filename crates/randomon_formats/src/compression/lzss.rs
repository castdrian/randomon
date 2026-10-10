use crate::error::{FormatError, Result};

const CONTEXT: &str = "BLZCoder.LZSS";
const MAX_INPUT: usize = 0xFF_FFFF;
const WINDOW: usize = 0x1000;
const MAX_MATCH: usize = 0x10110;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LzssDecoded {
    Data(Vec<u8>),
    Rejected(&'static str),
}

pub fn decode(data: &[u8]) -> Result<LzssDecoded> {
    let first = *data.first().ok_or(FormatError::out_of_bounds(CONTEXT))?;
    if first != 0x11 {
        return Ok(LzssDecoded::Rejected("Not a valid LZSS compressed file"));
    }
    let mut input = Reader { data, position: 0 };
    let mut size = (input.int()? as u32 >> 8) as i64;
    if size == 0 {
        size = input.int()? as i64;
    }
    if size < 0 {
        return Err(FormatError::invalid(CONTEXT, "negative decompressed size"));
    }
    let size = size as usize;
    let mut output: Vec<u8> = Vec::with_capacity(size);
    let mut flags: u8 = 0;
    let mut mask: u32 = 1;
    while output.len() < size {
        if mask == 1 {
            if input.position >= data.len() {
                return Ok(LzssDecoded::Rejected("Not enough data"));
            }
            flags = input.byte()?;
            mask = 0x80;
        } else {
            mask >>= 1;
        }
        if flags as u32 & mask > 0 {
            match decode_reference(&mut input, data.len())? {
                Some((length, displacement)) => {
                    if displacement > output.len() {
                        return Ok(LzssDecoded::Rejected("oops"));
                    }
                    let start = output.len() - displacement;
                    for index in (start..).take(length) {
                        if output.len() == size {
                            return Err(FormatError::invalid(CONTEXT, "BufferOverflowException"));
                        }
                        output.push(output[index]);
                    }
                }
                None => return Ok(LzssDecoded::Rejected("Not enough data")),
            }
        } else {
            let next = input.byte()?;
            output.push(next);
        }
    }
    let position = input.position;
    if (position ^ (position & 0x3)) + 4 < data.len() {
        return Ok(LzssDecoded::Rejected("Too much input"));
    }
    Ok(LzssDecoded::Data(output))
}

fn decode_reference(input: &mut Reader<'_>, length: usize) -> Result<Option<(usize, usize)>> {
    if input.position >= length {
        return Ok(None);
    }
    let first = input.byte()? as usize;
    match first >> 4 {
        0 => {
            if input.position + 1 >= length {
                return Ok(None);
            }
            let second = input.byte()? as usize;
            let third = input.byte()? as usize;
            let count = (((first & 0x0F) << 4) | (second >> 4)) + 0x11;
            let displacement = (((second & 0x0F) << 8) | third) + 1;
            Ok(Some((count, displacement)))
        }
        1 => {
            if input.position + 2 >= length {
                return Ok(None);
            }
            let second = input.byte()? as usize;
            let third = input.byte()? as usize;
            let fourth = input.byte()? as usize;
            let count = (((first & 0x0F) << 12) | (second << 4) | (third >> 4)) + 0x111;
            let displacement = (((third & 0x0F) << 8) | fourth) + 1;
            Ok(Some((count, displacement)))
        }
        _ => {
            let second = input.byte()? as usize;
            let count = (first >> 4) + 1;
            let displacement = (((first & 0x0F) << 8) | second) + 1;
            Ok(Some((count, displacement)))
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8> {
        let value = *self
            .data
            .get(self.position)
            .ok_or_else(|| FormatError::invalid(CONTEXT, "BufferUnderflowException"))?;
        self.position += 1;
        Ok(value)
    }

    fn int(&mut self) -> Result<i32> {
        if self.position + 4 > self.data.len() {
            return Err(FormatError::invalid(CONTEXT, "BufferUnderflowException"));
        }
        let bytes = [
            self.data[self.position],
            self.data[self.position + 1],
            self.data[self.position + 2],
            self.data[self.position + 3],
        ];
        self.position += 4;
        Ok(i32::from_le_bytes(bytes))
    }
}

pub fn encode(data: &[u8]) -> Result<Option<Vec<u8>>> {
    if data.len() > MAX_INPUT {
        return Ok(None);
    }
    let capacity = data.len() + 8;
    let mut output = Vec::with_capacity(capacity);
    output.push(0x11);
    output.extend_from_slice(&(data.len() as u32).to_le_bytes()[..3]);
    if data.is_empty() {
        output.extend_from_slice(&[0, 0, 0, 0]);
        return Ok(Some(output));
    }

    let mut block = vec![0u8];
    let mut buffered_tokens = 0;
    let mut position = 0usize;
    while position < data.len() {
        if buffered_tokens == 8 {
            flush_block(&mut output, &block, capacity)?;
            block.clear();
            block.push(0);
            buffered_tokens = 0;
        }
        let old_length = position.min(WINDOW);
        let new_length = (data.len() - position).min(MAX_MATCH);
        let (length, displacement) = occurrence_length(
            data,
            position,
            new_length,
            position - old_length,
            old_length,
        );
        if length < 3 {
            block.push(data[position]);
            position += 1;
        } else {
            position += length;
            block[0] |= 1 << (7 - buffered_tokens);
            push_reference(&mut block, length, displacement);
        }
        buffered_tokens += 1;
    }
    if buffered_tokens > 0 {
        flush_block(&mut output, &block, capacity)?;
    }
    Ok(Some(output))
}

fn flush_block(output: &mut Vec<u8>, block: &[u8], capacity: usize) -> Result<()> {
    if output.len() + block.len() > capacity {
        return Err(FormatError::invalid(
            CONTEXT,
            "BufferOverflowException while encoding",
        ));
    }
    output.extend_from_slice(block);
    Ok(())
}

fn push_reference(block: &mut Vec<u8>, length: usize, displacement: usize) {
    let length = length as i32;
    let displacement_high = (((displacement as i32 - 1) as u32 >> 8) & 0x0F) as u8;
    let displacement_low = ((displacement as i32 - 1) & 0xFF) as u8;
    if length > 0x110 {
        let biased = length - 0x111;
        block.push((0x10 | ((biased as u32 >> 12) & 0x0F)) as u8);
        block.push(((biased as u32 >> 4) & 0xFF) as u8);
        block.push(((biased << 4) & 0xF0) as u8 | displacement_high);
    } else if length > 0x10 {
        let biased = length - 0x111;
        block.push(((biased as u32 >> 4) & 0x0F) as u8);
        block.push(((biased << 4) & 0xF0) as u8 | displacement_high);
    } else {
        block.push((((length - 1) << 4) & 0xF0) as u8 | displacement_high);
    }
    block.push(displacement_low);
}

fn occurrence_length(
    data: &[u8],
    new_index: usize,
    new_length: usize,
    old_index: usize,
    old_length: usize,
) -> (usize, usize) {
    if new_length == 0 {
        return (0, 0);
    }
    let mut max_length = 0;
    let mut displacement = 0;
    let candidates = old_length.saturating_sub(1);
    for offset in 0..candidates {
        let old_start = old_index + offset;
        let current = data[old_start..]
            .iter()
            .zip(&data[new_index..new_index + new_length])
            .take_while(|(old, new)| old == new)
            .count();
        if current > max_length {
            max_length = current;
            displacement = old_length - offset;
            if max_length == new_length {
                break;
            }
        }
    }
    (max_length, displacement)
}
