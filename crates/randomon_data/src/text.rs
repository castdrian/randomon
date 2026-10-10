pub fn camel_case(original: &str) -> String {
    let mut capitalize_next = true;
    let mut out = String::with_capacity(original.len());
    for current in java_lower_case(original).chars() {
        if capitalize_next && current.is_alphabetic() {
            out.push(single_upper(current));
            capitalize_next = false;
        } else {
            if !capitalize_next && !current.is_alphabetic() && current != '\'' && current != '’' {
                capitalize_next = true;
            }
            out.push(current);
        }
    }
    out
}

pub fn java_lower_case(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}

pub fn java_upper_case(text: &str) -> String {
    text.chars().flat_map(char::to_uppercase).collect()
}

pub fn java_equals_ignore_case(left: &str, right: &str) -> bool {
    let left_units: Vec<u16> = left.encode_utf16().collect();
    let right_units: Vec<u16> = right.encode_utf16().collect();
    left_units.len() == right_units.len()
        && left_units
            .iter()
            .zip(&right_units)
            .all(|(&a, &b)| units_equal_ignoring_case(a, b))
}

fn units_equal_ignoring_case(a: u16, b: u16) -> bool {
    if a == b {
        return true;
    }
    let (Some(a), Some(b)) = (char::from_u32(u32::from(a)), char::from_u32(u32::from(b))) else {
        return false;
    };
    let upper_a = single_upper(a);
    let upper_b = single_upper(b);
    upper_a == upper_b || single_lower(upper_a) == single_lower(upper_b)
}

fn single_upper(value: char) -> char {
    let mut upper = value.to_uppercase();
    match (upper.next(), upper.next()) {
        (Some(only), None) => only,
        _ => value,
    }
}

fn single_lower(value: char) -> char {
    let mut lower = value.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(only), None) => only,
        _ => value,
    }
}

pub fn utf16_length(text: &str) -> usize {
    text.encode_utf16().count()
}
