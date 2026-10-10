use crate::bytes::{read_long, read_word};
use crate::error::{FormatError, Result};
use crate::java::arrays::{array_copy, new_array};

const CONTEXT: &str = "Mini";

pub fn pack(files: &[Vec<u8>], identifier: &str) -> Result<Vec<u8>> {
    let mut identifier_units = identifier.encode_utf16();
    let (Some(first), Some(second)) = (identifier_units.next(), identifier_units.next()) else {
        return Err(FormatError::invalid(
            CONTEXT,
            "identifier needs two characters",
        ));
    };
    let count = files.len();
    let data_offset = 4 + 4 + count * 4;
    let mut header = vec![first as u8, second as u8];
    header.extend_from_slice(&(count as u16).to_le_bytes());
    let mut offsets = Vec::with_capacity((count + 1) * 4);
    let mut payload: Vec<u8> = Vec::new();
    for file in files {
        offsets.extend_from_slice(&((payload.len() + data_offset) as i32).to_le_bytes());
        payload.extend_from_slice(file);
        while !payload.len().is_multiple_of(4) {
            payload.push(0);
        }
    }
    offsets.extend_from_slice(&((payload.len() + data_offset) as i32).to_le_bytes());
    let mut packed = header;
    packed.extend_from_slice(&offsets);
    packed.extend_from_slice(&payload);
    Ok(packed)
}

pub fn unpack(data: &[u8], identifier: &str) -> Result<Option<Vec<Vec<u8>>>> {
    if data.len() < 4 {
        return Ok(None);
    }
    let mut identifier_units = identifier.encode_utf16();
    let (Some(first), Some(second)) = (identifier_units.next(), identifier_units.next()) else {
        return Err(FormatError::invalid(
            CONTEXT,
            "identifier needs two characters",
        ));
    };
    let signed = |byte: u8| byte as i8 as i32;
    if first as i32 != signed(data[0]) || second as i32 != signed(data[1]) {
        return Ok(None);
    }
    let count = read_word(data, 2) as usize;
    let mut cursor = 4usize;
    let read_offset = |cursor: usize| -> Result<i64> {
        if cursor + 4 > data.len() {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        Ok(read_long(data, cursor) as i64)
    };
    let mut start = read_offset(cursor)?;
    cursor += 4;
    let mut files = Vec::with_capacity(count);
    for _ in 0..count {
        let end = read_offset(cursor)?;
        cursor += 4;
        let length = end - start;
        let mut file = new_array(length, CONTEXT)?;
        array_copy(data, start, &mut file, 0, length, CONTEXT)?;
        files.push(file);
        start = end;
    }
    Ok(Some(files))
}
