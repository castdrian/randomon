use std::path::{Path, PathBuf};
use std::sync::Arc;

use randomon_java::{RandomSource, SharedRandomSource};
use randomon_log::LogWriter;
use randomon_settings::{CustomNames, LineSeparator, Settings, default_custom_names_checksum};

use crate::error::{RandomizationError, Result};
use crate::handler::{OutputFile, RomHandler};
use crate::progress::{ProgressPlan, ProgressReporter};
use crate::randomizer::{Randomizer, RomOutput, planned_stages};
use crate::switch::SwitchLayout;
use crate::tweak::{TweakForRomFeedback, tweak_for_rom};

pub trait RomHandlerFactory: Send + Sync {
    fn generation(&self) -> i32;
    fn is_loadable(&self, rom: &[u8]) -> bool;
    fn create(
        &self,
        rom: Vec<u8>,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>>;
    fn is_loadable_file(&self, _path: &Path) -> bool {
        false
    }
    fn detect_file(&self, _path: &Path) -> Option<DetectedGame> {
        None
    }
    fn create_from_file(
        &self,
        path: &Path,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>> {
        self.create(read_rom_file(path)?, file_name, random_source)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectedGame {
    pub generation: i32,
    pub rom_name: String,
    pub rom_code: String,
}

fn read_rom_file(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path)
        .map_err(|error| RandomizationError::Io(format!("{}: {error}", path.display())))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameInfo {
    pub generation: i32,
    pub rom_name: String,
    pub rom_code: String,
    pub support_level: String,
    pub default_extension: String,
    pub is_rom_valid: bool,
}

#[derive(Clone, Debug)]
pub struct RandomizationOptions {
    pub file_name: String,
    pub save_as_directory: bool,
    pub handler_logging: bool,
    pub line_separator: LineSeparator,
    pub custom_names_checksum: Option<i32>,
    pub output_path: Option<PathBuf>,
    pub game_update: Option<PathBuf>,
    pub progress: Option<ProgressReporter>,
    pub switch_layout: SwitchLayout,
}

impl Default for RandomizationOptions {
    fn default() -> Self {
        Self {
            file_name: String::from("rom"),
            save_as_directory: false,
            handler_logging: true,
            line_separator: LineSeparator::platform(),
            custom_names_checksum: None,
            output_path: None,
            game_update: None,
            progress: None,
            switch_layout: SwitchLayout::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RandomizationOutput {
    pub rom: RomOutput,
    pub log: String,
    pub check_value: i32,
    pub settings: Settings,
    pub feedback: TweakForRomFeedback,
}

impl RandomizationOutput {
    pub fn rom_bytes(&self) -> Option<&[u8]> {
        match &self.rom {
            RomOutput::File(bytes) => Some(bytes),
            RomOutput::Directory(_) | RomOutput::Written(_) => None,
        }
    }
}

pub struct HandlerRegistry {
    factories: Vec<Box<dyn RomHandlerFactory>>,
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    pub fn with(mut self, factory: impl RomHandlerFactory + 'static) -> Self {
        self.factories.push(Box::new(factory));
        self
    }

    pub fn register(&mut self, factory: impl RomHandlerFactory + 'static) {
        self.factories.push(Box::new(factory));
    }

    pub fn find(&self, rom: &[u8]) -> Option<&dyn RomHandlerFactory> {
        self.factories
            .iter()
            .find(|factory| factory.is_loadable(rom))
            .map(|factory| factory.as_ref())
    }

    pub fn detect_generation(&self, rom: &[u8]) -> Option<i32> {
        self.find(rom).map(|factory| factory.generation())
    }

    pub fn load(
        &self,
        rom: Vec<u8>,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>> {
        let factory = self
            .find(&rom)
            .ok_or_else(|| RandomizationError::UnsupportedRom(file_name.to_owned()))?;
        factory.create(rom, file_name, random_source)
    }

    pub fn game_info(&self, rom: &[u8], file_name: &str) -> Result<GameInfo> {
        let source = RandomSource::with_seed(0).into_shared();
        let handler = self.load(rom.to_vec(), file_name, &source)?;
        Ok(game_info_of(handler.as_ref()))
    }

    pub fn find_file(&self, path: &Path) -> Option<&dyn RomHandlerFactory> {
        self.factories
            .iter()
            .find(|factory| factory.is_loadable_file(path))
            .map(|factory| factory.as_ref())
    }

    pub fn detect_file(&self, path: &Path) -> Option<DetectedGame> {
        self.factories
            .iter()
            .find_map(|factory| factory.detect_file(path))
    }

    pub fn load_file(
        &self,
        path: &Path,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>> {
        if let Some(factory) = self.find_file(path) {
            return factory.create_from_file(path, file_name, random_source);
        }
        self.load(read_rom_file(path)?, file_name, random_source)
    }

    pub fn game_info_file(&self, path: &Path, file_name: &str) -> Result<GameInfo> {
        let source = RandomSource::with_seed(0).into_shared();
        let handler = self.load_file(path, file_name, &source)?;
        Ok(game_info_of(handler.as_ref()))
    }

    pub fn randomize_file(
        &self,
        path: &Path,
        settings: &Settings,
        seed: i64,
        options: &RandomizationOptions,
    ) -> Result<RandomizationOutput> {
        let random_source = RandomSource::with_seed(seed).into_shared();
        let mut handler = self.load_file(path, &options.file_name, &random_source)?;
        if let Some(update) = &options.game_update {
            handler.load_game_update(update)?;
        }
        randomize_with_handler(handler.as_mut(), &random_source, settings, seed, options)
    }

    pub fn randomize(
        &self,
        rom: &[u8],
        settings: &Settings,
        seed: i64,
        options: &RandomizationOptions,
    ) -> Result<RandomizationOutput> {
        let random_source = RandomSource::with_seed(seed).into_shared();
        let mut handler = self.load(rom.to_vec(), &options.file_name, &random_source)?;
        randomize_with_handler(handler.as_mut(), &random_source, settings, seed, options)
    }
}

pub(crate) fn top_level_directories(files: &[OutputFile]) -> Vec<String> {
    let mut roots: Vec<String> = Vec::new();
    for file in files {
        if let Some((root, _)) = file.path.split_once('/')
            && !roots.iter().any(|known| known == root)
        {
            roots.push(root.to_owned());
        }
    }
    roots
}

fn purge_directory(directory: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

pub fn write_directory_output(root: &Path, files: &[OutputFile]) -> std::io::Result<()> {
    write_directory_output_with_progress(root, files, &|_, _| {})
}

pub fn write_directory_output_with_progress(
    root: &Path,
    files: &[OutputFile],
    on_progress: &dyn Fn(f32, &str),
) -> std::io::Result<()> {
    for directory in top_level_directories(files) {
        let path = root.join(directory);
        if path.is_dir() {
            purge_directory(&path)?;
        }
    }
    let total = files.len().max(1) as f32;
    for (index, file) in files.iter().enumerate() {
        on_progress(index as f32 / total, &format!("Writing {}", file.path));
        let destination = root.join(&file.path);
        if file.path.ends_with('/') {
            std::fs::create_dir_all(&destination)?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&destination, &file.bytes)?;
    }
    on_progress(1.0, "All files written");
    Ok(())
}

fn game_info_of(handler: &dyn RomHandler) -> GameInfo {
    GameInfo {
        generation: handler.generation_of_pokemon(),
        rom_name: handler.rom_name(),
        rom_code: handler.rom_code(),
        support_level: handler.support_level(),
        default_extension: handler.default_extension(),
        is_rom_valid: handler.is_rom_valid(),
    }
}

pub fn randomize_with_handler(
    handler: &mut dyn RomHandler,
    random_source: &SharedRandomSource,
    settings: &Settings,
    seed: i64,
    options: &RandomizationOptions,
) -> Result<RandomizationOutput> {
    let mut settings = settings.clone();
    if settings.custom_names.is_none() {
        settings.custom_names = Some(CustomNames::default_names());
    }
    let feedback = tweak_for_rom(&mut settings, handler);
    let checksum = options
        .custom_names_checksum
        .unwrap_or_else(default_custom_names_checksum);
    let settings_string = settings
        .to_settings_string_with_checksum(checksum)
        .map_err(|error| RandomizationError::randomization(error.to_string()))?;
    {
        let engine = &mut handler.core_mut().engine;
        engine.log = LogWriter::new(options.line_separator.as_str());
        engine.handler_logging = options.handler_logging;
    }
    let is_gen1 = handler.generation_of_pokemon() == 1;
    let progress = Arc::new(ProgressPlan::new(
        planned_stages(&settings, &*handler),
        options.progress.clone(),
    ));
    handler.core_mut().progress = Some(Arc::clone(&progress));
    handler.set_switch_layout(options.switch_layout);
    let save_as_directory = options.save_as_directory || handler.is_switch_game();
    let run = Randomizer {
        settings: &settings,
        handler: &mut *handler,
        random_source,
        save_as_directory,
        settings_string,
        is_gen1,
        output_path: options.output_path.as_deref(),
        progress: &progress,
    }
    .randomize(seed);
    handler.core_mut().progress = None;
    let run = run?;
    let log = std::mem::take(&mut handler.core_mut().engine.log).into_string();
    progress.finish();
    Ok(RandomizationOutput {
        rom: run.rom,
        log,
        check_value: run.check_value,
        settings,
        feedback,
    })
}
