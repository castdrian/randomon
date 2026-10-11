use crate::common_strings::COMMON_STRINGS;
use crate::error::{Result, UnityError};
use crate::stream::{ByteReader, ByteWriter};

pub const META_FLAG_ALIGN: u32 = 0x4000;
const COMMON_STRING_BIT: u32 = 0x8000_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeTreeNode {
    pub version: u16,
    pub level: u8,
    pub type_flags: u8,
    pub type_offset: u32,
    pub name_offset: u32,
    pub byte_size: i32,
    pub index: i32,
    pub meta_flag: u32,
    pub ref_type_hash: u64,
    pub type_name: String,
    pub name: String,
}

impl TypeTreeNode {
    pub fn aligns(&self) -> bool {
        self.meta_flag & META_FLAG_ALIGN != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeTree {
    pub nodes: Vec<TypeTreeNode>,
    pub string_buffer: Vec<u8>,
    subtree_ends: Vec<usize>,
}

fn node_record_size(version: u32) -> usize {
    if version >= 19 { 32 } else { 24 }
}

fn resolve_string(buffer: &[u8], offset: u32) -> Result<String> {
    if offset & COMMON_STRING_BIT != 0 {
        let key = offset & !COMMON_STRING_BIT;
        return COMMON_STRINGS
            .iter()
            .find(|(common_offset, _)| *common_offset == key)
            .map(|(_, text)| (*text).to_owned())
            .ok_or_else(|| UnityError::InvalidTypeTree(format!("unknown common string {key}")));
    }
    let start = offset as usize;
    let rest = buffer
        .get(start..)
        .ok_or_else(|| UnityError::InvalidTypeTree(format!("string offset {start}")))?;
    let end = rest
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(rest.len());
    String::from_utf8(rest[..end].to_vec()).map_err(|_| UnityError::InvalidString(start))
}

fn compute_subtree_ends(nodes: &[TypeTreeNode]) -> Vec<usize> {
    (0..nodes.len())
        .map(|index| {
            let level = nodes[index].level;
            nodes[index + 1..]
                .iter()
                .position(|node| node.level <= level)
                .map_or(nodes.len(), |offset| index + 1 + offset)
        })
        .collect()
}

impl TypeTree {
    pub fn new(nodes: Vec<TypeTreeNode>, string_buffer: Vec<u8>) -> Self {
        let subtree_ends = compute_subtree_ends(&nodes);
        Self {
            nodes,
            string_buffer,
            subtree_ends,
        }
    }

    pub fn parse_blob(reader: &mut ByteReader<'_>, version: u32) -> Result<Self> {
        let node_count = usize::try_from(reader.i32()?)
            .map_err(|_| UnityError::InvalidTypeTree("negative node count".into()))?;
        let buffer_size = usize::try_from(reader.i32()?)
            .map_err(|_| UnityError::InvalidTypeTree("negative string buffer size".into()))?;
        let mut raw = Vec::with_capacity(node_count);
        for _ in 0..node_count {
            let version_field = reader.u16()?;
            let level = reader.u8()?;
            let type_flags = reader.u8()?;
            let type_offset = reader.u32()?;
            let name_offset = reader.u32()?;
            let byte_size = reader.i32()?;
            let index = reader.i32()?;
            let meta_flag = reader.u32()?;
            let ref_type_hash = if version >= 19 { reader.u64()? } else { 0 };
            raw.push((
                version_field,
                level,
                type_flags,
                type_offset,
                name_offset,
                byte_size,
                index,
                meta_flag,
                ref_type_hash,
            ));
        }
        let string_buffer = reader.take(buffer_size)?.to_vec();
        let nodes = raw
            .into_iter()
            .map(
                |(
                    version,
                    level,
                    type_flags,
                    type_offset,
                    name_offset,
                    byte_size,
                    index,
                    meta_flag,
                    ref_type_hash,
                )| {
                    Ok(TypeTreeNode {
                        version,
                        level,
                        type_flags,
                        type_offset,
                        name_offset,
                        byte_size,
                        index,
                        meta_flag,
                        ref_type_hash,
                        type_name: resolve_string(&string_buffer, type_offset)?,
                        name: resolve_string(&string_buffer, name_offset)?,
                    })
                },
            )
            .collect::<Result<Vec<_>>>()?;
        Ok(Self::new(nodes, string_buffer))
    }

    pub fn write_blob(&self, writer: &mut ByteWriter, version: u32) {
        writer.i32(self.nodes.len() as i32);
        writer.i32(self.string_buffer.len() as i32);
        for node in &self.nodes {
            writer.u16(node.version);
            writer.u8(node.level);
            writer.u8(node.type_flags);
            writer.u32(node.type_offset);
            writer.u32(node.name_offset);
            writer.i32(node.byte_size);
            writer.i32(node.index);
            writer.u32(node.meta_flag);
            if version >= 19 {
                writer.u64(node.ref_type_hash);
            }
        }
        writer.bytes(&self.string_buffer);
    }

    pub fn blob_size(&self, version: u32) -> usize {
        8 + self.nodes.len() * node_record_size(version) + self.string_buffer.len()
    }

    pub fn subtree_end(&self, index: usize) -> usize {
        self.subtree_ends[index]
    }

    pub fn children(&self, index: usize) -> ChildIter<'_> {
        ChildIter {
            tree: self,
            next: index + 1,
            end: self.subtree_ends[index],
        }
    }

    pub fn root(&self) -> Option<&TypeTreeNode> {
        self.nodes.first()
    }
}

pub struct ChildIter<'a> {
    tree: &'a TypeTree,
    next: usize,
    end: usize,
}

impl Iterator for ChildIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.next >= self.end {
            return None;
        }
        let current = self.next;
        self.next = self.tree.subtree_ends[current];
        Some(current)
    }
}
