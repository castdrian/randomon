use randomon_data::constants::gb;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Region {
    pub offset: i32,
    pub size: i32,
}

impl Region {
    pub fn new(offset: i32, size: i32) -> Self {
        Self {
            offset,
            size: size.max(0),
        }
    }

    pub fn end(self) -> i32 {
        self.offset + self.size
    }
}

pub fn merge_regions(mut regions: Vec<Region>) -> Vec<Region> {
    regions.retain(|region| region.size > 0);
    regions.sort();
    let mut merged: Vec<Region> = Vec::with_capacity(regions.len());
    for region in regions {
        match merged.last_mut() {
            Some(last) if region.offset <= last.end() => {
                let end = last.end().max(region.end());
                last.size = end - last.offset;
            }
            _ => merged.push(region),
        }
    }
    merged
}

pub fn bank_end(offset: i32) -> i32 {
    (offset / gb::BANK_SIZE + 1) * gb::BANK_SIZE
}

pub fn uniform_tail(rom: &[u8], start: i32) -> i32 {
    let end = bank_end(start).min(rom.len() as i32);
    if start >= end || start < 0 {
        return 0;
    }
    let tail = &rom[start as usize..end as usize];
    let filler = tail[0];
    if (filler == 0x00 || filler == 0xFF) && tail.iter().all(|&byte| byte == filler) {
        end - start
    } else {
        0
    }
}

struct Fill {
    region: Region,
    bytes: Vec<u8>,
}

impl Fill {
    fn free(&self) -> i32 {
        self.region.size - self.bytes.len() as i32
    }

    fn ends_with_zero(&self) -> bool {
        self.bytes.last() == Some(&0)
    }
}

pub struct BankPacker {
    fills: Vec<Fill>,
    filler: u8,
    placed: Vec<(Vec<u8>, i32)>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Placement<'a> {
    pub preference: &'a [usize],
    pub share_identical: bool,
    pub share_leading_zero: bool,
}

impl BankPacker {
    pub fn new(regions: &[Region], filler: u8) -> Self {
        Self {
            fills: regions
                .iter()
                .map(|&region| Fill {
                    region,
                    bytes: Vec::with_capacity(region.size as usize),
                })
                .collect(),
            filler,
            placed: Vec::new(),
        }
    }

    pub fn free_space(&self) -> i32 {
        self.fills.iter().map(Fill::free).sum()
    }

    fn order(&self, preference: &[usize]) -> Vec<usize> {
        preference
            .iter()
            .copied()
            .filter(|&index| index < self.fills.len())
            .collect()
    }

    pub fn place(&mut self, data: &[u8], placement: Placement<'_>) -> Option<i32> {
        if data.is_empty() {
            return None;
        }
        if placement.share_identical
            && let Some((_, offset)) = self.placed.iter().find(|(bytes, _)| bytes == data)
        {
            return Some(*offset);
        }
        for index in self.order(placement.preference) {
            let fill = &mut self.fills[index];
            let shares_zero = placement.share_leading_zero && data[0] == 0 && fill.ends_with_zero();
            let needed = data.len() as i32 - i32::from(shares_zero);
            if needed > fill.free() {
                continue;
            }
            let offset = if shares_zero {
                let offset = fill.region.offset + fill.bytes.len() as i32 - 1;
                fill.bytes.extend_from_slice(&data[1..]);
                offset
            } else {
                let offset = fill.region.offset + fill.bytes.len() as i32;
                fill.bytes.extend_from_slice(data);
                offset
            };
            if placement.share_identical {
                self.placed.push((data.to_vec(), offset));
            }
            return Some(offset);
        }
        None
    }

    pub fn write_into(&self, rom: &mut [u8]) {
        for fill in &self.fills {
            let start = fill.region.offset as usize;
            let end = fill.region.end() as usize;
            if end > rom.len() {
                continue;
            }
            rom[start..start + fill.bytes.len()].copy_from_slice(&fill.bytes);
            rom[start + fill.bytes.len()..end].fill(self.filler);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_touching_regions() {
        let merged = merge_regions(vec![
            Region::new(10, 5),
            Region::new(0, 10),
            Region::new(30, 0),
        ]);
        assert_eq!(merged, vec![Region::new(0, 15)]);
    }

    #[test]
    fn places_by_preference_and_spills_over() {
        let mut packer = BankPacker::new(&[Region::new(0, 4), Region::new(100, 8)], 0);
        let first = Placement {
            preference: &[1, 0],
            ..Placement::default()
        };
        assert_eq!(packer.place(&[1, 2, 3], first), Some(100));
        assert_eq!(packer.place(&[4, 5, 6, 7, 8, 9], first), None);
        assert_eq!(packer.place(&[4, 5, 6, 7], first), Some(103));
        assert_eq!(packer.place(&[1, 2, 3], first), Some(0));
        assert_eq!(packer.free_space(), 2);
    }

    #[test]
    fn shares_identical_blobs_and_leading_zeros() {
        let mut packer = BankPacker::new(&[Region::new(0, 6)], 0xFF);
        let sharing = Placement {
            preference: &[0],
            share_identical: true,
            share_leading_zero: true,
            ..Placement::default()
        };
        assert_eq!(packer.place(&[1, 0], sharing), Some(0));
        assert_eq!(packer.place(&[0, 2, 0], sharing), Some(1));
        assert_eq!(packer.place(&[1, 0], sharing), Some(0));
        let mut rom = vec![9; 8];
        packer.write_into(&mut rom);
        assert_eq!(rom, vec![1, 0, 2, 0, 0xFF, 0xFF, 9, 9]);
    }

    #[test]
    fn measures_uniform_bank_tails() {
        let mut rom = vec![7u8; gb::BANK_SIZE as usize];
        rom[0x3F00..].fill(0xFF);
        assert_eq!(uniform_tail(&rom, 0x3F00), 0x100);
        assert_eq!(uniform_tail(&rom, 0x3EFF), 0);
    }
}
