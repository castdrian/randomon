use std::collections::HashMap;

pub fn parse(source: &str) -> HashMap<String, String> {
    logical_lines(source)
        .into_iter()
        .filter_map(|line| split_entry(&line))
        .collect()
}

fn logical_lines(source: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut pending: Option<String> = None;
    for raw in source.lines() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let continued = pending.is_some();
        let content = if continued {
            line.trim_start_matches([' ', '\t', '\u{c}'])
        } else {
            let trimmed = line.trim_start_matches([' ', '\t', '\u{c}']);
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                continue;
            }
            trimmed
        };
        let mut joined = pending.take().unwrap_or_default();
        if ends_with_continuation(content) {
            joined.push_str(&content[..content.len() - 1]);
            pending = Some(joined);
        } else {
            joined.push_str(content);
            lines.push(joined);
        }
    }
    if let Some(rest) = pending {
        lines.push(rest);
    }
    lines
}

fn ends_with_continuation(line: &str) -> bool {
    line.chars().rev().take_while(|&c| c == '\\').count() % 2 == 1
}

fn split_entry(line: &str) -> Option<(String, String)> {
    let mut key_end = line.len();
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '=' | ':' | ' ' | '\t' | '\u{c}' => {
                key_end = index;
                break;
            }
            _ => {}
        }
    }
    let key = unescape(&line[..key_end]);
    if key.is_empty() {
        return None;
    }
    let rest = &line[key_end..];
    let rest = rest.trim_start_matches([' ', '\t', '\u{c}']);
    let rest = rest
        .strip_prefix(['=', ':'])
        .map(|after| after.trim_start_matches([' ', '\t', '\u{c}']))
        .unwrap_or(rest);
    Some((key, unescape(rest)))
}

fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('f') => out.push('\u{c}'),
            Some('u') => push_unicode_escape(&mut out, &mut chars),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn push_unicode_escape(out: &mut String, chars: &mut std::str::Chars<'_>) {
    let digits: String = chars.by_ref().take(4).collect();
    let decoded = u32::from_str_radix(&digits, 16)
        .ok()
        .and_then(char::from_u32);
    match decoded {
        Some(character) => out.push(character),
        None => {
            out.push_str("\\u");
            out.push_str(&digits);
        }
    }
}

pub fn html_to_plain(value: &str) -> String {
    let flattened = value.replace(['\n', '\r'], " ");
    let mut out = String::with_capacity(flattened.len());
    let mut rest = flattened.as_str();
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        out.push_str(tag_replacement(&rest[start + 1..start + end]));
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    tidy_lines(&out)
}

fn tag_replacement(tag: &str) -> &'static str {
    let name = tag
        .trim_start_matches('/')
        .split([' ', '/'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match name.as_str() {
        "br" => "\n",
        "li" if !tag.starts_with('/') => "\n• ",
        "ul" | "center" => "\n",
        _ => "",
    }
}

fn tidy_lines(text: &str) -> String {
    text.split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn strip_mnemonic(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '&' {
            if chars.peek() == Some(&'&') {
                chars.next();
                out.push('&');
            }
            continue;
        }
        out.push(character);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_separators_and_whitespace() {
        let entries = parse("a=1\nb : 2\nc 3\n  d=  spaced value  \n");
        assert_eq!(entries["a"], "1");
        assert_eq!(entries["b"], "2");
        assert_eq!(entries["c"], "3");
        assert_eq!(entries["d"], "spaced value  ");
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        let entries = parse("# heading\n! bang\n\n   \nkey=value\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries["key"], "value");
    }

    #[test]
    fn decodes_escapes() {
        let entries = parse(r"key=Pokémon\nline\ttab \\ backslash \= equals");
        assert_eq!(entries["key"], "Pokémon\nline\ttab \\ backslash = equals");
    }

    #[test]
    fn joins_continuation_lines() {
        let entries = parse("key=first \\\n    second \\\n  third\nother=x\n");
        assert_eq!(entries["key"], "first second third");
        assert_eq!(entries["other"], "x");
    }

    #[test]
    fn escaped_separator_stays_in_key() {
        let entries = parse(r"a\=b=c");
        assert_eq!(entries["a=b"], "c");
    }

    #[test]
    fn even_backslashes_do_not_continue() {
        let entries = parse("key=ends with \\\\\nnext=1\n");
        assert_eq!(entries["key"], "ends with \\");
        assert_eq!(entries["next"], "1");
    }

    #[test]
    fn converts_html_tooltips_to_plain_text() {
        let plain = html_to_plain(
            "<html>First line.<br />\nSecond <b>bold</b> line.<ul><li>One<li>Two</ul>",
        );
        assert_eq!(plain, "First line.\nSecond bold line.\n• One\n• Two");
    }

    #[test]
    fn strips_swing_mnemonics() {
        assert_eq!(
            strip_mnemonic("Swap Legendaries && Swap Standards"),
            "Swap Legendaries & Swap Standards"
        );
        assert_eq!(strip_mnemonic("&Open"), "Open");
    }
}
