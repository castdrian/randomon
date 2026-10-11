mod properties;

use std::collections::HashMap;
use std::sync::LazyLock;

pub use properties::{html_to_plain, parse, strip_mnemonic};

const SOURCE: &str = include_str!("../../assets/Bundle.properties");

struct Bundle {
    labels: HashMap<String, String>,
    help: HashMap<String, String>,
}

static BUNDLE: LazyLock<Bundle> = LazyLock::new(|| {
    let raw = parse(SOURCE);
    let labels = raw
        .iter()
        .map(|(key, value)| (key.clone(), strip_mnemonic(&html_to_plain(value))))
        .collect();
    let help = raw
        .into_iter()
        .map(|(key, value)| (key, html_to_plain(&value)))
        .collect();
    Bundle { labels, help }
});

pub fn label(key: &'static str) -> &'static str {
    find_label(key).unwrap_or(key)
}

pub fn find_label(key: &str) -> Option<&'static str> {
    BUNDLE.labels.get(key).map(String::as_str)
}

pub fn help(key: &str) -> Option<&'static str> {
    BUNDLE
        .help
        .get(key)
        .map(String::as_str)
        .filter(|text| !text.is_empty())
}

pub fn title(key: &'static str) -> &'static str {
    help(key).unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_upstream_labels() {
        assert_eq!(label("GUI.pbsUnchangedRadioButton.text"), "Unchanged");
        assert_eq!(
            label("GUI.stpSwapLegendariesSwapStandardsRadioButton.text"),
            "Swap Legendaries & Swap Standards"
        );
        assert_eq!(title("GUI.tmPanel.title"), "TMs & HMs");
    }

    #[test]
    fn resolves_upstream_tooltips_as_plain_text() {
        let tooltip = help("GUI.pmsMetronomeOnlyModeRadioButton.toolTipText").expect("present");
        assert!(tooltip.starts_with("Where possible, every Pokémon in the entire game"));
        assert!(!tooltip.contains('<'));
    }

    #[test]
    fn missing_keys_fall_back_gracefully() {
        assert_eq!(label("GUI.doesNotExist.text"), "GUI.doesNotExist.text");
        assert_eq!(help("GUI.doesNotExist.toolTipText"), None);
    }
}
