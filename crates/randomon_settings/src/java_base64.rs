use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::error::SettingsError;

const PADDING: u8 = b'=';

pub fn encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub fn decode(text: &str) -> Result<Vec<u8>, SettingsError> {
    let latin1: Vec<u8> = text.chars().map(latin1_or_question_mark).collect();
    decode_latin1(&latin1)
}

fn latin1_or_question_mark(c: char) -> u8 {
    u8::try_from(u32::from(c)).unwrap_or(b'?')
}

fn sextet(byte: u8) -> Option<u32> {
    let value = match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    };
    Some(u32::from(value))
}

fn invalid(reason: &'static str) -> SettingsError {
    SettingsError::InvalidBase64(reason)
}

fn decode_latin1(source: &[u8]) -> Result<Vec<u8>, SettingsError> {
    if source.len() == 1 {
        return Err(invalid(
            "input should at least have 2 bytes for base64 bytes",
        ));
    }
    let mut output = Vec::with_capacity(source.len() / 4 * 3 + 3);
    let mut bits: u32 = 0;
    let mut shift: i32 = 18;
    let mut position = 0;
    while position < source.len() {
        let byte = source[position];
        position += 1;
        let Some(value) = sextet(byte) else {
            if byte != PADDING {
                return Err(invalid("illegal base64 character"));
            }
            let padding_completes_unit =
                (shift == 6 && source.get(position) == Some(&PADDING)) || shift == 12 || shift == 0;
            if !padding_completes_unit {
                return Err(invalid("input byte array has wrong 4-byte ending unit"));
            }
            if shift == 6 {
                position += 1;
            }
            break;
        };
        bits |= value << shift;
        shift -= 6;
        if shift < 0 {
            output.extend_from_slice(&[(bits >> 16) as u8, (bits >> 8) as u8, bits as u8]);
            shift = 18;
            bits = 0;
        }
    }
    match shift {
        6 => output.push((bits >> 16) as u8),
        0 => output.extend_from_slice(&[(bits >> 16) as u8, (bits >> 8) as u8]),
        12 => return Err(invalid("last unit does not have enough valid bits")),
        _ => {}
    }
    if position < source.len() {
        return Err(invalid("input byte array has incorrect ending byte"));
    }
    Ok(output)
}
