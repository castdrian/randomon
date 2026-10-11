use crate::bundle::{Bundle, BundleWriteOptions, is_unity_fs};
use crate::error::{Result, UnityError};
use crate::serialized::{ObjectInfo, SerializedFile};
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum AssetFile {
    Bundle {
        bundle: Bundle,
        serialized: Vec<Option<SerializedFile>>,
    },
    Serialized(SerializedFile),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectLocation {
    pub file: usize,
    pub object: usize,
}

impl AssetFile {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if is_unity_fs(data) {
            let bundle = Bundle::parse(data)?;
            let serialized = bundle
                .nodes
                .iter()
                .map(|node| {
                    if node.is_serialized_file() {
                        SerializedFile::parse(&node.data).map(Some)
                    } else {
                        Ok(None)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(Self::Bundle { bundle, serialized })
        } else {
            Ok(Self::Serialized(SerializedFile::parse(data)?))
        }
    }

    pub fn serialized_files(&self) -> Vec<&SerializedFile> {
        match self {
            Self::Bundle { serialized, .. } => serialized.iter().flatten().collect(),
            Self::Serialized(file) => vec![file],
        }
    }

    pub fn serialized_file(&self, index: usize) -> Option<&SerializedFile> {
        self.serialized_files().get(index).copied()
    }

    pub fn serialized_file_mut(&mut self, index: usize) -> Option<&mut SerializedFile> {
        match self {
            Self::Bundle { serialized, .. } => serialized.iter_mut().flatten().nth(index),
            Self::Serialized(file) => (index == 0).then_some(file),
        }
    }

    pub fn objects(&self) -> Vec<(ObjectLocation, &SerializedFile, &ObjectInfo)> {
        self.serialized_files()
            .into_iter()
            .enumerate()
            .flat_map(|(file_index, file)| {
                file.objects
                    .iter()
                    .enumerate()
                    .map(move |(object_index, object)| {
                        (
                            ObjectLocation {
                                file: file_index,
                                object: object_index,
                            },
                            file,
                            object,
                        )
                    })
            })
            .collect()
    }

    pub fn object_path_ids(&self) -> Vec<i64> {
        self.objects()
            .into_iter()
            .map(|(_, _, object)| object.path_id)
            .collect()
    }

    pub fn locate(&self, path_id: i64) -> Option<ObjectLocation> {
        self.objects()
            .into_iter()
            .find(|(_, _, object)| object.path_id == path_id)
            .map(|(location, _, _)| location)
    }

    pub fn read_value(&self, path_id: i64) -> Result<Value> {
        let location = self
            .locate(path_id)
            .ok_or(UnityError::ObjectNotFound(path_id))?;
        let file = self
            .serialized_file(location.file)
            .ok_or(UnityError::ObjectNotFound(path_id))?;
        file.decode(&file.objects[location.object])
    }

    pub fn write_value(&mut self, path_id: i64, value: &Value) -> Result<()> {
        let location = self
            .locate(path_id)
            .ok_or(UnityError::ObjectNotFound(path_id))?;
        self.serialized_file_mut(location.file)
            .ok_or(UnityError::ObjectNotFound(path_id))?
            .write_value(path_id, value)
    }

    pub fn object_data(&self, path_id: i64) -> Result<&[u8]> {
        let location = self
            .locate(path_id)
            .ok_or(UnityError::ObjectNotFound(path_id))?;
        let file = self
            .serialized_file(location.file)
            .ok_or(UnityError::ObjectNotFound(path_id))?;
        Ok(&file.objects[location.object].data)
    }

    pub fn set_object_data(&mut self, path_id: i64, data: Vec<u8>) -> Result<()> {
        let location = self
            .locate(path_id)
            .ok_or(UnityError::ObjectNotFound(path_id))?;
        self.serialized_file_mut(location.file)
            .ok_or(UnityError::ObjectNotFound(path_id))?
            .set_object_data(path_id, data)
    }

    pub fn is_modified(&self) -> bool {
        self.serialized_files()
            .iter()
            .any(|file| file.is_modified())
    }

    pub fn to_bytes(&self, options: &BundleWriteOptions) -> Result<Vec<u8>> {
        match self {
            Self::Serialized(file) => Ok(file.to_bytes()),
            Self::Bundle { bundle, serialized } => {
                let mut rebuilt = bundle.clone();
                for (node, file) in rebuilt.nodes.iter_mut().zip(serialized) {
                    if let Some(file) = file {
                        node.data = file.to_bytes();
                    }
                }
                rebuilt.to_bytes(options)
            }
        }
    }

    pub fn decompressed_nodes(&self) -> Vec<(String, Vec<u8>)> {
        match self {
            Self::Serialized(file) => vec![(String::new(), file.to_bytes())],
            Self::Bundle { bundle, serialized } => bundle
                .nodes
                .iter()
                .zip(serialized)
                .map(|(node, file)| {
                    (
                        node.path.clone(),
                        file.as_ref()
                            .map_or_else(|| node.data.clone(), SerializedFile::to_bytes),
                    )
                })
                .collect(),
        }
    }
}
