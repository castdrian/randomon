use std::io::Cursor;
use std::path::Path;

use randomon_core::{RandomizationError, Result};
use randomon_formats::FormatError;
use randomon_formats::nds::{NarcArchive, NdsRom};
use randomon_log::LogWriter;

pub fn format_error(error: FormatError) -> RandomizationError {
    RandomizationError::Io(error.to_string())
}

pub struct DsRom {
    rom: NdsRom<Cursor<Vec<u8>>>,
    file_name: String,
    source_length: usize,
}

impl DsRom {
    pub fn open(bytes: Vec<u8>, file_name: &str) -> Result<Self> {
        let source_length = bytes.len();
        let rom = NdsRom::open(Cursor::new(bytes)).map_err(format_error)?;
        Ok(Self {
            rom,
            file_name: file_name.to_owned(),
            source_length,
        })
    }

    pub fn code(&self) -> &str {
        self.rom.code()
    }

    pub fn version(&self) -> i32 {
        i32::from(self.rom.version() as i8)
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>> {
        self.rom
            .get_file(path)
            .map_err(format_error)?
            .ok_or(RandomizationError::NullPointer("NDSRom.getFile"))
    }

    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<()> {
        self.rom.write_file(path, data).map_err(format_error)
    }

    pub fn read_narc(&mut self, path: &str) -> Result<NarcArchive> {
        let data = self.read_file(path)?;
        NarcArchive::parse(&data).map_err(format_error)
    }

    pub fn write_narc(&mut self, path: &str, narc: &NarcArchive) -> Result<()> {
        self.write_file(path, &narc.to_bytes())
    }

    pub fn read_arm9(&mut self) -> Result<Vec<u8>> {
        self.rom.get_arm9().map_err(format_error)
    }

    pub fn write_arm9(&mut self, data: &[u8]) -> Result<()> {
        self.rom.write_arm9(data).map_err(format_error)
    }

    pub fn read_overlay(&mut self, number: i32) -> Result<Vec<u8>> {
        self.rom
            .get_overlay(number)
            .map_err(format_error)?
            .ok_or(RandomizationError::NullPointer("NDSRom.getOverlay"))
    }

    pub fn write_overlay(&mut self, number: i32, data: &[u8]) -> Result<()> {
        self.rom.write_overlay(number, data).map_err(format_error)
    }

    pub fn save(&mut self) -> Result<Vec<u8>> {
        let output = Cursor::new(Vec::with_capacity(self.source_length));
        self.rom
            .save_to(output)
            .map(Cursor::into_inner)
            .map_err(format_error)
    }

    pub fn print_diagnostics(&self, log: &mut LogWriter) {
        let short_name = Path::new(&self.file_name).file_name().map_or_else(
            || self.file_name.clone(),
            |name| name.to_string_lossy().into_owned(),
        );
        for line in self.rom.diagnostics(&short_name) {
            log.println(&line);
        }
    }
}
