use std::io::{Read, Seek};

use super::CryptoError;
use super::aes::{Aes128, Block};
use super::keys::{KeyStore, NCCH_PRIMARY_SLOT};
use super::source::{Layer, Mode, Patch, Region};
use crate::error::Result;
use crate::io::read_exact_at;

const MEDIA_UNIT: u64 = 0x200;
const HEADER_SIZE: usize = 0x200;
const EXHEADER_OFFSET: u64 = 0x200;
const EXHEADER_CRYPTO_SIZE: u64 = 0x800;
const EXEFS_HEADER_SIZE: u64 = 0x200;
const EXEFS_SLOTS: usize = 10;
const FLAGS_OFFSET: usize = 0x188;
const CRYPTO_METHOD_FLAG: usize = FLAGS_OFFSET + 3;
const CRYPTO_OPTIONS_FLAG: usize = FLAGS_OFFSET + 7;
pub const FIXED_KEY: u8 = 0x01;
pub const NO_CRYPTO: u8 = 0x04;
pub const SEED_CRYPTO: u8 = 0x20;
const ROMFS_MAGIC: &[u8; 4] = b"IVFC";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    ExHeader = 1,
    ExeFs = 2,
    RomFs = 3,
}

fn read_u32_le(header: &[u8], offset: usize) -> u64 {
    u32::from_le_bytes([
        header[offset],
        header[offset + 1],
        header[offset + 2],
        header[offset + 3],
    ]) as u64
}

pub(crate) struct Geometry {
    ncch: u64,
    exheader_size: u64,
    exefs: (u64, u64),
    romfs: (u64, u64),
}

impl Geometry {
    fn parse(ncch: u64, header: &[u8]) -> Self {
        let span = |offset_field: usize| {
            let offset = read_u32_le(header, offset_field) * MEDIA_UNIT;
            let size = read_u32_le(header, offset_field + 4) * MEDIA_UNIT;
            (ncch + offset, size)
        };
        Geometry {
            ncch,
            exheader_size: read_u32_le(header, 0x180),
            exefs: span(0x1A0),
            romfs: span(0x1B0),
        }
    }

    fn has_exefs(&self) -> bool {
        self.exefs.1 >= EXEFS_HEADER_SIZE
    }

    fn has_romfs(&self) -> bool {
        self.romfs.1 >= ROMFS_MAGIC.len() as u64
    }
}

fn is_plausible_name(name: &[u8]) -> bool {
    let length = name
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(name.len());
    name[..length].iter().all(|byte| byte.is_ascii_graphic())
        && name[length..].iter().all(|&byte| byte == 0)
}

fn is_plausible_exefs_header(header: &[u8]) -> bool {
    (0..EXEFS_SLOTS).all(|slot| is_plausible_name(&header[slot * 16..slot * 16 + 8]))
}

pub(crate) fn has_plaintext_evidence<R: Read + Seek>(
    source: &mut R,
    geometry: &Geometry,
) -> Result<bool> {
    if geometry.has_exefs() {
        let header = read_exact_at(source, geometry.exefs.0, EXEFS_HEADER_SIZE as usize)?;
        if is_plausible_exefs_header(&header) {
            return Ok(true);
        }
    }
    if geometry.has_romfs() {
        let magic = read_exact_at(source, geometry.romfs.0, ROMFS_MAGIC.len())?;
        if magic == ROMFS_MAGIC {
            return Ok(true);
        }
    }
    Ok(!geometry.has_exefs() && !geometry.has_romfs())
}

pub(crate) fn looks_decrypted<R: Read + Seek>(source: &mut R, geometry: &Geometry) -> Result<bool> {
    if geometry.has_exefs() {
        let header = read_exact_at(source, geometry.exefs.0, EXEFS_HEADER_SIZE as usize)?;
        if !is_plausible_exefs_header(&header) {
            return Ok(false);
        }
    }
    if geometry.has_romfs() {
        let magic = read_exact_at(source, geometry.romfs.0, ROMFS_MAGIC.len())?;
        if magic != ROMFS_MAGIC {
            return Ok(false);
        }
    }
    Ok(true)
}

fn section_counter(header: &[u8], geometry: &Geometry, section: Section) -> Result<u128> {
    let partition_id: [u8; 8] = header[0x108..0x110]
        .try_into()
        .map_err(|_| CryptoError::UnsupportedNcchVersion(0))?;
    let version = u16::from_le_bytes([header[0x112], header[0x113]]);
    match version {
        0 | 2 => Ok((u64::from_le_bytes(partition_id) as u128) << 64 | (section as u128) << 56),
        1 => {
            let offset = match section {
                Section::ExHeader => EXHEADER_OFFSET,
                Section::ExeFs => geometry.exefs.0 - geometry.ncch,
                Section::RomFs => geometry.romfs.0 - geometry.ncch,
            };
            Ok((u64::from_be_bytes(partition_id) as u128) << 64 | (offset as u32) as u128)
        }
        other => Err(CryptoError::UnsupportedNcchVersion(other).into()),
    }
}

pub fn secondary_slot(crypto_method: u8) -> Option<u8> {
    match crypto_method {
        0x00 => Some(NCCH_PRIMARY_SLOT),
        0x01 => Some(0x25),
        0x0A => Some(0x18),
        0x0B => Some(0x1B),
        _ => None,
    }
}

struct NcchKeys {
    primary: Aes128,
    secondary: Aes128,
}

fn ncch_keys(header: &[u8], keys: &KeyStore) -> Result<NcchKeys> {
    let flags = header[CRYPTO_OPTIONS_FLAG];
    if flags & SEED_CRYPTO != 0 {
        return Err(CryptoError::SeedCrypto.into());
    }
    if flags & FIXED_KEY != 0 {
        let zero = Aes128::new(&[0u8; 16]);
        return Ok(NcchKeys {
            primary: zero.clone(),
            secondary: zero,
        });
    }
    let method = header[CRYPTO_METHOD_FLAG];
    let slot = secondary_slot(method).ok_or(CryptoError::UnknownCryptoMethod(method))?;
    let key_y: Block = header[..16]
        .try_into()
        .map_err(|_| CryptoError::UnknownCryptoMethod(method))?;
    if keys.is_empty() {
        return Err(CryptoError::MissingKeys.into());
    }
    let primary = keys
        .normal_key(NCCH_PRIMARY_SLOT, &key_y)
        .ok_or(CryptoError::MissingKeySlot(NCCH_PRIMARY_SLOT))?;
    let secondary = keys
        .normal_key(slot, &key_y)
        .ok_or(CryptoError::MissingKeySlot(slot))?;
    Ok(NcchKeys {
        primary: Aes128::new(&primary),
        secondary: Aes128::new(&secondary),
    })
}

fn ctr_region(start: u64, end: u64, cipher: &Aes128, origin: u64, counter: u128) -> Region {
    Region {
        start,
        end,
        cipher: cipher.clone(),
        mode: Mode::Ctr { origin, counter },
    }
}

fn exefs_file_regions(
    plain_header: &[u8],
    geometry: &Geometry,
    keys: &NcchKeys,
    counter: u128,
) -> Vec<Region> {
    let (exefs_start, exefs_size) = geometry.exefs;
    let exefs_end = exefs_start + exefs_size;
    let data_start = exefs_start + EXEFS_HEADER_SIZE;
    (0..EXEFS_SLOTS)
        .filter_map(|slot| {
            let entry = &plain_header[slot * 16..slot * 16 + 16];
            let name_length = entry[..8].iter().position(|&byte| byte == 0).unwrap_or(8);
            let name = &entry[..name_length];
            let offset = read_u32_le(entry, 8);
            let size = read_u32_le(entry, 12);
            if name.is_empty() || size == 0 {
                return None;
            }
            let start = data_start.checked_add(offset)?;
            let end = start.checked_add(size)?.min(exefs_end);
            let cipher = if name == b"icon" || name == b"banner" {
                &keys.primary
            } else {
                &keys.secondary
            };
            (start < end).then(|| ctr_region(start, end, cipher, exefs_start, counter))
        })
        .collect()
}

pub(crate) fn read_geometry<R: Read + Seek>(
    source: &mut R,
    ncch: u64,
) -> Result<(Vec<u8>, Geometry)> {
    let header = read_exact_at(source, ncch, HEADER_SIZE)?;
    let geometry = Geometry::parse(ncch, &header);
    Ok((header, geometry))
}

pub(crate) fn is_marked_plaintext(header: &[u8]) -> bool {
    header[CRYPTO_OPTIONS_FLAG] & NO_CRYPTO != 0
}

pub(crate) fn ncch_layer<R: Read + Seek>(
    source: &mut R,
    header: &[u8],
    geometry: &Geometry,
    keys: &KeyStore,
) -> Result<Layer> {
    let keys = ncch_keys(header, keys)?;
    let ncch = geometry.ncch;
    let mut regions = Vec::new();
    if geometry.exheader_size > 0 {
        let start = ncch + EXHEADER_OFFSET;
        let counter = section_counter(header, geometry, Section::ExHeader)?;
        regions.push(ctr_region(
            start,
            start + EXHEADER_CRYPTO_SIZE,
            &keys.primary,
            start,
            counter,
        ));
    }
    if geometry.has_exefs() {
        let (start, _) = geometry.exefs;
        let counter = section_counter(header, geometry, Section::ExeFs)?;
        let header_region = ctr_region(
            start,
            start + EXEFS_HEADER_SIZE,
            &keys.primary,
            start,
            counter,
        );
        let mut plain_header = read_exact_at(source, start, EXEFS_HEADER_SIZE as usize)?;
        Layer {
            regions: vec![header_region.clone()],
            patches: Vec::new(),
        }
        .apply_to(start, &mut plain_header);
        regions.push(header_region);
        regions.extend(exefs_file_regions(&plain_header, geometry, &keys, counter));
    }
    if geometry.romfs.1 > 0 {
        let (start, size) = geometry.romfs;
        let counter = section_counter(header, geometry, Section::RomFs)?;
        regions.push(ctr_region(
            start,
            start + size,
            &keys.secondary,
            start,
            counter,
        ));
    }
    let options = header[CRYPTO_OPTIONS_FLAG];
    let patches = vec![
        Patch {
            offset: ncch + CRYPTO_METHOD_FLAG as u64,
            bytes: vec![0],
        },
        Patch {
            offset: ncch + CRYPTO_OPTIONS_FLAG as u64,
            bytes: vec![(options & !(FIXED_KEY | SEED_CRYPTO)) | NO_CRYPTO],
        },
    ];
    Ok(Layer { regions, patches })
}
