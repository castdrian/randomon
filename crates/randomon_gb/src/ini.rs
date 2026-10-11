use randomon_settings::java_text::{java_trim, scanner_lines};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StaticOffsets {
    pub species_offsets: Vec<i32>,
    pub level_offsets: Vec<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IniLine<'a> {
    Section(&'a str),
    Entry(&'a str, &'a str),
}

fn without_comment(line: &str) -> &str {
    let line = java_trim(line);
    match line.find("//") {
        Some(comment) => java_trim(&line[..comment]),
        None => line,
    }
}

fn classify_line(line: &str) -> Option<IniLine<'_>> {
    if line.is_empty() {
        return None;
    }
    if line.starts_with('[') && line.ends_with(']') {
        return Some(IniLine::Section(&line[1..line.len() - 1]));
    }
    let (key, value) = line.split_once('=')?;
    Some(IniLine::Entry(java_trim(key), java_trim(value)))
}

pub fn ini_lines(text: &str) -> impl Iterator<Item = IniLine<'_>> {
    scanner_lines(text)
        .into_iter()
        .filter_map(|line| classify_line(without_comment(line)))
}

fn parse_with_radix(text: &str) -> (String, u32) {
    let lowered = java_trim(text).to_lowercase();
    if let Some(stripped) = lowered
        .strip_prefix("0x")
        .or_else(|| lowered.strip_prefix("&h"))
    {
        (stripped.to_owned(), 16)
    } else {
        (lowered, 10)
    }
}

pub fn parse_ri_int(text: &str) -> i32 {
    let (digits, radix) = parse_with_radix(text);
    i32::from_str_radix(&digits, radix).unwrap_or(0)
}

pub fn parse_ri_long(text: &str) -> i64 {
    let (digits, radix) = parse_with_radix(text);
    i64::from_str_radix(&digits, radix).unwrap_or(0)
}

pub fn parse_int_array(value: &str) -> Option<Vec<i32>> {
    if !(value.starts_with('[') && value.ends_with(']')) || value.len() < 2 {
        return None;
    }
    let inner = &value[1..value.len() - 1];
    let offsets: Vec<&str> = inner.split(',').collect();
    if offsets.len() == 1 && java_trim(offsets[0]).is_empty() {
        return Some(Vec::new());
    }
    Some(offsets.iter().map(|offset| parse_ri_int(offset)).collect())
}

fn is_identifier_char(character: char) -> bool {
    ('A'..='z').contains(&character)
}

fn parse_offset_group(inner: &str) -> Option<Vec<i32>> {
    let parts: Vec<&str> = inner.split(',').collect();
    let valid = parts.iter().all(|part| {
        let trimmed = part.trim_start();
        trimmed.strip_prefix("0x").is_some_and(|hex| {
            !hex.trim_end().is_empty() && hex.trim_end().chars().all(|c| c.is_ascii_hexdigit())
        })
    });
    valid.then(|| parts.iter().map(|part| parse_ri_int(part)).collect())
}

pub fn parse_static_offsets(text: &str) -> StaticOffsets {
    let mut entry = StaticOffsets::default();
    let characters: Vec<char> = text.chars().collect();
    let mut position = 0;
    while position < characters.len() {
        if !is_identifier_char(characters[position]) {
            position += 1;
            continue;
        }
        let name_start = position;
        while position < characters.len() && is_identifier_char(characters[position]) {
            position += 1;
        }
        let name: String = characters[name_start..position].iter().collect();
        if characters.get(position) != Some(&'=') || characters.get(position + 1) != Some(&'[') {
            continue;
        }
        let Some(close) = characters[position + 2..].iter().position(|&c| c == ']') else {
            break;
        };
        let inner: String = characters[position + 2..position + 2 + close]
            .iter()
            .collect();
        if let Some(offsets) = parse_offset_group(&inner) {
            let segment_name = name
                .rsplit(|c: char| !c.is_ascii_alphabetic())
                .next()
                .unwrap_or(&name);
            match segment_name {
                "Species" => entry.species_offsets = offsets,
                "Level" => entry.level_offsets = offsets,
                _ => {}
            }
        }
        position += 2 + close + 1;
    }
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radix_prefixes_follow_the_ini_rules() {
        assert_eq!(parse_ri_int(" 0x1F "), 31);
        assert_eq!(parse_ri_int("&h10"), 16);
        assert_eq!(parse_ri_int("42"), 42);
        assert_eq!(parse_ri_int("garbage"), 0);
        assert_eq!(parse_ri_long("0x6BDE3C3E"), 0x6BDE_3C3E);
    }

    #[test]
    fn arrays_parse_empty_and_filled_lists() {
        assert_eq!(parse_int_array("[]"), Some(Vec::new()));
        assert_eq!(parse_int_array("[1, 0x10]"), Some(vec![1, 16]));
        assert_eq!(parse_int_array("7"), None);
    }

    #[test]
    fn static_offsets_parse_species_and_levels() {
        let parsed = parse_static_offsets("{Species=[0x111772, 0x111775], Level=[0x111776]}");
        assert_eq!(parsed.species_offsets, vec![0x111772, 0x111775]);
        assert_eq!(parsed.level_offsets, vec![0x111776]);
        let egg = parse_static_offsets("{Species=[0x15924F]}");
        assert_eq!(egg.species_offsets, vec![0x15924F]);
        assert!(egg.level_offsets.is_empty());
    }
}
