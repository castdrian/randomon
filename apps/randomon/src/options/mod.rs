pub mod catalog;
pub mod edit;
pub mod effective;
pub mod rules;
pub mod spec;
pub mod visibility;

use randomon_settings::{MiscTweak, Settings};

use crate::bundle;
use crate::capabilities::Capabilities;
use crate::section::Section;
use spec::{Card, CardBody, Row};

pub use edit::Edit;

pub fn misc_tweak_label(tweak: MiscTweak) -> &'static str {
    bundle::find_label(&format!("CodeTweaks.{}.name", tweak.bundle_key())).unwrap_or(tweak.name())
}

pub fn misc_tweak_help(tweak: MiscTweak) -> Option<&'static str> {
    bundle::help(&format!("CodeTweaks.{}.toolTipText", tweak.bundle_key()))
}

pub fn reset_section(settings: &mut Settings, section: Section) {
    let defaults = Settings::default();
    for card in catalog::cards_in(section) {
        if card.body == CardBody::MiscTweaks {
            settings.current_misc_tweaks = defaults.current_misc_tweaks;
        }
        for row in card.rows {
            row.reset(settings, &defaults);
        }
    }
}

#[derive(Clone)]
pub struct CardMatch {
    pub card: &'static Card,
    pub rows: Vec<&'static Row>,
    pub tweaks: Vec<MiscTweak>,
}

impl CardMatch {
    pub fn whole(card: &'static Card, caps: &Capabilities) -> Self {
        Self {
            card,
            rows: card.rows.iter().collect(),
            tweaks: available_tweaks(caps),
        }
    }

    pub fn has_options(&self, caps: &Capabilities) -> bool {
        if !self.card.is_visible(caps) {
            return false;
        }
        match self.card.body {
            CardBody::MiscTweaks => !self.tweaks.is_empty(),
            CardBody::Rows => self
                .rows
                .iter()
                .any(|row| !matches!(row, Row::Caption { .. }) && row.is_visible(caps)),
        }
    }

    pub fn match_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| !matches!(row, Row::Caption { .. }))
            .count()
            + self.tweaks.len()
    }
}

pub fn available_tweaks(caps: &Capabilities) -> Vec<MiscTweak> {
    MiscTweak::ALL
        .iter()
        .copied()
        .filter(|tweak| caps.has_misc_tweak(*tweak))
        .collect()
}

pub fn search(query: &str, caps: &Capabilities) -> Vec<CardMatch> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    catalog::CARDS
        .iter()
        .filter(|card| card.is_visible(caps))
        .filter_map(|card| search_card(card, &needle, caps))
        .collect()
}

fn search_card(card: &'static Card, needle: &str, caps: &Capabilities) -> Option<CardMatch> {
    if contains(card.title.title(), needle) {
        return Some(CardMatch::whole(card, caps));
    }
    let rows: Vec<&'static Row> = card
        .rows
        .iter()
        .filter(|row| row.is_visible(caps) && row_matches(row, needle, caps))
        .collect();
    let tweaks: Vec<MiscTweak> = match card.body {
        CardBody::MiscTweaks => available_tweaks(caps)
            .into_iter()
            .filter(|tweak| contains(misc_tweak_label(*tweak), needle))
            .collect(),
        CardBody::Rows => Vec::new(),
    };
    (!rows.is_empty() || !tweaks.is_empty()).then_some(CardMatch { card, rows, tweaks })
}

fn row_matches(row: &Row, needle: &str, caps: &Capabilities) -> bool {
    match row {
        Row::Item(item) => contains(item.label.label(), needle),
        Row::Radio(radio) => {
            radio
                .title
                .is_some_and(|title| contains(title.title(), needle))
                || radio
                    .choices
                    .iter()
                    .any(|choice| choice.is_visible(caps) && contains(choice.label.label(), needle))
        }
        Row::Caption { .. } => false,
    }
}

fn contains(haystack: &str, needle: &str) -> bool {
    fold(haystack).contains(&fold(needle))
}

fn fold(text: &str) -> String {
    text.to_lowercase().replace('é', "e")
}

pub fn matches_per_section(results: &[CardMatch], section: Section) -> usize {
    results
        .iter()
        .filter(|result| result.card.section == section)
        .map(CardMatch::match_count)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_option_label_resolves_from_the_bundle() {
        for card in catalog::CARDS {
            assert_ne!(card.title.title(), "", "card in {:?}", card.section);
            for row in card.rows {
                let labels: Vec<_> = match row {
                    Row::Item(item) => vec![item.label],
                    Row::Radio(radio) => radio.choices.iter().map(|choice| choice.label).collect(),
                    Row::Caption { text, .. } => vec![*text],
                };
                for label in labels {
                    if let spec::Text::Bundle(key) = label {
                        assert!(bundle::find_label(key).is_some(), "missing label {key}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_option_tooltip_resolves_from_the_bundle() {
        const WITHOUT_TOOLTIP: &[&str] = &[
            "GUI.tpDontUseLegendariesCheckBox.toolTipText",
            "GUI.wpDontUseLegendariesCheckBox.toolTipText",
        ];
        let items = catalog::CARDS
            .iter()
            .flat_map(|card| card.rows)
            .flat_map(|row| match row {
                Row::Item(item) => vec![*item],
                Row::Radio(radio) => radio.choices.to_vec(),
                Row::Caption { .. } => Vec::new(),
            });
        for item in items {
            if let Some(key) = item.help.filter(|key| !WITHOUT_TOOLTIP.contains(key)) {
                assert!(bundle::help(key).is_some(), "missing tooltip {key}");
            }
        }
    }

    #[test]
    fn search_finds_options_across_sections() {
        let results = search("level modifier", &Capabilities::preview());
        let sections: Vec<_> = results.iter().map(|result| result.card.section).collect();
        assert!(sections.contains(&Section::StartersStaticsTrades));
        assert!(sections.contains(&Section::FoePokemon));
        assert!(sections.contains(&Section::WildPokemon));
    }

    #[test]
    fn search_matches_misc_tweaks_by_name() {
        let results = search("lucky egg", &Capabilities::preview());
        let tweaks: Vec<_> = results
            .iter()
            .flat_map(|result| result.tweaks.clone())
            .collect();
        assert!(tweaks.contains(&MiscTweak::BanLuckyEgg));
        assert!(tweaks.contains(&MiscTweak::NoFreeLuckyEgg));
    }

    #[test]
    fn reset_section_restores_defaults_only_there() {
        let mut settings = Settings {
            race_mode: true,
            wild_level_modifier: 20,
            wild_levels_modified: true,
            ..Settings::default()
        };
        reset_section(&mut settings, Section::PokemonTraits);
        assert!(!settings.race_mode);
        assert!(settings.wild_levels_modified);
        reset_section(&mut settings, Section::WildPokemon);
        assert!(!settings.wild_levels_modified);
        assert_eq!(settings.wild_level_modifier, 0);
    }
}
