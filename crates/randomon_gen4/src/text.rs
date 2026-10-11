use std::collections::BTreeMap;

use randomon_core::Result;
use randomon_data::constants::gen4;
use randomon_formats::text::poketext;
use randomon_nds::format_error;

use crate::bytes::{file, file_mut};
use crate::handler::Gen4RomHandler;

const NEWLINE: &str = "\\n";
const EXTRA_LINE: &str = "\\l";
const NEW_PARAGRAPH: &str = "\\p";

pub(crate) fn java_split<'a>(text: &'a str, separator: &str) -> Vec<&'a str> {
    if text.is_empty() {
        return vec![text];
    }
    let mut parts: Vec<&str> = text.split(separator).collect();
    while parts.last().is_some_and(|part| part.is_empty()) {
        parts.pop();
    }
    parts
}

fn utf16_length(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

fn last_unit(word: &str) -> Option<char> {
    word.chars().next_back()
}

struct LineBuilder {
    full: String,
    line: String,
    word_count: i32,
    char_count: i32,
    lines_written: i32,
}

impl LineBuilder {
    fn new() -> Self {
        Self {
            full: String::new(),
            line: String::new(),
            word_count: 0,
            char_count: 0,
            lines_written: 0,
        }
    }

    fn required_length(&self, word: &str) -> i32 {
        utf16_length(word) + i32::from(self.word_count > 0)
    }

    fn append(&mut self, word: &str) {
        let required = self.required_length(word);
        if self.word_count > 0 {
            self.line.push(' ');
        }
        self.line.push_str(word);
        self.word_count += 1;
        self.char_count += required;
    }

    fn flush_with(&mut self, separator: impl Fn(i32) -> &'static str) {
        if self.word_count > 0 {
            self.full.push_str(separator(self.lines_written));
            self.full.push_str(&self.line);
            self.lines_written += 1;
            self.line.clear();
        }
    }

    fn start_line(&mut self, word: &str) {
        self.line.push_str(word);
        self.word_count = 1;
        self.char_count = utf16_length(word);
    }
}

fn description_separator(lines_written: i32) -> &'static str {
    if lines_written > 0 { NEWLINE } else { "" }
}

fn paragraph_separator(lines_written: i32) -> &'static str {
    match lines_written {
        0 => "",
        1 => NEWLINE,
        _ => EXTRA_LINE,
    }
}

pub(crate) fn rewrite_description_for_new_line_size(description: &str, line_size: i32) -> String {
    let joined = description
        .replace(&format!("-{NEWLINE}"), "")
        .replace(NEWLINE, " ")
        .replace("Sp. Atk", "Sp__Atk")
        .replace("Sp. Def", "Sp__Def")
        .replace("SP. ATK", "SP__ATK")
        .replace("SP. DEF", "SP__DEF");
    let mut builder = LineBuilder::new();
    for raw_word in java_split(&joined, " ") {
        let word = raw_word.replace("SP__", "SP. ").replace("Sp__", "Sp. ");
        if builder.char_count + builder.required_length(&word) <= line_size {
            builder.append(&word);
        } else {
            builder.flush_with(description_separator);
            builder.start_line(&word);
        }
    }
    builder.flush_with(description_separator);
    builder.full
}

fn ends_sentence(character: Option<char>) -> bool {
    matches!(character, Some('.' | '?' | '!' | '…' | ','))
}

fn apply_replacements(text: &str, replacements: &BTreeMap<String, String>) -> String {
    let mut result = text.to_owned();
    for (index, key) in replacements.keys().enumerate() {
        result = result.replace(key.as_str(), &format!("<tmpreplace{}>", index + 1));
    }
    for (index, value) in replacements.values().enumerate() {
        result = result.replace(&format!("<tmpreplace{}>", index + 1), value);
    }
    result
}

fn format_paragraph(paragraph: &str, max_line_length: i32) -> String {
    let sentence_new_line_size = 10.max(max_line_length / 2);
    let mut builder = LineBuilder::new();
    let mut last_character: Option<char> = None;
    for word in java_split(paragraph, " ") {
        let required = builder.required_length(word);
        let breaks_line = builder.char_count + required > max_line_length
            || (builder.char_count >= sentence_new_line_size && ends_sentence(last_character));
        if breaks_line {
            builder.flush_with(paragraph_separator);
            builder.start_line(word);
        } else {
            builder.append(word);
        }
        last_character = last_unit(word);
    }
    builder.flush_with(paragraph_separator);
    builder.full
}

pub(crate) fn format_text_with_replacements(
    text: &str,
    replacements: &BTreeMap<String, String>,
    max_line_length: i32,
) -> String {
    let (body, ends_with_paragraph) = match text.strip_suffix(NEW_PARAGRAPH) {
        Some(stripped) => (stripped, true),
        None => (text, false),
    };
    let flattened = body.replace(NEWLINE, " ").replace(EXTRA_LINE, " ");
    let replaced = apply_replacements(&flattened, replacements);
    let mut result = String::new();
    for (index, paragraph) in java_split(&replaced, NEW_PARAGRAPH).into_iter().enumerate() {
        if index > 0 {
            result.push_str(NEW_PARAGRAPH);
        }
        result.push_str(&format_paragraph(paragraph, max_line_length));
    }
    if ends_with_paragraph {
        result.push_str(NEW_PARAGRAPH);
    }
    result
}

impl Gen4RomHandler {
    pub(crate) fn get_strings_and_flag(&self, index: i32) -> Result<(Vec<String>, bool)> {
        let raw = file(&self.msg_narc, index)?;
        let decoded = poketext::read_strings(raw).map_err(format_error)?;
        Ok((decoded.strings, decoded.compressed))
    }

    pub(crate) fn get_strings(&self, index: i32) -> Result<Vec<String>> {
        self.get_strings_and_flag(index).map(|(strings, _)| strings)
    }

    pub(crate) fn set_strings_compressed(
        &mut self,
        index: i32,
        strings: &[String],
        compressed: bool,
    ) -> Result<()> {
        let encoded = poketext::write_strings(strings, compressed).map_err(format_error)?;
        *file_mut(&mut self.msg_narc, index)? = encoded;
        Ok(())
    }

    pub(crate) fn set_strings(&mut self, index: i32, strings: &[String]) -> Result<()> {
        self.set_strings_compressed(index, strings, false)
    }

    pub(crate) fn text_chars_per_line(&self) -> i32 {
        gen4::text_chars_per_line(self.rom_type())
    }

    pub(crate) fn replace_all_strings_in_entry(
        &mut self,
        entry: i32,
        replacements: &BTreeMap<String, String>,
    ) -> Result<()> {
        let replacements: BTreeMap<String, String> = replacements
            .iter()
            .map(|(key, value)| (key.clone(), value.replace(' ', "_")))
            .collect();
        let line_length = self.text_chars_per_line();
        let mut strings = self.get_strings(entry)?;
        for string in &mut strings {
            let needs_replacement = replacements.keys().any(|key| string.contains(key.as_str()));
            if needs_replacement {
                *string = format_text_with_replacements(string, &replacements, line_length)
                    .replace('_', " ");
            }
        }
        self.set_strings(entry, &strings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_split_drops_trailing_empty_parts() {
        assert_eq!(java_split("", " "), vec![""]);
        assert_eq!(java_split(" ", " "), Vec::<&str>::new());
        assert_eq!(java_split(" a  b ", " "), vec!["", "a", "", "b"]);
        assert_eq!(java_split("a\\pb\\p", "\\p"), vec!["a", "b"]);
    }

    #[test]
    fn descriptions_rewrap_like_upstream() {
        let cases = [
            (
                "Lowers the foe's\\nSp. Atk by two-\\nlevels harshly.",
                "Lowers the foe's\\nSp. Atk by\\ntwolevels\\nharshly.",
            ),
            (
                "A move that\\nhits. SP. DEF is\\nraised a lot\\nwith this odd one.",
                "A move that\\nhits. SP. DEF is\\nraised a lot\\nwith this odd\\none.",
            ),
            ("", ""),
            (
                "Superlongwordthatcannotfitanywhereatall here",
                "Superlongwordthatcannotfitanywhereatall\\nhere",
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(rewrite_description_for_new_line_size(input, 16), expected);
        }
    }

    #[test]
    fn replacements_rewrap_like_upstream() {
        let replacements = BTreeMap::from([
            ("SPEAROW".to_owned(), "Bulbasaur".to_owned()),
            ("ROAR".to_owned(), "Hyper_Beam".to_owned()),
        ]);
        let cases = [
            (
                "I traded my SPEAROW\\nfor a nicer one.\\pThanks!\\p",
                "I traded my\\nBulbasaur for a\\lnicer one.\\pThanks!\\p",
            ),
            (
                "ROAR, ROAR! What a SPEAROW... it's so cute,\\lisn't it? Yes, it is.",
                "Hyper_Beam,\\nHyper_Beam!\\lWhat a Bulbasaur...\\lit's so cute,\\lisn't it? Yes,\\lit is.",
            ),
            ("No match here at all", "No match here at all"),
            ("SPEAROW\\p\\pROAR", "Bulbasaur\\p\\pHyper_Beam"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                format_text_with_replacements(input, &replacements, 20),
                expected
            );
        }
    }
}
