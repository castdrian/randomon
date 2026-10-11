use std::io::{Read, Seek};

use super::CryptoError;
use super::aes::{Aes128, Block};
use super::keys::KeyStore;
use super::source::{Layer, Mode, Region};
use crate::ctr::container::{CIA_HEADER_SIZE, NCCH_MAGIC, cia_layout, signed_body_offset};
use crate::error::Result;
use crate::io::{non_negative_offset, read_exact_at, read_i32_be_at, read_i32_le_at};

const CONTEXT: &str = "CIA";
const TMD_HEADER_SIZE: i64 = 0xC4;
const TMD_CONTENT_INFO_SIZE: i64 = 0x24 * 64;
const CONTENT_CHUNK_SIZE: usize = 0x30;
const CONTENT_TYPE_ENCRYPTED: u16 = 0x0001;
const TICKET_TITLE_KEY: i64 = 0x7F;
const TICKET_TITLE_ID: i64 = 0x9C;
const TICKET_COMMON_KEY_INDEX: i64 = 0xB1;

pub(crate) struct EncryptedContent {
    pub start: u64,
    pub end: u64,
    pub content_index: u16,
    pub ticket_body: i64,
}

fn at(value: i64) -> Result<u64> {
    non_negative_offset(value, CONTEXT)
}

pub(crate) fn encrypted_first_content<R: Read + Seek>(
    source: &mut R,
) -> Result<Option<EncryptedContent>> {
    if read_i32_le_at(source, 0)? != CIA_HEADER_SIZE {
        return Ok(None);
    }
    let layout = cia_layout(source)?;
    if read_i32_be_at(source, at(layout.content_offset)? + 0x100)? == NCCH_MAGIC {
        return Ok(None);
    }
    let Some(tmd_body) = signed_body_offset(source, layout.tmd_offset)? else {
        return Ok(None);
    };
    let chunk = read_exact_at(
        source,
        at(tmd_body + TMD_HEADER_SIZE + TMD_CONTENT_INFO_SIZE)?,
        CONTENT_CHUNK_SIZE,
    )?;
    let content_index = u16::from_be_bytes([chunk[4], chunk[5]]);
    let content_type = u16::from_be_bytes([chunk[6], chunk[7]]);
    let mut size_bytes = [0u8; 8];
    size_bytes.copy_from_slice(&chunk[8..16]);
    let content_size = u64::from_be_bytes(size_bytes);
    if content_type & CONTENT_TYPE_ENCRYPTED == 0 {
        return Ok(None);
    }
    let Some(ticket_body) = signed_body_offset(source, layout.ticket_offset)? else {
        return Ok(None);
    };
    let start = at(layout.content_offset)?;
    Ok(Some(EncryptedContent {
        start,
        end: start.saturating_add(content_size),
        content_index,
        ticket_body,
    }))
}

fn title_key<R: Read + Seek>(source: &mut R, ticket_body: i64, keys: &KeyStore) -> Result<Block> {
    let encrypted: Block = read_exact_at(source, at(ticket_body + TICKET_TITLE_KEY)?, 16)?
        .try_into()
        .map_err(|_| CryptoError::MissingCommonKeys)?;
    let mut iv = [0u8; 16];
    iv[..8].copy_from_slice(&read_exact_at(
        source,
        at(ticket_body + TICKET_TITLE_ID)?,
        8,
    )?);
    let index = read_exact_at(source, at(ticket_body + TICKET_COMMON_KEY_INDEX)?, 1)?[0] as usize;
    if keys.is_empty() && !keys.pending_downloads().is_empty() {
        return Err(keys.missing_keys_error().into());
    }
    let common = keys
        .common_key(index)
        .ok_or(CryptoError::MissingCommonKeys)?;
    let mut key = Aes128::new(&common).decrypt_block(&encrypted);
    for (byte, iv_byte) in key.iter_mut().zip(iv) {
        *byte ^= iv_byte;
    }
    Ok(key)
}

pub(crate) fn cia_layer<R: Read + Seek>(
    source: &mut R,
    content: &EncryptedContent,
    keys: &KeyStore,
) -> Result<Layer> {
    let key = title_key(source, content.ticket_body, keys)?;
    let mut iv = [0u8; 16];
    iv[..2].copy_from_slice(&content.content_index.to_be_bytes());
    Ok(Layer {
        regions: vec![Region {
            start: content.start,
            end: content.end,
            cipher: Aes128::new(&key),
            mode: Mode::Cbc { iv },
        }],
        patches: Vec::new(),
    })
}
