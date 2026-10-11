#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpoilerLog {
    sections: Vec<(String, Vec<String>)>,
}

impl SpoilerLog {
    pub fn section(&mut self, title: &str) -> &mut Vec<String> {
        if !self.sections.iter().any(|(existing, _)| existing == title) {
            self.sections.push((title.to_owned(), Vec::new()));
        }
        let index = self
            .sections
            .iter()
            .position(|(existing, _)| existing == title)
            .unwrap_or_default();
        &mut self.sections[index].1
    }

    pub fn line(&mut self, title: &str, text: impl Into<String>) {
        self.section(title).push(text.into());
    }

    pub fn render(&self, header: &[String]) -> String {
        let mut out = String::new();
        for line in header {
            out.push_str(line);
            out.push('\n');
        }
        for (title, lines) in &self.sections {
            out.push('\n');
            out.push_str("== ");
            out.push_str(title);
            out.push_str(" ==\n");
            for line in lines {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }
}
