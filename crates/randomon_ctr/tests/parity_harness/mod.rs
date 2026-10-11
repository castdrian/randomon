pub mod grid;

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use randomon_core::{HandlerRegistry, OutputFile, RandomizationOptions, RomOutput};
use randomon_settings::{LineSeparator, Settings};

pub const FILTER_ENV: &str = "RANDOMON_PARITY_FILTER";
pub const SEEDS_ENV: &str = "RANDOMON_PARITY_SEEDS";
pub const REUSE_ENV: &str = "RANDOMON_PARITY_REUSE";
pub const WORKERS_ENV: &str = "RANDOMON_PARITY_WORKERS";
pub const FULL_ROM_ENV: &str = "RANDOMON_PARITY_FULL_ROM";
pub const HEAP_ENV: &str = "RANDOMON_PARITY_JAVA_HEAP";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputKind {
    Directory,
    FullRom,
}

#[derive(Clone, Debug)]
pub struct Job {
    pub id: String,
    pub seed: i64,
    pub versioned: bool,
    pub settings_text: String,
    pub output: OutputKind,
    pub with_update: bool,
}

pub struct Case {
    pub name: String,
    pub settings: Settings,
}

pub struct ParityRun<'a> {
    pub label: &'a str,
    pub registry: HandlerRegistry,
    pub rom: PathBuf,
    pub update: Option<PathBuf>,
    pub cases: Vec<Case>,
    pub seeds: Vec<i64>,
    pub update_seeds: Vec<i64>,
    pub full_rom_cases: Vec<String>,
}

#[derive(Default)]
struct Tally {
    identical: usize,
    both_failed: usize,
    failures: Vec<String>,
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root exists")
}

fn ensure_java_runner(root: &Path) {
    if root
        .join("target/parity/classes/ParityRunner.class")
        .exists()
    {
        return;
    }
    let status = Command::new("go")
        .args(["run", "./cmd/parity-build"])
        .current_dir(root.join("tools"))
        .status()
        .expect("go runs the parity build");
    assert!(status.success(), "building the Java parity runner failed");
}

pub fn mask_timing(log: &str) -> String {
    log.lines()
        .map(|line| {
            if line.starts_with("Time elapsed: ") {
                "Time elapsed: <masked>"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn env_seeds(default: &[i64]) -> Vec<i64> {
    std::env::var(SEEDS_ENV)
        .ok()
        .map(|text| {
            text.split(',')
                .filter_map(|seed| seed.trim().parse().ok())
                .collect()
        })
        .unwrap_or_else(|| default.to_vec())
}

fn settings_text(settings: &Settings, versioned: bool) -> String {
    if versioned {
        settings.to_versioned_settings_string()
    } else {
        settings.to_settings_string()
    }
    .expect("grid settings encode")
}

pub fn parse_job_settings(job: &Job) -> Settings {
    if job.versioned {
        Settings::from_versioned_settings_string(&job.settings_text)
            .expect("versioned settings decode")
            .settings
    } else {
        Settings::from_settings_string(&job.settings_text).expect("settings decode")
    }
}

impl ParityRun<'_> {
    fn build_jobs(&self) -> Vec<Job> {
        let filter = std::env::var(FILTER_ENV).ok();
        let seeds = env_seeds(&self.seeds);
        let full_rom_only = std::env::var(FULL_ROM_ENV).is_ok();
        let mut jobs = Vec::new();
        for case in &self.cases {
            if filter
                .as_deref()
                .is_some_and(|filter| !case.name.contains(filter))
            {
                continue;
            }
            let full_rom = self.full_rom_cases.contains(&case.name);
            if !full_rom_only {
                for (index, &seed) in seeds.iter().enumerate() {
                    jobs.push(self.job(case, index, seed, OutputKind::Directory, false));
                }
                if self.update.is_some() {
                    for (index, &seed) in self.update_seeds.iter().enumerate() {
                        jobs.push(self.job(case, index, seed, OutputKind::Directory, true));
                    }
                }
            }
            if full_rom {
                jobs.push(self.job(case, 0, seeds[0], OutputKind::FullRom, false));
            }
        }
        jobs
    }

    fn job(
        &self,
        case: &Case,
        index: usize,
        seed: i64,
        output: OutputKind,
        with_update: bool,
    ) -> Job {
        let versioned = index % 2 == 1;
        let suffix = match (output, with_update) {
            (OutputKind::FullRom, _) => "full",
            (OutputKind::Directory, true) => "upd",
            (OutputKind::Directory, false) => "dir",
        };
        Job {
            id: format!("{}__{suffix}{index}", case.name),
            seed,
            versioned,
            settings_text: settings_text(&case.settings, versioned),
            output,
            with_update,
        }
    }

    pub fn run(self) {
        let root = repo_root();
        ensure_java_runner(&root);
        let work = root.join(format!("target/parity/{}", self.label));
        let reuse = std::env::var(REUSE_ENV).is_ok();
        if !reuse {
            let _ = fs::remove_dir_all(&work);
        }
        let java_out = work.join("java");
        let rust_out = work.join("rust");
        fs::create_dir_all(&java_out).expect("java output directory");
        fs::create_dir_all(&rust_out).expect("rust output directory");
        let jobs = self.build_jobs();
        let workers: usize = std::env::var(WORKERS_ENV)
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(3);
        let next = AtomicUsize::new(0);
        let tally = Mutex::new(Tally::default());
        std::thread::scope(|scope| {
            for worker in 0..workers {
                let (jobs, next, tally) = (&jobs, &next, &tally);
                let (root, work, java_out, rust_out) = (&root, &work, &java_out, &rust_out);
                let this = &self;
                scope.spawn(move || {
                    let classes = worker_classes(root, work, worker);
                    loop {
                        let index = next.fetch_add(1, Ordering::SeqCst);
                        let Some(job) = jobs.get(index) else {
                            break;
                        };
                        let outcome =
                            this.run_job(root, &classes, work, java_out, rust_out, job, reuse);
                        record(tally, &job.id, outcome);
                    }
                });
            }
        });
        let tally = tally.into_inner().expect("tally");
        let summary = format!(
            "{} jobs, {} identical successes, {} matching failures, {} mismatches",
            jobs.len(),
            tally.identical,
            tally.both_failed,
            tally.failures.len()
        );
        let mut failures = tally.failures.clone();
        failures.sort();
        fs::write(
            work.join("summary.txt"),
            format!("{summary}\n{}\n", failures.join("\n")),
        )
        .expect("summary");
        eprintln!("{summary}");
        for failure in failures.iter().take(60) {
            eprintln!("  {failure}");
        }
        assert!(failures.is_empty(), "{summary}");
    }

    #[allow(clippy::too_many_arguments)]
    fn run_job(
        &self,
        root: &Path,
        classes: &Path,
        work: &Path,
        java_out: &Path,
        rust_out: &Path,
        job: &Job,
        reuse: bool,
    ) -> Outcome {
        let java_rom = java_out.join(format!("{}.rom", job.id));
        let finished = java_out.join(format!("{}.cv", job.id)).exists()
            || java_out.join(format!("{}.err", job.id)).exists();
        if !(reuse && finished) {
            run_java(
                root,
                classes,
                work,
                &self.rom,
                self.update_for(job),
                java_out,
                job,
            );
        }
        let java_error = fs::read_to_string(java_out.join(format!("{}.err", job.id))).ok();
        let rust_rom = rust_out.join(format!("{}.cxi", job.id));
        let outcome = self.compare_job(job, java_error, java_out, &java_rom, &rust_rom, rust_out);
        if job.output == OutputKind::FullRom {
            let _ = fs::remove_file(&java_rom);
            let _ = fs::remove_file(&rust_rom);
        }
        if matches!(outcome, Outcome::Identical | Outcome::BothFailed) {
            let _ = fs::remove_dir_all(&java_rom);
        }
        outcome
    }

    fn update_for(&self, job: &Job) -> Option<&Path> {
        if job.with_update {
            self.update.as_deref()
        } else {
            None
        }
    }

    fn compare_job(
        &self,
        job: &Job,
        java_error: Option<String>,
        java_out: &Path,
        java_rom: &Path,
        rust_rom: &Path,
        rust_out: &Path,
    ) -> Outcome {
        let settings = parse_job_settings(job);
        let options = RandomizationOptions {
            file_name: self.rom.to_string_lossy().into_owned(),
            line_separator: LineSeparator::Lf,
            save_as_directory: job.output == OutputKind::Directory,
            output_path: (job.output == OutputKind::FullRom).then(|| rust_rom.to_path_buf()),
            game_update: self.update_for(job).map(Path::to_path_buf),
            ..RandomizationOptions::default()
        };
        let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.registry
                .randomize_file(&self.rom, &settings, job.seed, &options)
        }));
        let Ok(rust) = rust else {
            return Outcome::Mismatch("Randomon panicked".to_owned());
        };
        match (java_error, rust) {
            (Some(_), Err(_)) => Outcome::BothFailed,
            (Some(error), Ok(_)) => Outcome::Mismatch(format!(
                "Java failed but Randomon succeeded: {}",
                error.lines().next().unwrap_or_default()
            )),
            (None, Err(error)) => Outcome::Mismatch(format!("Randomon failed: {error}")),
            (None, Ok(output)) => {
                let mut problems = Vec::new();
                match &output.rom {
                    RomOutput::Directory(files) => {
                        problems.extend(compare_directory(java_rom, files));
                    }
                    RomOutput::Written(path) => {
                        problems.extend(compare_files(java_rom, path));
                    }
                    RomOutput::File(bytes) => {
                        problems.extend(compare_bytes(java_rom, bytes));
                    }
                }
                let java_log = fs::read_to_string(java_out.join(format!("{}.log", job.id)))
                    .unwrap_or_default();
                let rust_log = mask_timing(&output.log);
                let java_masked = mask_timing(&java_log);
                if rust_log != java_masked {
                    let line = rust_log
                        .lines()
                        .zip(java_masked.lines())
                        .position(|(a, b)| a != b)
                        .map_or_else(|| "length".to_owned(), |line| (line + 1).to_string());
                    problems.push(format!("log differs (first at line {line})"));
                    let _ = fs::write(rust_out.join(format!("{}.log", job.id)), &output.log);
                }
                let java_cv: Option<i32> =
                    fs::read_to_string(java_out.join(format!("{}.cv", job.id)))
                        .ok()
                        .and_then(|text| text.trim().parse().ok());
                if Some(output.check_value) != java_cv {
                    problems.push(format!("check value {} != {java_cv:?}", output.check_value));
                }
                if problems.is_empty() {
                    Outcome::Identical
                } else {
                    Outcome::Mismatch(problems.join("; "))
                }
            }
        }
    }
}

enum Outcome {
    Identical,
    BothFailed,
    Mismatch(String),
}

fn record(tally: &Mutex<Tally>, id: &str, outcome: Outcome) {
    let mut tally = tally.lock().expect("tally");
    match outcome {
        Outcome::Identical => tally.identical += 1,
        Outcome::BothFailed => tally.both_failed += 1,
        Outcome::Mismatch(problem) => {
            eprintln!("MISMATCH {id}: {problem}");
            tally.failures.push(format!("{id}: {problem}"));
        }
    }
}

fn worker_classes(root: &Path, work: &Path, worker: usize) -> PathBuf {
    let directory = work.join(format!("workers/w{worker}"));
    fs::create_dir_all(&directory).expect("worker directory");
    let classes = directory.join("classes");
    if !classes.exists() {
        let status = Command::new("cp")
            .arg("-R")
            .arg(root.join("target/parity/classes"))
            .arg(&classes)
            .status()
            .expect("cp runs");
        assert!(status.success(), "copying the Java classes failed");
    }
    classes
}

fn run_java(
    root: &Path,
    classes: &Path,
    work: &Path,
    rom: &Path,
    update: Option<&Path>,
    java_out: &Path,
    job: &Job,
) {
    let jobs_dir = work.join("jobs");
    fs::create_dir_all(&jobs_dir).expect("jobs directory");
    let jobs_file = jobs_dir.join(format!("{}.tsv", job.id));
    let mode = if job.versioned { "versioned" } else { "raw" };
    let output = match job.output {
        OutputKind::Directory => "dir",
        OutputKind::FullRom => "file",
    };
    fs::write(
        &jobs_file,
        format!(
            "{}\t{}\t{mode}\t{}\t{output}\n",
            job.id, job.seed, job.settings_text
        ),
    )
    .expect("job file");
    let classpath = format!(
        "{}:{}",
        classes.display(),
        upstream_root(root).join("src").display()
    );
    let heap = format!(
        "-Xmx{}",
        std::env::var(HEAP_ENV).unwrap_or_else(|_| "4g".to_owned())
    );
    let mut command = Command::new("java");
    command
        .current_dir(classes.parent().expect("worker directory"))
        .arg(&heap)
        .args([
            "-Djava.awt.headless=true",
            "-Dline.separator=\n",
            "-Dfile.encoding=UTF-8",
            "-cp",
            &classpath,
            "ParityRunner",
        ])
        .arg(rom)
        .arg(&jobs_file)
        .arg(java_out);
    if let Some(update) = update {
        command.arg(update);
    }
    for _ in 0..2 {
        let console =
            fs::File::create(jobs_dir.join(format!("{}.out", job.id))).expect("console file");
        command.stdout(console);
        if command.status().expect("java runs").success() {
            return;
        }
    }
    fs::write(
        java_out.join(format!("{}.err", job.id)),
        "the Java process itself exited abnormally",
    )
    .expect("error file");
}

fn collect_files(root: &Path, directory: &Path, files: &mut BTreeMap<String, PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("inside root")
                .components()
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.insert(relative, path);
        }
    }
}

fn compare_directory(java_root: &Path, rust_files: &[OutputFile]) -> Vec<String> {
    let mut java_files = BTreeMap::new();
    collect_files(java_root, java_root, &mut java_files);
    let mut problems = Vec::new();
    let mut rust_names = Vec::new();
    for file in rust_files {
        if let Some(directory) = file.path.strip_suffix('/') {
            if !java_root.join(directory).is_dir() {
                problems.push(format!("Java has no directory {directory}"));
            }
            continue;
        }
        rust_names.push(file.path.clone());
        match java_files.get(&file.path) {
            None => problems.push(format!("only Randomon wrote {}", file.path)),
            Some(path) => {
                let java_bytes = fs::read(path).expect("java output file");
                if java_bytes != file.bytes {
                    problems.push(describe_difference(&file.path, &java_bytes, &file.bytes));
                }
            }
        }
    }
    for name in java_files.keys() {
        if !rust_names.contains(name) {
            problems.push(format!("only Java wrote {name}"));
        }
    }
    problems
}

fn describe_difference(name: &str, java: &[u8], rust: &[u8]) -> String {
    let first = java.iter().zip(rust).position(|(a, b)| a != b);
    format!(
        "{name} differs (java {} bytes, rust {} bytes, first difference at {first:X?})",
        java.len(),
        rust.len()
    )
}

fn compare_bytes(java_rom: &Path, rust: &[u8]) -> Vec<String> {
    let java = fs::read(java_rom).unwrap_or_default();
    if java == rust {
        Vec::new()
    } else {
        vec![describe_difference("ROM", &java, rust)]
    }
}

fn compare_files(java_rom: &Path, rust_rom: &Path) -> Vec<String> {
    let (Ok(java), Ok(rust)) = (fs::File::open(java_rom), fs::File::open(rust_rom)) else {
        return vec!["missing full ROM output".to_owned()];
    };
    let mut java = BufReader::with_capacity(1 << 22, java);
    let mut rust = BufReader::with_capacity(1 << 22, rust);
    let mut java_chunk = vec![0u8; 1 << 22];
    let mut rust_chunk = vec![0u8; 1 << 22];
    let mut position = 0u64;
    loop {
        let java_read = read_full(&mut java, &mut java_chunk);
        let rust_read = read_full(&mut rust, &mut rust_chunk);
        if java_read != rust_read || java_chunk[..java_read] != rust_chunk[..rust_read] {
            let offset = java_chunk[..java_read.min(rust_read)]
                .iter()
                .zip(&rust_chunk[..rust_read])
                .position(|(a, b)| a != b)
                .unwrap_or(java_read.min(rust_read));
            return vec![format!(
                "full ROM differs at {:X}",
                position + offset as u64
            )];
        }
        if java_read == 0 {
            return Vec::new();
        }
        position += java_read as u64;
    }
}

fn read_full(reader: &mut impl Read, buffer: &mut [u8]) -> usize {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) | Err(_) => break,
            Ok(read) => filled += read,
        }
    }
    filled
}

fn upstream_root(root: &std::path::Path) -> std::path::PathBuf {
    std::env::var_os("RANDOMON_UPSTREAM")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target/upstream"))
}
