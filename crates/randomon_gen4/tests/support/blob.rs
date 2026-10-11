pub fn grow(data: &mut Vec<u8>, end: usize) {
    if data.len() < end {
        data.resize(end, 0);
    }
}

pub fn put_bytes(data: &mut Vec<u8>, offset: usize, bytes: &[u8]) {
    grow(data, offset + bytes.len());
    data[offset..offset + bytes.len()].copy_from_slice(bytes);
}

pub fn put_u8(data: &mut Vec<u8>, offset: usize, value: u32) {
    put_bytes(data, offset, &[value as u8]);
}

pub fn put_u16(data: &mut Vec<u8>, offset: usize, value: u32) {
    put_bytes(data, offset, &(value as u16).to_le_bytes());
}

pub fn put_u32(data: &mut Vec<u8>, offset: usize, value: u32) {
    put_bytes(data, offset, &value.to_le_bytes());
}

pub fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex"))
        .collect()
}

pub fn words(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|&value| (value as u16).to_le_bytes())
        .collect()
}

pub fn longs(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|&value| value.to_le_bytes())
        .collect()
}

pub fn ips_ranges(patch: &[u8]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut offset = 5;
    while offset + 2 < patch.len() {
        let write =
            u32::from_be_bytes([0, patch[offset], patch[offset + 1], patch[offset + 2]]) as usize;
        if write == 0x454F46 {
            break;
        }
        offset += 3;
        let size = u16::from_be_bytes([patch[offset], patch[offset + 1]]) as usize;
        offset += 2;
        if size == 0 {
            let run = u16::from_be_bytes([patch[offset], patch[offset + 1]]) as usize;
            ranges.push((write, write + run));
            offset += 3;
        } else {
            ranges.push((write, write + size));
            offset += size;
        }
    }
    ranges
}

pub struct Allocator {
    cursor: usize,
    limit: usize,
    reserved: Vec<(usize, usize)>,
}

impl Allocator {
    pub fn new(start: usize, limit: usize, reserved: Vec<(usize, usize)>) -> Self {
        Self {
            cursor: start,
            limit,
            reserved,
        }
    }

    pub fn reserve(&mut self, start: usize, end: usize) {
        self.reserved.push((start, end));
    }

    pub fn alloc(&mut self, length: usize) -> usize {
        loop {
            let start = self.cursor.div_ceil(4) * 4;
            let end = start + length;
            match self
                .reserved
                .iter()
                .find(|&&(low, high)| start < high && low < end)
            {
                Some(&(_, high)) => self.cursor = high,
                None => {
                    assert!(end <= self.limit, "synthetic layout ran out of space");
                    self.cursor = end + 0x10;
                    self.reserved.push((start, end));
                    return start;
                }
            }
        }
    }
}
