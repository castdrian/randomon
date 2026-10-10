mod common;

use common::{as_array, as_i32, as_i64, as_str, load};
use randomon_java::math::{round_double, round_float};
use randomon_java::{
    FormatArg, JavaHash, double_to_string, float_to_string, java_format, long_hash_code,
    strict_math, string_compare_to, string_hash_code,
};
use serde_json::Value;

#[test]
fn double_to_string_matches_jdk8_floating_decimal() {
    let vectors = load("number_to_string.json");
    for case in as_array(&vectors["doubles"]) {
        let value = f64::from_bits(as_i64(&case[0]) as u64);
        assert_eq!(
            double_to_string(value),
            as_str(&case[1]),
            "Double.toString of bits {:#x}",
            value.to_bits()
        );
    }
    assert!(
        vectors["doubles_differing_from_jdk19_plus"]
            .as_i64()
            .unwrap()
            > 0
    );
}

#[test]
fn float_to_string_matches_jdk8_floating_decimal() {
    let vectors = load("number_to_string.json");
    for case in as_array(&vectors["floats"]) {
        let value = f32::from_bits(as_i32(&case[0]) as u32);
        assert_eq!(
            float_to_string(value),
            as_str(&case[1]),
            "Float.toString of bits {:#x}",
            value.to_bits()
        );
    }
}

#[test]
fn math_round_and_casts_match_java() {
    let vectors = load("round.json");
    for case in as_array(&vectors["doubles"]) {
        let value = f64::from_bits(as_i64(&case[0]) as u64);
        assert_eq!(
            round_double(value),
            as_i64(&case[1]),
            "Math.round({value:e})"
        );
        assert_eq!(value as i32, as_i32(&case[2]), "(int) {value:e}");
        assert_eq!(value as i64, as_i64(&case[3]), "(long) {value:e}");
    }
    for case in as_array(&vectors["floats"]) {
        let value = f32::from_bits(as_i32(&case[0]) as u32);
        assert_eq!(
            round_float(value),
            as_i32(&case[1]),
            "Math.round({value:e}f)"
        );
        assert_eq!(value as i32, as_i32(&case[2]), "(int) {value:e}f");
        assert_eq!(value as i64, as_i64(&case[3]), "(long) {value:e}f");
    }
}

#[test]
fn strict_math_log_matches_fdlibm() {
    let vectors = load("strict_math.json");
    for case in as_array(&vectors["log"]) {
        let input = f64::from_bits(as_i64(&case[0]) as u64);
        let expected = as_i64(&case[1]) as u64;
        let actual = strict_math::log(input).to_bits();
        let both_nan = f64::from_bits(expected).is_nan() && f64::from_bits(actual).is_nan();
        assert!(both_nan || actual == expected, "StrictMath.log({input:e})");
    }
}

#[test]
fn hash_codes_and_string_order_match_java() {
    let vectors = load("hash_code.json");
    for case in as_array(&vectors["strings"]) {
        let text = as_str(&case[0]);
        assert_eq!(
            string_hash_code(text),
            as_i32(&case[1]),
            "hashCode of {text:?}"
        );
        assert_eq!(text.to_owned().java_hash_code(), as_i32(&case[1]));
    }
    for case in as_array(&vectors["longs"]) {
        assert_eq!(long_hash_code(as_i64(&case[0])), as_i32(&case[1]));
    }
    for case in as_array(&vectors["string_compare_signum"]) {
        let ordering = string_compare_to(as_str(&case[0]), as_str(&case[1]));
        assert_eq!(
            ordering as i32,
            as_i32(&case[2]),
            "compareTo {:?} {:?}",
            case[0],
            case[1]
        );
    }
}

fn format_arg(argument: &Value) -> FormatArg {
    let argument = as_array(argument);
    let value = &argument[1];
    match as_str(&argument[0]) {
        "int" => FormatArg::Int(as_i32(value)),
        "long" => FormatArg::Long(as_i64(value)),
        "short" => FormatArg::Short(as_i64(value) as i16),
        "byte" => FormatArg::Byte(as_i64(value) as i8),
        "double_bits" => FormatArg::Double(f64::from_bits(as_i64(value) as u64)),
        "float_bits" => FormatArg::Float(f32::from_bits(as_i32(value) as u32)),
        "string" => FormatArg::Str(as_str(value).to_owned()),
        "char" => FormatArg::Char(as_str(value).chars().next().unwrap()),
        "bool" => FormatArg::Bool(value.as_bool().unwrap()),
        "null" => FormatArg::Null,
        other => panic!("unknown argument type {other}"),
    }
}

#[test]
fn string_format_matches_java_formatter() {
    let vectors = load("format.json");
    let cases = as_array(&vectors["cases"]);
    assert!(cases.len() > 1000);
    for case in cases {
        let format = as_str(&case["format"]);
        let args: Vec<FormatArg> = as_array(&case["args"]).iter().map(format_arg).collect();
        let actual = java_format(format, &args)
            .unwrap_or_else(|error| panic!("{format:?} {args:?}: {error}"));
        assert_eq!(
            actual,
            as_str(&case["expected"]),
            "String.format({format:?}, {args:?})"
        );
    }
}
