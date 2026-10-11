use randomon_fuzz::{FuzzPlan, Target, env_path, fuzz};

fn fuzz_game(rom_env: &str, update_env: Option<&str>, default_cases: u64, default_threads: usize) {
    let Some(rom) = env_path(rom_env) else {
        eprintln!("{rom_env} is not set; skipping");
        return;
    };
    let update = update_env.and_then(|name| {
        let requested = std::env::var_os(name)?;
        let path = env_path(name);
        assert!(
            path.is_some(),
            "{name} points at a missing file: {requested:?}"
        );
        path
    });
    let target = Target::new(rom, update);
    let plan = FuzzPlan::from_env(default_cases, default_threads);
    let summary = fuzz(&target, &plan);
    eprintln!(
        "{rom_env}: {} cases x {} seeds = {} runs, {} failures",
        plan.cases,
        plan.seeds_per_case,
        summary.runs,
        summary.failures.len()
    );
    assert!(
        summary.failures.is_empty(),
        "{} failing runs",
        summary.failures.len()
    );
}

#[test]
fn fuzz_red() {
    fuzz_game("RANDOMON_FUZZ_RED", None, 200, 6);
}

#[test]
fn fuzz_yellow() {
    fuzz_game("RANDOMON_FUZZ_YELLOW", None, 200, 6);
}

#[test]
fn fuzz_gold() {
    fuzz_game("RANDOMON_FUZZ_GOLD", None, 200, 6);
}

#[test]
fn fuzz_black2() {
    fuzz_game("RANDOMON_FUZZ_BLACK2", None, 60, 4);
}

#[test]
fn fuzz_alpha_sapphire() {
    fuzz_game(
        "RANDOMON_FUZZ_ORAS",
        Some("RANDOMON_FUZZ_ORAS_UPDATE"),
        30,
        3,
    );
}

#[test]
fn fuzz_ultra_sun() {
    fuzz_game("RANDOMON_FUZZ_USUM", None, 30, 3);
}
