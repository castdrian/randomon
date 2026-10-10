use crate::error::{FormatError, Result};

const CONTEXT: &str = "RomFunctions.search";

pub fn search(haystack: &[u8], needle: &[u8]) -> Result<Vec<usize>> {
    search_range(haystack, 0, haystack.len(), needle)
}

pub fn search_from(haystack: &[u8], begin: usize, needle: &[u8]) -> Result<Vec<usize>> {
    search_range(haystack, begin, haystack.len(), needle)
}

pub fn search_range(
    haystack: &[u8],
    begin: usize,
    end: usize,
    needle: &[u8],
) -> Result<Vec<usize>> {
    let table = kmp_table(needle)?;
    let mut results = Vec::new();
    let mut match_start = begin as i64;
    let mut position: i64 = 0;
    while match_start + position < end as i64 {
        let haystack_index = (match_start + position) as usize;
        let byte = *haystack
            .get(haystack_index)
            .ok_or(FormatError::out_of_bounds(CONTEXT))?;
        if needle[position as usize] == byte {
            position += 1;
            if position as usize == needle.len() {
                results.push(match_start as usize);
                position = 0;
                match_start += needle.len() as i64;
            }
        } else {
            let fallback = table[position as usize];
            match_start += position - fallback;
            position = if fallback > -1 { fallback } else { 0 };
        }
    }
    Ok(results)
}

pub fn search_for_first(haystack: &[u8], begin: usize, needle: &[u8]) -> Result<Option<usize>> {
    let table = kmp_table(needle)?;
    let mut match_start = begin as i64;
    let mut position: i64 = 0;
    while match_start + position < haystack.len() as i64 {
        let byte = haystack[(match_start + position) as usize];
        if needle[position as usize] == byte {
            position += 1;
            if position as usize == needle.len() {
                return Ok(Some(match_start as usize));
            }
        } else {
            let fallback = table[position as usize];
            match_start += position - fallback;
            position = if fallback > -1 { fallback } else { 0 };
        }
    }
    Ok(None)
}

fn kmp_table(needle: &[u8]) -> Result<Vec<i64>> {
    if needle.len() < 2 {
        return Err(FormatError::invalid(
            CONTEXT,
            "UPR's KMP table needs a needle of at least two bytes",
        ));
    }
    let mut table = vec![0i64; needle.len()];
    table[0] = -1;
    table[1] = 0;
    let mut pos = 2;
    let mut candidate = 0usize;
    while pos < needle.len() {
        if needle[pos - 1] == needle[candidate] {
            table[pos] = candidate as i64 + 1;
            pos += 1;
            candidate += 1;
        } else if candidate > 0 {
            candidate = table[candidate] as usize;
        } else {
            table[pos] = 0;
            pos += 1;
        }
    }
    Ok(table)
}

pub fn free_space_finder(
    rom: &[u8],
    free_space: u8,
    amount: usize,
    offset: usize,
    long_aligned: bool,
) -> Result<i32> {
    let found_or_minus_one = |needle: Vec<u8>| -> Result<i32> {
        Ok(search_for_first(rom, offset, &needle)?.map_or(-1, |found| found as i32))
    };
    if long_aligned {
        let found = found_or_minus_one(vec![free_space; amount + 5])?;
        Ok((found + 5) & !3)
    } else {
        let found = found_or_minus_one(vec![free_space; amount + 2])?;
        Ok(found + 2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexFind {
    Unique(usize),
    NotFound,
    NotUnique,
    OddLength,
}

impl HexFind {
    pub fn upr_code(self) -> i32 {
        match self {
            HexFind::Unique(offset) => offset as i32,
            HexFind::NotFound => -1,
            HexFind::NotUnique => -2,
            HexFind::OddLength => -3,
        }
    }
}

pub fn find_hex(haystack: &[u8], hex: &str) -> Result<HexFind> {
    if !hex.encode_utf16().count().is_multiple_of(2) {
        return Ok(HexFind::OddLength);
    }
    let needle = parse_hex_bytes(hex)?;
    let found = search(haystack, &needle)?;
    Ok(match found.as_slice() {
        [] => HexFind::NotFound,
        [only] => HexFind::Unique(*only),
        _ => HexFind::NotUnique,
    })
}

pub fn find_hex_multiple(haystack: &[u8], hex: &str) -> Result<Vec<usize>> {
    if !hex.encode_utf16().count().is_multiple_of(2) {
        return Ok(Vec::new());
    }
    search(haystack, &parse_hex_bytes(hex)?)
}

pub fn write_hex_string(data: &mut [u8], hex: &str, offset: usize) -> Result<()> {
    if !hex.encode_utf16().count().is_multiple_of(2) {
        return Ok(());
    }
    for (index, byte) in parse_hex_bytes(hex)?.into_iter().enumerate() {
        *data
            .get_mut(offset + index)
            .ok_or(FormatError::out_of_bounds("writeHexString"))? = byte;
    }
    Ok(())
}

pub fn parse_hex_bytes(hex: &str) -> Result<Vec<u8>> {
    let units: Vec<u16> = hex.encode_utf16().collect();
    units
        .chunks(2)
        .map(|pair| java_parse_hex_pair(pair).map(|value| value as u8))
        .collect()
}

fn java_parse_hex_pair(pair: &[u16]) -> Result<i32> {
    let invalid = || FormatError::invalid("Integer.parseInt", String::from_utf16_lossy(pair));
    let digit = |unit: u16| {
        char::from_u32(unit as u32)
            .and_then(|c| c.to_digit(16))
            .map(|d| d as i32)
    };
    match pair {
        [sign, last] if *sign == '-' as u16 => digit(*last).map(|d| -d).ok_or_else(invalid),
        [sign, last] if *sign == '+' as u16 => digit(*last).ok_or_else(invalid),
        [high, low] => match (digit(*high), digit(*low)) {
            (Some(high), Some(low)) => Ok(high * 16 + low),
            _ => Err(invalid()),
        },
        _ => Err(invalid()),
    }
}
