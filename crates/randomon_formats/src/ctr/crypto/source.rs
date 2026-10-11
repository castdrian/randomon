use std::io::{self, Read, Seek, SeekFrom};

use super::aes::{Aes128, BLOCK_SIZE, Block};

const BLOCK: u64 = BLOCK_SIZE as u64;

#[derive(Clone, Debug)]
pub enum Mode {
    Ctr { origin: u64, counter: u128 },
    Cbc { iv: Block },
}

#[derive(Clone, Debug)]
pub struct Region {
    pub start: u64,
    pub end: u64,
    pub cipher: Aes128,
    pub mode: Mode,
}

#[derive(Clone, Debug)]
pub struct Patch {
    pub offset: u64,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct Layer {
    pub regions: Vec<Region>,
    pub patches: Vec<Patch>,
}

fn align_down(value: u64, origin: u64) -> u64 {
    origin + (value - origin) / BLOCK * BLOCK
}

fn align_up(value: u64, origin: u64) -> u64 {
    origin + (value - origin).div_ceil(BLOCK) * BLOCK
}

fn xor_into(target: &mut [u8], mask: &[u8]) {
    for (byte, mask_byte) in target.iter_mut().zip(mask) {
        *byte ^= mask_byte;
    }
}

impl Region {
    fn overlap(&self, start: u64, end: u64) -> Option<(u64, u64)> {
        let low = self.start.max(start);
        let high = self.end.min(end);
        (low < high).then_some((low, high))
    }

    fn required_span(&self, start: u64, end: u64) -> (u64, u64) {
        let Mode::Cbc { .. } = self.mode else {
            return (start, end);
        };
        let Some((low, high)) = self.overlap(start, end) else {
            return (start, end);
        };
        let first_block = align_down(low, self.start);
        let with_previous = first_block.saturating_sub(BLOCK).max(self.start);
        let last = align_up(high, self.start).min(self.end);
        (start.min(with_previous), end.max(last))
    }

    fn apply(&self, base: u64, data: &mut [u8]) {
        let Some((low, high)) = self.overlap(base, base + data.len() as u64) else {
            return;
        };
        match &self.mode {
            Mode::Ctr { origin, counter } => {
                self.apply_ctr(*origin, *counter, base, low, high, data)
            }
            Mode::Cbc { iv } => self.apply_cbc(iv, base, low, high, data),
        }
    }

    fn apply_ctr(
        &self,
        origin: u64,
        counter: u128,
        base: u64,
        low: u64,
        high: u64,
        data: &mut [u8],
    ) {
        let mut position = low;
        while position < high {
            let block_index = (position - origin) / BLOCK;
            let within = ((position - origin) % BLOCK) as usize;
            let keystream = self
                .cipher
                .encrypt_block(&counter.wrapping_add(block_index as u128).to_be_bytes());
            let length = ((BLOCK_SIZE - within) as u64).min(high - position) as usize;
            let offset = (position - base) as usize;
            xor_into(
                &mut data[offset..offset + length],
                &keystream[within..within + length],
            );
            position += length as u64;
        }
    }

    fn apply_cbc(&self, iv: &Block, base: u64, low: u64, high: u64, data: &mut [u8]) {
        let data_end = base + data.len() as u64;
        let first = align_down(low, self.start).max(align_up(base, self.start));
        let mut block_start = align_down(high - 1, self.start);
        while block_start >= first {
            let block_end = block_start + BLOCK;
            if block_end <= data_end && block_end <= self.end {
                let previous = if block_start == self.start {
                    Some(*iv)
                } else if block_start - BLOCK >= base {
                    let offset = (block_start - BLOCK - base) as usize;
                    Block::try_from(&data[offset..offset + BLOCK_SIZE]).ok()
                } else {
                    None
                };
                if let Some(previous) = previous {
                    let offset = (block_start - base) as usize;
                    let ciphertext = Block::try_from(&data[offset..offset + BLOCK_SIZE])
                        .unwrap_or([0; BLOCK_SIZE]);
                    let mut plaintext = self.cipher.decrypt_block(&ciphertext);
                    xor_into(&mut plaintext, &previous);
                    data[offset..offset + BLOCK_SIZE].copy_from_slice(&plaintext);
                }
            }
            if block_start < BLOCK || block_start - BLOCK < first {
                break;
            }
            block_start -= BLOCK;
        }
    }
}

impl Layer {
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty() && self.patches.is_empty()
    }

    fn required_span(&self, start: u64, end: u64) -> (u64, u64) {
        self.regions
            .iter()
            .fold((start, end), |(low, high), region| {
                let (region_low, region_high) = region.required_span(start, end);
                (low.min(region_low), high.max(region_high))
            })
    }

    pub(crate) fn apply_to(&self, base: u64, data: &mut [u8]) {
        for region in &self.regions {
            region.apply(base, data);
        }
        let end = base + data.len() as u64;
        for patch in &self.patches {
            let patch_end = patch.offset + patch.bytes.len() as u64;
            let low = patch.offset.max(base);
            let high = patch_end.min(end);
            if low < high {
                let target = (low - base) as usize..(high - base) as usize;
                let source = (low - patch.offset) as usize..(high - patch.offset) as usize;
                data[target].copy_from_slice(&patch.bytes[source]);
            }
        }
    }
}

pub struct CryptoSource<R> {
    inner: R,
    layers: Vec<Layer>,
    position: u64,
}

impl<R> CryptoSource<R> {
    pub fn new(inner: R) -> Self {
        CryptoSource {
            inner,
            layers: Vec::new(),
            position: 0,
        }
    }

    pub fn is_decrypting(&self) -> bool {
        !self.layers.is_empty()
    }

    pub fn push_layer(&mut self, layer: Layer) {
        if !layer.is_empty() {
            self.layers.push(layer);
        }
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

fn fill<R: Read>(reader: &mut R, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
}

impl<R: Read + Seek> CryptoSource<R> {
    fn read_at(&mut self, level: usize, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if level == 0 {
            self.inner.seek(SeekFrom::Start(offset))?;
            return fill(&mut self.inner, buffer);
        }
        let end = offset + buffer.len() as u64;
        let (span_start, span_end) = self.layers[level - 1].required_span(offset, end);
        if span_start == offset && span_end == end {
            let read = self.read_at(level - 1, offset, buffer)?;
            self.layers[level - 1].apply_to(offset, &mut buffer[..read]);
            return Ok(read);
        }
        let mut span = vec![0u8; (span_end - span_start) as usize];
        let read = self.read_at(level - 1, span_start, &mut span)?;
        span.truncate(read);
        self.layers[level - 1].apply_to(span_start, &mut span);
        let skip = (offset - span_start) as usize;
        let available = span.len().saturating_sub(skip).min(buffer.len());
        buffer[..available].copy_from_slice(&span[skip..skip + available]);
        Ok(available)
    }
}

impl<R: Read + Seek> Read for CryptoSource<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.read_at(self.layers.len(), self.position, buffer)?;
        self.position += read as u64;
        Ok(read)
    }
}

impl<R: Read + Seek> Seek for CryptoSource<R> {
    fn seek(&mut self, target: SeekFrom) -> io::Result<u64> {
        let position = match target {
            SeekFrom::Start(position) => Some(position),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
            SeekFrom::End(delta) => {
                let length = self.inner.seek(SeekFrom::End(0))?;
                length.checked_add_signed(delta)
            }
        };
        self.position = position.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "seek before start of ROM")
        })?;
        Ok(self.position)
    }
}
