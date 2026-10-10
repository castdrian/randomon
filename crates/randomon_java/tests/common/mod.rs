#![allow(dead_code)]

use std::path::PathBuf;

use serde_json::Value;

pub fn load(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()))
}

pub fn as_i64(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("expected integer, found {value}"))
}

pub fn as_i32(value: &Value) -> i32 {
    i32::try_from(as_i64(value)).unwrap_or_else(|_| panic!("expected i32, found {value}"))
}

pub fn as_str(value: &Value) -> &str {
    value
        .as_str()
        .unwrap_or_else(|| panic!("expected string, found {value}"))
}

pub fn as_array(value: &Value) -> &Vec<Value> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("expected array, found {value}"))
}

pub fn i64_list(value: &Value) -> Vec<i64> {
    as_array(value).iter().map(as_i64).collect()
}

pub fn i32_list(value: &Value) -> Vec<i32> {
    as_array(value).iter().map(as_i32).collect()
}
