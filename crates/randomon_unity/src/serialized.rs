use crate::codec::{read_object, write_object};
use crate::error::{Result, UnityError};
use crate::stream::{ByteReader, ByteWriter, Endian};
use crate::typetree::TypeTree;
use crate::value::Value;

pub const CLASS_MONO_BEHAVIOUR: i32 = 114;
const MIN_VERSION: u32 = 17;
const MAX_VERSION: u32 = 22;
const OBJECT_ALIGNMENT: usize = 8;
const LARGE_HEADER_VERSION: u32 = 22;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedType {
    pub class_id: i32,
    pub is_stripped: bool,
    pub script_type_index: i16,
    pub script_id: Option<[u8; 16]>,
    pub old_type_hash: [u8; 16],
    pub type_tree: Option<TypeTree>,
    pub dependencies: Vec<i32>,
    pub reference_names: Option<(String, String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectInfo {
    pub path_id: i64,
    pub byte_start: u64,
    pub type_index: i32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptReference {
    pub file_index: i32,
    pub path_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct External {
    pub temp_empty: String,
    pub guid: [u8; 16],
    pub kind: i32,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedFile {
    pub version: u32,
    pub endian: Endian,
    pub reserved: [u8; 3],
    pub large_header_unknown: i64,
    pub unity_version: String,
    pub target_platform: i32,
    pub type_tree_enabled: bool,
    pub types: Vec<SerializedType>,
    pub objects: Vec<ObjectInfo>,
    pub scripts: Vec<ScriptReference>,
    pub externals: Vec<External>,
    pub reference_types: Vec<SerializedType>,
    pub user_information: String,
    pub data_offset: u64,
    original: Option<Vec<u8>>,
}

struct Header {
    metadata_size: u64,
    data_offset: u64,
    version: u32,
    endian: Endian,
    reserved: [u8; 3],
    unknown: i64,
}

fn read_header(reader: &mut ByteReader<'_>) -> Result<Header> {
    let mut metadata_size = u64::from(reader.u32()?);
    let _file_size = reader.u32()?;
    let version = reader.u32()?;
    let mut data_offset = u64::from(reader.u32()?);
    if !(MIN_VERSION..=MAX_VERSION).contains(&version) {
        return Err(UnityError::UnsupportedSerializedVersion(version));
    }
    let endian = if reader.u8()? == 0 {
        Endian::Little
    } else {
        Endian::Big
    };
    let reserved = reader.take_array::<3>()?;
    let mut unknown = 0;
    if version >= LARGE_HEADER_VERSION {
        metadata_size = u64::from(reader.u32()?);
        let _large_file_size = reader.i64()?;
        data_offset = reader.u64()?;
        unknown = reader.i64()?;
    }
    Ok(Header {
        metadata_size,
        data_offset,
        version,
        endian,
        reserved,
        unknown,
    })
}

fn read_type(
    reader: &mut ByteReader<'_>,
    version: u32,
    type_tree_enabled: bool,
    is_reference: bool,
) -> Result<SerializedType> {
    let class_id = reader.i32()?;
    let is_stripped = reader.u8()? != 0;
    let script_type_index = if version >= 17 { reader.i16()? } else { -1 };
    let script_id = if (is_reference && script_type_index >= 0) || class_id == CLASS_MONO_BEHAVIOUR
    {
        Some(reader.take_array::<16>()?)
    } else {
        None
    };
    let old_type_hash = reader.take_array::<16>()?;
    let mut type_tree = None;
    let mut dependencies = Vec::new();
    let mut reference_names = None;
    if type_tree_enabled {
        type_tree = Some(TypeTree::parse_blob(reader, version)?);
        if version >= 21 {
            if is_reference {
                reference_names = Some((reader.cstring()?, reader.cstring()?, reader.cstring()?));
            } else {
                let count = usize::try_from(reader.i32()?)
                    .map_err(|_| UnityError::InvalidTypeTree("negative dependency count".into()))?;
                dependencies = (0..count).map(|_| reader.i32()).collect::<Result<_>>()?;
            }
        }
    }
    Ok(SerializedType {
        class_id,
        is_stripped,
        script_type_index,
        script_id,
        old_type_hash,
        type_tree,
        dependencies,
        reference_names,
    })
}

fn write_type(writer: &mut ByteWriter, kind: &SerializedType, version: u32, enabled: bool) {
    writer.i32(kind.class_id);
    writer.u8(u8::from(kind.is_stripped));
    if version >= 17 {
        writer.i16(kind.script_type_index);
    }
    if let Some(script_id) = &kind.script_id {
        writer.bytes(script_id);
    }
    writer.bytes(&kind.old_type_hash);
    if !enabled {
        return;
    }
    if let Some(tree) = &kind.type_tree {
        tree.write_blob(writer, version);
    }
    if version >= 21 {
        if let Some((class_name, namespace, assembly)) = &kind.reference_names {
            writer.cstring(class_name);
            writer.cstring(namespace);
            writer.cstring(assembly);
        } else {
            writer.i32(kind.dependencies.len() as i32);
            for dependency in &kind.dependencies {
                writer.i32(*dependency);
            }
        }
    }
}

fn read_count(reader: &mut ByteReader<'_>) -> Result<usize> {
    let value = reader.i32()?;
    usize::try_from(value).map_err(|_| UnityError::InvalidBundle(format!("negative count {value}")))
}

struct ObjectRecord {
    path_id: i64,
    byte_start: u64,
    byte_size: u32,
    type_index: i32,
}

fn read_object_records(reader: &mut ByteReader<'_>, version: u32) -> Result<Vec<ObjectRecord>> {
    let count = read_count(reader)?;
    (0..count)
        .map(|_| {
            reader.align(4);
            let path_id = reader.i64()?;
            let byte_start = if version >= LARGE_HEADER_VERSION {
                reader.u64()?
            } else {
                u64::from(reader.u32()?)
            };
            let byte_size = reader.u32()?;
            let type_index = reader.i32()?;
            Ok(ObjectRecord {
                path_id,
                byte_start,
                byte_size,
                type_index,
            })
        })
        .collect()
}

impl SerializedFile {
    pub fn from_parts(
        version: u32,
        unity_version: &str,
        target_platform: i32,
        types: Vec<SerializedType>,
        objects: Vec<ObjectInfo>,
    ) -> Self {
        Self {
            version,
            endian: Endian::Little,
            reserved: [0; 3],
            large_header_unknown: 0,
            unity_version: unity_version.to_owned(),
            target_platform,
            type_tree_enabled: types.iter().any(|kind| kind.type_tree.is_some()),
            types,
            objects,
            scripts: Vec::new(),
            externals: Vec::new(),
            reference_types: Vec::new(),
            user_information: String::new(),
            data_offset: 0,
            original: None,
        }
    }

    pub fn looks_like_serialized_file(data: &[u8]) -> bool {
        if data.len() < 20 {
            return false;
        }
        let version = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        (MIN_VERSION..=MAX_VERSION).contains(&version)
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data, Endian::Big);
        let header = read_header(&mut reader)?;
        let version = header.version;
        reader.set_endian(header.endian);
        let unity_version = reader.cstring()?;
        let target_platform = reader.i32()?;
        let type_tree_enabled = reader.u8()? != 0;
        let type_count = read_count(&mut reader)?;
        let types = (0..type_count)
            .map(|_| read_type(&mut reader, version, type_tree_enabled, false))
            .collect::<Result<Vec<_>>>()?;
        let records = read_object_records(&mut reader, version)?;
        let script_count = read_count(&mut reader)?;
        let scripts = (0..script_count)
            .map(|_| {
                let file_index = reader.i32()?;
                reader.align(4);
                Ok(ScriptReference {
                    file_index,
                    path_id: reader.i64()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let external_count = read_count(&mut reader)?;
        let externals = (0..external_count)
            .map(|_| {
                Ok(External {
                    temp_empty: reader.cstring()?,
                    guid: reader.take_array::<16>()?,
                    kind: reader.i32()?,
                    path: reader.cstring()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let reference_types = if version >= 20 {
            let count = read_count(&mut reader)?;
            (0..count)
                .map(|_| read_type(&mut reader, version, type_tree_enabled, true))
                .collect::<Result<Vec<_>>>()?
        } else {
            Vec::new()
        };
        let user_information = reader.cstring()?;
        let objects = records
            .into_iter()
            .map(|record| {
                let start = (header.data_offset + record.byte_start) as usize;
                let end = start + record.byte_size as usize;
                let bytes = data.get(start..end).ok_or_else(|| {
                    UnityError::InvalidBundle(format!("object {} out of range", record.path_id))
                })?;
                Ok(ObjectInfo {
                    path_id: record.path_id,
                    byte_start: record.byte_start,
                    type_index: record.type_index,
                    data: bytes.to_vec(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let file = Self {
            version,
            endian: header.endian,
            reserved: header.reserved,
            large_header_unknown: header.unknown,
            unity_version,
            target_platform,
            type_tree_enabled,
            types,
            objects,
            scripts,
            externals,
            reference_types,
            user_information,
            data_offset: header.data_offset,
            original: Some(data.to_vec()),
        };
        let expected_metadata = file.metadata_bytes().len() as u64;
        if expected_metadata != header.metadata_size {
            return Err(UnityError::InvalidBundle(format!(
                "metadata is {} bytes but re-encodes to {expected_metadata}",
                header.metadata_size
            )));
        }
        Ok(file)
    }

    pub fn is_modified(&self) -> bool {
        self.original.is_none()
    }

    pub fn object_index(&self, path_id: i64) -> Option<usize> {
        self.objects
            .iter()
            .position(|object| object.path_id == path_id)
    }

    pub fn object(&self, path_id: i64) -> Option<&ObjectInfo> {
        self.objects.iter().find(|object| object.path_id == path_id)
    }

    pub fn object_type(&self, object: &ObjectInfo) -> Option<&SerializedType> {
        usize::try_from(object.type_index)
            .ok()
            .and_then(|index| self.types.get(index))
    }

    pub fn class_id(&self, object: &ObjectInfo) -> Option<i32> {
        self.object_type(object).map(|kind| kind.class_id)
    }

    pub fn type_tree(&self, object: &ObjectInfo) -> Option<&TypeTree> {
        self.object_type(object)
            .and_then(|kind| kind.type_tree.as_ref())
    }

    pub fn read_value(&self, path_id: i64) -> Result<Value> {
        let object = self
            .object(path_id)
            .ok_or(UnityError::ObjectNotFound(path_id))?;
        self.decode(object)
    }

    pub fn decode(&self, object: &ObjectInfo) -> Result<Value> {
        let tree = self
            .type_tree(object)
            .ok_or(UnityError::MissingTypeTree(object.path_id))?;
        read_object(tree, &object.data, self.endian)
    }

    pub fn encode(&self, path_id: i64, value: &Value) -> Result<Vec<u8>> {
        let object = self
            .object(path_id)
            .ok_or(UnityError::ObjectNotFound(path_id))?;
        let tree = self
            .type_tree(object)
            .ok_or(UnityError::MissingTypeTree(path_id))?;
        write_object(tree, value, self.endian)
    }

    pub fn write_value(&mut self, path_id: i64, value: &Value) -> Result<()> {
        let bytes = self.encode(path_id, value)?;
        self.set_object_data(path_id, bytes)
    }

    pub fn set_object_data(&mut self, path_id: i64, data: Vec<u8>) -> Result<()> {
        let index = self
            .object_index(path_id)
            .ok_or(UnityError::ObjectNotFound(path_id))?;
        if self.objects[index].data != data {
            self.objects[index].data = data;
            self.original = None;
        }
        Ok(())
    }

    pub fn mark_modified(&mut self) {
        self.original = None;
    }

    fn metadata_bytes(&self) -> Vec<u8> {
        let version = self.version;
        let mut writer = ByteWriter::new(self.endian);
        writer.cstring(&self.unity_version);
        writer.i32(self.target_platform);
        writer.u8(u8::from(self.type_tree_enabled));
        writer.i32(self.types.len() as i32);
        for kind in &self.types {
            write_type(&mut writer, kind, version, self.type_tree_enabled);
        }
        writer.i32(self.objects.len() as i32);
        let base = self.header_size();
        for object in &self.objects {
            let padding = (base + writer.len()).next_multiple_of(4) - (base + writer.len());
            writer.bytes(&vec![0u8; padding]);
            writer.i64(object.path_id);
            if version >= LARGE_HEADER_VERSION {
                writer.u64(object.byte_start);
            } else {
                writer.u32(object.byte_start as u32);
            }
            writer.u32(object.data.len() as u32);
            writer.i32(object.type_index);
        }
        writer.i32(self.scripts.len() as i32);
        for script in &self.scripts {
            writer.i32(script.file_index);
            let padding = (base + writer.len()).next_multiple_of(4) - (base + writer.len());
            writer.bytes(&vec![0u8; padding]);
            writer.i64(script.path_id);
        }
        writer.i32(self.externals.len() as i32);
        for external in &self.externals {
            writer.cstring(&external.temp_empty);
            writer.bytes(&external.guid);
            writer.i32(external.kind);
            writer.cstring(&external.path);
        }
        if version >= 20 {
            writer.i32(self.reference_types.len() as i32);
            for kind in &self.reference_types {
                write_type(&mut writer, kind, version, self.type_tree_enabled);
            }
        }
        writer.cstring(&self.user_information);
        writer.into_bytes()
    }

    fn header_size(&self) -> usize {
        if self.version >= LARGE_HEADER_VERSION {
            48
        } else {
            20
        }
    }

    fn relayout_objects(&mut self) {
        let mut order: Vec<usize> = (0..self.objects.len()).collect();
        order.sort_by_key(|&index| (self.objects[index].byte_start, index));
        let mut position = 0usize;
        for index in order {
            position = position.next_multiple_of(OBJECT_ALIGNMENT);
            self.objects[index].byte_start = position as u64;
            position += self.objects[index].data.len();
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        if let Some(original) = &self.original {
            return original.clone();
        }
        self.rebuild()
    }

    pub fn rebuild(&self) -> Vec<u8> {
        let mut file = self.clone();
        file.relayout_objects();
        let metadata = file.metadata_bytes();
        let header_size = file.header_size();
        let minimum_offset = (header_size + metadata.len()).next_multiple_of(16);
        let data_offset = minimum_offset.max(file.data_offset as usize);
        let data_length = file
            .objects
            .iter()
            .map(|object| object.byte_start as usize + object.data.len())
            .max()
            .unwrap_or(0);
        let mut data = vec![0u8; data_length];
        for object in &file.objects {
            let start = object.byte_start as usize;
            data[start..start + object.data.len()].copy_from_slice(&object.data);
        }
        let file_size = data_offset + data_length;
        let mut writer = ByteWriter::new(Endian::Big);
        if file.version >= LARGE_HEADER_VERSION {
            writer.u32(0);
            writer.u32(0);
            writer.u32(file.version);
            writer.u32(0);
        } else {
            writer.u32(metadata.len() as u32);
            writer.u32(file_size as u32);
            writer.u32(file.version);
            writer.u32(data_offset as u32);
        }
        writer.u8(u8::from(file.endian == Endian::Big));
        writer.bytes(&file.reserved);
        if file.version >= LARGE_HEADER_VERSION {
            writer.u32(metadata.len() as u32);
            writer.u64(file_size as u64);
            writer.u64(data_offset as u64);
            writer.i64(file.large_header_unknown);
        }
        writer.bytes(&metadata);
        let padding = data_offset - writer.len();
        writer.bytes(&vec![0u8; padding]);
        writer.bytes(&data);
        writer.into_bytes()
    }
}
