use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use randomon_unity::{AssetFile, BundleWriteOptions, Value};

use crate::assets::BundleId;
use crate::error::{BdspError, Result};

pub struct Bundles {
    files: BTreeMap<BundleId, AssetFile>,
    modified: BTreeSet<BundleId>,
}

pub fn data_directory(romfs: &Path) -> PathBuf {
    let nested = romfs.join("Data");
    if nested.join(BundleId::Masterdatas.relative_path()).is_file() {
        nested
    } else {
        romfs.to_path_buf()
    }
}

pub fn looks_like_bdsp_romfs(path: &Path) -> bool {
    let data = data_directory(path);
    BundleId::ALL
        .iter()
        .filter(|bundle| bundle.is_required())
        .all(|bundle| data.join(bundle.relative_path()).is_file())
}

impl Bundles {
    pub fn load(romfs: &Path) -> Result<Self> {
        let data = data_directory(romfs);
        let mut files = BTreeMap::new();
        for bundle in BundleId::ALL {
            let path = data.join(bundle.relative_path());
            if !path.is_file() {
                if bundle.is_required() {
                    return Err(BdspError::MissingFile(path));
                }
                continue;
            }
            let bytes = std::fs::read(&path)
                .map_err(|error| BdspError::Io(path.clone(), error.to_string()))?;
            let asset =
                AssetFile::parse(&bytes).map_err(|error| BdspError::Unity(bundle, error))?;
            files.insert(bundle, asset);
        }
        Ok(Self {
            files,
            modified: BTreeSet::new(),
        })
    }

    pub fn from_assets(files: BTreeMap<BundleId, AssetFile>) -> Self {
        Self {
            files,
            modified: BTreeSet::new(),
        }
    }

    pub fn has(&self, bundle: BundleId) -> bool {
        self.files.contains_key(&bundle)
    }

    fn asset(&self, bundle: BundleId) -> Result<&AssetFile> {
        self.files
            .get(&bundle)
            .ok_or_else(|| BdspError::MissingFile(PathBuf::from(bundle.relative_path())))
    }

    fn asset_mut(&mut self, bundle: BundleId) -> Result<&mut AssetFile> {
        self.files
            .get_mut(&bundle)
            .ok_or_else(|| BdspError::MissingFile(PathBuf::from(bundle.relative_path())))
    }

    pub fn read(&self, bundle: BundleId, path_id: i64) -> Result<Value> {
        self.asset(bundle)?
            .read_value(path_id)
            .map_err(|error| BdspError::Unity(bundle, error))
    }

    pub fn write(&mut self, bundle: BundleId, path_id: i64, value: &Value) -> Result<()> {
        self.asset_mut(bundle)?
            .write_value(path_id, value)
            .map_err(|error| BdspError::Unity(bundle, error))?;
        self.modified.insert(bundle);
        Ok(())
    }

    pub fn update<T>(
        &mut self,
        bundle: BundleId,
        path_id: i64,
        edit: impl FnOnce(&mut Value) -> Result<T>,
    ) -> Result<T> {
        let mut value = self.read(bundle, path_id)?;
        let result = edit(&mut value)?;
        self.write(bundle, path_id, &value)?;
        Ok(result)
    }

    pub fn raw(&self, bundle: BundleId, path_id: i64) -> Result<Vec<u8>> {
        Ok(self
            .asset(bundle)?
            .object_data(path_id)
            .map_err(|error| BdspError::Unity(bundle, error))?
            .to_vec())
    }

    pub fn class_id(&self, bundle: BundleId, path_id: i64) -> Option<i32> {
        let asset = self.files.get(&bundle)?;
        asset
            .objects()
            .into_iter()
            .find(|(_, _, object)| object.path_id == path_id)
            .and_then(|(_, file, object)| file.class_id(object))
    }

    pub fn set_raw(&mut self, bundle: BundleId, path_id: i64, data: Vec<u8>) -> Result<()> {
        self.asset_mut(bundle)?
            .set_object_data(path_id, data)
            .map_err(|error| BdspError::Unity(bundle, error))?;
        self.modified.insert(bundle);
        Ok(())
    }

    pub fn monobehaviours_named(
        &self,
        bundle: BundleId,
        prefix: &str,
    ) -> Result<Vec<(i64, Value)>> {
        let asset = self.asset(bundle)?;
        let mut found = Vec::new();
        for (_, file, object) in asset.objects() {
            if file.class_id(object) != Some(randomon_unity::CLASS_MONO_BEHAVIOUR) {
                continue;
            }
            let Ok(value) = file.decode(object) else {
                continue;
            };
            if value
                .field("m_Name")
                .ok()
                .and_then(Value::as_str)
                .is_some_and(|name| name.starts_with(prefix))
            {
                found.push((object.path_id, value));
            }
        }
        Ok(found)
    }

    pub fn modified(&self) -> impl Iterator<Item = BundleId> + '_ {
        self.modified.iter().copied()
    }

    pub fn serialize(&self, bundle: BundleId) -> Result<Vec<u8>> {
        let options = BundleWriteOptions::default();
        self.asset(bundle)?
            .to_bytes(&options)
            .map_err(|error| BdspError::Unity(bundle, error))
    }

    pub fn asset_file(&self, bundle: BundleId) -> Option<&AssetFile> {
        self.files.get(&bundle)
    }
}
