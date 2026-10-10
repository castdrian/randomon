use crate::error::{FormatError, Result};

pub fn checked_range(len: usize, offset: i64, count: i64) -> Option<std::ops::Range<usize>> {
    if offset < 0 || count < 0 {
        return None;
    }
    let end = offset.checked_add(count)?;
    if end as u64 > len as u64 {
        return None;
    }
    Some(offset as usize..end as usize)
}

pub fn slice_at<'a>(
    data: &'a [u8],
    offset: i64,
    count: i64,
    context: &'static str,
) -> Result<&'a [u8]> {
    checked_range(data.len(), offset, count)
        .map(|range| &data[range])
        .ok_or(FormatError::out_of_bounds(context))
}

pub fn slice_at_mut<'a>(
    data: &'a mut [u8],
    offset: i64,
    count: i64,
    context: &'static str,
) -> Result<&'a mut [u8]> {
    match checked_range(data.len(), offset, count) {
        Some(range) => Ok(&mut data[range]),
        None => Err(FormatError::out_of_bounds(context)),
    }
}

pub fn byte_at(data: &[u8], offset: i64, context: &'static str) -> Result<u8> {
    slice_at(data, offset, 1, context).map(|bytes| bytes[0])
}

pub fn copy_of_range(
    original: &[u8],
    from: i64,
    to: i64,
    context: &'static str,
) -> Result<Vec<u8>> {
    if from > to {
        return Err(FormatError::invalid(
            context,
            format!("copyOfRange {from} > {to}"),
        ));
    }
    if from < 0 || from > original.len() as i64 {
        return Err(FormatError::out_of_bounds(context));
    }
    let new_length = (to - from) as usize;
    let available = (original.len() - from as usize).min(new_length);
    let mut copy = vec![0u8; new_length];
    copy[..available].copy_from_slice(&original[from as usize..from as usize + available]);
    Ok(copy)
}

pub fn array_copy(
    source: &[u8],
    source_offset: i64,
    destination: &mut [u8],
    destination_offset: i64,
    length: i64,
    context: &'static str,
) -> Result<()> {
    let from = slice_at(source, source_offset, length, context)?;
    let to = slice_at_mut(destination, destination_offset, length, context)?;
    to.copy_from_slice(from);
    Ok(())
}

pub fn new_array(length: i64, context: &'static str) -> Result<Vec<u8>> {
    if length < 0 {
        return Err(FormatError::invalid(
            context,
            format!("negative array size {length}"),
        ));
    }
    Ok(vec![0u8; length as usize])
}
