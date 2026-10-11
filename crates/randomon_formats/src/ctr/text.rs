use thiserror::Error;

const KEY_BASE: u16 = 0x7C89;
const KEY_ADVANCE: u16 = 0x2983;
const KEY_TERMINATOR: u16 = 0x0000;
const KEY_VARIABLE: u16 = 0x0010;
const KEY_TEXT_RETURN: u16 = 0xBE00;
const KEY_TEXT_CLEAR: u16 = 0xBE01;
const KEY_TEXT_WAIT: u16 = 0xBE02;
const KEY_TEXT_NULL: u16 = 0xBDFF;
const NEWLINE: u16 = b'\n' as u16;
const BACKSLASH: u16 = b'\\' as u16;
const OPEN_BRACKET: u16 = b'[' as u16;
const CLOSE_BRACKET: u16 = b']' as u16;

const TEXT_VARIABLE_CODES: &[(u16, &str)] = &[
    (0xFF00, "COLOR"),
    (0x0100, "TRNAME"),
    (0x0101, "PKNAME"),
    (0x0102, "PKNICK"),
    (0x0103, "TYPE"),
    (0x0105, "LOCATION"),
    (0x0106, "ABILITY"),
    (0x0107, "MOVE"),
    (0x0108, "ITEM1"),
    (0x0109, "ITEM2"),
    (0x010A, "sTRBAG"),
    (0x010B, "BOX"),
    (0x010D, "EVSTAT"),
    (0x0110, "OPOWER"),
    (0x0127, "RIBBON"),
    (0x0134, "MIINAME"),
    (0x013E, "WEATHER"),
    (0x0189, "TRNICK"),
    (0x018A, "1stchrTR"),
    (0x018B, "SHOUTOUT"),
    (0x018E, "BERRY"),
    (0x018F, "REMFEEL"),
    (0x0190, "REMQUAL"),
    (0x0191, "WEBSITE"),
    (0x019C, "CHOICECOS"),
    (0x01A1, "GSYNCID"),
    (0x0192, "PRVIDSAY"),
    (0x0193, "BTLTEST"),
    (0x0195, "GENLOC"),
    (0x0199, "CHOICEFOOD"),
    (0x019A, "HOTELITEM"),
    (0x019B, "TAXISTOP"),
    (0x019F, "MAISTITLE"),
    (0x1000, "ITEMPLUR0"),
    (0x1001, "ITEMPLUR1"),
    (0x1100, "GENDBR"),
    (0x1101, "NUMBRNCH"),
    (0x1302, "iCOLOR2"),
    (0x1303, "iCOLOR3"),
    (0x0200, "NUM1"),
    (0x0201, "NUM2"),
    (0x0202, "NUM3"),
    (0x0203, "NUM4"),
    (0x0204, "NUM5"),
    (0x0205, "NUM6"),
    (0x0206, "NUM7"),
    (0x0207, "NUM8"),
    (0x0208, "NUM9"),
];

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TextError {
    #[error("java.io.IOException: {0}")]
    Rejected(String),
    #[error("{0}")]
    Crashed(String),
}

type TextResult<T> = Result<T, TextError>;

fn out_of_bounds(index: i64) -> TextError {
    TextError::Crashed(format!("java.lang.ArrayIndexOutOfBoundsException: {index}"))
}

fn read_short(data: &[u8], offset: i64) -> TextResult<u16> {
    if offset < 0 || offset + 1 >= data.len() as i64 {
        return Err(out_of_bounds(offset.max(data.len() as i64)));
    }
    let start = offset as usize;
    Ok(u16::from_le_bytes([data[start], data[start + 1]]))
}

fn read_int(data: &[u8], offset: i64) -> TextResult<i32> {
    if offset < 0 || offset + 3 >= data.len() as i64 {
        return Err(out_of_bounds(offset.max(data.len() as i64)));
    }
    let start = offset as usize;
    Ok(i32::from_le_bytes([
        data[start],
        data[start + 1],
        data[start + 2],
        data[start + 3],
    ]))
}

fn copy_of_range(data: &[u8], from: i64, to: i64) -> TextResult<Vec<u8>> {
    if from > to {
        return Err(TextError::Crashed(format!(
            "java.lang.IllegalArgumentException: {from} > {to}"
        )));
    }
    if from < 0 || from > data.len() as i64 {
        return Err(out_of_bounds(from));
    }
    let start = from as usize;
    let mut copy = vec![0u8; (to - from) as usize];
    let available = (data.len() - start).min(copy.len());
    copy[..available].copy_from_slice(&data[start..start + available]);
    Ok(copy)
}

fn entry_key(index: usize) -> u16 {
    (0..index).fold(KEY_BASE, |key, _| key.wrapping_add(KEY_ADVANCE))
}

fn crypt_entry_data(data: &[u8], key: u16) -> Vec<u8> {
    let mut result = vec![0u8; data.len()];
    let mut key = key;
    for (output, input) in result.chunks_mut(2).zip(data.chunks(2)) {
        let low = input[0];
        let high = input.get(1).copied().unwrap_or(0);
        let value = u16::from_le_bytes([low, high]) ^ key;
        let bytes = value.to_le_bytes();
        output[0] = bytes[0];
        if output.len() > 1 {
            output[1] = bytes[1];
        }
        key = key.rotate_left(3);
    }
    result
}

fn variable_name(code: u16) -> String {
    TEXT_VARIABLE_CODES
        .iter()
        .find(|(candidate, _)| *candidate == code)
        .map_or_else(|| format!("{code:04X}"), |(_, name)| (*name).to_owned())
}

fn variable_code(name: &str) -> Option<u16> {
    TEXT_VARIABLE_CODES
        .iter()
        .find(|(_, candidate)| *candidate == name)
        .map(|(code, _)| *code)
}

fn unmap_char(value: u16) -> u16 {
    match value {
        0xE07F => 0x202F,
        0xE08D => 0x2026,
        0xE08E => 0x2642,
        0xE08F => 0x2640,
        other => other,
    }
}

fn remap_char(value: u16) -> u16 {
    match value {
        0x202F => 0xE07F,
        0x2026 => 0xE08D,
        0x2642 => 0xE08E,
        0x2640 => 0xE08F,
        other => other,
    }
}

fn push_str(units: &mut Vec<u16>, text: &str) {
    units.extend(text.encode_utf16());
}

fn variable_string(data: &[u8], position: &mut i64) -> TextResult<String> {
    let mut count = read_short(data, *position)?;
    *position += 2;
    let variable = read_short(data, *position)?;
    *position += 2;
    match variable {
        KEY_TEXT_RETURN => return Ok("\\r".to_owned()),
        KEY_TEXT_CLEAR => return Ok("\\c".to_owned()),
        KEY_TEXT_WAIT => {
            let time = read_short(data, *position)?;
            *position += 2;
            return Ok(format!("[WAIT {time}]"));
        }
        KEY_TEXT_NULL => {
            let line = read_short(data, *position)?;
            *position += 2;
            return Ok(format!("[~ {line}]"));
        }
        _ => {}
    }
    let mut text = format!("[VAR {}", variable_name(variable));
    if count > 1 {
        text.push('(');
        while count > 1 && *position < data.len() as i64 {
            let argument = read_short(data, *position)?;
            *position += 2;
            text.push_str(&format!("{argument:04X}"));
            count -= 1;
            if count == 1 || *position >= data.len() as i64 {
                break;
            }
            text.push(',');
        }
        text.push(')');
    }
    text.push(']');
    Ok(text)
}

fn entry_string(data: &[u8]) -> TextResult<String> {
    let mut units: Vec<u16> = Vec::new();
    let mut position: i64 = 0;
    while position < data.len() as i64 {
        let value = read_short(data, position)?;
        if value == KEY_TERMINATOR {
            break;
        }
        position += 2;
        match value {
            KEY_VARIABLE => {
                let variable = variable_string(data, &mut position)?;
                push_str(&mut units, &variable);
            }
            NEWLINE => push_str(&mut units, "\\n"),
            BACKSLASH => push_str(&mut units, "\\\\"),
            OPEN_BRACKET => push_str(&mut units, "\\["),
            other => units.push(unmap_char(other)),
        }
    }
    Ok(String::from_utf16_lossy(&units))
}

pub fn read_texts(data: &[u8]) -> TextResult<Vec<String>> {
    let section_count = read_short(data, 0)?;
    let entry_count = read_short(data, 2)? as usize;
    let total_length = read_int(data, 4)?;
    let initial_key = read_int(data, 8)?;
    let section_data_offset = read_int(data, 0xC)? as i64;
    let section_length = read_int(data, section_data_offset)?;
    if section_count != 1 || initial_key != 0 || section_length != total_length {
        return Ok(Vec::new());
    }
    let mut strings = Vec::with_capacity(entry_count);
    for index in 0..entry_count {
        let table_position = index as i64 * 8 + section_data_offset;
        let offset = read_int(data, table_position + 4)? as i64 + section_data_offset;
        let length = read_short(data, table_position + 8)? as i64;
        let encrypted = copy_of_range(data, offset, offset + length * 2)?;
        strings.push(entry_string(&crypt_entry_data(
            &encrypted,
            entry_key(index),
        ))?);
    }
    Ok(strings)
}

fn java_split(text: &str, separator: char) -> Vec<&str> {
    if !text.contains(separator) {
        return vec![text];
    }
    let mut parts: Vec<&str> = text.split(separator).collect();
    while parts.last() == Some(&"") {
        parts.pop();
    }
    parts
}

fn number_format_error(text: &str, radix: u32) -> TextError {
    let suffix = if radix == 10 {
        String::new()
    } else {
        format!(" under radix {radix}")
    };
    TextError::Crashed(format!(
        "java.lang.NumberFormatException: For input string: \"{text}\"{suffix}"
    ))
}

fn parse_java_int(text: &str, radix: u32) -> TextResult<i32> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    if digits.is_empty() {
        return Err(number_format_error(text, radix));
    }
    let mut value: i64 = 0;
    for character in digits.chars() {
        let digit = character
            .to_digit(radix)
            .ok_or_else(|| number_format_error(text, radix))?;
        value = value * radix as i64 + digit as i64;
        if value > i32::MAX as i64 + 1 {
            return Err(number_format_error(text, radix));
        }
    }
    let signed = if negative { -value } else { value };
    i32::try_from(signed).map_err(|_| number_format_error(text, radix))
}

fn variable_number(variable: &str) -> TextResult<i32> {
    if let Some(code) = variable_code(variable) {
        return Ok(code as i32);
    }
    parse_java_int(variable, 10)
        .map_err(|_| TextError::Rejected(format!("Variable parse error: {variable}")))
}

fn variable_parameters(text: &str) -> TextResult<Vec<i32>> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let bracket = units.iter().position(|&unit| unit == b'(' as u16);
    let Some(bracket) = bracket else {
        return Ok(vec![1, variable_number(text)?]);
    };
    let variable = String::from_utf16_lossy(&units[..bracket]);
    let variable_value = variable_number(&variable)?;
    let arguments_end = units.len().checked_sub(1).filter(|end| *end > bracket);
    let Some(arguments_end) = arguments_end else {
        return Err(TextError::Crashed(format!(
            "java.lang.StringIndexOutOfBoundsException: begin {}, end {}, length {}",
            bracket + 1,
            units.len() as i64 - 1,
            units.len()
        )));
    };
    let arguments_text = String::from_utf16_lossy(&units[bracket + 1..arguments_end]);
    let arguments = java_split(&arguments_text, ',');
    let mut values = vec![1 + arguments.len() as i32, variable_value];
    for argument in arguments {
        values.push(parse_java_int(argument, 16)?);
    }
    Ok(values)
}

fn variable_values(variable: &str) -> TextResult<Vec<i32>> {
    let parts = java_split(variable, ' ');
    if parts.len() < 2 {
        return Err(TextError::Rejected(format!(
            "Incorrectly formatted variable text: {variable}"
        )));
    }
    let mut values = vec![KEY_VARIABLE as i32];
    match parts[0] {
        "~" => values.extend([1, KEY_TEXT_NULL as i32, parse_java_int(parts[1], 10)?]),
        "WAIT" => values.extend([1, KEY_TEXT_WAIT as i32, parse_java_int(parts[1], 10)?]),
        "VAR" => values.extend(variable_parameters(parts[1])?),
        _ => {
            return Err(TextError::Rejected(format!(
                "Unknown variable method type: {variable}"
            )));
        }
    }
    Ok(values)
}

fn escape_values(escape: u16) -> TextResult<Vec<i32>> {
    match escape {
        unit if unit == b'n' as u16 => Ok(vec![NEWLINE as i32]),
        BACKSLASH => Ok(vec![BACKSLASH as i32]),
        OPEN_BRACKET => Ok(vec![OPEN_BRACKET as i32]),
        unit if unit == b'r' as u16 => Ok(vec![KEY_VARIABLE as i32, 1, KEY_TEXT_RETURN as i32]),
        unit if unit == b'c' as u16 => Ok(vec![KEY_VARIABLE as i32, 1, KEY_TEXT_CLEAR as i32]),
        other => Err(TextError::Rejected(format!(
            "Invalid terminated line: \\{}",
            String::from_utf16_lossy(&[other])
        ))),
    }
}

fn push_unit(output: &mut Vec<u8>, value: i32) {
    output.push((value & 0xFF) as u8);
    output.push(((value >> 8) & 0xFF) as u8);
}

fn entry_data(entry: &str) -> TextResult<Vec<u8>> {
    let units: Vec<u16> = entry.encode_utf16().collect();
    let mut output = Vec::new();
    let mut index = 0;
    while index < units.len() {
        let value = remap_char(units[index]);
        index += 1;
        if value == OPEN_BRACKET {
            let Some(length) = units[index..]
                .iter()
                .position(|&unit| unit == CLOSE_BRACKET)
            else {
                return Err(TextError::Rejected(format!(
                    "Variable text is not capped properly: {entry}"
                )));
            };
            let variable = String::from_utf16_lossy(&units[index..index + length]);
            for value in variable_values(&variable)? {
                push_unit(&mut output, value);
            }
            index += 1 + length;
        } else if value == BACKSLASH {
            let Some(&escape) = units.get(index) else {
                return Err(TextError::Crashed(format!(
                    "java.lang.StringIndexOutOfBoundsException: index {index}, length {}",
                    units.len()
                )));
            };
            index += 1;
            for value in escape_values(escape)? {
                push_unit(&mut output, value);
            }
        } else {
            push_unit(&mut output, value as i32);
        }
    }
    push_unit(&mut output, KEY_TERMINATOR as i32);
    Ok(output)
}

fn java_trim(text: &str) -> &str {
    text.trim_matches(|character: char| character <= ' ')
}

pub fn save_entry(values: &[String]) -> TextResult<Vec<u8>> {
    let mut key = KEY_BASE;
    let mut data: Vec<u8> = Vec::new();
    let mut offsets: Vec<u8> = Vec::with_capacity(values.len() * 8);
    let data_offset = 4 + values.len() as i32 * 8;
    for value in values {
        let decrypted = entry_data(java_trim(value))?;
        let encrypted = crypt_entry_data(&decrypted, key);
        offsets.extend_from_slice(&(data_offset + data.len() as i32).to_le_bytes());
        offsets.extend_from_slice(&((encrypted.len() / 2) as u16).to_le_bytes());
        offsets.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&encrypted);
        if encrypted.len() % 4 == 2 {
            data.extend_from_slice(&[0, 0]);
        }
        key = key.wrapping_add(KEY_ADVANCE);
    }
    let section_length = data_offset + data.len() as i32;
    let mut output = Vec::with_capacity(0x14 + offsets.len() + data.len());
    output.extend_from_slice(&1u16.to_le_bytes());
    output.extend_from_slice(&(values.len() as u16).to_le_bytes());
    output.extend_from_slice(&section_length.to_le_bytes());
    output.extend_from_slice(&0i32.to_le_bytes());
    output.extend_from_slice(&0x10i32.to_le_bytes());
    output.extend_from_slice(&section_length.to_le_bytes());
    output.extend_from_slice(&offsets);
    output.extend_from_slice(&data);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn round_trips_plain_and_variable_text() {
        let values = strings(&[
            "Hello\\nWorld",
            "[VAR PKNAME(0000)] used [VAR MOVE(0001)]!",
            "Wait[WAIT 30]\\c",
            "[~ 4]",
            "Back\\\\slash \\[bracket",
            "Pok\u{e9}mon \u{2026}\u{2642}\u{2640}",
            "",
        ]);
        let saved = save_entry(&values).unwrap();
        assert_eq!(read_texts(&saved).unwrap(), values);
    }

    #[test]
    fn trims_entries_before_saving() {
        let saved = save_entry(&strings(&["  padded \t"])).unwrap();
        assert_eq!(read_texts(&saved).unwrap(), strings(&["padded"]));
    }

    #[test]
    fn unknown_variables_print_hex_but_parse_decimal() {
        let saved = save_entry(&strings(&["[VAR 0103]"])).unwrap();
        assert_eq!(read_texts(&saved).unwrap(), strings(&["[VAR 0067]"]));
        assert!(matches!(
            save_entry(&strings(&["[VAR 01A2]"])),
            Err(TextError::Rejected(_))
        ));
    }

    #[test]
    fn rejects_like_upstream() {
        assert!(matches!(
            save_entry(&strings(&["[VAR"])),
            Err(TextError::Rejected(_))
        ));
        assert!(matches!(
            save_entry(&strings(&["bad \\q"])),
            Err(TextError::Rejected(_))
        ));
        assert!(matches!(
            save_entry(&strings(&["[WAIT x]"])),
            Err(TextError::Crashed(_))
        ));
    }

    #[test]
    fn invalid_header_reads_as_empty() {
        let mut saved = save_entry(&strings(&["a"])).unwrap();
        saved[0] = 2;
        assert!(read_texts(&saved).unwrap().is_empty());
    }

    #[test]
    fn entry_keys_advance() {
        assert_eq!(entry_key(0), KEY_BASE);
        assert_eq!(
            entry_key(2),
            KEY_BASE.wrapping_add(KEY_ADVANCE).wrapping_add(KEY_ADVANCE)
        );
    }
}
