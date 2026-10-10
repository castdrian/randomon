use randomon_java::{FormatArg, java_format_with_line_separator};

#[derive(Clone, Debug)]
pub struct LogWriter {
    text: String,
    line_separator: &'static str,
}

impl Default for LogWriter {
    fn default() -> Self {
        Self::new("\n")
    }
}

impl LogWriter {
    pub fn new(line_separator: &'static str) -> Self {
        Self {
            text: String::new(),
            line_separator,
        }
    }

    pub fn line_separator(&self) -> &'static str {
        self.line_separator
    }

    pub fn print(&mut self, text: &str) {
        self.text.push_str(text);
    }

    pub fn println(&mut self, text: &str) {
        self.text.push_str(text);
        self.text.push_str(self.line_separator);
    }

    pub fn newline(&mut self) {
        self.text.push_str(self.line_separator);
    }

    pub fn printf(&mut self, format: &str, args: &[FormatArg]) {
        let formatted = java_format_with_line_separator(format, args, self.line_separator)
            .unwrap_or_else(|error| format!("<format error: {error}>"));
        self.text.push_str(&formatted);
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn into_string(self) -> String {
        self.text
    }
}

pub fn format(format: &str, args: &[FormatArg]) -> String {
    java_format_with_line_separator(format, args, "\n")
        .unwrap_or_else(|error| format!("<format error: {error}>"))
}
