use std::ops::Range;

use randomon_core::{RandomizationError, Result};
use randomon_formats::nds::NarcArchive;
use randomon_nds::find;

const CONTEXT: &str = "Gen 4 ROM data";

pub(crate) fn out_of_bounds() -> RandomizationError {
    RandomizationError::IndexOutOfBounds(CONTEXT)
}

fn span(data: &[u8], offset: i32, length: usize) -> Result<Range<usize>> {
    let start = usize::try_from(offset).map_err(|_| out_of_bounds())?;
    let end = start.checked_add(length).ok_or_else(out_of_bounds)?;
    if end > data.len() {
        return Err(out_of_bounds());
    }
    Ok(start..end)
}

pub(crate) fn u8_at(data: &[u8], offset: i32) -> Result<i32> {
    Ok(i32::from(data[span(data, offset, 1)?][0]))
}

pub(crate) fn i8_at(data: &[u8], offset: i32) -> Result<i32> {
    Ok(i32::from(data[span(data, offset, 1)?][0] as i8))
}

pub(crate) fn u16_at(data: &[u8], offset: i32) -> Result<i32> {
    let bytes = &data[span(data, offset, 2)?];
    Ok(i32::from(u16::from_le_bytes([bytes[0], bytes[1]])))
}

pub(crate) fn i32_at(data: &[u8], offset: i32) -> Result<i32> {
    let bytes = &data[span(data, offset, 4)?];
    Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub(crate) fn relative_pointer_at(data: &[u8], offset: i32) -> Result<i32> {
    Ok(i32_at(data, offset)?.wrapping_add(offset).wrapping_add(4))
}

pub(crate) fn set_u8(data: &mut [u8], offset: i32, value: i32) -> Result<()> {
    let range = span(data, offset, 1)?;
    data[range][0] = value as u8;
    Ok(())
}

pub(crate) fn set_u16(data: &mut [u8], offset: i32, value: i32) -> Result<()> {
    let range = span(data, offset, 2)?;
    data[range].copy_from_slice(&(value as u16).to_le_bytes());
    Ok(())
}

pub(crate) fn set_i32(data: &mut [u8], offset: i32, value: i32) -> Result<()> {
    let range = span(data, offset, 4)?;
    data[range].copy_from_slice(&value.to_le_bytes());
    Ok(())
}

pub(crate) fn set_bytes(data: &mut [u8], offset: i32, values: &[u8]) -> Result<()> {
    let range = span(data, offset, values.len())?;
    data[range].copy_from_slice(values);
    Ok(())
}

pub(crate) fn file(narc: &NarcArchive, index: i32) -> Result<&Vec<u8>> {
    usize::try_from(index)
        .ok()
        .and_then(|index| narc.files.get(index))
        .ok_or_else(out_of_bounds)
}

pub(crate) fn file_mut(narc: &mut NarcArchive, index: i32) -> Result<&mut Vec<u8>> {
    usize::try_from(index)
        .ok()
        .and_then(|index| narc.files.get_mut(index))
        .ok_or_else(out_of_bounds)
}

pub(crate) fn after_prefix(data: &[u8], prefix: &str) -> Option<i32> {
    let offset = find(data, prefix);
    (offset > 0).then(|| offset + prefix.len() as i32 / 2)
}

pub(crate) fn found_at(data: &[u8], prefix: &str) -> Option<i32> {
    let offset = find(data, prefix);
    (offset > 0).then_some(offset)
}

pub(crate) fn search_all(data: &[u8], needle: &[u8]) -> Vec<i32> {
    randomon_formats::search::search(data, needle)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|offset| i32::try_from(offset).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_are_little_endian_and_bounded() {
        let data = [0x34, 0x12, 0xFF, 0xFF, 0x80];
        assert_eq!(u16_at(&data, 0).unwrap(), 0x1234);
        assert_eq!(i32_at(&data, 0).unwrap(), 0xFFFF_1234_u32 as i32);
        assert_eq!(i8_at(&data, 4).unwrap(), -128);
        assert_eq!(u8_at(&data, 4).unwrap(), 0x80);
        assert!(u16_at(&data, 4).is_err());
        assert!(u8_at(&data, -1).is_err());
        assert!(i32_at(&data, 2).is_err());
    }

    #[test]
    fn writes_never_grow_or_panic() {
        let mut data = vec![0u8; 4];
        set_u16(&mut data, 2, 0xABCD).unwrap();
        assert_eq!(data, [0, 0, 0xCD, 0xAB]);
        assert!(set_i32(&mut data, 1, 7).is_err());
        assert!(set_u8(&mut data, 4, 7).is_err());
        assert!(set_bytes(&mut data, i32::MAX, &[1]).is_err());
        assert_eq!(data.len(), 4);
    }

    #[test]
    fn prefixes_follow_upstream_find_rules() {
        let data = [0x00, 0xAA, 0xBB, 0x01];
        assert_eq!(after_prefix(&data, "AABB"), Some(3));
        assert_eq!(after_prefix(&data, "00AA"), None);
        assert_eq!(found_at(&data, "BB01"), Some(2));
        assert_eq!(search_all(&[1, 2, 1, 2], &[1, 2]), vec![0, 2]);
    }
}
