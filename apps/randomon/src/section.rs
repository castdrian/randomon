#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    PokemonTraits,
    StartersStaticsTrades,
    MovesAndMovesets,
    FoePokemon,
    WildPokemon,
    TmsAndTutors,
    Items,
    MiscTweaks,
}

impl Section {
    pub const ALL: [Section; 8] = [
        Section::PokemonTraits,
        Section::StartersStaticsTrades,
        Section::MovesAndMovesets,
        Section::FoePokemon,
        Section::WildPokemon,
        Section::TmsAndTutors,
        Section::Items,
        Section::MiscTweaks,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Section::PokemonTraits => "pokemon",
            Section::StartersStaticsTrades => "starters",
            Section::MovesAndMovesets => "moves",
            Section::FoePokemon => "foes",
            Section::WildPokemon => "wild",
            Section::TmsAndTutors => "tms",
            Section::Items => "items",
            Section::MiscTweaks => "misc",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        let slug = slug.trim().to_ascii_lowercase();
        Self::ALL.into_iter().find(|section| section.slug() == slug)
    }

    pub fn title(self) -> &'static str {
        match self {
            Section::PokemonTraits => "Pokémon Traits",
            Section::StartersStaticsTrades => "Starters, Statics & Trades",
            Section::MovesAndMovesets => "Moves & Movesets",
            Section::FoePokemon => "Foe Pokémon",
            Section::WildPokemon => "Wild Pokémon",
            Section::TmsAndTutors => "TMs, HMs & Tutors",
            Section::Items => "Items",
            Section::MiscTweaks => "Misc. Tweaks",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Section::PokemonTraits => "Base stats, types, abilities and evolutions",
            Section::StartersStaticsTrades => {
                "Starter picks, legendaries, gifts and in-game trades"
            }
            Section::MovesAndMovesets => "Move data and learnsets",
            Section::FoePokemon => "Trainer teams, levels and held items",
            Section::WildPokemon => "Encounter tables, levels and held items",
            Section::TmsAndTutors => "TM and tutor moves and compatibility",
            Section::Items => "Field items, shops and pickup",
            Section::MiscTweaks => "Quality-of-life patches and fixes",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Section::PokemonTraits => "◆",
            Section::StartersStaticsTrades => "★",
            Section::MovesAndMovesets => "✦",
            Section::FoePokemon => "⚔",
            Section::WildPokemon => "❀",
            Section::TmsAndTutors => "◉",
            Section::Items => "◈",
            Section::MiscTweaks => "⚙",
        }
    }
}
