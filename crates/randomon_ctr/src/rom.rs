use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::Path;

use randomon_core::{OutputFile, RandomizationError, Result};
use randomon_formats::FormatError;
use randomon_formats::ctr::ncch::diagnostics_lines;
use randomon_formats::ctr::{Decompression, GarcArchive, Ncch, container};

pub trait RomSource: Read + Seek + Send {}

impl<T: Read + Seek + Send> RomSource for T {}

pub type BoxedSource = Box<dyn RomSource>;

pub fn format_error(error: FormatError) -> RandomizationError {
    match error {
        FormatError::EncryptedRom(_) => {
            RandomizationError::randomization(format!("EncryptedROMException: {error}"))
        }
        other => RandomizationError::Io(other.to_string()),
    }
}

fn io_error(path: &Path, error: std::io::Error) -> RandomizationError {
    RandomizationError::Io(format!("{}: {error}", path.display()))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CtrIdentity {
    pub product_code: Option<String>,
    pub title_id: Option<String>,
}

pub fn identify<R: Read + Seek>(source: &mut R) -> CtrIdentity {
    let product_code = container::product_code(source).ok().flatten();
    let title_id = container::title_id(source).ok().flatten();
    CtrIdentity {
        product_code,
        title_id,
    }
}

pub fn identify_file(path: &Path) -> CtrIdentity {
    match File::open(path) {
        Ok(file) => identify(&mut BufReader::new(file)),
        Err(_) => CtrIdentity::default(),
    }
}

pub fn identify_bytes(rom: &[u8]) -> CtrIdentity {
    identify(&mut Cursor::new(rom))
}

pub fn open_file_source(path: &Path) -> Result<BoxedSource> {
    let file = File::open(path).map_err(|error| io_error(path, error))?;
    Ok(Box::new(BufReader::new(file)))
}

pub fn bytes_source(rom: Vec<u8>) -> BoxedSource {
    Box::new(Cursor::new(rom))
}

fn open_ncch(mut source: BoxedSource) -> Result<Ncch<BoxedSource>> {
    let identity = identify(&mut source);
    let mut ncch =
        Ncch::open(source, identity.product_code, identity.title_id).map_err(format_error)?;
    if !ncch.is_decrypted().map_err(format_error)? {
        return Err(RandomizationError::randomization(
            "EncryptedROMException: the ROM is encrypted",
        ));
    }
    Ok(ncch)
}

pub struct CtrRom {
    base: Ncch<BoxedSource>,
    update: Option<Ncch<BoxedSource>>,
    update_only_files: BTreeMap<String, Vec<u8>>,
    file_name: String,
}

impl CtrRom {
    pub fn open(source: BoxedSource, file_name: &str) -> Result<Self> {
        Ok(Self {
            base: open_ncch(source)?,
            update: None,
            update_only_files: BTreeMap::new(),
            file_name: file_name.to_owned(),
        })
    }

    pub fn load_update(&mut self, source: BoxedSource) -> Result<i32> {
        let update = open_ncch(source)?;
        let version = update.version();
        self.update = Some(update);
        self.update_only_files.clear();
        Ok(version)
    }

    pub fn has_update(&self) -> bool {
        self.update.is_some()
    }

    pub fn product_code(&self) -> Option<&str> {
        self.base.product_code()
    }

    pub fn title_id(&self) -> Option<&str> {
        self.base.title_id()
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn read_code(&mut self) -> Result<Vec<u8>> {
        match &mut self.update {
            Some(update) => update.get_code(),
            None => self.base.get_code(),
        }
        .map_err(format_error)
    }

    pub fn write_code(&mut self, code: &[u8]) -> Result<()> {
        self.base.write_code(code).map_err(format_error)
    }

    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>> {
        if let Some(update) = &mut self.update
            && update.has_file(path).map_err(format_error)?
        {
            return update
                .get_file(path)
                .map_err(format_error)?
                .ok_or(RandomizationError::NullPointer("readFile"));
        }
        self.base
            .get_file(path)
            .map_err(format_error)?
            .ok_or(RandomizationError::NullPointer("readFile"))
    }

    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<()> {
        let in_base = self.base.has_file(path).map_err(format_error)?;
        self.base.write_file(path, data).map_err(format_error)?;
        if let Some(update) = &mut self.update
            && update.has_file(path).map_err(format_error)?
        {
            update.write_file(path, data).map_err(format_error)?;
            if !in_base {
                self.update_only_files
                    .insert(path.to_owned(), data.to_vec());
            }
        }
        Ok(())
    }

    pub fn read_garc(&mut self, path: &str, decompression: Decompression) -> Result<GarcArchive> {
        let data = self.read_file(path)?;
        GarcArchive::parse(&data, decompression).map_err(format_error)
    }

    pub fn write_garc(&mut self, path: &str, garc: &GarcArchive) -> Result<()> {
        let bytes = garc.to_bytes().map_err(format_error)?;
        self.write_file(path, &bytes)
    }

    pub fn save_as_ncch(&mut self, path: &Path, game_acronym: &str, seed: i64) -> Result<()> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|error| io_error(path, error))?;
        self.base
            .save_as_ncch(file, game_acronym, seed)
            .map_err(format_error)?;
        Ok(())
    }

    pub fn save_as_ncch_bytes(&mut self, game_acronym: &str, seed: i64) -> Result<Vec<u8>> {
        let output = self
            .base
            .save_as_ncch(Cursor::new(Vec::new()), game_acronym, seed)
            .map_err(format_error)?;
        Ok(output.into_inner())
    }

    pub fn layered_fs_files(&mut self) -> Result<Vec<OutputFile>> {
        let title_id = self
            .base
            .title_id()
            .ok_or(RandomizationError::NullPointer("titleId"))?
            .to_owned();
        let entries = self.base.layered_fs_entries().map_err(format_error)?;
        let mut files = vec![OutputFile {
            path: format!("{title_id}/romfs/"),
            bytes: Vec::new(),
        }];
        files.extend(entries.into_iter().map(|(relative, bytes)| OutputFile {
            path: format!("{title_id}/{}", slash_path(&relative)),
            bytes,
        }));
        files.extend(
            self.update_only_files
                .iter()
                .map(|(path, bytes)| OutputFile {
                    path: format!("{title_id}/romfs/{}", path.trim_start_matches('/')),
                    bytes: bytes.clone(),
                }),
        );
        Ok(files)
    }

    pub fn diagnostics(&self) -> Result<Vec<String>> {
        let base = self.base.diagnostics().map_err(format_error)?;
        let update = match &self.update {
            Some(update) => Some(update.diagnostics().map_err(format_error)?),
            None => None,
        };
        let file_name = Path::new(&self.file_name)
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        Ok(diagnostics_lines(&file_name, &base, update.as_ref()))
    }
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
