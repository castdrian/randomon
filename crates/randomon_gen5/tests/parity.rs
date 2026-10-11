mod grid;

use std::fs;
use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use randomon_core::{HandlerRegistry, RandomizationOptions};
use randomon_formats::nds::NdsRom;
use randomon_formats::sha256::sha256;
use randomon_gen5::Gen5Factory;
use randomon_settings::{LineSeparator, Settings};

const ROM_ENV: &str = "RANDOMON_PARITY_ROM";
const FILTER_ENV: &str = "RANDOMON_PARITY_FILTER";
const SEEDS_ENV: &str = "RANDOMON_PARITY_SEEDS";
const REUSE_ENV: &str = "RANDOMON_PARITY_REUSE";
const KEEP_ENV: &str = "RANDOMON_PARITY_KEEP";
const FUZZ_ENV: &str = "RANDOMON_PARITY_FUZZ";
const JAVA_WORKERS_ENV: &str = "RANDOMON_PARITY_JAVA_WORKERS";
const RUST_WORKERS_ENV: &str = "RANDOMON_PARITY_RUST_WORKERS";
const DEFAULT_JAVA_WORKERS: usize = 4;
const DEFAULT_RUST_WORKERS: usize = 3;
const KEPT_RUST_ROMS: usize = 2;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root exists")
}

fn modified(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn ensure_java_runner(root: &Path) {
    let class = root.join("target/parity/classes/ParityRunner.class");
    let source = root.join("tools/parity/java/ParityRunner.java");
    if modified(&class).is_some() && modified(&class) >= modified(&source) {
        return;
    }
    let status = Command::new(root.join("tools/parity/build.sh"))
        .status()
        .expect("build.sh runs");
    assert!(status.success(), "building the Java parity runner failed");
}

fn env_count(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(default)
}

fn mask_timing(log: &str) -> String {
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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct Job {
    id: String,
    seed: i64,
    mode: &'static str,
    settings_text: String,
    settings: Settings,
}

fn build_jobs(rom_name: &str) -> Vec<Job> {
    let filter = std::env::var(FILTER_ENV).ok();
    let seeds: Vec<i64> = std::env::var(SEEDS_ENV)
        .ok()
        .map(|text| {
            text.split(',')
                .filter_map(|seed| seed.trim().parse().ok())
                .collect()
        })
        .unwrap_or_else(|| grid::SEEDS.to_vec());
    let cases = grid::settings_grid(rom_name)
        .into_iter()
        .chain(grid::fuzz_grid(
            rom_name,
            env_count(FUZZ_ENV, grid::DEFAULT_FUZZ_CASES),
        ));
    let mut jobs = Vec::new();
    for (name, settings) in cases {
        if filter
            .as_deref()
            .is_some_and(|filter| !filter.split(',').any(|part| name.contains(part)))
        {
            continue;
        }
        let raw = settings.to_settings_string().expect("grid settings encode");
        for (index, &seed) in seeds.iter().enumerate() {
            let versioned = index % 2 == 1;
            let settings_text = if versioned {
                settings
                    .to_versioned_settings_string()
                    .expect("grid settings encode")
            } else {
                raw.clone()
            };
            jobs.push(Job {
                id: format!("{name}__{index}"),
                seed,
                mode: if versioned { "versioned" } else { "raw" },
                settings_text,
                settings: settings.clone(),
            });
        }
    }
    jobs
}

fn parse_job_settings(job: &Job) -> Settings {
    if job.mode == "versioned" {
        Settings::from_versioned_settings_string(&job.settings_text)
            .expect("versioned settings decode")
            .settings
    } else {
        Settings::from_settings_string(&job.settings_text).expect("settings decode")
    }
}

fn job_line(job: &Job) -> String {
    format!(
        "{}\t{}\t{}\t{}",
        job.id, job.seed, job.mode, job.settings_text
    )
}

#[test]
fn gen5_matches_upr_zx() {
    let Ok(rom_path) = std::env::var(ROM_ENV) else {
        eprintln!("{ROM_ENV} is not set; skipping the Gen 5 parity grid");
        return;
    };
    let root = repo_root();
    ensure_java_runner(&root);
    let rom = fs::read(&rom_path).expect("ROM is readable");
    let registry = HandlerRegistry::new().with(Gen5Factory);
    let info = registry
        .game_info(&rom, &rom_path)
        .expect("ROM is a supported Gen 5 game");
    let jobs = build_jobs(&info.rom_name);
    for job in &jobs {
        assert_eq!(
            parse_job_settings(job).to_settings_string().ok(),
            job.settings.to_settings_string().ok(),
            "settings round trip for {}",
            job.id
        );
    }
    let work = root.join("target/parity/gen5");
    let reuse = std::env::var(REUSE_ENV).is_ok();
    if !reuse {
        let _ = fs::remove_dir_all(&work);
    }
    let java_out = work.join("java");
    fs::create_dir_all(&java_out).expect("work directory");
    let pending: Vec<&Job> = jobs
        .iter()
        .filter(|job| !reuse || !java_finished(&java_out, &job.id))
        .collect();
    run_java(&root, &work, &rom_path, &pending, &java_out);
    let options = RandomizationOptions {
        file_name: rom_path.clone(),
        line_separator: LineSeparator::Lf,
        ..RandomizationOptions::default()
    };
    compare_all(&registry, &rom, &jobs, &java_out, &work, &options);
}

fn java_finished(java_out: &Path, id: &str) -> bool {
    java_out.join(format!("{id}.err")).exists() || java_out.join(format!("{id}.cv")).exists()
}

fn worker_classes(root: &Path, work: &Path, worker: usize) -> PathBuf {
    let worker_directory = work.join("jvm").join(format!("worker{worker}"));
    let classes = worker_directory.join("classes");
    let _ = fs::remove_dir_all(&worker_directory);
    fs::create_dir_all(&worker_directory).expect("worker directory");
    let status = Command::new("cp")
        .arg("-R")
        .arg(root.join("target/parity/classes"))
        .arg(&classes)
        .status()
        .expect("cp runs");
    assert!(status.success(), "copying the parity runner classes failed");
    classes
}

fn run_java(root: &Path, work: &Path, rom_path: &str, jobs: &[&Job], java_out: &Path) {
    let keep = std::env::var(KEEP_ENV).is_ok();
    let next = AtomicUsize::new(0);
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for worker in 0..env_count(JAVA_WORKERS_ENV, DEFAULT_JAVA_WORKERS) {
            let next = &next;
            let failures = &failures;
            scope.spawn(move || {
                let classpath = format!(
                    "{}:{}",
                    worker_classes(root, work, worker).display(),
                    upstream_root(root).join("src").display()
                );
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(job) = jobs.get(index) else {
                        return;
                    };
                    let job_directory = work.join("jvm").join(&job.id);
                    let _ = fs::remove_dir_all(&job_directory);
                    fs::create_dir_all(&job_directory).expect("job directory");
                    let job_file = job_directory.join("job.tsv");
                    fs::write(&job_file, job_line(job)).expect("job file");
                    let status = Command::new("java")
                        .current_dir(&job_directory)
                        .arg("-Xmx2g")
                        .arg("-Djava.awt.headless=true")
                        .arg("-Dline.separator=\n")
                        .arg("-Dfile.encoding=UTF-8")
                        .arg("-Dparity.digest=true")
                        .arg(format!("-Dparity.keepRom={keep}"))
                        .arg("-cp")
                        .arg(&classpath)
                        .arg("ParityRunner")
                        .arg(rom_path)
                        .arg(&job_file)
                        .arg(java_out)
                        .status()
                        .expect("java runs");
                    let _ = fs::remove_dir_all(&job_directory);
                    if !status.success() {
                        failures.lock().expect("lock").push(job.id.clone());
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("lock");
    assert!(
        failures.is_empty(),
        "Java parity runner failed for {failures:?}"
    );
}

enum Outcome {
    Identical,
    BothFailed,
    Mismatch(String),
}

fn differing_files(rust_rom: &[u8], java_rom: &[u8]) -> String {
    let (Ok(mut rust), Ok(mut java)) = (
        NdsRom::open(Cursor::new(rust_rom.to_vec())),
        NdsRom::open(Cursor::new(java_rom.to_vec())),
    ) else {
        return "unparseable ROM".to_owned();
    };
    let mut differing = Vec::new();
    let paths: Vec<String> = rust
        .files()
        .iter()
        .map(|file| file.full_path.clone())
        .collect();
    for path in paths {
        if rust.get_file(&path).ok() != java.get_file(&path).ok() {
            differing.push(path);
        }
    }
    for overlay in 0..rust.overlays().len() as i32 {
        if rust.get_overlay(overlay).ok() != java.get_overlay(overlay).ok() {
            differing.push(format!("overlay {overlay}"));
        }
    }
    if rust.get_arm9().ok() != java.get_arm9().ok() {
        differing.push("arm9".to_owned());
    }
    differing.join(", ")
}

fn compare_job(
    registry: &HandlerRegistry,
    rom: &[u8],
    job: &Job,
    java_out: &Path,
    work: &Path,
    options: &RandomizationOptions,
    kept_roms: &AtomicUsize,
) -> Outcome {
    let java_error = fs::read_to_string(java_out.join(format!("{}.err", job.id))).ok();
    let settings = parse_job_settings(job);
    let rust = catch_unwind(AssertUnwindSafe(|| {
        registry.randomize(rom, &settings, job.seed, options)
    }));
    let rust = match rust {
        Ok(rust) => rust,
        Err(panic) => {
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|text| (*text).to_owned()))
                .unwrap_or_default();
            return match java_error {
                Some(_) => Outcome::BothFailed,
                None => Outcome::Mismatch(format!("Randomon panicked: {message}")),
            };
        }
    };
    let output = match (java_error, rust) {
        (Some(_), Err(_)) => return Outcome::BothFailed,
        (Some(error), Ok(_)) => {
            return Outcome::Mismatch(format!(
                "Java failed but Randomon succeeded: {}",
                error.lines().next().unwrap_or_default()
            ));
        }
        (None, Err(error)) => return Outcome::Mismatch(format!("Randomon failed: {error}")),
        (None, Ok(output)) => output,
    };
    let read =
        |extension: &str| fs::read_to_string(java_out.join(format!("{}.{extension}", job.id)));
    let java_log = read("log").expect("Java log");
    let java_cv: i32 = read("cv")
        .expect("Java check value")
        .trim()
        .parse()
        .expect("check value parses");
    let java_digest = read("sha256").expect("Java ROM digest");
    let rust_rom = output.rom_bytes().expect("file output");
    let mut problems = Vec::new();
    if hex(&sha256(rust_rom)) != java_digest.trim() {
        let java_rom_path = java_out.join(format!("{}.rom", job.id));
        let detail = fs::read(&java_rom_path)
            .map(|java_rom| differing_files(rust_rom, &java_rom))
            .unwrap_or_else(|_| "rerun with RANDOMON_PARITY_KEEP for a file diff".to_owned());
        problems.push(format!("ROM differs ({detail})"));
        if kept_roms.fetch_add(1, Ordering::SeqCst) < KEPT_RUST_ROMS {
            fs::write(work.join(format!("{}.rust.rom", job.id)), rust_rom).expect("write rust rom");
        }
    }
    let rust_log = mask_timing(&output.log);
    let java_log = mask_timing(&java_log);
    if rust_log != java_log {
        let line = rust_log
            .lines()
            .zip(java_log.lines())
            .position(|(a, b)| a != b)
            .map_or_else(|| "length".to_owned(), |line| (line + 1).to_string());
        problems.push(format!("log differs (first at line {line})"));
        fs::write(work.join(format!("{}.rust.log", job.id)), &output.log).expect("write rust log");
    }
    if output.check_value != java_cv {
        problems.push(format!("check value {} != {java_cv}", output.check_value));
    }
    if problems.is_empty() {
        Outcome::Identical
    } else {
        Outcome::Mismatch(problems.join("; "))
    }
}

fn compare_all(
    registry: &HandlerRegistry,
    rom: &[u8],
    jobs: &[Job],
    java_out: &Path,
    work: &Path,
    options: &RandomizationOptions,
) {
    let next = AtomicUsize::new(0);
    let kept_roms = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..env_count(RUST_WORKERS_ENV, DEFAULT_RUST_WORKERS) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(job) = jobs.get(index) else {
                        return;
                    };
                    let outcome =
                        compare_job(registry, rom, job, java_out, work, options, &kept_roms);
                    results.lock().expect("lock").push((index, outcome));
                }
            });
        }
    });
    let mut results = results.into_inner().expect("lock");
    results.sort_by_key(|(index, _)| *index);
    let mut failures = Vec::new();
    let mut both_failed = 0;
    let mut identical = 0;
    for (index, outcome) in results {
        match outcome {
            Outcome::Identical => identical += 1,
            Outcome::BothFailed => both_failed += 1,
            Outcome::Mismatch(problem) => failures.push(format!("{}: {problem}", jobs[index].id)),
        }
    }
    let summary = format!(
        "{} jobs, {identical} identical successes, {both_failed} matching failures, {} mismatches",
        jobs.len(),
        failures.len()
    );
    fs::write(
        work.join("summary.txt"),
        format!("{summary}\n{}\n", failures.join("\n")),
    )
    .expect("summary");
    eprintln!("{summary}");
    for failure in failures.iter().take(80) {
        eprintln!("  {failure}");
    }
    assert!(failures.is_empty(), "{summary}");
}

fn upstream_root(root: &std::path::Path) -> std::path::PathBuf {
    std::env::var_os("RANDOMON_UPSTREAM")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target/upstream"))
}
