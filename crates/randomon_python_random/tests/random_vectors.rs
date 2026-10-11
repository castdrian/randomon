use randomon_python_random::{PythonRandom, sample_set_threshold};
use serde_json::Value;

fn vectors() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/random.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("vector file")).expect("vector json")
}

fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("expected integer, found {value}"))
}

fn text_u64(value: &Value) -> u64 {
    value.as_str().expect("string").parse().expect("u64")
}

fn ints(value: &Value) -> Vec<i64> {
    value.as_array().expect("array").iter().map(int).collect()
}

fn check_case(case: &Value) {
    let seed: i128 = case["seed"]
        .as_str()
        .expect("seed")
        .parse()
        .expect("seed parses");
    let mut rng = PythonRandom::new(seed);
    let words: Vec<i64> = (0..8).map(|_| i64::from(rng.next_u32())).collect();
    assert_eq!(words, ints(&case["u32"]), "u32 seed {seed}");
    for expected in case["random"].as_array().expect("random") {
        assert_eq!(
            rng.random().to_bits(),
            text_u64(expected),
            "random seed {seed}"
        );
    }
    for entry in case["getrandbits"].as_array().expect("getrandbits") {
        let bits = int(&entry["bits"]) as u32;
        assert_eq!(
            rng.getrandbits(bits),
            text_u64(&entry["value"]),
            "getrandbits({bits}) seed {seed}"
        );
    }
    for entry in case["randrange"].as_array().expect("randrange") {
        let value = rng
            .randrange(int(&entry["start"]), int(&entry["stop"]))
            .expect("randrange");
        assert_eq!(value, int(&entry["value"]), "randrange seed {seed}");
    }
    for entry in case["randint"].as_array().expect("randint") {
        let value = rng
            .randint(int(&entry["low"]), int(&entry["high"]))
            .expect("randint");
        assert_eq!(value, int(&entry["value"]), "randint seed {seed}");
    }
    let sequence: Vec<i64> = (100..117).collect();
    for expected in case["choice"].as_array().expect("choice") {
        assert_eq!(
            *rng.choice(&sequence).expect("choice"),
            int(expected),
            "choice seed {seed}"
        );
    }
    let mut shuffled: Vec<i64> = (0..25).collect();
    rng.shuffle(&mut shuffled);
    assert_eq!(shuffled, ints(&case["shuffle"]), "shuffle seed {seed}");
    for entry in case["sample"].as_array().expect("sample") {
        let population = int(&entry["population"]);
        let count = int(&entry["count"]) as usize;
        let value = rng.sample_range(0, population, count).expect("sample");
        assert_eq!(
            value,
            ints(&entry["value"]),
            "sample({population}, {count}) seed {seed}"
        );
    }
    for expected in case["uniform"].as_array().expect("uniform") {
        assert_eq!(
            rng.uniform(-2.5, 7.25).to_bits(),
            text_u64(expected),
            "uniform seed {seed}"
        );
    }
    assert_eq!(
        i64::from(rng.next_u32()),
        int(&case["after"]),
        "state after case seed {seed}"
    );
}

#[test]
fn matches_cpython_vectors() {
    let vectors = vectors();
    let cases = vectors["cases"].as_array().expect("cases");
    assert!(!cases.is_empty());
    cases.iter().for_each(check_case);
}

#[test]
fn sample_threshold_matches_cpython_formula() {
    assert_eq!(sample_set_threshold(3), 21);
    assert_eq!(sample_set_threshold(6), 21 + 64);
    assert_eq!(sample_set_threshold(100), 21 + 1024);
}

#[test]
fn empty_ranges_are_errors() {
    let mut rng = PythonRandom::new(5);
    assert!(rng.randrange(3, 3).is_err());
    assert!(rng.choice::<u8>(&[]).is_err());
    assert!(rng.sample_indices(2, 3).is_err());
}
