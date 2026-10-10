mod common;

use common::{as_array, as_i32, as_i64, as_str, i32_list, i64_list, load};
use randomon_java::{JavaRandom, JavaRng, RandomSource, shuffle};
use serde_json::Value;

fn bytes_of(random: &mut impl JavaRng, length: usize) -> Vec<i32> {
    let mut bytes = vec![0i8; length];
    random.next_bytes(&mut bytes);
    bytes.into_iter().map(i32::from).collect()
}

fn check_case(case: &Value) {
    let seed = as_i64(&case["seed"]);
    let fresh = || JavaRandom::new(seed);

    let mut random = fresh();
    let ints: Vec<i32> = (0..16).map(|_| random.next_int()).collect();
    assert_eq!(ints, i32_list(&case["next_int"]), "next_int seed {seed}");

    for bounded in as_array(&case["next_int_bounded"]) {
        let bound = as_i32(&bounded["bound"]);
        let mut random = fresh();
        let values: Vec<i32> = (0..12).map(|_| random.next_int_bounded(bound)).collect();
        assert_eq!(
            values,
            i32_list(&bounded["values"]),
            "next_int({bound}) seed {seed}"
        );
        assert_eq!(
            random.next_int(),
            as_i32(&bounded["next_int_after"]),
            "state after next_int({bound})"
        );
    }

    let mut random = fresh();
    let longs: Vec<i64> = (0..8).map(|_| random.next_long()).collect();
    assert_eq!(longs, i64_list(&case["next_long"]), "next_long seed {seed}");

    let mut random = fresh();
    let booleans: Vec<bool> = (0..16).map(|_| random.next_boolean()).collect();
    let expected: Vec<bool> = as_array(&case["next_boolean"])
        .iter()
        .map(|v| v.as_bool().unwrap())
        .collect();
    assert_eq!(booleans, expected, "next_boolean seed {seed}");

    let mut random = fresh();
    let floats: Vec<i64> = (0..8)
        .map(|_| i64::from(random.next_float().to_bits() as i32))
        .collect();
    assert_eq!(
        floats,
        i64_list(&case["next_float_bits"]),
        "next_float seed {seed}"
    );

    let mut random = fresh();
    let doubles: Vec<i64> = (0..8)
        .map(|_| random.next_double().to_bits() as i64)
        .collect();
    assert_eq!(
        doubles,
        i64_list(&case["next_double_bits"]),
        "next_double seed {seed}"
    );

    let mut random = fresh();
    let gaussians: Vec<i64> = (0..25)
        .map(|_| random.next_gaussian().to_bits() as i64)
        .collect();
    assert_eq!(
        gaussians,
        i64_list(&case["next_gaussian_bits"]),
        "next_gaussian seed {seed}"
    );

    let mut random = fresh();
    for (length, expected) in [0usize, 1, 2, 3, 4, 5, 7, 8, 9, 13, 16]
        .iter()
        .zip(as_array(&case["next_bytes"]))
    {
        assert_eq!(
            bytes_of(&mut random, *length),
            i32_list(expected),
            "next_bytes({length}) seed {seed}"
        );
    }

    let mut random = fresh();
    for step in as_array(&case["mixed"]) {
        check_mixed_step(&mut random, as_array(step), seed);
    }
}

fn check_mixed_step(random: &mut JavaRandom, step: &[Value], seed: i64) {
    match as_str(&step[0]) {
        "next_int" => assert_eq!(
            random.next_int(),
            as_i32(&step[1]),
            "mixed next_int seed {seed}"
        ),
        "next_int_bounded" => {
            assert_eq!(
                random.next_int_bounded(as_i32(&step[1])),
                as_i32(&step[2]),
                "mixed bounded seed {seed}"
            )
        }
        "next_long" => assert_eq!(
            random.next_long(),
            as_i64(&step[1]),
            "mixed next_long seed {seed}"
        ),
        "next_boolean" => assert_eq!(
            random.next_boolean(),
            step[1].as_bool().unwrap(),
            "mixed boolean seed {seed}"
        ),
        "next_float_bits" => assert_eq!(
            random.next_float().to_bits() as i32,
            as_i32(&step[1]),
            "mixed float"
        ),
        "next_double_bits" => assert_eq!(
            random.next_double().to_bits() as i64,
            as_i64(&step[1]),
            "mixed double"
        ),
        "next_gaussian_bits" => assert_eq!(
            random.next_gaussian().to_bits() as i64,
            as_i64(&step[1]),
            "mixed gaussian"
        ),
        "next_bytes" => {
            let length = as_i64(&step[1]) as usize;
            assert_eq!(
                bytes_of(random, length),
                i32_list(&step[2]),
                "mixed bytes seed {seed}"
            );
        }
        "set_seed" => random.set_seed(as_i64(&step[1])),
        other => panic!("unknown step {other}"),
    }
}

#[test]
fn java_random_matches_jdk_for_every_method() {
    let vectors = load("random.json");
    let cases = as_array(&vectors["cases"]);
    assert!(cases.len() >= 30);
    cases.iter().for_each(check_case);
}

#[test]
fn shuffle_matches_collections_shuffle() {
    let vectors = load("shuffle.json");
    for case in as_array(&vectors["cases"]) {
        let seed = as_i64(&case["seed"]);
        let size = as_i64(&case["size"]) as i32;
        let mut list: Vec<i32> = (0..size).collect();
        let mut random = JavaRandom::new(seed);
        shuffle(&mut list, &mut random);
        assert_eq!(
            list,
            i32_list(&case["result"]),
            "shuffle size {size} seed {seed}"
        );
        assert_eq!(
            random.next_int(),
            as_i32(&case["next_int_after"]),
            "rng state after shuffle size {size}"
        );
    }
}

#[test]
fn random_source_counts_calls_and_forwards_through_instances() {
    let mut source = RandomSource::with_seed(0);
    source.seed(123456789);
    let mut reference = JavaRandom::new(123456789);
    let mut cosmetic_reference = JavaRandom::new(123456789);

    assert_eq!(source.next_int_bounded(10), reference.next_int_bounded(10));
    assert_eq!(source.random().to_bits(), reference.next_double().to_bits());
    assert_eq!(
        source.next_int_cosmetic(7),
        cosmetic_reference.next_int_bounded(7)
    );
    let mut list: Vec<i32> = (0..20).collect();
    let mut expected_list = list.clone();
    shuffle(&mut list, &mut source.instance());
    shuffle(&mut expected_list, &mut reference);
    assert_eq!(list, expected_list);
    assert_eq!(source.instance().next_boolean(), reference.next_boolean());
    assert_eq!(
        source.cosmetic_instance().next_int_bounded(5),
        cosmetic_reference.next_int_bounded(5)
    );
    assert_eq!(source.cosmetic_instance().next_double(), 0.0);
    assert_eq!(source.cosmetic_instance().next_int(), 0);
    assert_eq!(source.calls_since_seed(), 2 + 19 + 1 + 2);

    source.instance().set_seed(5);
    assert_eq!(source.calls_since_seed(), 0);
    assert_eq!(source.next_long(), JavaRandom::new(5).next_long());
}

#[test]
fn shared_random_source_instances_share_one_stream() {
    let shared = RandomSource::with_seed(99).into_shared();
    let mut main = shared.instance();
    let mut cosmetic = shared.cosmetic_instance();
    let mut reference = JavaRandom::new(99);
    let mut cosmetic_reference = JavaRandom::new(99);
    assert_eq!(main.next_int_bounded(100), reference.next_int_bounded(100));
    assert_eq!(
        cosmetic.next_int_bounded(100),
        cosmetic_reference.next_int_bounded(100)
    );
    assert_eq!(
        shared.instance().next_gaussian().to_bits(),
        reference.next_gaussian().to_bits()
    );
    assert_eq!(
        main.next_gaussian().to_bits(),
        reference.next_gaussian().to_bits()
    );
    assert_eq!(shared.calls_since_seed(), 4);
}

#[test]
fn pick_seed_assembles_six_little_endian_bytes() {
    assert_eq!(
        RandomSource::seed_from_bytes([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]),
        0x0605_0403_0201
    );
    assert_eq!(RandomSource::seed_from_bytes([0xFF; 6]), 0xFFFF_FFFF_FFFF);
    let picked = RandomSource::pick_seed();
    assert!((0..1 << 48).contains(&picked));
}
