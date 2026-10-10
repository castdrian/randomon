use randomon_core::engine::util::{distinct, modified_level, remove_first, rotate_check_value};

#[test]
fn check_values_rotate_then_xor() {
    assert_eq!(rotate_check_value(0, &[1]), 1);
    assert_eq!(rotate_check_value(1, &[0]), 8);
    assert_eq!(rotate_check_value(i32::MIN, &[0]), 4);
    assert_eq!(rotate_check_value(0, &[5, 3]), (5 << 3) ^ 3);
}

#[test]
fn level_modifiers_round_like_java_and_cap_at_100() {
    assert_eq!(modified_level(10, 25), 13);
    assert_eq!(modified_level(2, 25), 3);
    assert_eq!(modified_level(90, 50), 100);
    assert_eq!(modified_level(5, -50), 3);
}

#[test]
fn list_helpers_keep_java_order() {
    let mut list = vec![3, 1, 3, 2];
    assert!(remove_first(&mut list, &3));
    assert_eq!(list, vec![1, 3, 2]);
    assert_eq!(distinct(&[2, 2, 1, 2, 3]), vec![2, 1, 3]);
}
