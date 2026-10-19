use crate::bytes::{bytes_at, i32_at, u16_at, u32_at};
use crate::error::{Result, invalid};

const CONTEXT: &str = "text file";
const KEY_BASE: u16 = 0x7C89;
const KEY_ADVANCE: u16 = 0x2983;
const HEADER_SIZE: usize = 0x10;
const LINE_ENTRY_SIZE: usize = 8;

const TERMINATOR: u16 = 0x0000;
const VARIABLE: u16 = 0x0010;
const TEXT_RETURN: u16 = 0xBE00;
const TEXT_CLEAR: u16 = 0xBE01;
const TEXT_WAIT: u16 = 0xBE02;
const TEXT_NULL: u16 = 0xBDFF;
const TEXT_RUBY: u16 = 0xFF01;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLine {
    pub units: Vec<u16>,
    pub flags: u16,
}

impl TextLine {
    pub fn from_text(text: &str, flags: u16) -> Result<Self> {
        Ok(TextLine {
            units: encode_line(text)?,
            flags,
        })
    }

    pub fn text(&self) -> String {
        decode_line(&self.units)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFile {
    pub lines: Vec<TextLine>,
}

fn line_key(index: usize) -> u16 {
    (0..index).fold(KEY_BASE, |key, _| key.wrapping_add(KEY_ADVANCE))
}

fn crypt(units: &mut [u16], mut key: u16) {
    for unit in units {
        *unit ^= key;
        key = key.rotate_left(3);
    }
}

impl TextFile {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let sections = u16_at(data, 0, CONTEXT)?;
        let line_count = usize::from(u16_at(data, 2, CONTEXT)?);
        let total_length = u32_at(data, 4, CONTEXT)? as usize;
        let initial_key = u32_at(data, 8, CONTEXT)?;
        let section_offset = u32_at(data, 12, CONTEXT)? as usize;
        if sections != 1 || initial_key != 0 {
            return Err(invalid(CONTEXT, "unsupported section count or initial key"));
        }
        if section_offset.checked_add(total_length) != Some(data.len()) {
            return Err(invalid(
                CONTEXT,
                "section size does not match the file size",
            ));
        }
        if u32_at(data, section_offset, CONTEXT)? as usize != total_length {
            return Err(invalid(CONTEXT, "section length does not match the header"));
        }
        let mut key = KEY_BASE;
        let lines = (0..line_count)
            .map(|index| {
                let entry = section_offset + 4 + index * LINE_ENTRY_SIZE;
                let offset = usize::try_from(i32_at(data, entry, CONTEXT)?)
                    .map_err(|_| invalid(CONTEXT, format!("line {index} has a negative offset")))?;
                let length = usize::from(u16_at(data, entry + 4, CONTEXT)?);
                let flags = u16_at(data, entry + 6, CONTEXT)?;
                let bytes = bytes_at(data, section_offset + offset, length * 2, CONTEXT)?;
                let mut units: Vec<u16> = bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u16::from_le_bytes(*pair))
                    .collect();
                crypt(&mut units, key);
                key = key.wrapping_add(KEY_ADVANCE);
                Ok(TextLine { units, flags })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(TextFile { lines })
    }

    pub fn from_lines<S: AsRef<str>>(lines: &[S]) -> Result<Self> {
        let lines = lines
            .iter()
            .map(|line| TextLine::from_text(line.as_ref(), 0))
            .collect::<Result<Vec<_>>>()?;
        Ok(TextFile { lines })
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn strings(&self) -> Vec<String> {
        self.lines.iter().map(TextLine::text).collect()
    }

    pub fn line(&self, index: usize) -> Option<String> {
        self.lines.get(index).map(TextLine::text)
    }

    pub fn set_line(&mut self, index: usize, text: &str) -> Result<()> {
        let line_count = self.lines.len();
        let line = self
            .lines
            .get_mut(index)
            .ok_or_else(|| invalid(CONTEXT, format!("line {index} of {line_count}")))?;
        line.units = encode_line(text)?;
        Ok(())
    }

    pub fn set_strings<S: AsRef<str>>(&mut self, lines: &[S]) -> Result<()> {
        if lines.len() != self.lines.len() {
            return Err(invalid(
                CONTEXT,
                format!("expected {} lines, got {}", self.lines.len(), lines.len()),
            ));
        }
        for (index, line) in lines.iter().enumerate() {
            self.set_line(index, line.as_ref())?;
        }
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let line_count =
            u16::try_from(self.lines.len()).map_err(|_| invalid(CONTEXT, "too many lines"))?;
        let mut section = vec![0u8; 4 + self.lines.len() * LINE_ENTRY_SIZE];
        for (index, line) in self.lines.iter().enumerate() {
            let length = u16::try_from(line.units.len())
                .map_err(|_| invalid(CONTEXT, format!("line {index} is too long")))?;
            let offset = section.len() as i32;
            let entry = 4 + index * LINE_ENTRY_SIZE;
            section[entry..entry + 4].copy_from_slice(&offset.to_le_bytes());
            section[entry + 4..entry + 6].copy_from_slice(&length.to_le_bytes());
            section[entry + 6..entry + 8].copy_from_slice(&line.flags.to_le_bytes());
            let mut units = line.units.clone();
            crypt(&mut units, line_key(index));
            section.extend(units.iter().flat_map(|unit| unit.to_le_bytes()));
            if section.len() % 4 == 2 {
                section.extend_from_slice(&[0, 0]);
            }
        }
        let section_length = section.len() as u32;
        section[0..4].copy_from_slice(&section_length.to_le_bytes());
        let mut output = Vec::with_capacity(HEADER_SIZE + section.len());
        output.extend_from_slice(&1u16.to_le_bytes());
        output.extend_from_slice(&line_count.to_le_bytes());
        output.extend_from_slice(&section_length.to_le_bytes());
        output.extend_from_slice(&0u32.to_le_bytes());
        output.extend_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        output.extend_from_slice(&section);
        Ok(output)
    }
}

pub fn decode_line(units: &[u16]) -> String {
    let mut output = String::new();
    append_units(units, &mut output);
    output
}

fn append_units(units: &[u16], output: &mut String) {
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        index += 1;
        match unit {
            TERMINATOR => break,
            VARIABLE => index = append_variable(units, index, output),
            0x000A => output.push_str("\\n"),
            0x005C => output.push_str("\\\\"),
            0x005B => output.push_str("\\["),
            0x007B => output.push_str("\\{"),
            0xD800..=0xDBFF if matches!(units.get(index), Some(0xDC00..=0xDFFF)) => {
                let pair = [unit, units[index]];
                index += 1;
                output.extend(char::decode_utf16(pair).map(|result| result.unwrap_or('\u{FFFD}')));
            }
            0xD800..=0xDFFF => output.push_str(&format!("[RAW {unit:04X}]")),
            _ => output.push(char::from_u32(u32::from(unit)).unwrap_or('\u{FFFD}')),
        }
    }
}

fn append_raw_until_terminator(units: &[u16], start: usize, output: &mut String) -> usize {
    let end = units[start..]
        .iter()
        .position(|&unit| unit == TERMINATOR)
        .map_or(units.len(), |offset| start + offset);
    for unit in &units[start..end] {
        output.push_str(&format!("[RAW {unit:04X}]"));
    }
    end
}

fn append_variable(units: &[u16], start: usize, output: &mut String) -> usize {
    let (Some(&count), Some(&variable)) = (units.get(start), units.get(start + 1)) else {
        return append_raw_until_terminator(units, start - 1, output);
    };
    let arguments_start = start + 2;
    if variable == TEXT_RUBY
        && let Some(end) = ruby_end(units, arguments_start, count)
        && append_ruby(&units[arguments_start..end], output)
    {
        return end;
    }
    let argument_count = usize::from(count).saturating_sub(1);
    if count == 0 || arguments_start + argument_count > units.len() {
        return append_raw_until_terminator(units, start - 1, output);
    }
    let arguments = &units[arguments_start..arguments_start + argument_count];
    match (variable, arguments) {
        (TEXT_RETURN, []) => output.push_str("\\r"),
        (TEXT_CLEAR, []) => output.push_str("\\c"),
        (TEXT_WAIT, [time]) => output.push_str(&format!("[WAIT {time}]")),
        (TEXT_NULL, [line]) => output.push_str(&format!("[~ {line}]")),
        _ => {
            output.push_str(&format!("[VAR {variable:04X}"));
            if !arguments.is_empty() {
                let joined: Vec<String> = arguments
                    .iter()
                    .map(|argument| format!("{argument:04X}"))
                    .collect();
                output.push_str(&format!("({})", joined.join(",")));
            }
            output.push(']');
        }
    }
    arguments_start + argument_count
}

fn ruby_end(units: &[u16], arguments_start: usize, count: u16) -> Option<usize> {
    let base = usize::from(*units.get(arguments_start)?);
    let ruby = usize::from(*units.get(arguments_start + 1)?);
    let end = arguments_start + 2 + base * 2 + ruby;
    (usize::from(count) == 3 + base + ruby && end <= units.len()).then_some(end)
}

fn append_ruby(arguments: &[u16], output: &mut String) -> bool {
    let [base_length, ruby_length, rest @ ..] = arguments else {
        return false;
    };
    let base = usize::from(*base_length);
    let ruby = usize::from(*ruby_length);
    if rest.len() != base * 2 + ruby {
        return false;
    }
    let (first, remainder) = rest.split_at(base);
    let (reading, second) = remainder.split_at(ruby);
    let mut text = String::new();
    append_units(first, &mut text);
    text.push('|');
    append_units(reading, &mut text);
    if first != second {
        text.push('|');
        append_units(second, &mut text);
    }
    output.push('{');
    output.push_str(&text);
    output.push('}');
    true
}

pub fn encode_line(text: &str) -> Result<Vec<u16>> {
    let mut units = encode_units(text)?;
    units.push(TERMINATOR);
    Ok(units)
}

fn encode_units(text: &str) -> Result<Vec<u16>> {
    let characters: Vec<char> = text.chars().collect();
    let mut units = Vec::with_capacity(characters.len());
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        index += 1;
        match character {
            '\\' => {
                let escaped = characters
                    .get(index)
                    .ok_or_else(|| invalid(CONTEXT, format!("dangling escape in {text:?}")))?;
                index += 1;
                units.extend(escape_units(*escaped, text)?);
            }
            '[' => {
                let close = find_closing(&characters, index, ']', text)?;
                let command: String = characters[index..close].iter().collect();
                units.extend(command_units(&command)?);
                index = close + 1;
            }
            '{' => {
                let close = find_closing(&characters, index, '}', text)?;
                let ruby: String = characters[index..close].iter().collect();
                units.extend(ruby_units(&ruby)?);
                index = close + 1;
            }
            _ => {
                let mut buffer = [0u16; 2];
                units.extend_from_slice(character.encode_utf16(&mut buffer));
            }
        }
    }
    Ok(units)
}

fn find_closing(characters: &[char], start: usize, close: char, text: &str) -> Result<usize> {
    characters[start..]
        .iter()
        .position(|&character| character == close)
        .map(|offset| start + offset)
        .ok_or_else(|| invalid(CONTEXT, format!("unterminated {close:?} in {text:?}")))
}

fn escape_units(escaped: char, text: &str) -> Result<Vec<u16>> {
    match escaped {
        'n' => Ok(vec![0x000A]),
        '\\' => Ok(vec![0x005C]),
        '[' => Ok(vec![0x005B]),
        '{' => Ok(vec![0x007B]),
        'r' => Ok(vec![VARIABLE, 1, TEXT_RETURN]),
        'c' => Ok(vec![VARIABLE, 1, TEXT_CLEAR]),
        other => Err(invalid(
            CONTEXT,
            format!("unknown escape \\{other} in {text:?}"),
        )),
    }
}

fn parse_hex(text: &str) -> Result<u16> {
    u16::from_str_radix(text.trim(), 16)
        .map_err(|_| invalid(CONTEXT, format!("invalid hex value {text:?}")))
}

fn parse_decimal(text: &str) -> Result<u16> {
    text.trim()
        .parse::<u16>()
        .map_err(|_| invalid(CONTEXT, format!("invalid number {text:?}")))
}

fn command_units(command: &str) -> Result<Vec<u16>> {
    let (name, argument) = command
        .split_once(' ')
        .ok_or_else(|| invalid(CONTEXT, format!("malformed command [{command}]")))?;
    match name {
        "~" => Ok(vec![VARIABLE, 2, TEXT_NULL, parse_decimal(argument)?]),
        "WAIT" => Ok(vec![VARIABLE, 2, TEXT_WAIT, parse_decimal(argument)?]),
        "RAW" => Ok(vec![parse_hex(argument)?]),
        "VAR" => variable_units(argument),
        _ => Err(invalid(CONTEXT, format!("unknown command [{command}]"))),
    }
}

fn variable_units(argument: &str) -> Result<Vec<u16>> {
    let (variable, arguments) = match argument.split_once('(') {
        Some((variable, rest)) => {
            let inner = rest.strip_suffix(')').ok_or_else(|| {
                invalid(
                    CONTEXT,
                    format!("unterminated arguments in [VAR {argument}]"),
                )
            })?;
            let values = inner
                .split(',')
                .map(parse_hex)
                .collect::<Result<Vec<_>>>()?;
            (variable, values)
        }
        None => (argument, Vec::new()),
    };
    let count = u16::try_from(arguments.len() + 1)
        .map_err(|_| invalid(CONTEXT, "too many variable arguments"))?;
    let mut units = vec![VARIABLE, count, parse_hex(variable)?];
    units.extend(arguments);
    Ok(units)
}

fn ruby_units(ruby: &str) -> Result<Vec<u16>> {
    let parts: Vec<&str> = ruby.split('|').collect();
    let (base, reading, second) = match parts.as_slice() {
        [base, reading] => (*base, *reading, *base),
        [base, reading, second] => (*base, *reading, *second),
        _ => return Err(invalid(CONTEXT, format!("malformed ruby text {{{ruby}}}"))),
    };
    let base_units = encode_units(base)?;
    let reading_units = encode_units(reading)?;
    let second_units = encode_units(second)?;
    if base_units.len() != second_units.len() {
        return Err(invalid(
            CONTEXT,
            format!("ruby base texts differ in length in {{{ruby}}}"),
        ));
    }
    let count = u16::try_from(3 + base_units.len() + reading_units.len())
        .map_err(|_| invalid(CONTEXT, "ruby text is too long"))?;
    let mut units = vec![
        VARIABLE,
        count,
        TEXT_RUBY,
        base_units.len() as u16,
        reading_units.len() as u16,
    ];
    units.extend(base_units);
    units.extend(reading_units);
    units.extend(second_units);
    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_lines_round_trip_through_bytes() {
        let file = TextFile::from_lines(&["Bulbasaur", "Ivysaur", ""]).expect("encode");
        let bytes = file.to_bytes().expect("write");
        let parsed = TextFile::parse(&bytes).expect("parse");
        assert_eq!(parsed.strings(), vec!["Bulbasaur", "Ivysaur", ""]);
        assert_eq!(parsed.to_bytes().expect("rewrite"), bytes);
    }

    #[test]
    fn lines_are_encrypted_on_disk() {
        let bytes = TextFile::from_lines(&["AAAA"])
            .expect("encode")
            .to_bytes()
            .expect("write");
        assert!(!bytes.windows(2).any(|pair| pair == [0x41, 0x00]));
        let first = u16::from_le_bytes([bytes[0x1C], bytes[0x1D]]);
        assert_eq!(first, 0x41 ^ KEY_BASE);
    }

    #[test]
    fn control_codes_round_trip_through_text() {
        let samples = [
            "Line one\\nLine two",
            "Wait\\rScroll\\cClear",
            "[VAR 0100(0000)] got [VAR 0201]!",
            "[WAIT 30]pause",
            "[~ 12]",
            "Escaped \\[ and \\{ and \\\\",
            "{漢字|かんじ}",
            "Male ♂ and \u{E08F}",
        ];
        for sample in samples {
            let units = encode_line(sample).expect("encode");
            assert_eq!(decode_line(&units), sample, "{sample}");
            assert_eq!(encode_line(&decode_line(&units)).expect("reencode"), units);
        }
    }

    #[test]
    fn replaced_lines_change_length_and_keep_order() {
        let mut file = TextFile::from_lines(&["a", "b", "c"]).expect("encode");
        file.set_line(1, "a much longer line").expect("set");
        let parsed = TextFile::parse(&file.to_bytes().expect("write")).expect("parse");
        assert_eq!(parsed.strings(), vec!["a", "a much longer line", "c"]);
        assert!(file.set_line(3, "x").is_err());
        assert!(file.set_strings(&["only one"]).is_err());
    }

    #[test]
    fn malformed_text_is_rejected() {
        assert!(encode_line("[VAR").is_err());
        assert!(encode_line("{no reading}").is_err());
        assert!(encode_line("trailing \\").is_err());
        assert!(encode_line("[NOPE 1]").is_err());
        assert!(encode_line("[VAR 12(zz)]").is_err());
    }

    #[test]
    fn malformed_files_are_rejected() {
        assert!(TextFile::parse(&[]).is_err());
        let mut bytes = TextFile::from_lines(&["abc"])
            .expect("encode")
            .to_bytes()
            .expect("write");
        bytes[4] ^= 1;
        assert!(TextFile::parse(&bytes).is_err());
        let mut bytes = TextFile::from_lines(&["abc"])
            .expect("encode")
            .to_bytes()
            .expect("write");
        bytes[0x18] = 0xFF;
        assert!(TextFile::parse(&bytes).is_err());
    }

    #[test]
    fn truncated_variables_decode_as_raw_units() {
        let units = [0x41, VARIABLE, 5, 0x1234, TERMINATOR];
        let text = decode_line(&units);
        assert_eq!(text, "A[RAW 0010][RAW 0005][RAW 1234]");
        assert_eq!(encode_line(&text).expect("encode"), units);
    }
}
