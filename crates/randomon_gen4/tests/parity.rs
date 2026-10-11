mod grid;
mod support;

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use randomon_core::{HandlerRegistry, RandomizationOptions};
use randomon_formats::nds::NdsRom;
use randomon_formats::sha256::sha256;
use randomon_gen4::Gen4Factory;
use randomon_settings::{LineSeparator, Settings};

use support::{Quirks, build_rom, entry};

const ROM_ENV: &str = "RANDOMON_GEN4_ROM";
const SYNTHETIC_ENV: &str = "RANDOMON_GEN4_SYNTHETIC_PARITY";
const FILTER_ENV: &str = "RANDOMON_PARITY_FILTER";
const FUZZ_ENV: &str = "RANDOMON_PARITY_FUZZ";
const STRICT_ENV: &str = "RANDOMON_PARITY_STRICT";
const JAVA_WORKERS_ENV: &str = "RANDOMON_PARITY_JAVA_WORKERS";
const DEFAULT_JAVA_WORKERS: usize = 4;
const SYNTHETIC_GAMES: &[(&str, i32)] = &[("ADAE", 5), ("CPUE", 0), ("IPKE", 0)];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root exists")
}

fn env_count(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(default)
}

fn ensure_java_runner(root: &Path) {
    let class = root.join("target/parity/classes/ParityRunner.class");
    if class.exists() {
        return;
    }
    let status = Command::new("go")
        .args(["run", "./cmd/parity-build"])
        .current_dir(root.join("tools"))
        .status()
        .expect("go runs the parity build");
    assert!(status.success(), "building the Java parity runner failed");
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

struct Job {
    id: String,
    seed: i64,
    settings_text: String,
    settings: Settings,
}

fn build_jobs(rom_name: &str) -> Vec<Job> {
    let filter = std::env::var(FILTER_ENV).ok();
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
        let settings_text = settings.to_settings_string().expect("grid settings encode");
        for (index, &seed) in grid::SEEDS.iter().enumerate() {
            jobs.push(Job {
                id: format!("{name}__{index}"),
                seed,
                settings_text: settings_text.clone(),
                settings: settings.clone(),
            });
        }
    }
    jobs
}

fn run_java(root: &Path, work: &Path, rom_path: &Path, jobs: &[Job], java_out: &Path) {
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for worker in 0..env_count(JAVA_WORKERS_ENV, DEFAULT_JAVA_WORKERS) {
            let next = &next;
            scope.spawn(move || {
                let worker_directory = work.join("jvm").join(format!("worker{worker}"));
                let classes = worker_directory.join("classes");
                let _ = fs::remove_dir_all(&worker_directory);
                fs::create_dir_all(&worker_directory).expect("worker directory");
                let copied = Command::new("cp")
                    .arg("-R")
                    .arg(root.join("target/parity/classes"))
                    .arg(&classes)
                    .status()
                    .expect("cp runs");
                assert!(copied.success());
                let classpath = format!(
                    "{}:{}",
                    classes.display(),
                    root.join("upstream/src").display()
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
                    fs::write(
                        &job_file,
                        format!("{}\t{}\traw\t{}", job.id, job.seed, job.settings_text),
                    )
                    .expect("job file");
                    let status = Command::new("java")
                        .current_dir(&job_directory)
                        .arg("-Xmx2g")
                        .arg("-Djava.awt.headless=true")
                        .arg("-Dline.separator=\n")
                        .arg("-Dfile.encoding=UTF-8")
                        .arg("-Dparity.keepRom=true")
                        .arg("-cp")
                        .arg(&classpath)
                        .arg("ParityRunner")
                        .arg(rom_path)
                        .arg(&job_file)
                        .arg(java_out)
                        .status()
                        .expect("java runs");
                    assert!(status.success(), "Java runner crashed for {}", job.id);
                    let _ = fs::remove_dir_all(&job_directory);
                }
            });
        }
    });
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
) -> Result<(), String> {
    let java_error = fs::read_to_string(java_out.join(format!("{}.err", job.id))).ok();
    let rust = registry.randomize(rom, &job.settings, job.seed, options);
    let output = match (java_error, rust) {
        (Some(error), Err(_)) => {
            return Err(format!(
                "both failed (Java: {})",
                error.lines().next().unwrap_or_default()
            ));
        }
        (Some(error), Ok(_)) => {
            return Err(format!(
                "Java failed but Randomon succeeded: {}",
                error.lines().next().unwrap_or_default()
            ));
        }
        (None, Err(error)) => return Err(format!("Randomon failed: {error}")),
        (None, Ok(output)) => output,
    };
    let read = |extension: &str| fs::read(java_out.join(format!("{}.{extension}", job.id)));
    let java_log = String::from_utf8(read("log").expect("Java log")).expect("utf8 log");
    let java_cv: i32 = String::from_utf8(read("cv").expect("Java check value"))
        .expect("utf8")
        .trim()
        .parse()
        .expect("check value parses");
    let java_rom = read("rom").expect("Java ROM");
    let rust_rom = output.rom_bytes().expect("file output");
    let mut problems = Vec::new();
    if sha256(rust_rom) != sha256(&java_rom) {
        problems.push(format!(
            "ROM differs ({})",
            differing_files(rust_rom, &java_rom)
        ));
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
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

fn run_parity(rom: &[u8], rom_path: &Path, label: &str) -> (usize, Vec<String>) {
    let root = repo_root();
    ensure_java_runner(&root);
    let registry = HandlerRegistry::new().with(Gen4Factory);
    let info = registry
        .game_info(rom, &rom_path.display().to_string())
        .expect("ROM is a supported Gen 4 game");
    let jobs = build_jobs(&info.rom_name);
    let work = root.join("target/parity/gen4").join(label);
    let java_out = work.join("java");
    let _ = fs::remove_dir_all(&java_out);
    fs::create_dir_all(&java_out).expect("work directory");
    run_java(&root, &work, rom_path, &jobs, &java_out);
    let options = RandomizationOptions {
        file_name: rom_path.display().to_string(),
        line_separator: LineSeparator::Lf,
        ..RandomizationOptions::default()
    };
    let failures = Mutex::new(Vec::new());
    let identical = AtomicUsize::new(0);
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(job) = jobs.get(index) else {
                        return;
                    };
                    match compare_job(&registry, rom, job, &java_out, &work, &options) {
                        Ok(()) => {
                            identical.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(problem) => failures
                            .lock()
                            .expect("lock")
                            .push(format!("{}: {problem}", job.id)),
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().expect("lock");
    failures.sort();
    let summary = format!(
        "{label}: {} jobs, {} identical, {} differ",
        jobs.len(),
        identical.load(Ordering::SeqCst),
        failures.len()
    );
    fs::write(
        work.join("summary.txt"),
        format!("{summary}\n{}\n", failures.join("\n")),
    )
    .expect("summary");
    eprintln!("{summary}");
    for failure in failures.iter().take(120) {
        eprintln!("  {failure}");
    }
    (jobs.len(), failures)
}

#[test]
fn gen4_matches_upr_zx_on_a_real_rom() {
    let Ok(rom_path) = std::env::var(ROM_ENV) else {
        eprintln!("{ROM_ENV} is not set; skipping the Gen 4 real-ROM parity grid");
        return;
    };
    let rom = fs::read(&rom_path).expect("ROM is readable");
    let (_, failures) = run_parity(&rom, Path::new(&rom_path), "real");
    assert!(failures.is_empty(), "{} parity differences", failures.len());
}

#[test]
fn gen4_matches_upr_zx_on_synthetic_roms() {
    if std::env::var(SYNTHETIC_ENV).is_err() {
        eprintln!("{SYNTHETIC_ENV} is not set; skipping the synthetic Gen 4 parity grid");
        return;
    }
    let root = repo_root();
    let mut total_failures = 0;
    for &(code, version) in SYNTHETIC_GAMES {
        let rom = build_rom(entry(code, version), Quirks::default());
        let directory = root.join("target/parity/gen4");
        fs::create_dir_all(&directory).expect("parity directory");
        let rom_path = directory.join(format!("synthetic_{code}.nds"));
        fs::write(&rom_path, &rom).expect("write synthetic ROM");
        let (_, failures) = run_parity(&rom, &rom_path, code);
        total_failures += failures.len();
    }
    if std::env::var(STRICT_ENV).is_ok() {
        assert_eq!(total_failures, 0, "synthetic parity differences");
    }
}
