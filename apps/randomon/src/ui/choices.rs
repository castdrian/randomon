use std::fmt;

use randomon_settings::{ExpCurve, TrainersMod};

use crate::bundle;
use crate::capabilities::Capabilities;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpCurveChoice(pub ExpCurve);

impl fmt::Display for ExpCurveChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationChoice(pub i32);

impl fmt::Display for GenerationChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Gen {}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainerModeChoice(pub TrainersMod);

impl TrainerModeChoice {
    pub fn help(self) -> Option<&'static str> {
        bundle::help(trainer_mode_keys(self.0).1)
    }
}

impl fmt::Display for TrainerModeChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(bundle::label(trainer_mode_keys(self.0).0))
    }
}

fn trainer_mode_keys(mode: TrainersMod) -> (&'static str, &'static str) {
    match mode {
        TrainersMod::Unchanged => (
            "GUI.tpMain0Unchanged.text",
            "GUI.tpMain0Unchanged.toolTipText",
        ),
        TrainersMod::Random => ("GUI.tpMain1Random.text", "GUI.tpMain1Random.toolTipText"),
        TrainersMod::Distributed => (
            "GUI.tpMain2RandomEvenDistribution.text",
            "GUI.tpMain2RandomEvenDistribution.toolTipText",
        ),
        TrainersMod::MainPlaythrough => (
            "GUI.tpMain3RandomEvenDistributionMainGame.text",
            "GUI.tpMain3RandomEvenDistributionMainGame.toolTipText",
        ),
        TrainersMod::TypeThemed => (
            "GUI.tpMain4TypeThemed.text",
            "GUI.tpMain4TypeThemed.toolTipText",
        ),
        TrainersMod::TypeThemedElite4Gyms => (
            "GUI.tpMain5TypeThemedEliteFourGyms.text",
            "GUI.tpMain5TypeThemedEliteFourGyms.toolTipText",
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarterChoice {
    pub value: i32,
    pub label: String,
}

impl fmt::Display for StarterChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

pub fn starter_choices(caps: &Capabilities) -> Vec<StarterChoice> {
    std::iter::once(StarterChoice {
        value: 1,
        label: "Random".to_owned(),
    })
    .chain((1..=caps.species_count).map(|number| StarterChoice {
        value: number as i32 + 1,
        label: format!("Pokémon #{number:03}"),
    }))
    .collect()
}
