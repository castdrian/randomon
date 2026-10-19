use crate::bytes::{bytes_at, i32_at, i64_at, u8_at, u16_at, u32_at, u64_at};
use crate::error::{Result, SwshError, invalid};
use crate::formats::hash::fnv1a_64;

pub const MAGIC: u64 = 0x4B43_4150_584C_4647;
pub const DEFAULT_VERSION: u32 = 0x1000;
const CONTEXT: &str = "GFPAK";
const HEADER_SIZE: usize = 0x18;
const HASH_ENTRY_SIZE: usize = 0x10;
const FILE_ENTRY_SIZE: usize = 0x18;
const DATA_ALIGNMENT: usize = 0x10;
const LZ4_MAX_RATIO: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Zlib,
    Lz4,
    Oodle(u8),
}

impl Compression {
    pub fn from_id(id: u8) -> Result<Self> {
        match id {
            0 => Ok(Compression::None),
            1 => Ok(Compression::Zlib),
            2 => Ok(Compression::Lz4),
            3..=7 => Ok(Compression::Oodle(id)),
            _ => Err(SwshError::UnsupportedCompression(id)),
        }
    }

    pub fn id(self) -> u8 {
        match self {
            Compression::None => 0,
            Compression::Zlib => 1,
            Compression::Lz4 => 2,
            Compression::Oodle(id) => id,
        }
    }

    fn decompress(self, stored: &[u8], size: usize) -> Result<Vec<u8>> {
        let data = match self {
            Compression::None => stored.to_vec(),
            Compression::Lz4 if size > stored.len().saturating_mul(LZ4_MAX_RATIO) + 16 => {
                return Err(invalid(
                    CONTEXT,
                    format!(
                        "{size} decompressed bytes cannot come from {} LZ4 bytes",
                        stored.len()
                    ),
                ));
            }
            Compression::Lz4 => lz4_flex::block::decompress(stored, size)
                .map_err(|error| invalid(CONTEXT, format!("LZ4 decompression failed: {error}")))?,
            Compression::Zlib | Compression::Oodle(_) => {
                return Err(SwshError::UnsupportedCompression(self.id()));
            }
        };
        if data.len() != size {
            return Err(invalid(
                CONTEXT,
                format!("expected {size} decompressed bytes, got {}", data.len()),
            ));
        }
        Ok(data)
    }

    fn compress(self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Compression::None => Ok(data.to_vec()),
            Compression::Lz4 => Ok(lz4_flex::block::compress(data)),
            Compression::Zlib | Compression::Oodle(_) => {
                Err(SwshError::UnsupportedCompression(self.id()))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderEntry {
    pub name_hash: u64,
    pub file_index: usize,
    pub padding: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackFolder {
    pub hash: u64,
    pub padding: u32,
    pub entries: Vec<FolderEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackFile {
    pub level: u16,
    pub compression: Compression,
    pub reserved: u8,
    pub padding: i32,
    pub unused: u32,
    stored: Vec<u8>,
    data: Vec<u8>,
    replaced: bool,
}

impl PackFile {
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn is_replaced(&self) -> bool {
        self.replaced
    }

    fn stored_bytes(&self) -> Result<Vec<u8>> {
        if self.replaced {
            self.compression.compress(&self.data)
        } else {
            Ok(self.stored.clone())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GfPack {
    pub version: u32,
    pub relocated: u32,
    pub path_hashes: Vec<u64>,
    pub folders: Vec<PackFolder>,
    files: Vec<PackFile>,
}

struct Pointers {
    file_table: usize,
    path_hashes: usize,
    folders: Vec<usize>,
}

fn pointer_at(data: &[u8], offset: usize) -> Result<usize> {
    let value = i64_at(data, offset, CONTEXT)?;
    usize::try_from(value).map_err(|_| invalid(CONTEXT, format!("negative pointer {value}")))
}

fn count_at(data: &[u8], offset: usize) -> Result<usize> {
    let value = i32_at(data, offset, CONTEXT)?;
    usize::try_from(value).map_err(|_| invalid(CONTEXT, format!("negative count {value}")))
}

fn read_pointers(data: &[u8], folder_count: usize) -> Result<Pointers> {
    let folders = (0..folder_count)
        .map(|index| pointer_at(data, HEADER_SIZE + 0x10 + index * 8))
        .collect::<Result<Vec<_>>>()?;
    Ok(Pointers {
        file_table: pointer_at(data, HEADER_SIZE)?,
        path_hashes: pointer_at(data, HEADER_SIZE + 8)?,
        folders,
    })
}

fn read_folder(data: &[u8], offset: usize, file_count: usize) -> Result<PackFolder> {
    let entry_count = count_at(data, offset + 8)?;
    let entries = (0..entry_count)
        .map(|index| {
            let entry = offset + HASH_ENTRY_SIZE * (index + 1);
            let file_index = count_at(data, entry + 8)?;
            if file_index >= file_count {
                return Err(invalid(
                    CONTEXT,
                    format!("folder entry points at file {file_index} of {file_count}"),
                ));
            }
            Ok(FolderEntry {
                name_hash: u64_at(data, entry, CONTEXT)?,
                file_index,
                padding: u32_at(data, entry + 12, CONTEXT)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PackFolder {
        hash: u64_at(data, offset, CONTEXT)?,
        padding: u32_at(data, offset + 12, CONTEXT)?,
        entries,
    })
}

fn read_file(data: &[u8], offset: usize) -> Result<PackFile> {
    let compression = Compression::from_id(u8_at(data, offset + 2, CONTEXT)?)?;
    let decompressed_size = count_at(data, offset + 4)?;
    let compressed_size = count_at(data, offset + 8)?;
    let data_offset = count_at(data, offset + 0x10)?;
    let stored = bytes_at(data, data_offset, compressed_size, CONTEXT)?.to_vec();
    let content = compression.decompress(&stored, decompressed_size)?;
    Ok(PackFile {
        level: u16_at(data, offset, CONTEXT)?,
        compression,
        reserved: u8_at(data, offset + 3, CONTEXT)?,
        padding: i32_at(data, offset + 0x0C, CONTEXT)?,
        unused: u32_at(data, offset + 0x14, CONTEXT)?,
        stored,
        data: content,
        replaced: false,
    })
}

impl GfPack {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if u64_at(data, 0, CONTEXT)? != MAGIC {
            return Err(invalid(CONTEXT, "missing GFLXPACK magic"));
        }
        let file_count = count_at(data, 0x10)?;
        let folder_count = count_at(data, 0x14)?;
        let pointers = read_pointers(data, folder_count)?;
        let path_hashes = (0..file_count)
            .map(|index| u64_at(data, pointers.path_hashes + index * 8, CONTEXT))
            .collect::<Result<Vec<_>>>()?;
        let folders = pointers
            .folders
            .iter()
            .map(|&offset| read_folder(data, offset, file_count))
            .collect::<Result<Vec<_>>>()?;
        let files = (0..file_count)
            .map(|index| read_file(data, pointers.file_table + index * FILE_ENTRY_SIZE))
            .collect::<Result<Vec<_>>>()?;
        Ok(GfPack {
            version: u32_at(data, 8, CONTEXT)?,
            relocated: u32_at(data, 12, CONTEXT)?,
            path_hashes,
            folders,
            files,
        })
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn files(&self) -> &[PackFile] {
        &self.files
    }

    pub fn index_of_name_hash(&self, hash: u64) -> Option<usize> {
        self.folders
            .iter()
            .flat_map(|folder| folder.entries.iter())
            .find(|entry| entry.name_hash == hash)
            .map(|entry| entry.file_index)
    }

    pub fn index_of(&self, file_name: &str) -> Option<usize> {
        self.index_of_name_hash(fnv1a_64(file_name))
    }

    pub fn index_of_path(&self, full_path: &str) -> Option<usize> {
        let hash = fnv1a_64(full_path);
        self.path_hashes.iter().position(|&value| value == hash)
    }

    pub fn contains(&self, file_name: &str) -> bool {
        self.index_of(file_name).is_some()
    }

    pub fn data(&self, index: usize) -> Option<&[u8]> {
        self.files.get(index).map(PackFile::data)
    }

    pub fn file(&self, file_name: &str) -> Result<&[u8]> {
        self.index_of(file_name)
            .and_then(|index| self.data(index))
            .ok_or_else(|| SwshError::PackEntryNotFound(file_name.to_string()))
    }

    pub fn replace(&mut self, index: usize, data: Vec<u8>) -> Result<()> {
        let file = self
            .files
            .get_mut(index)
            .ok_or_else(|| SwshError::PackEntryNotFound(format!("file index {index}")))?;
        if file.data != data {
            file.data = data;
            file.replaced = true;
        }
        Ok(())
    }

    pub fn replace_file(&mut self, file_name: &str, data: Vec<u8>) -> Result<()> {
        let index = self
            .index_of(file_name)
            .ok_or_else(|| SwshError::PackEntryNotFound(file_name.to_string()))?;
        self.replace(index, data)
    }

    pub fn is_modified(&self) -> bool {
        self.files.iter().any(PackFile::is_replaced)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let stored = self
            .files
            .iter()
            .map(PackFile::stored_bytes)
            .collect::<Result<Vec<_>>>()?;
        let layout = Layout::new(self);
        let mut output = Vec::with_capacity(layout.data_start);
        self.write_header(&mut output, &layout)?;
        self.write_tables(&mut output, &layout, &stored)?;
        for bytes in &stored {
            pad_to(&mut output, DATA_ALIGNMENT);
            output.extend_from_slice(bytes);
        }
        pad_to(&mut output, DATA_ALIGNMENT);
        Ok(output)
    }

    fn write_header(&self, output: &mut Vec<u8>, layout: &Layout) -> Result<()> {
        output.extend_from_slice(&MAGIC.to_le_bytes());
        output.extend_from_slice(&self.version.to_le_bytes());
        output.extend_from_slice(&self.relocated.to_le_bytes());
        output.extend_from_slice(&count_i32(self.files.len())?.to_le_bytes());
        output.extend_from_slice(&count_i32(self.folders.len())?.to_le_bytes());
        output.extend_from_slice(&pointer_i64(layout.file_table)?.to_le_bytes());
        output.extend_from_slice(&pointer_i64(layout.path_hashes)?.to_le_bytes());
        for &folder in &layout.folders {
            output.extend_from_slice(&pointer_i64(folder)?.to_le_bytes());
        }
        Ok(())
    }

    fn write_tables(
        &self,
        output: &mut Vec<u8>,
        layout: &Layout,
        stored: &[Vec<u8>],
    ) -> Result<()> {
        for hash in &self.path_hashes {
            output.extend_from_slice(&hash.to_le_bytes());
        }
        for folder in &self.folders {
            output.extend_from_slice(&folder.hash.to_le_bytes());
            output.extend_from_slice(&count_i32(folder.entries.len())?.to_le_bytes());
            output.extend_from_slice(&folder.padding.to_le_bytes());
            for entry in &folder.entries {
                output.extend_from_slice(&entry.name_hash.to_le_bytes());
                output.extend_from_slice(&count_i32(entry.file_index)?.to_le_bytes());
                output.extend_from_slice(&entry.padding.to_le_bytes());
            }
        }
        let mut data_offset = layout.data_start;
        for (file, bytes) in self.files.iter().zip(stored) {
            output.extend_from_slice(&file.level.to_le_bytes());
            output.push(file.compression.id());
            output.push(file.reserved);
            output.extend_from_slice(&count_i32(file.data.len())?.to_le_bytes());
            output.extend_from_slice(&count_i32(bytes.len())?.to_le_bytes());
            output.extend_from_slice(&file.padding.to_le_bytes());
            output.extend_from_slice(&count_i32(data_offset)?.to_le_bytes());
            output.extend_from_slice(&file.unused.to_le_bytes());
            data_offset = (data_offset + bytes.len()).next_multiple_of(DATA_ALIGNMENT);
        }
        Ok(())
    }
}

struct Layout {
    path_hashes: usize,
    folders: Vec<usize>,
    file_table: usize,
    data_start: usize,
}

impl Layout {
    fn new(pack: &GfPack) -> Self {
        let path_hashes = HEADER_SIZE + 0x10 + pack.folders.len() * 8;
        let mut cursor = path_hashes + pack.path_hashes.len() * 8;
        let folders = pack
            .folders
            .iter()
            .map(|folder| {
                let start = cursor;
                cursor += HASH_ENTRY_SIZE * (folder.entries.len() + 1);
                start
            })
            .collect();
        let file_table = cursor;
        let data_start =
            (file_table + pack.files.len() * FILE_ENTRY_SIZE).next_multiple_of(DATA_ALIGNMENT);
        Layout {
            path_hashes,
            folders,
            file_table,
            data_start,
        }
    }
}

fn pad_to(output: &mut Vec<u8>, alignment: usize) {
    output.resize(output.len().next_multiple_of(alignment), 0);
}

fn count_i32(value: usize) -> Result<i32> {
    i32::try_from(value).map_err(|_| invalid(CONTEXT, format!("{value} does not fit in 32 bits")))
}

fn pointer_i64(value: usize) -> Result<i64> {
    i64::try_from(value).map_err(|_| invalid(CONTEXT, format!("{value} does not fit in 64 bits")))
}

pub struct PackBuilder {
    pack: GfPack,
}

impl PackBuilder {
    pub fn new() -> Self {
        PackBuilder {
            pack: GfPack {
                version: DEFAULT_VERSION,
                relocated: 0,
                path_hashes: Vec::new(),
                folders: Vec::new(),
                files: Vec::new(),
            },
        }
    }

    pub fn add(
        mut self,
        folder: &str,
        file_name: &str,
        data: Vec<u8>,
        compression: Compression,
    ) -> Self {
        let file_index = self.pack.files.len();
        let folder_hash = fnv1a_64(folder);
        self.pack
            .path_hashes
            .push(fnv1a_64(&format!("{folder}/{file_name}")));
        self.pack.files.push(PackFile {
            level: 9,
            compression,
            reserved: 0,
            padding: 0xCC,
            unused: 0,
            stored: Vec::new(),
            data,
            replaced: true,
        });
        let entry = FolderEntry {
            name_hash: fnv1a_64(file_name),
            file_index,
            padding: 0xCC,
        };
        match self
            .pack
            .folders
            .iter_mut()
            .find(|existing| existing.hash == folder_hash)
        {
            Some(existing) => existing.entries.push(entry),
            None => self.pack.folders.push(PackFolder {
                hash: folder_hash,
                padding: 0xCC,
                entries: vec![entry],
            }),
        }
        self
    }

    pub fn build(self) -> GfPack {
        self.pack
    }
}

impl Default for PackBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> GfPack {
        PackBuilder::new()
            .add(
                "bin/a",
                "first.bin",
                b"hello hello hello hello".to_vec(),
                Compression::Lz4,
            )
            .add("bin/a", "second.bin", vec![1, 2, 3], Compression::None)
            .add("bin/b", "third.bin", vec![9; 40], Compression::Lz4)
            .build()
    }

    #[test]
    fn written_pack_parses_back_with_the_same_files() {
        let bytes = sample().to_bytes().expect("write");
        let pack = GfPack::parse(&bytes).expect("parse");
        assert_eq!(pack.file_count(), 3);
        assert_eq!(pack.folders.len(), 2);
        assert_eq!(
            pack.file("first.bin").expect("first"),
            b"hello hello hello hello"
        );
        assert_eq!(pack.file("second.bin").expect("second"), &[1, 2, 3]);
        assert_eq!(pack.file("third.bin").expect("third"), &[9; 40]);
        assert_eq!(pack.index_of_path("bin/b/third.bin"), Some(2));
        assert_eq!(pack.files()[0].compression, Compression::Lz4);
        assert_eq!(bytes.len() % DATA_ALIGNMENT, 0);
    }

    #[test]
    fn unmodified_pack_round_trips_exactly() {
        let bytes = sample().to_bytes().expect("write");
        let pack = GfPack::parse(&bytes).expect("parse");
        assert!(!pack.is_modified());
        assert_eq!(pack.to_bytes().expect("rewrite"), bytes);
    }

    #[test]
    fn replaced_file_is_recompressed_and_read_back() {
        let bytes = sample().to_bytes().expect("write");
        let mut pack = GfPack::parse(&bytes).expect("parse");
        pack.replace_file("first.bin", vec![7; 300])
            .expect("replace");
        assert!(pack.is_modified());
        let rewritten = GfPack::parse(&pack.to_bytes().expect("write")).expect("parse");
        assert_eq!(rewritten.file("first.bin").expect("first"), &[7; 300]);
        assert_eq!(rewritten.file("third.bin").expect("third"), &[9; 40]);
        assert!(rewritten.files()[0].stored.len() < 300);
    }

    #[test]
    fn replacing_with_identical_data_is_not_a_modification() {
        let mut pack = GfPack::parse(&sample().to_bytes().expect("write")).expect("parse");
        pack.replace_file("second.bin", vec![1, 2, 3])
            .expect("replace");
        assert!(!pack.is_modified());
    }

    #[test]
    fn missing_entries_and_bad_input_are_errors() {
        let pack = sample();
        assert!(matches!(
            pack.file("nope.bin"),
            Err(SwshError::PackEntryNotFound(_))
        ));
        assert!(GfPack::parse(&[0u8; 8]).is_err());
        let mut bytes = sample().to_bytes().expect("write");
        bytes.truncate(bytes.len() - 0x20);
        assert!(GfPack::parse(&bytes).is_err());
        let mut corrupted = sample().to_bytes().expect("write");
        corrupted[0x10] = 0xFF;
        corrupted[0x13] = 0x7F;
        assert!(GfPack::parse(&corrupted).is_err());
    }

    #[test]
    fn unsupported_compression_is_reported() {
        assert_eq!(Compression::from_id(4), Ok(Compression::Oodle(4)));
        assert!(Compression::from_id(9).is_err());
        assert!(Compression::Zlib.decompress(&[0], 1).is_err());
        assert!(Compression::Oodle(3).compress(&[0]).is_err());
    }
}
