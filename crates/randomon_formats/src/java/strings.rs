const REPLACEMENT: char = '\u{FFFD}';

pub fn decode_us_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| {
            if byte < 0x80 {
                byte as char
            } else {
                REPLACEMENT
            }
        })
        .collect()
}

pub fn encode_us_ascii(text: &str) -> Vec<u8> {
    text.chars()
        .map(|character| {
            if (character as u32) < 0x80 {
                character as u8
            } else {
                b'?'
            }
        })
        .collect()
}

pub fn decode_utf16le(bytes: &[u8]) -> String {
    let unit_at = |index: usize| u16::from_le_bytes([bytes[index], bytes[index + 1]]);
    let is_high = |unit: u16| (0xD800..0xDC00).contains(&unit);
    let is_low = |unit: u16| (0xDC00..0xE000).contains(&unit);
    let mut text = String::new();
    let mut index = 0;
    while index + 1 < bytes.len() {
        let unit = unit_at(index);
        if is_high(unit) {
            if bytes.len() - (index + 2) < 2 {
                text.push(REPLACEMENT);
                return text;
            }
            let next = unit_at(index + 2);
            if is_low(next) {
                let combined =
                    0x10000 + (((unit as u32) - 0xD800) << 10) + ((next as u32) - 0xDC00);
                text.push(char::from_u32(combined).unwrap_or(REPLACEMENT));
            } else {
                text.push(REPLACEMENT);
            }
            index += 4;
        } else if is_low(unit) {
            text.push(REPLACEMENT);
            index += 2;
        } else {
            text.push(char::from_u32(unit as u32).unwrap_or(REPLACEMENT));
            index += 2;
        }
    }
    if index < bytes.len() {
        text.push(REPLACEMENT);
    }
    text
}

pub fn encode_utf16le(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

pub fn decode_utf8(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub fn java_trim(text: &str) -> &str {
    text.trim_matches(|character: char| character <= ' ')
}

pub fn sanitize_path(path: &str, keep_dots: bool) -> String {
    path.chars()
        .filter(|&character| {
            character.is_ascii_alphanumeric() || character == '_' || (keep_dots && character == '.')
        })
        .collect()
}
