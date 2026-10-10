use crate::error::{FormatError, Result};

const CONTEXT: &str = "BLZCoder";
const BLZ_SHIFT: u32 = 1;
const BLZ_MASK: i32 = 0x80;
const BLZ_THRESHOLD: i64 = 2;
const BLZ_N: i64 = 0x1002;
const BLZ_F: i64 = 0x12;
const RAW_MAXIM: i64 = 0x00FF_FFFF;
const BLZ_MAXIM: i64 = 0x0140_0000;
const ARM9_UNCOMPRESSED_HEAD: i64 = 0x4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlzMode {
    Normal,
    Best,
}

struct IntBuffer {
    values: Vec<i32>,
}

impl IntBuffer {
    fn padded_from(data: &[u8]) -> Self {
        let mut values: Vec<i32> = data.iter().map(|&byte| byte as i32).collect();
        values.extend([0, 0, 0]);
        IntBuffer { values }
    }

    fn zeroed(length: i64) -> Result<Self> {
        if length < 0 {
            return Err(FormatError::invalid(CONTEXT, "negative buffer length"));
        }
        Ok(IntBuffer {
            values: vec![0; length as usize],
        })
    }

    fn get(&self, index: i64) -> Result<i32> {
        if index < 0 {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        self.values
            .get(index as usize)
            .copied()
            .ok_or(FormatError::out_of_bounds(CONTEXT))
    }

    fn set(&mut self, index: i64, value: i32) -> Result<()> {
        if index < 0 {
            return Err(FormatError::out_of_bounds(CONTEXT));
        }
        let slot = self
            .values
            .get_mut(index as usize)
            .ok_or(FormatError::out_of_bounds(CONTEXT))?;
        *slot = value;
        Ok(())
    }

    fn invert(&mut self, offset: i64, length: i64) -> Result<()> {
        let mut bottom = offset + length - 1;
        let mut top = offset;
        while top < bottom {
            let low = self.get(top)?;
            let high = self.get(bottom)?;
            self.set(top, high)?;
            self.set(bottom, low)?;
            top += 1;
            bottom -= 1;
        }
        Ok(())
    }

    fn read_unsigned(&self, offset: i64) -> Result<i64> {
        Ok((self.get(offset)?
            | self.get(offset + 1)? << 8
            | self.get(offset + 2)? << 16
            | (self.get(offset + 3)? & 0x7F) << 24) as i64)
    }

    fn write_unsigned(&mut self, offset: i64, value: i32) -> Result<()> {
        self.set(offset, value & 0xFF)?;
        self.set(offset + 1, (value >> 8) & 0xFF)?;
        self.set(offset + 2, (value >> 16) & 0xFF)?;
        self.set(offset + 3, (value >> 24) & 0x7F)
    }

    fn to_bytes(&self, length: i64) -> Vec<u8> {
        self.values[..length as usize]
            .iter()
            .map(|&value| value as u8)
            .collect()
    }
}

fn exit(message: &'static str) -> FormatError {
    FormatError::UpstreamExit {
        context: CONTEXT,
        message,
    }
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
    let mut pak_buffer = IntBuffer::padded_from(data);
    let mut pak_len = data.len() as i64;
    let inc_len = pak_buffer.read_unsigned(pak_len - 4)?;
    let (dec_len, raw_len);
    if inc_len < 1 {
        dec_len = pak_len;
        pak_len = 0;
        raw_len = dec_len;
    } else {
        if pak_len < 8 {
            return Err(exit("File has a bad header"));
        }
        let hdr_len = pak_buffer.get(pak_len - 5)? as i64;
        if !(8..=0xB).contains(&hdr_len) {
            return Err(exit("Bad header length"));
        }
        if pak_len <= hdr_len {
            return Err(exit("Bad length"));
        }
        let enc_len = pak_buffer.read_unsigned(pak_len - 8)? & 0x00FF_FFFF;
        dec_len = pak_len - enc_len;
        pak_len = enc_len - hdr_len;
        raw_len = dec_len + enc_len + inc_len;
        if raw_len > RAW_MAXIM {
            return Err(exit("Bad decoded length"));
        }
    }

    let mut raw_buffer = IntBuffer::zeroed(raw_len)?;
    let mut pak: i64 = 0;
    let mut raw: i64 = 0;
    let pak_end = dec_len + pak_len;
    let raw_end = raw_len;

    for _ in 0..dec_len.max(0) {
        raw_buffer.set(raw, pak_buffer.get(pak)?)?;
        raw += 1;
        pak += 1;
    }

    pak_buffer.invert(dec_len, pak_len)?;

    let mut mask: i32 = 0;
    let mut flags: i32 = 0;
    while raw < raw_end {
        mask = ((mask as u32) >> BLZ_SHIFT) as i32;
        if mask == 0 {
            if pak == pak_end {
                break;
            }
            flags = pak_buffer.get(pak)?;
            pak += 1;
            mask = BLZ_MASK;
        }
        if flags & mask == 0 {
            if pak == pak_end {
                break;
            }
            raw_buffer.set(raw, pak_buffer.get(pak)?)?;
            raw += 1;
            pak += 1;
        } else {
            if pak + 1 >= pak_end {
                break;
            }
            let mut pos = (pak_buffer.get(pak)? << 8) as i64;
            pos |= pak_buffer.get(pak + 1)? as i64;
            pak += 2;
            let mut len = (pos >> 12) + BLZ_THRESHOLD + 1;
            if raw + len > raw_end {
                len = raw_end - raw;
            }
            pos = (pos & 0xFFF) + 3;
            while len > 0 {
                let value = raw_buffer.get(raw - pos)?;
                raw_buffer.set(raw, value)?;
                raw += 1;
                len -= 1;
            }
        }
    }

    raw_buffer.invert(dec_len, raw_len - dec_len)?;
    Ok(raw_buffer.to_bytes(raw))
}

pub fn encode(data: &[u8], arm9: bool, mode: BlzMode) -> Result<Vec<u8>> {
    let (buffer, new_len) = code(data, arm9, mode)?;
    if new_len < BLZ_MAXIM + 1 {
        Ok(buffer.to_bytes(new_len))
    } else {
        Err(FormatError::invalid(
            CONTEXT,
            "encoded result exceeds BLZ_MAXIM",
        ))
    }
}

struct MatchFinder {
    previous: Vec<u32>,
}

const NO_PREVIOUS: u32 = u32::MAX;
const HASH_BITS: u32 = 16;

impl MatchFinder {
    fn build(buffer: &[i32], raw_end: i64) -> Self {
        let count = if raw_end >= 3 {
            (raw_end - 2) as usize
        } else {
            0
        };
        let mut heads = vec![NO_PREVIOUS; 1 << HASH_BITS];
        let mut previous = vec![NO_PREVIOUS; count];
        for (index, slot) in previous.iter_mut().enumerate() {
            let key = hash_key(buffer, index);
            *slot = heads[key];
            heads[key] = index as u32;
        }
        MatchFinder { previous }
    }

    fn search(&self, best_position: i64, buffer: &[i32], raw: i64, raw_end: i64) -> (i64, i64) {
        let mut length = BLZ_THRESHOLD;
        let mut position = best_position;
        if raw + 3 > raw_end {
            return (length, position);
        }
        let max = if raw >= BLZ_N { BLZ_N } else { raw };
        let mut candidate = self.previous[raw as usize];
        while candidate != NO_PREVIOUS {
            let distance = raw - candidate as i64;
            if distance > max {
                break;
            }
            if distance >= 3 {
                let matched = match_length(buffer, raw, raw_end, distance);
                if matched > length {
                    position = distance;
                    length = matched;
                    if length == BLZ_F {
                        break;
                    }
                }
            }
            candidate = self.previous[candidate as usize];
        }
        (length, position)
    }
}

fn hash_key(buffer: &[i32], index: usize) -> usize {
    let combined =
        (buffer[index] as u32) << 16 | (buffer[index + 1] as u32) << 8 | buffer[index + 2] as u32;
    (combined.wrapping_mul(0x9E37_79B1) >> (32 - HASH_BITS)) as usize
}

fn match_length(buffer: &[i32], raw: i64, raw_end: i64, position: i64) -> i64 {
    let mut length = 0;
    while length < BLZ_F {
        if raw + length == raw_end || length >= position {
            break;
        }
        if buffer[(raw + length) as usize] != buffer[(raw + length - position) as usize] {
            break;
        }
        length += 1;
    }
    length
}

fn code(data: &[u8], arm9: bool, mode: BlzMode) -> Result<(IntBuffer, i64)> {
    let mut raw_buffer = IntBuffer::padded_from(data);
    let raw_len = data.len() as i64;
    let mut pak_tmp: i64 = 0;
    let mut raw_tmp: i64 = raw_len;
    let mut pak_buffer = IntBuffer::zeroed(raw_len + (raw_len + 7) / 8 + 11)?;
    let raw_new = if arm9 {
        raw_len - ARM9_UNCOMPRESSED_HEAD
    } else {
        raw_len
    };

    raw_buffer.invert(0, raw_len)?;

    let raw_end = raw_new;
    let finder = MatchFinder::build(&raw_buffer.values, raw_end);
    let mut pak: i64 = 0;
    let mut raw: i64 = 0;
    let mut flg: i64 = 0;
    let mut mask: i32 = 0;
    let (mut pos_best, mut pos_next, mut pos_post) = (0i64, 0i64, 0i64);

    while raw < raw_end {
        mask = ((mask as u32) >> BLZ_SHIFT) as i32;
        if mask == 0 {
            flg = pak;
            pak += 1;
            pak_buffer.set(flg, 0)?;
            mask = BLZ_MASK;
        }

        let (found_len, found_pos) = finder.search(pos_best, &raw_buffer.values, raw, raw_end);
        let mut len_best = found_len;
        pos_best = found_pos;

        if mode == BlzMode::Best && len_best > BLZ_THRESHOLD && raw + len_best < raw_end {
            raw += len_best;
            let (mut len_next, next_pos) =
                finder.search(pos_next, &raw_buffer.values, raw, raw_end);
            pos_next = next_pos;
            raw -= len_best - 1;
            let (mut len_post, post_pos) =
                finder.search(pos_post, &raw_buffer.values, raw, raw_end);
            pos_post = post_pos;
            raw -= 1;
            if len_next <= BLZ_THRESHOLD {
                len_next = 1;
            }
            if len_post <= BLZ_THRESHOLD {
                len_post = 1;
            }
            if len_best + len_next <= 1 + len_post {
                len_best = 1;
            }
        }

        pak_buffer.set(flg, pak_buffer.get(flg)? << 1)?;
        if len_best > BLZ_THRESHOLD {
            raw += len_best;
            pak_buffer.set(flg, pak_buffer.get(flg)? | 1)?;
            let encoded_len = ((len_best - (BLZ_THRESHOLD + 1)) << 4) as i32;
            let encoded_pos = (pos_best - 3) as i32;
            pak_buffer.set(pak, encoded_len | ((encoded_pos as u32) >> 8) as i32)?;
            pak_buffer.set(pak + 1, encoded_pos & 0xFF)?;
            pak += 2;
        } else {
            pak_buffer.set(pak, raw_buffer.get(raw)?)?;
            pak += 1;
            raw += 1;
        }

        if pak + raw_len - raw < pak_tmp + raw_tmp {
            pak_tmp = pak;
            raw_tmp = raw_len - raw;
        }
    }

    while mask > 0 && mask != 1 {
        mask = ((mask as u32) >> BLZ_SHIFT) as i32;
        pak_buffer.set(flg, pak_buffer.get(flg)? << 1)?;
    }

    let pak_len = pak;
    raw_buffer.invert(0, raw_len)?;
    pak_buffer.invert(0, pak_len)?;

    let would_store_uncompressed = pak_tmp == 0 || raw_len + 4 < ((pak_tmp + raw_tmp + 3) & !3) + 8;
    if would_store_uncompressed {
        if raw_len > 0 {
            return Err(FormatError::UpstreamHang { context: CONTEXT });
        }
        return Ok((IntBuffer::zeroed(4)?, 4));
    }

    let mut output = IntBuffer::zeroed(raw_tmp + pak_tmp + 11)?;
    for index in 0..raw_tmp {
        output.set(index, raw_buffer.get(index)?)?;
    }
    for index in 0..pak_tmp {
        output.set(raw_tmp + index, pak_buffer.get(index + pak_len - pak_tmp)?)?;
    }
    let mut pak = raw_tmp + pak_tmp;
    let enc_len = pak_tmp;
    let mut hdr_len: i64 = 8;
    let inc_len = raw_len - pak_tmp - raw_tmp;
    while pak & 3 > 0 {
        output.set(pak, 0xFF)?;
        pak += 1;
        hdr_len += 1;
    }
    output.write_unsigned(pak, (enc_len + hdr_len) as i32)?;
    pak += 3;
    output.set(pak, hdr_len as i32)?;
    pak += 1;
    output.write_unsigned(pak, (inc_len - hdr_len) as i32)?;
    pak += 4;
    Ok((output, pak))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal_search(best_position: i64, buffer: &[i32], raw: i64, raw_end: i64) -> (i64, i64) {
        let mut length = BLZ_THRESHOLD;
        let mut position = best_position;
        let max = if raw >= BLZ_N { BLZ_N } else { raw };
        let mut candidate = 3;
        while candidate <= max {
            let matched = match_length(buffer, raw, raw_end, candidate);
            if matched > length {
                position = candidate;
                length = matched;
                if length == BLZ_F {
                    break;
                }
            }
            candidate += 1;
        }
        (length, position)
    }

    fn pseudo_random_bytes(seed: u64, length: usize, alphabet: u8) -> Vec<u8> {
        let mut state = seed;
        (0..length)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((state >> 33) % alphabet as u64) as u8
            })
            .collect()
    }

    #[test]
    fn hash_chain_search_matches_literal_search() {
        for (seed, alphabet) in [(1u64, 2u8), (2, 3), (3, 4), (4, 16), (5, 255)] {
            let data = pseudo_random_bytes(seed, 9000, alphabet);
            let buffer = IntBuffer::padded_from(&data);
            for raw_end in [data.len() as i64, data.len() as i64 - 0x400] {
                let finder = MatchFinder::build(&buffer.values, raw_end);
                for raw in (0..raw_end).step_by(7) {
                    let expected = literal_search(41, &buffer.values, raw, raw_end);
                    let actual = finder.search(41, &buffer.values, raw, raw_end);
                    assert_eq!(actual, expected, "seed {seed} raw {raw} raw_end {raw_end}");
                }
            }
        }
    }

    #[test]
    fn round_trips_compressible_data() {
        for mode in [BlzMode::Normal, BlzMode::Best] {
            let data = pseudo_random_bytes(9, 20000, 4);
            let encoded = encode(&data, false, mode).unwrap();
            assert!(encoded.len() < data.len());
            assert_eq!(decode(&encoded).unwrap(), data);
        }
    }

    #[test]
    fn incompressible_data_would_hang_upstream() {
        let data = pseudo_random_bytes(11, 4096, 255);
        assert!(matches!(
            encode(&data, false, BlzMode::Normal),
            Err(FormatError::UpstreamHang { .. })
        ));
    }

    #[test]
    fn empty_input_encodes_to_four_zero_bytes() {
        assert_eq!(
            encode(&[], false, BlzMode::Normal).unwrap(),
            vec![0, 0, 0, 0]
        );
    }
}
