mod common;

use std::fmt::Debug;

use common::{as_array, as_i32, as_i64, as_str, i32_list, load};
use randomon_java::{JavaHash, JavaHashMap, JavaHashSet};
use serde_json::Value;

trait VectorKey: JavaHash + Eq + Clone + Debug {
    fn from_json(value: &Value) -> Self;
}

impl VectorKey for i32 {
    fn from_json(value: &Value) -> Self {
        as_i32(value)
    }
}

impl VectorKey for String {
    fn from_json(value: &Value) -> Self {
        as_str(value).to_owned()
    }
}

fn increment_or_remove(old: Option<&i32>) -> Option<i32> {
    match old {
        None => Some(1),
        Some(value) if value % 2 == 0 => None,
        Some(value) => Some(value + 1),
    }
}

fn initial_map<K>(init: &Value) -> JavaHashMap<K, i32> {
    match init.as_i64() {
        Some(capacity) => JavaHashMap::with_capacity(capacity as i32),
        None => JavaHashMap::new(),
    }
}

fn replay_map_scenario<K: VectorKey>(scenario: &Value) {
    let name = as_str(&scenario["name"]);
    let mut snapshots = as_array(&scenario["snapshots"]).iter();
    let mut map: JavaHashMap<K, i32> = JavaHashMap::new();
    for (index, op) in as_array(&scenario["ops"]).iter().enumerate() {
        let op = as_array(op);
        let key = || K::from_json(&op[1]);
        match as_str(&op[0]) {
            "init" => map = initial_map(&op[1]),
            "put" => {
                map.put(key(), as_i32(&op[2]));
            }
            "remove" => {
                map.remove(&key());
            }
            "compute_if_absent" => {
                let value = as_i32(&op[2]);
                map.compute_if_absent(key(), |_| value);
            }
            "merge_sum" => {
                map.merge(key(), as_i32(&op[2]), |old, new| Some(old + new));
            }
            "put_if_absent" => {
                map.put_if_absent(key(), as_i32(&op[2]));
            }
            "compute_increment_or_remove" => {
                map.compute(key(), |_, old| increment_or_remove(old));
            }
            "retain_not_divisible_by_three" => map.retain(|_, value| *value % 3 != 0),
            "clear" => map.clear(),
            "jdk8_copy" => {
                let entries: Vec<(K, i32)> = map.iter().map(|(k, v)| (k.clone(), *v)).collect();
                map = JavaHashMap::from_entries_like_copy_constructor(entries);
            }
            "snapshot" => {
                let snapshot = snapshots.next().expect("snapshot recorded");
                assert_eq!(
                    as_i64(&snapshot["op_index"]) as usize,
                    index,
                    "{name}: snapshot position"
                );
                let keys: Vec<K> = as_array(&snapshot["keys"])
                    .iter()
                    .map(K::from_json)
                    .collect();
                let actual_keys: Vec<K> = map.keys().cloned().collect();
                assert_eq!(actual_keys, keys, "{name}: key order at op {index}");
                let values: Vec<i32> = map.values().copied().collect();
                assert_eq!(
                    values,
                    i32_list(&snapshot["values"]),
                    "{name}: values at op {index}"
                );
                assert_eq!(
                    map.capacity(),
                    as_i64(&snapshot["capacity"]) as usize,
                    "{name}: capacity at op {index}"
                );
                assert_eq!(map.len(), keys.len(), "{name}: size at op {index}");
            }
            other => panic!("unknown op {other}"),
        }
    }
    assert!(snapshots.next().is_none(), "{name}: unreplayed snapshots");
}

#[test]
fn hash_map_iteration_order_matches_java() {
    let vectors = load("hash_map.json");
    let scenarios = as_array(&vectors["scenarios"]);
    assert!(scenarios.len() > 30);
    let mut saw_tree_bins = false;
    for scenario in scenarios {
        saw_tree_bins |= as_array(&scenario["snapshots"])
            .iter()
            .any(|s| s["tree_bins"].as_bool() == Some(true));
        match as_str(&scenario["kind"]) {
            "int" => replay_map_scenario::<i32>(scenario),
            "string" => replay_map_scenario::<String>(scenario),
            other => panic!("unknown kind {other}"),
        }
    }
    assert!(saw_tree_bins, "vectors must exercise treeified bins");
}

#[test]
fn hash_set_iteration_order_matches_java() {
    let vectors = load("hash_set.json");
    for scenario in as_array(&vectors["scenarios"]) {
        let name = as_str(&scenario["name"]);
        let mut snapshots = as_array(&scenario["snapshots"]).iter();
        let mut set: JavaHashSet<i32> = JavaHashSet::new();
        for (index, op) in as_array(&scenario["ops"]).iter().enumerate() {
            let op = as_array(op);
            match as_str(&op[0]) {
                "add" => {
                    set.add(as_i32(&op[1]));
                }
                "remove" => {
                    set.remove(&as_i32(&op[1]));
                }
                "remove_all_list" => {
                    set.remove_all_list(&i32_list(&op[1]));
                }
                "from_collection" => set = JavaHashSet::from_collection(i32_list(&op[1])),
                "snapshot" => {
                    let snapshot = snapshots.next().expect("snapshot recorded");
                    assert_eq!(
                        as_i64(&snapshot["op_index"]) as usize,
                        index,
                        "{name}: snapshot position"
                    );
                    let actual: Vec<i32> = set.iter().copied().collect();
                    assert_eq!(
                        actual,
                        i32_list(&snapshot["elements"]),
                        "{name}: order at op {index}"
                    );
                }
                other => panic!("unknown op {other}"),
            }
        }
    }
}

#[test]
fn java_clone_rebuilds_with_copy_constructor_capacity() {
    let mut map: JavaHashMap<i32, i32> = JavaHashMap::with_capacity(1024);
    for key in 0..12 {
        map.put(key * 100, key);
    }
    let clone = map.java_clone();
    assert_eq!(map.capacity(), 1024);
    assert_eq!(clone.capacity(), 32);
    assert_eq!(clone.len(), 12);
}
