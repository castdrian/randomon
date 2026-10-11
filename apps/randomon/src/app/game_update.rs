use std::path::{Path, PathBuf};

const UNKNOWN_VERSION_MARKER: &str = "Unpatched";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameUpdate {
    path: PathBuf,
    state: State,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Checking,
    Ready(Option<String>),
}

impl GameUpdate {
    pub fn checking(path: PathBuf) -> Self {
        Self {
            path,
            state: State::Checking,
        }
    }

    pub fn verified(path: PathBuf, version: Option<String>) -> Self {
        Self {
            path,
            state: State::Ready(version),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.state, State::Ready(_))
    }

    pub fn label(&self) -> String {
        match &self.state {
            State::Checking => "Checking update…".to_owned(),
            State::Ready(Some(version)) => version_label(version),
            State::Ready(None) => "Update loaded".to_owned(),
        }
    }
}

fn version_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case(UNKNOWN_VERSION_MARKER) {
        return "Update loaded".to_owned();
    }
    let number = strip_version_prefix(trimmed);
    if number.starts_with(|character: char| character.is_ascii_digit()) {
        format!("Update v{number}")
    } else {
        format!("Update {trimmed}")
    }
}

fn strip_version_prefix(version: &str) -> &str {
    let lowercase = version.to_ascii_lowercase();
    ["version", "ver.", "ver", "v."]
        .iter()
        .find(|prefix| lowercase.starts_with(*prefix))
        .map_or_else(
            || version.strip_prefix(['v', 'V']).unwrap_or(version),
            |prefix| &version[prefix.len()..],
        )
        .trim_start()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label_for(version: &str) -> String {
        GameUpdate::verified(PathBuf::from("update.cia"), Some(version.to_owned())).label()
    }

    #[test]
    fn labels_show_a_readable_version() {
        assert_eq!(label_for("1.4"), "Update v1.4");
        assert_eq!(label_for("Ver. 1.5"), "Update v1.5");
        assert_eq!(label_for("v1.2"), "Update v1.2");
        assert_eq!(label_for("Unpatched"), "Update loaded");
        assert_eq!(label_for("Final"), "Update Final");
        assert_eq!(
            GameUpdate::checking(PathBuf::from("update.cia")).label(),
            "Checking update…"
        );
    }
}
