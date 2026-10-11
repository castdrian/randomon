use crate::compression::{Compression, compress, decompress};
use crate::error::{Result, UnityError};
use crate::stream::{ByteReader, ByteWriter, Endian};

pub const UNITY_FS_SIGNATURE: &str = "UnityFS";
pub const NODE_FLAG_SERIALIZED_FILE: u32 = 4;
const FLAG_BLOCKS_AND_DIRECTORY_COMBINED: u32 = 0x40;
const FLAG_BLOCKS_INFO_AT_END: u32 = 0x80;
const FLAG_BLOCK_INFO_NEEDS_PADDING: u32 = 0x200;
const DEFAULT_BLOCK_SIZE: usize = 0x20000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageBlock {
    pub uncompressed_size: u32,
    pub compressed_size: u32,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleNode {
    pub path: String,
    pub flags: u32,
    pub data: Vec<u8>,
}

impl BundleNode {
    pub fn is_serialized_file(&self) -> bool {
        self.flags & NODE_FLAG_SERIALIZED_FILE != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub format_version: u32,
    pub player_version: String,
    pub engine_version: String,
    pub header_flags: u32,
    pub blocks: Vec<StorageBlock>,
    pub nodes: Vec<BundleNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleWriteOptions {
    pub data_compression: Compression,
    pub block_info_compression: Compression,
    pub block_size: usize,
}

impl Default for BundleWriteOptions {
    fn default() -> Self {
        Self {
            data_compression: Compression::Lz4,
            block_info_compression: Compression::Lz4,
            block_size: DEFAULT_BLOCK_SIZE,
        }
    }
}

struct RawHeader {
    format_version: u32,
    player_version: String,
    engine_version: String,
    compressed_info_size: u32,
    uncompressed_info_size: u32,
    flags: u32,
}

pub fn is_unity_fs(data: &[u8]) -> bool {
    data.starts_with(b"UnityFS\0")
}

fn read_raw_header(reader: &mut ByteReader<'_>) -> Result<RawHeader> {
    let signature = reader.cstring()?;
    if signature != UNITY_FS_SIGNATURE {
        return Err(UnityError::UnsupportedSignature(signature));
    }
    let format_version = reader.u32()?;
    if !(6..=8).contains(&format_version) {
        return Err(UnityError::UnsupportedBundleVersion(format_version));
    }
    let player_version = reader.cstring()?;
    let engine_version = reader.cstring()?;
    let _bundle_size = reader.i64()?;
    let compressed_info_size = reader.u32()?;
    let uncompressed_info_size = reader.u32()?;
    let flags = reader.u32()?;
    Ok(RawHeader {
        format_version,
        player_version,
        engine_version,
        compressed_info_size,
        uncompressed_info_size,
        flags,
    })
}

struct BlocksInfo {
    blocks: Vec<StorageBlock>,
    nodes: Vec<(i64, i64, u32, String)>,
}

fn parse_blocks_info(info: &[u8]) -> Result<BlocksInfo> {
    let mut reader = ByteReader::new(info, Endian::Big);
    reader.take(16)?;
    let block_count = count(reader.i32()?)?;
    let blocks = (0..block_count)
        .map(|_| {
            Ok(StorageBlock {
                uncompressed_size: reader.u32()?,
                compressed_size: reader.u32()?,
                flags: reader.u16()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let node_count = count(reader.i32()?)?;
    let nodes = (0..node_count)
        .map(|_| {
            Ok((
                reader.i64()?,
                reader.i64()?,
                reader.u32()?,
                reader.cstring()?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(BlocksInfo { blocks, nodes })
}

fn count(value: i32) -> Result<usize> {
    usize::try_from(value).map_err(|_| UnityError::InvalidBundle(format!("negative count {value}")))
}

fn decompress_blocks(data: &[u8], start: usize, blocks: &[StorageBlock]) -> Result<Vec<u8>> {
    let mut reader = ByteReader::new(data, Endian::Big);
    reader.set_position(start);
    let total: usize = blocks
        .iter()
        .map(|block| block.uncompressed_size as usize)
        .sum();
    let mut stream = Vec::with_capacity(total);
    for block in blocks {
        let compressed = reader.take(block.compressed_size as usize)?;
        let compression = Compression::from_flags(u32::from(block.flags))?;
        stream.extend(decompress(
            compression,
            compressed,
            block.uncompressed_size as usize,
        )?);
    }
    Ok(stream)
}

fn slice_nodes(stream: &[u8], nodes: Vec<(i64, i64, u32, String)>) -> Result<Vec<BundleNode>> {
    nodes
        .into_iter()
        .map(|(offset, size, flags, path)| {
            let start = usize::try_from(offset)
                .map_err(|_| UnityError::InvalidBundle(format!("node offset {offset}")))?;
            let length = usize::try_from(size)
                .map_err(|_| UnityError::InvalidBundle(format!("node size {size}")))?;
            let data = stream
                .get(start..start + length)
                .ok_or_else(|| UnityError::InvalidBundle(format!("node {path} out of range")))?
                .to_vec();
            Ok(BundleNode { path, flags, data })
        })
        .collect()
}

impl Bundle {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data, Endian::Big);
        let header = read_raw_header(&mut reader)?;
        if header.format_version >= 7 {
            reader.align(16);
        }
        let info_compression = Compression::from_flags(header.flags)?;
        let compressed_info = if header.flags & FLAG_BLOCKS_INFO_AT_END != 0 {
            let start = data
                .len()
                .checked_sub(header.compressed_info_size as usize)
                .ok_or_else(|| UnityError::InvalidBundle("block info past end".into()))?;
            &data[start..]
        } else {
            reader.take(header.compressed_info_size as usize)?
        };
        let info = decompress(
            info_compression,
            compressed_info,
            header.uncompressed_info_size as usize,
        )?;
        if header.flags & FLAG_BLOCK_INFO_NEEDS_PADDING != 0 {
            reader.align(16);
        }
        let parsed = parse_blocks_info(&info)?;
        let stream = decompress_blocks(data, reader.position(), &parsed.blocks)?;
        Ok(Self {
            format_version: header.format_version,
            player_version: header.player_version,
            engine_version: header.engine_version,
            header_flags: header.flags,
            blocks: parsed.blocks,
            nodes: slice_nodes(&stream, parsed.nodes)?,
        })
    }

    pub fn node(&self, path: &str) -> Option<&BundleNode> {
        self.nodes.iter().find(|node| node.path == path)
    }

    pub fn node_mut(&mut self, path: &str) -> Option<&mut BundleNode> {
        self.nodes.iter_mut().find(|node| node.path == path)
    }

    pub fn data_stream(&self) -> Vec<u8> {
        self.nodes
            .iter()
            .flat_map(|node| node.data.iter().copied())
            .collect()
    }

    pub fn to_bytes(&self, options: &BundleWriteOptions) -> Result<Vec<u8>> {
        let stream = self.data_stream();
        let (blocks, block_data) = compress_stream(&stream, options)?;
        let info = self.blocks_info(&blocks);
        let compressed_info = compress(options.block_info_compression, &info)?;
        let flags = FLAG_BLOCKS_AND_DIRECTORY_COMBINED | options.block_info_compression.flag_bits();
        let mut writer = ByteWriter::new(Endian::Big);
        writer.cstring(UNITY_FS_SIGNATURE);
        writer.u32(self.format_version);
        writer.cstring(&self.player_version);
        writer.cstring(&self.engine_version);
        let header_length = writer.len() + 8 + 4 + 4 + 4;
        let aligned_header_length = if self.format_version >= 7 {
            header_length.next_multiple_of(16)
        } else {
            header_length
        };
        let total = aligned_header_length + compressed_info.len() + block_data.len();
        writer.i64(total as i64);
        writer.u32(compressed_info.len() as u32);
        writer.u32(info.len() as u32);
        writer.u32(flags);
        if self.format_version >= 7 {
            writer.align(16);
        }
        writer.bytes(&compressed_info);
        writer.bytes(&block_data);
        Ok(writer.into_bytes())
    }

    fn blocks_info(&self, blocks: &[StorageBlock]) -> Vec<u8> {
        let mut writer = ByteWriter::new(Endian::Big);
        writer.bytes(&[0u8; 16]);
        writer.i32(blocks.len() as i32);
        for block in blocks {
            writer.u32(block.uncompressed_size);
            writer.u32(block.compressed_size);
            writer.u16(block.flags);
        }
        writer.i32(self.nodes.len() as i32);
        let mut offset = 0i64;
        for node in &self.nodes {
            writer.i64(offset);
            writer.i64(node.data.len() as i64);
            writer.u32(node.flags);
            writer.cstring(&node.path);
            offset += node.data.len() as i64;
        }
        writer.into_bytes()
    }
}

fn compress_stream(
    stream: &[u8],
    options: &BundleWriteOptions,
) -> Result<(Vec<StorageBlock>, Vec<u8>)> {
    let mut blocks = Vec::new();
    let mut data = Vec::new();
    for chunk in stream.chunks(options.block_size.max(1)) {
        let compressed = compress(options.data_compression, chunk)?;
        let (bytes, compression) = if compressed.len() < chunk.len() {
            (compressed, options.data_compression)
        } else {
            (chunk.to_vec(), Compression::None)
        };
        blocks.push(StorageBlock {
            uncompressed_size: chunk.len() as u32,
            compressed_size: bytes.len() as u32,
            flags: compression.flag_bits() as u16,
        });
        data.extend(bytes);
    }
    Ok((blocks, data))
}
