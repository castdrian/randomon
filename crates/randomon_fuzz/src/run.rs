use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use randomon_core::{
    HandlerRegistry, RandomizationOptions, RomHandler, RomOutput, randomize_with_handler,
};
use randomon_gen1::Gen1Factory;
use randomon_gen2::Gen2Factory;
use randomon_gen5::Gen5Factory;
use randomon_gen6::Gen6Factory;
use randomon_gen7::Gen7Factory;
use randomon_java::{RandomSource, SharedRandomSource};
use randomon_settings::Settings;

use crate::dice::Dice;
use crate::settings_space::{GameShape, random_settings};
use crate::validate::{Baseline, validate};

pub fn registry() -> HandlerRegistry {
    HandlerRegistry::new()
        .with(Gen1Factory)
        .with(Gen2Factory)
        .with(Gen5Factory)
        .with(Gen6Factory)
        .with(Gen7Factory)
}

#[derive(Clone, Debug)]
pub struct Target {
    pub rom: PathBuf,
    pub update: Option<PathBuf>,
    pub rom_bytes: Option<Arc<Vec<u8>>>,
}

impl Target {
    pub fn new(rom: PathBuf, update: Option<PathBuf>) -> Self {
        let streamed = registry().find_file(&rom).is_some();
        let rom_bytes = (!streamed)
            .then(|| std::fs::read(&rom).ok().map(Arc::new))
            .flatten();
        Self {
            rom,
            update,
            rom_bytes,
        }
    }

    fn load(
        &self,
        registry: &HandlerRegistry,
        source: &SharedRandomSource,
    ) -> Result<Box<dyn RomHandler>, String> {
        let name = self.rom.to_string_lossy().into_owned();
        let mut handler = match &self.rom_bytes {
            Some(bytes) => registry.load(bytes.to_vec(), &name, source),
            None => registry.load_file(&self.rom, &name, source),
        }
        .map_err(|error| format!("load: {error}"))?;
        if let Some(update) = &self.update {
            handler
                .load_game_update(update)
                .map_err(|error| format!("update: {error}"))?;
        }
        Ok(handler)
    }
}

#[derive(Clone, Debug)]
pub struct CaseResult {
    pub case: u64,
    pub seed: i64,
    pub settings_string: String,
    pub problems: Vec<String>,
}

pub fn case_settings(case: u64, rom_name: &str, shape: GameShape) -> (Settings, i64) {
    let mut dice = Dice::new(case.wrapping_add(0x5EED));
    let settings = random_settings(&mut dice, rom_name, shape);
    let seed = dice.seed();
    (settings, seed)
}

fn describe_panic(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .unwrap_or_else(|| "panic".to_owned())
}

pub fn run_case(target: &Target, case: u64, seeds_per_case: u64) -> Vec<CaseResult> {
    (0..seeds_per_case)
        .map(|seed_index| {
            let registry = registry();
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                run_one(&registry, target, case, seed_index)
            }));
            match outcome {
                Ok(result) => result,
                Err(payload) => CaseResult {
                    case,
                    seed: seed_index as i64,
                    settings_string: String::new(),
                    problems: vec![format!("panic: {}", describe_panic(payload))],
                },
            }
        })
        .collect()
}

fn settings_string_of(settings: &Settings) -> String {
    settings
        .to_settings_string_with_checksum(randomon_settings::default_custom_names_checksum())
        .map(|text| format!("{}{text}", randomon_settings::VERSION))
        .unwrap_or_default()
}

fn run_one(registry: &HandlerRegistry, target: &Target, case: u64, seed_index: u64) -> CaseResult {
    let source = RandomSource::with_seed(0).into_shared();
    let mut result = CaseResult {
        case,
        seed: 0,
        settings_string: String::new(),
        problems: Vec::new(),
    };
    let mut handler = match target.load(registry, &source) {
        Ok(handler) => handler,
        Err(problem) => {
            result.problems.push(problem);
            return result;
        }
    };
    let shape = GameShape {
        generation: handler.generation_of_pokemon(),
        species_count: handler.get_pokemon().len().saturating_sub(1) as i32,
    };
    let (settings, base_seed) = case_settings(case, &handler.rom_name(), shape);
    let seed = base_seed.wrapping_add(seed_index as i64 * 7919);
    result.seed = seed;
    result.settings_string = settings_string_of(&settings);
    let baseline = match Baseline::capture(handler.as_ref()) {
        Ok(baseline) => baseline,
        Err(problem) => {
            result.problems.push(problem);
            return result;
        }
    };
    let streamed = target.rom_bytes.is_none();
    let full_image = full_image_path(target, case, seed_index);
    let options = RandomizationOptions {
        file_name: target.rom.to_string_lossy().into_owned(),
        save_as_directory: streamed && full_image.is_none(),
        output_path: full_image.clone(),
        ..RandomizationOptions::default()
    };
    let output = match randomize_with_handler(handler.as_mut(), &source, &settings, seed, &options)
    {
        Ok(output) => output,
        Err(error) => {
            result.problems.push(format!("randomize: {error}"));
            return result;
        }
    };
    let effective = output.settings.clone();
    result.problems.extend(
        validate(handler.as_ref(), &effective, &baseline)
            .into_iter()
            .map(|p| format!("in memory: {p}")),
    );
    match &output.rom {
        RomOutput::File(bytes) => {
            let reload_source = RandomSource::with_seed(0).into_shared();
            match registry.load(bytes.clone(), &options.file_name, &reload_source) {
                Ok(reloaded) => result.problems.extend(
                    validate(reloaded.as_ref(), &effective, &baseline)
                        .into_iter()
                        .map(|p| format!("reloaded: {p}")),
                ),
                Err(error) => result.problems.push(format!("reload: {error}")),
            }
        }
        RomOutput::Directory(files) => {
            if files.is_empty() {
                result.problems.push("LayeredFS output is empty".to_owned());
            }
        }
        RomOutput::Written(path) => {
            let reload_source = RandomSource::with_seed(0).into_shared();
            match registry.load_file(path, &options.file_name, &reload_source) {
                Ok(reloaded) => result.problems.extend(
                    validate(reloaded.as_ref(), &effective, &baseline)
                        .into_iter()
                        .map(|p| format!("reloaded image: {p}")),
                ),
                Err(error) => result.problems.push(format!("reload image: {error}")),
            }
        }
    }
    if let Some(path) = &full_image {
        let _ = std::fs::remove_file(path);
    }
    result
}

fn full_image_path(target: &Target, case: u64, seed_index: u64) -> Option<PathBuf> {
    if target.rom_bytes.is_some() || target.update.is_some() || seed_index != 0 {
        return None;
    }
    let every: u64 = std::env::var("RANDOMON_FUZZ_FULL_IMAGE_EVERY")
        .ok()
        .and_then(|text| text.parse().ok())
        .filter(|&every| every > 0)?;
    case.is_multiple_of(every).then(|| {
        let directory = std::env::var_os("RANDOMON_FUZZ_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        directory.join(format!("randomon-fuzz-{case}.cxi"))
    })
}

pub struct FuzzPlan {
    pub cases: u64,
    pub first_case: u64,
    pub seeds_per_case: u64,
    pub threads: usize,
    pub case_timeout: Duration,
}

impl FuzzPlan {
    pub fn from_env(default_cases: u64, default_threads: usize) -> Self {
        let number = |name: &str, default: u64| {
            std::env::var(name)
                .ok()
                .and_then(|text| text.parse().ok())
                .unwrap_or(default)
        };
        Self {
            cases: number("RANDOMON_FUZZ_CASES", default_cases),
            first_case: number("RANDOMON_FUZZ_FIRST_CASE", 0),
            seeds_per_case: number("RANDOMON_FUZZ_SEEDS", 2),
            threads: number("RANDOMON_FUZZ_THREADS", default_threads as u64) as usize,
            case_timeout: Duration::from_secs(number("RANDOMON_FUZZ_TIMEOUT_SECS", 600)),
        }
    }
}

pub struct FuzzSummary {
    pub runs: usize,
    pub failures: Vec<CaseResult>,
}

pub fn fuzz(target: &Target, plan: &FuzzPlan) -> FuzzSummary {
    let next = Arc::new(AtomicUsize::new(0));
    let failures = Arc::new(Mutex::new(Vec::new()));
    let runs = Arc::new(AtomicUsize::new(0));
    let (heartbeat, beats) = mpsc::channel::<(usize, Option<u64>)>();
    let mut workers = Vec::new();
    for worker in 0..plan.threads.max(1) {
        let target = target.clone();
        let next = Arc::clone(&next);
        let failures = Arc::clone(&failures);
        let runs = Arc::clone(&runs);
        let heartbeat = heartbeat.clone();
        let cases = plan.cases;
        let first_case = plan.first_case;
        let seeds = plan.seeds_per_case;
        workers.push(std::thread::spawn(move || {
            loop {
                let index = next.fetch_add(1, Ordering::SeqCst) as u64;
                if index >= cases {
                    break;
                }
                let case = first_case + index;
                let _ = heartbeat.send((worker, Some(case)));
                for result in run_case(&target, case, seeds) {
                    runs.fetch_add(1, Ordering::SeqCst);
                    if !result.problems.is_empty() {
                        eprintln!(
                            "FAIL case {} seed {} settings {}\n  {}",
                            result.case,
                            result.seed,
                            result.settings_string,
                            result.problems.join("\n  ")
                        );
                        if let Ok(mut list) = failures.lock() {
                            list.push(result);
                        }
                    }
                }
                let _ = heartbeat.send((worker, None));
            }
        }));
    }
    drop(heartbeat);
    watch(beats, plan.case_timeout);
    for worker in workers {
        let _ = worker.join();
    }
    let failures = failures.lock().map(|list| list.clone()).unwrap_or_default();
    FuzzSummary {
        runs: runs.load(Ordering::SeqCst),
        failures,
    }
}

fn watch(beats: mpsc::Receiver<(usize, Option<u64>)>, timeout: Duration) {
    let mut busy: std::collections::HashMap<usize, (u64, Instant)> = Default::default();
    loop {
        match beats.recv_timeout(Duration::from_secs(5)) {
            Ok((worker, Some(case))) => {
                busy.insert(worker, (case, Instant::now()));
            }
            Ok((worker, None)) => {
                busy.remove(&worker);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if let Some((case, _)) = busy
            .values()
            .find(|(_, started)| started.elapsed() > timeout)
        {
            eprintln!("HANG case {case} exceeded {timeout:?}");
            std::process::exit(2);
        }
    }
}

pub fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| Path::new(path).exists())
}
