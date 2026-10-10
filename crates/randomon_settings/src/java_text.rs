const REPLACEMENT: char = '\u{FFFD}';

const DECIMAL_DIGIT_ZEROS: [u16; 37] = [
    0x0030, 0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66, 0x0BE6, 0x0C66, 0x0CE6,
    0x0D66, 0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80,
    0x1A90, 0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0,
    0xFF10,
];

pub fn decode_utf8(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        index += decode_utf8_sequence(&bytes[index..], &mut out);
    }
    out
}

fn decode_utf8_sequence(rest: &[u8], out: &mut String) -> usize {
    let lead = rest[0];
    match lead {
        0x00..=0x7F => {
            out.push(lead as char);
            1
        }
        0xC2..=0xDF => decode_two_byte_sequence(rest, out),
        0xE0..=0xEF => decode_three_byte_sequence(rest, out),
        0xF0..=0xF7 => decode_four_byte_sequence(rest, out),
        _ => {
            out.push(REPLACEMENT);
            1
        }
    }
}

fn is_continuation(byte: u8) -> bool {
    byte & 0xC0 == 0x80
}

fn decode_two_byte_sequence(rest: &[u8], out: &mut String) -> usize {
    let Some(&second) = rest.get(1) else {
        out.push(REPLACEMENT);
        return rest.len();
    };
    if !is_continuation(second) {
        out.push(REPLACEMENT);
        return 1;
    }
    let code_point = (u32::from(rest[0] & 0x1F) << 6) | u32::from(second & 0x3F);
    out.push(char::from_u32(code_point).unwrap_or(REPLACEMENT));
    2
}

fn is_overlong_three_byte_prefix(lead: u8, second: u8) -> bool {
    lead == 0xE0 && second & 0xE0 == 0x80
}

fn is_malformed_three_byte_prefix(lead: u8, second: u8) -> bool {
    is_overlong_three_byte_prefix(lead, second) || !is_continuation(second)
}

fn decode_three_byte_sequence(rest: &[u8], out: &mut String) -> usize {
    let lead = rest[0];
    if rest.len() < 3 {
        out.push(REPLACEMENT);
        let has_bad_second = rest
            .get(1)
            .is_some_and(|&second| is_malformed_three_byte_prefix(lead, second));
        return if has_bad_second { 1 } else { rest.len() };
    }
    let (second, third) = (rest[1], rest[2]);
    if is_malformed_three_byte_prefix(lead, second) {
        out.push(REPLACEMENT);
        return 1;
    }
    if !is_continuation(third) {
        out.push(REPLACEMENT);
        return 2;
    }
    let code_point =
        (u32::from(lead & 0x0F) << 12) | (u32::from(second & 0x3F) << 6) | u32::from(third & 0x3F);
    out.push(char::from_u32(code_point).unwrap_or(REPLACEMENT));
    3
}

fn is_malformed_four_byte_prefix(lead: u8, second: u8) -> bool {
    lead > 0xF4
        || (lead == 0xF0 && !(0x90..=0xBF).contains(&second))
        || (lead == 0xF4 && second & 0xF0 != 0x80)
        || !is_continuation(second)
}

fn decode_four_byte_sequence(rest: &[u8], out: &mut String) -> usize {
    match malformed_four_byte_length(rest) {
        Some(consumed) => {
            out.push(REPLACEMENT);
            consumed
        }
        None => {
            out.push(char::from_u32(four_byte_code_point(rest)).unwrap_or(REPLACEMENT));
            4
        }
    }
}

fn four_byte_code_point(rest: &[u8]) -> u32 {
    (u32::from(rest[0] & 0x07) << 18)
        | (u32::from(rest[1] & 0x3F) << 12)
        | (u32::from(rest[2] & 0x3F) << 6)
        | u32::from(rest[3] & 0x3F)
}

fn malformed_four_byte_length(rest: &[u8]) -> Option<usize> {
    if rest.len() < 4 {
        return Some(truncated_four_byte_length(rest));
    }
    let continuations_valid = rest[1..4].iter().all(|&byte| is_continuation(byte));
    if continuations_valid && (0x10000..=0x10FFFF).contains(&four_byte_code_point(rest)) {
        return None;
    }
    if is_malformed_four_byte_prefix(rest[0], rest[1]) {
        Some(1)
    } else if !is_continuation(rest[2]) {
        Some(2)
    } else {
        Some(3)
    }
}

fn truncated_four_byte_length(rest: &[u8]) -> usize {
    let lead = rest[0];
    let bad_second = rest
        .get(1)
        .is_some_and(|&second| is_malformed_four_byte_prefix(lead, second));
    if lead > 0xF4 || bad_second {
        return 1;
    }
    if rest.get(2).is_some_and(|&third| !is_continuation(third)) {
        return 2;
    }
    rest.len()
}

pub fn java_trim(text: &str) -> &str {
    text.trim_matches(|c: char| c <= ' ')
}

pub fn scanner_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        if !matches!(c, '\n' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}') {
            continue;
        }
        lines.push(&text[start..index]);
        start = index + c.len_utf8();
        if c == '\r' && chars.peek().is_some_and(|&(_, next)| next == '\n') {
            chars.next();
            start += 1;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

pub fn scanner_names(bytes: &[u8]) -> Vec<String> {
    scanner_lines(&decode_utf8(bytes))
        .into_iter()
        .map(java_trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn decimal_digit(unit: u16) -> Option<u32> {
    DECIMAL_DIGIT_ZEROS
        .iter()
        .find(|&&zero| (zero..zero + 10).contains(&unit))
        .map(|&zero| u32::from(unit - zero))
}

pub fn parse_java_int(units: &[u16]) -> Option<i32> {
    let (&first, _) = units.split_first()?;
    let (negative, digits) = match first {
        0x2D => (true, &units[1..]),
        0x2B => (false, &units[1..]),
        _ if first < u16::from(b'0') => return None,
        _ => (false, units),
    };
    if digits.is_empty() {
        return None;
    }
    let limit: i64 = if negative {
        i64::from(i32::MIN)
    } else {
        -i64::from(i32::MAX)
    };
    let mut accumulated: i64 = 0;
    for &unit in digits {
        let digit = i64::from(decimal_digit(unit)?);
        accumulated = accumulated * 10 - digit;
        if accumulated < limit {
            return None;
        }
    }
    let value = if negative { accumulated } else { -accumulated };
    i32::try_from(value).ok()
}

pub fn encode_us_ascii(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
        .collect()
}

pub fn decode_us_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| {
            if byte.is_ascii() {
                byte as char
            } else {
                REPLACEMENT
            }
        })
        .collect()
}
