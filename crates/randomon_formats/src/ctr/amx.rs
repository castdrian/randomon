use crate::bytes::{read_long, read_word};
use crate::error::{FormatError, Result};
use crate::java::arrays::{copy_of_range, new_array};

const CONTEXT: &str = "AMX";
const AMX_MAGIC: i32 = 0x0A0AF1E0;
const EXTRA_DATA_START: i64 = 0x1C;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amx {
    pub dec_data: Vec<u8>,
    pub script_offset: i32,
    length: i32,
    ptr_offset: i32,
    ptr_count: i32,
    script_instr_start: i32,
    script_movement_start: i32,
    final_offset: i32,
    allocated_memory: i32,
    extra_data: Vec<u8>,
}

impl Amx {
    pub fn parse(encoded: &[u8]) -> Result<Self> {
        read_header_and_decompress(encoded)
    }

    pub fn find_script(data: &[u8], script_number: usize) -> Result<Option<Self>> {
        let mut found = 0usize;
        let limit = data.len() as i64 - 3;
        let mut index: i64 = 0;
        while index < limit {
            let position = index as usize;
            if read_long(data, position) == AMX_MAGIC {
                if found == script_number {
                    let start = index - 4;
                    if start < 0 {
                        return Err(FormatError::out_of_bounds(CONTEXT));
                    }
                    let length = read_long(data, start as usize) as i64;
                    let encoded = copy_of_range(data, start, start + length, CONTEXT)?;
                    let mut amx = read_header_and_decompress(&encoded)?;
                    amx.script_offset = start as i32;
                    return Ok(Some(amx));
                }
                found += 1;
            }
            index += 1;
        }
        Ok(None)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let compressed = compress_script(&self.dec_data).ok_or_else(|| {
            FormatError::invalid(CONTEXT, "decompressed script length is not a multiple of 4")
        })?;
        let capacity = (self.length as i64) * 2;
        let mut output = Vec::new();
        output.extend_from_slice(&self.length.to_le_bytes());
        output.extend_from_slice(&AMX_MAGIC.to_le_bytes());
        output.extend_from_slice(&(self.ptr_offset as u16).to_le_bytes());
        output.extend_from_slice(&(self.ptr_count as u16).to_le_bytes());
        for value in [
            self.script_instr_start,
            self.script_movement_start,
            self.final_offset,
            self.allocated_memory,
        ] {
            output.extend_from_slice(&value.to_le_bytes());
        }
        output.extend_from_slice(&self.extra_data);
        output.extend_from_slice(&compressed);
        if output.len() as i64 > capacity {
            return Err(FormatError::invalid(CONTEXT, "BufferOverflowException"));
        }
        let total = output.len() as i32;
        output[..4].copy_from_slice(&total.to_le_bytes());
        Ok(output)
    }
}

fn checked_long(data: &[u8], offset: usize) -> Result<i32> {
    if offset + 4 > data.len() {
        return Err(FormatError::out_of_bounds(CONTEXT));
    }
    Ok(read_long(data, offset))
}

fn checked_word(data: &[u8], offset: usize) -> Result<i32> {
    if offset + 2 > data.len() {
        return Err(FormatError::out_of_bounds(CONTEXT));
    }
    Ok(read_word(data, offset))
}

fn read_header_and_decompress(encoded: &[u8]) -> Result<Amx> {
    let length = checked_long(encoded, 0)?;
    if checked_long(encoded, 4)? != AMX_MAGIC {
        return Err(FormatError::invalid(CONTEXT, "missing AMX magic"));
    }
    let ptr_offset = checked_word(encoded, 8)?;
    let ptr_count = checked_word(encoded, 0xA)?;
    let script_instr_start = checked_long(encoded, 0xC)?;
    let script_movement_start = checked_long(encoded, 0x10)?;
    let final_offset = checked_long(encoded, 0x14)?;
    let allocated_memory = checked_long(encoded, 0x18)?;
    let compressed = copy_of_range(encoded, script_instr_start as i64, length as i64, CONTEXT)?;
    let decompressed_length = final_offset as i64 - script_instr_start as i64;
    let dec_data = decompress_bytes(&compressed, decompressed_length)?;
    let extra_data = copy_of_range(
        encoded,
        EXTRA_DATA_START,
        script_instr_start as i64,
        CONTEXT,
    )?;
    Ok(Amx {
        dec_data,
        script_offset: 0,
        length,
        ptr_offset,
        ptr_count,
        script_instr_start,
        script_movement_start,
        final_offset,
        allocated_memory,
        extra_data,
    })
}

fn decompress_bytes(data: &[u8], length: i64) -> Result<Vec<u8>> {
    let mut code = new_array(length, CONTEXT)?;
    let mut written = 0usize;
    let mut groups_in_value = 0;
    let mut value: i32 = 0;
    let mut source = data.iter();
    while written < code.len() {
        let byte = *source.next().ok_or(FormatError::out_of_bounds(CONTEXT))? as i8 as i32;
        let low_bits = byte & 0x7F;
        groups_in_value += 1;
        if groups_in_value == 1 {
            let sign_extension = if (low_bits >> 6) == 0 { 0 } else { -1 };
            value = (sign_extension << 6) | low_bits;
        } else {
            value = (value << 7) | (low_bits & 0xFF);
        }
        if byte & 0x80 != 0 {
            continue;
        }
        if written + 4 > code.len() {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        code[written..written + 4].copy_from_slice(&value.to_le_bytes());
        written += 4;
        groups_in_value = 0;
    }
    Ok(code)
}

fn compress_script(data: &[u8]) -> Option<Vec<u8>> {
    if !data.len().is_multiple_of(4) {
        return None;
    }
    let mut output = Vec::new();
    for word in data.as_chunks::<4>().0 {
        let instruction = i32::from_le_bytes(*word);
        output.extend(compress_instruction(instruction));
    }
    Some(output)
}

fn compress_instruction(instruction_bits: i32) -> Vec<u8> {
    let mut instruction = instruction_bits as u32 as u64;
    let negative = instruction & 0x8000_0000 != 0;
    let mut shadow = if negative {
        !instruction_bits
    } else {
        instruction_bits
    } as u32 as u64;
    let mut groups: Vec<u8> = Vec::new();
    loop {
        let mut byte = (instruction & 0x7F) as u8;
        if !groups.is_empty() {
            byte |= 0x80;
        }
        groups.push(byte);
        instruction >>= 7;
        shadow >>= 7;
        if shadow == 0 {
            break;
        }
    }
    if groups.len() < 5 {
        let sign_bit = if negative { 0x40 } else { 0x00 };
        if groups[groups.len() - 1] & 0x40 != sign_bit {
            groups.push(if negative { 0xFF } else { 0x80 });
        }
    }
    groups.reverse();
    groups
}
