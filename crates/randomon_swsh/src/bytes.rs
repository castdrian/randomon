use crate::error::{Result, SwshError};

pub(crate) fn bytes_at<'a>(
    data: &'a [u8],
    offset: usize,
    length: usize,
    context: &'static str,
) -> Result<&'a [u8]> {
    offset
        .checked_add(length)
        .and_then(|end| data.get(offset..end))
        .ok_or(SwshError::UnexpectedEof {
            context,
            offset,
            needed: length,
        })
}

pub(crate) fn bytes_at_mut<'a>(
    data: &'a mut [u8],
    offset: usize,
    length: usize,
    context: &'static str,
) -> Result<&'a mut [u8]> {
    offset
        .checked_add(length)
        .and_then(|end| data.get_mut(offset..end))
        .ok_or(SwshError::UnexpectedEof {
            context,
            offset,
            needed: length,
        })
}

pub(crate) fn array_at<const N: usize>(
    data: &[u8],
    offset: usize,
    context: &'static str,
) -> Result<[u8; N]> {
    let mut array = [0u8; N];
    array.copy_from_slice(bytes_at(data, offset, N, context)?);
    Ok(array)
}

pub(crate) fn u8_at(data: &[u8], offset: usize, context: &'static str) -> Result<u8> {
    Ok(array_at::<1>(data, offset, context)?[0])
}

pub(crate) fn u16_at(data: &[u8], offset: usize, context: &'static str) -> Result<u16> {
    Ok(u16::from_le_bytes(array_at(data, offset, context)?))
}

pub(crate) fn u32_at(data: &[u8], offset: usize, context: &'static str) -> Result<u32> {
    Ok(u32::from_le_bytes(array_at(data, offset, context)?))
}

pub(crate) fn i32_at(data: &[u8], offset: usize, context: &'static str) -> Result<i32> {
    Ok(i32::from_le_bytes(array_at(data, offset, context)?))
}

pub(crate) fn u64_at(data: &[u8], offset: usize, context: &'static str) -> Result<u64> {
    Ok(u64::from_le_bytes(array_at(data, offset, context)?))
}

pub(crate) fn i64_at(data: &[u8], offset: usize, context: &'static str) -> Result<i64> {
    Ok(i64::from_le_bytes(array_at(data, offset, context)?))
}

pub(crate) fn put_bytes(
    data: &mut [u8],
    offset: usize,
    bytes: &[u8],
    context: &'static str,
) -> Result<()> {
    bytes_at_mut(data, offset, bytes.len(), context)?.copy_from_slice(bytes);
    Ok(())
}

pub(crate) fn put_u8(
    data: &mut [u8],
    offset: usize,
    value: u8,
    context: &'static str,
) -> Result<()> {
    put_bytes(data, offset, &[value], context)
}

pub(crate) fn put_u16(
    data: &mut [u8],
    offset: usize,
    value: u16,
    context: &'static str,
) -> Result<()> {
    put_bytes(data, offset, &value.to_le_bytes(), context)
}

pub(crate) fn put_u32(
    data: &mut [u8],
    offset: usize,
    value: u32,
    context: &'static str,
) -> Result<()> {
    put_bytes(data, offset, &value.to_le_bytes(), context)
}

pub(crate) fn put_i32(
    data: &mut [u8],
    offset: usize,
    value: i32,
    context: &'static str,
) -> Result<()> {
    put_bytes(data, offset, &value.to_le_bytes(), context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_values() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        assert_eq!(u16_at(&data, 0, "test"), Ok(0x0201));
        assert_eq!(u32_at(&data, 4, "test"), Ok(0x0807_0605));
        assert_eq!(u64_at(&data, 0, "test"), Ok(0x0807_0605_0403_0201));
    }

    #[test]
    fn rejects_out_of_bounds_reads_and_writes() {
        let mut data = [0u8; 3];
        assert!(u32_at(&data, 0, "test").is_err());
        assert!(u16_at(&data, usize::MAX, "test").is_err());
        assert!(put_u32(&mut data, 1, 5, "test").is_err());
        assert_eq!(put_u16(&mut data, 1, 0xBEEF, "test"), Ok(()));
        assert_eq!(data, [0, 0xEF, 0xBE]);
    }
}
