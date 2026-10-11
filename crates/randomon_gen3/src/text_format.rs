pub fn java_split<'a>(text: &'a str, separator: &str) -> Vec<&'a str> {
    if text.is_empty() {
        return vec![text];
    }
    let mut parts: Vec<&str> = text.split(separator).collect();
    if parts.len() == 1 {
        return parts;
    }
    while parts.last().is_some_and(|part| part.is_empty()) {
        parts.pop();
    }
    parts
}

struct LineBuilder {
    text: String,
    words: usize,
    characters: i32,
}

impl LineBuilder {
    fn new() -> Self {
        Self {
            text: String::new(),
            words: 0,
            characters: 0,
        }
    }

    fn start_with(word: &str, length: i32) -> Self {
        Self {
            text: word.to_owned(),
            words: 1,
            characters: length,
        }
    }

    fn append(&mut self, word: &str, required: i32) {
        if self.words > 0 {
            self.text.push(' ');
        }
        self.text.push_str(word);
        self.words += 1;
        self.characters += required;
    }
}

pub fn rewrite_description_for_new_line_size(
    description: &str,
    newline: &str,
    line_size: i32,
    length_of: impl Fn(&str) -> i32,
) -> String {
    let flattened = description
        .replace(&format!("-{newline}"), "")
        .replace(newline, " ")
        .replace("Sp. Atk", "Sp__Atk")
        .replace("Sp. Def", "Sp__Def")
        .replace("SP. ATK", "SP__ATK")
        .replace("SP. DEF", "SP__DEF");
    let mut full = String::new();
    let mut line = LineBuilder::new();
    let mut lines_written = 0;
    for raw_word in java_split(&flattened, " ") {
        let word = raw_word.replace("SP__", "SP. ").replace("Sp__", "Sp. ");
        let mut required = length_of(&word);
        if line.words > 0 {
            required += 1;
        }
        if line.characters + required <= line_size {
            line.append(&word, required);
            continue;
        }
        if line.words > 0 {
            if lines_written > 0 {
                full.push_str(newline);
            }
            full.push_str(&line.text);
            lines_written += 1;
        }
        line = LineBuilder::start_with(&word, length_of(&word));
    }
    if line.words > 0 {
        if lines_written > 0 {
            full.push_str(newline);
        }
        full.push_str(&line.text);
    }
    full
}

fn ends_sentence(last: Option<char>) -> bool {
    matches!(last, Some('.' | '?' | '!' | '…' | ','))
}

pub struct TextBreaks<'a> {
    pub newline: &'a str,
    pub extra_line: &'a str,
    pub new_paragraph: &'a str,
}

fn push_line(paragraph: &mut String, breaks: &TextBreaks<'_>, lines_written: usize, line: &str) {
    if lines_written > 1 {
        paragraph.push_str(breaks.extra_line);
    } else if lines_written == 1 {
        paragraph.push_str(breaks.newline);
    }
    paragraph.push_str(line);
}

fn format_paragraph(
    text: &str,
    breaks: &TextBreaks<'_>,
    max_line_length: i32,
    length_of: &impl Fn(&str) -> i32,
) -> String {
    let sentence_new_line_size = 10.max(max_line_length / 2);
    let mut paragraph = String::new();
    let mut line = LineBuilder::new();
    let mut lines_written = 0;
    let mut last_char: Option<char> = None;
    for word in java_split(text, " ") {
        let mut required = length_of(word);
        if line.words > 0 {
            required += 1;
        }
        let breaks_line = line.characters + required > max_line_length
            || (line.characters >= sentence_new_line_size && ends_sentence(last_char));
        if breaks_line {
            if line.words > 0 {
                push_line(&mut paragraph, breaks, lines_written, &line.text);
                lines_written += 1;
            }
            line = LineBuilder::start_with(word, length_of(word));
        } else {
            line.append(word, required);
        }
        last_char = word.chars().last();
    }
    if line.words > 0 {
        push_line(&mut paragraph, breaks, lines_written, &line.text);
    }
    paragraph
}

pub fn format_text_with_replacements(
    text: &str,
    breaks: &TextBreaks<'_>,
    max_line_length: i32,
    length_of: impl Fn(&str) -> i32,
) -> String {
    let (body, ends_with_paragraph) = match text.strip_suffix(breaks.new_paragraph) {
        Some(body) => (body, true),
        None => (text, false),
    };
    let flattened = body
        .replace(breaks.newline, " ")
        .replace(breaks.extra_line, " ");
    let mut result = String::new();
    for (index, paragraph) in java_split(&flattened, breaks.new_paragraph)
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            result.push_str(breaks.new_paragraph);
        }
        result.push_str(&format_paragraph(
            paragraph,
            breaks,
            max_line_length,
            &length_of,
        ));
    }
    if ends_with_paragraph {
        result.push_str(breaks.new_paragraph);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const GBA_BREAKS: TextBreaks<'static> = TextBreaks {
        newline: "\\n",
        extra_line: "\\l",
        new_paragraph: "\\p",
    };

    fn characters(text: &str) -> i32 {
        text.chars().count() as i32
    }

    #[test]
    fn split_drops_trailing_empty_parts_like_java() {
        assert_eq!(java_split("a  b ", " "), vec!["a", "", "b"]);
        assert_eq!(java_split("", " "), vec![""]);
        assert_eq!(java_split("solo", " "), vec!["solo"]);
        assert_eq!(java_split("   ", " "), Vec::<&str>::new());
        assert_eq!(java_split("one\\ptwo\\p", "\\p"), vec!["one", "two"]);
    }

    #[test]
    fn descriptions_rewrap_to_the_new_width() {
        let rewritten = rewrite_description_for_new_line_size(
            "A strong attack\\nthat may lower Sp. Atk.",
            "\\n",
            18,
            characters,
        );
        assert_eq!(rewritten, "A strong attack\\nthat may lower\\nSp. Atk.");
        let joined = rewrite_description_for_new_line_size("Hyper-\\nbeam", "\\n", 18, characters);
        assert_eq!(joined, "Hyperbeam");
    }

    #[test]
    fn text_breaks_lines_and_keeps_paragraphs() {
        let text = "The TM I handed you contains ICE_BEAM. It is very cold indeed.\\pBye!\\p";
        let formatted = format_text_with_replacements(text, &GBA_BREAKS, 36, characters);
        assert_eq!(
            formatted,
            "The TM I handed you contains\\nICE_BEAM. It is very cold indeed.\\pBye!\\p"
        );
        let three_lines = format_text_with_replacements(
            "one two three four five six seven eight nine ten eleven twelve",
            &GBA_BREAKS,
            12,
            characters,
        );
        assert_eq!(
            three_lines,
            "one two\\nthree four\\lfive six\\lseven eight\\lnine ten\\leleven\\ltwelve"
        );
    }
}
