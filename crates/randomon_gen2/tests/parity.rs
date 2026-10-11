mod grid;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use randomon_core::{HandlerRegistry, RandomizationOptions};
use randomon_gen2::Gen2Factory;
use randomon_settings::{LineSeparator, Settings};

const ROM_ENV: &str = "RANDOMON_PARITY_ROM";
const FILTER_ENV: &str = "RANDOMON_PARITY_FILTER";
const SEEDS_ENV: &str = "RANDOMON_PARITY_SEEDS";
const REUSE_ENV: &str = "RANDOMON_PARITY_REUSE";
const FUZZ_ENV: &str = "RANDOMON_PARITY_FUZZ";
const JAVA_WORKERS: usize = 6;
const JAVA_JOB_TIMEOUT: Duration = Duration::from_secs(180);

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root exists")
}

fn ensure_java_runner(root: &Path) {
    let classes = root.join("target/parity/classes/ParityRunner.class");
    if classes.exists() {
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
    let mut jobs = Vec::new();
    let fuzz_count: usize = std::env::var(FUZZ_ENV)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(60);
    let cases = grid::settings_grid(rom_name)
        .into_iter()
        .chain(grid::fuzz_grid(rom_name, fuzz_count));
    for (name, settings) in cases {
        if filter
            .as_deref()
            .is_some_and(|filter| !name.contains(filter))
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

#[test]
fn gen2_matches_upr_zx() {
    let Ok(rom_path) = std::env::var(ROM_ENV) else {
        eprintln!("{ROM_ENV} is not set; skipping the Gen 2 parity grid");
        return;
    };
    let root = repo_root();
    ensure_java_runner(&root);
    let rom = fs::read(&rom_path).expect("ROM is readable");
    let registry = HandlerRegistry::new().with(Gen2Factory);
    let info = registry
        .game_info(&rom, &rom_path)
        .expect("ROM is a supported Gen 2 game");
    let jobs = build_jobs(&info.rom_name);
    for job in &jobs {
        assert_eq!(
            parse_job_settings(job).to_settings_string().ok(),
            job.settings.to_settings_string().ok(),
            "settings round trip for {}",
            job.id
        );
    }
    let work = root.join("target/parity/gen2");
    let reuse = std::env::var(REUSE_ENV).is_ok();
    if !reuse {
        let _ = fs::remove_dir_all(&work);
    }
    fs::create_dir_all(&work).expect("work directory");
    let jobs_file = work.join("jobs.tsv");
    let job_lines: Vec<String> = jobs
        .iter()
        .map(|job| {
            format!(
                "{}\t{}\t{}\t{}",
                job.id, job.seed, job.mode, job.settings_text
            )
        })
        .collect();
    let job_text = job_lines.join("\n");
    let java_out = work.join("java");
    let previous_jobs = fs::read_to_string(&jobs_file).ok();
    if !(reuse && previous_jobs.as_deref() == Some(job_text.as_str())) {
        fs::write(&jobs_file, &job_text).expect("jobs file");
        run_java(&root, &work, &rom_path, &jobs_file, &java_out);
    }
    let options = RandomizationOptions {
        file_name: rom_path.clone(),
        line_separator: LineSeparator::Lf,
        ..RandomizationOptions::default()
    };
    compare_all(&registry, &rom, &jobs, &java_out, &work, &options);
}

fn java_command(
    root: &Path,
    work: &Path,
    rom_path: &str,
    job_file: &Path,
    java_out: &Path,
) -> Command {
    let classpath = format!(
        "{}:{}",
        root.join("target/parity/classes").display(),
        upstream_root(root).join("src").display()
    );
    let mut command = Command::new("java");
    command
        .current_dir(work)
        .args([
            "-Xmx1g",
            "-XX:-OmitStackTraceInFastThrow",
            "-Djava.awt.headless=true",
            "-Dline.separator=\n",
            "-Dfile.encoding=UTF-8",
            "-cp",
            &classpath,
            "ParityRunner",
        ])
        .arg(rom_path)
        .arg(job_file)
        .arg(java_out)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn wait_with_timeout(mut child: Child) -> bool {
    let deadline = Instant::now() + JAVA_JOB_TIMEOUT;
    loop {
        if child.try_wait().expect("java status").is_some() {
            return true;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn run_java_job(root: &Path, work: &Path, rom_path: &str, line: &str, java_out: &Path) {
    let id = line.split('\t').next().unwrap_or_default();
    let job_file = work.join(format!("job.{id}.tsv"));
    fs::write(&job_file, line).expect("job file");
    let child = java_command(root, work, rom_path, &job_file, java_out)
        .spawn()
        .expect("java runs");
    if !wait_with_timeout(child) {
        fs::write(
            java_out.join(format!("{id}.err")),
            format!("timed out after {}s\n", JAVA_JOB_TIMEOUT.as_secs()),
        )
        .expect("timeout marker");
    }
    let _ = fs::remove_file(&job_file);
}

fn run_java(root: &Path, work: &Path, rom_path: &str, jobs_file: &Path, java_out: &Path) {
    fs::create_dir_all(java_out).expect("java output directory");
    let lines: Vec<String> = fs::read_to_string(jobs_file)
        .expect("jobs file")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect();
    let queue = Mutex::new(lines.into_iter());
    thread::scope(|scope| {
        for _ in 0..JAVA_WORKERS {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().expect("job queue").next();
                    let Some(line) = next else {
                        break;
                    };
                    run_java_job(root, work, rom_path, &line, java_out);
                }
            });
        }
    });
}

fn compare_all(
    registry: &HandlerRegistry,
    rom: &[u8],
    jobs: &[Job],
    java_out: &Path,
    work: &Path,
    options: &RandomizationOptions,
) {
    let mut failures: Vec<String> = Vec::new();
    let mut both_failed = 0;
    for job in jobs {
        let java_error = fs::read_to_string(java_out.join(format!("{}.err", job.id))).ok();
        let settings = parse_job_settings(job);
        let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            registry.randomize(rom, &settings, job.seed, options)
        }));
        let Ok(rust) = rust else {
            failures.push(format!("{}: Randomon panicked", job.id));
            continue;
        };
        match (java_error, rust) {
            (Some(_), Err(_)) => both_failed += 1,
            (Some(error), Ok(_)) => failures.push(format!(
                "{}: Java failed but Randomon succeeded: {}",
                job.id,
                error.lines().next().unwrap_or_default()
            )),
            (None, Err(error)) => failures.push(format!("{}: Randomon failed: {error}", job.id)),
            (None, Ok(output)) => {
                let java_rom =
                    fs::read(java_out.join(format!("{}.rom", job.id))).expect("Java ROM");
                let java_log =
                    fs::read_to_string(java_out.join(format!("{}.log", job.id))).expect("Java log");
                let java_cv: i32 = fs::read_to_string(java_out.join(format!("{}.cv", job.id)))
                    .expect("Java check value")
                    .trim()
                    .parse()
                    .expect("check value parses");
                let rust_rom = output.rom_bytes().expect("file output").to_vec();
                let mut problems = Vec::new();
                if rust_rom != java_rom {
                    let differing = rust_rom
                        .iter()
                        .zip(&java_rom)
                        .filter(|(a, b)| a != b)
                        .count();
                    let first = rust_rom.iter().zip(&java_rom).position(|(a, b)| a != b);
                    problems.push(format!(
                        "ROM differs in {differing} bytes (first at {first:X?})"
                    ));
                    fs::write(work.join(format!("{}.rust.rom", job.id)), &rust_rom)
                        .expect("write rust rom");
                }
                let rust_log = mask_timing(&output.log);
                let java_log_masked = mask_timing(&java_log);
                if rust_log != java_log_masked {
                    let line = rust_log
                        .lines()
                        .zip(java_log_masked.lines())
                        .position(|(a, b)| a != b)
                        .map_or_else(|| "length".to_owned(), |line| (line + 1).to_string());
                    problems.push(format!("log differs (first at line {line})"));
                    fs::write(work.join(format!("{}.rust.log", job.id)), &output.log)
                        .expect("write rust log");
                }
                if output.check_value != java_cv {
                    problems.push(format!("check value {} != {java_cv}", output.check_value));
                }
                if !problems.is_empty() {
                    failures.push(format!("{}: {}", job.id, problems.join("; ")));
                }
            }
        }
    }
    let summary = format!(
        "{} jobs, {} identical successes, {} matching failures, {} mismatches",
        jobs.len(),
        jobs.len() - failures.len() - both_failed,
        both_failed,
        failures.len()
    );
    fs::write(
        work.join("summary.txt"),
        format!("{summary}\n{}\n", failures.join("\n")),
    )
    .expect("summary");
    eprintln!("{summary}");
    for failure in failures.iter().take(50) {
        eprintln!("  {failure}");
    }
    assert!(failures.is_empty(), "{summary}");
}

fn upstream_root(root: &std::path::Path) -> std::path::PathBuf {
    std::env::var_os("RANDOMON_UPSTREAM")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target/upstream"))
}
