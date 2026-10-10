pub trait JavaEnum: Copy + Sized + 'static {
    const TYPE_NAME: &'static str;
    const ALL: &'static [Self];

    fn java_name(self) -> &'static str;

    fn ordinal(self) -> usize;

    fn from_ordinal(ordinal: usize) -> Option<Self> {
        Self::ALL.get(ordinal).copied()
    }

    fn from_java_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|value| value.java_name() == name)
    }
}

macro_rules! java_enum {
    ($type_name:ident = $java_type:literal { $first:ident => $first_java:literal $(, $variant:ident => $java:literal)* $(,)? }) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $type_name {
            #[default]
            $first,
            $($variant,)*
        }

        impl JavaEnum for $type_name {
            const TYPE_NAME: &'static str = $java_type;
            const ALL: &'static [Self] = &[Self::$first, $(Self::$variant,)*];

            fn java_name(self) -> &'static str {
                match self {
                    Self::$first => $first_java,
                    $(Self::$variant => $java,)*
                }
            }

            fn ordinal(self) -> usize {
                self as usize
            }
        }
    };
}

java_enum!(BaseStatisticsMod = "BaseStatisticsMod" {
    Unchanged => "UNCHANGED",
    Shuffle => "SHUFFLE",
    Random => "RANDOM",
});

java_enum!(ExpCurveMod = "ExpCurveMod" {
    Legendaries => "LEGENDARIES",
    StrongLegendaries => "STRONG_LEGENDARIES",
    All => "ALL",
});

java_enum!(AbilitiesMod = "AbilitiesMod" {
    Unchanged => "UNCHANGED",
    Randomize => "RANDOMIZE",
});

java_enum!(StartersMod = "StartersMod" {
    Unchanged => "UNCHANGED",
    Custom => "CUSTOM",
    CompletelyRandom => "COMPLETELY_RANDOM",
    RandomWithTwoEvolutions => "RANDOM_WITH_TWO_EVOLUTIONS",
});

java_enum!(TypesMod = "TypesMod" {
    Unchanged => "UNCHANGED",
    RandomFollowEvolutions => "RANDOM_FOLLOW_EVOLUTIONS",
    CompletelyRandom => "COMPLETELY_RANDOM",
});

java_enum!(EvolutionsMod = "EvolutionsMod" {
    Unchanged => "UNCHANGED",
    Random => "RANDOM",
    RandomEveryLevel => "RANDOM_EVERY_LEVEL",
});

java_enum!(MovesetsMod = "MovesetsMod" {
    Unchanged => "UNCHANGED",
    RandomPreferSameType => "RANDOM_PREFER_SAME_TYPE",
    CompletelyRandom => "COMPLETELY_RANDOM",
    MetronomeOnly => "METRONOME_ONLY",
});

java_enum!(TrainersMod = "TrainersMod" {
    Unchanged => "UNCHANGED",
    Random => "RANDOM",
    Distributed => "DISTRIBUTED",
    MainPlaythrough => "MAINPLAYTHROUGH",
    TypeThemed => "TYPE_THEMED",
    TypeThemedElite4Gyms => "TYPE_THEMED_ELITE4_GYMS",
});

java_enum!(WildPokemonMod = "WildPokemonMod" {
    Unchanged => "UNCHANGED",
    Random => "RANDOM",
    AreaMapping => "AREA_MAPPING",
    GlobalMapping => "GLOBAL_MAPPING",
});

java_enum!(WildPokemonRestrictionMod = "WildPokemonRestrictionMod" {
    None => "NONE",
    SimilarStrength => "SIMILAR_STRENGTH",
    CatchEmAll => "CATCH_EM_ALL",
    TypeThemeAreas => "TYPE_THEME_AREAS",
});

java_enum!(StaticPokemonMod = "StaticPokemonMod" {
    Unchanged => "UNCHANGED",
    RandomMatching => "RANDOM_MATCHING",
    CompletelyRandom => "COMPLETELY_RANDOM",
    SimilarStrength => "SIMILAR_STRENGTH",
});

java_enum!(TotemPokemonMod = "TotemPokemonMod" {
    Unchanged => "UNCHANGED",
    Random => "RANDOM",
    SimilarStrength => "SIMILAR_STRENGTH",
});

java_enum!(AllyPokemonMod = "AllyPokemonMod" {
    Unchanged => "UNCHANGED",
    Random => "RANDOM",
    SimilarStrength => "SIMILAR_STRENGTH",
});

java_enum!(AuraMod = "AuraMod" {
    Unchanged => "UNCHANGED",
    Random => "RANDOM",
    SameStrength => "SAME_STRENGTH",
});

java_enum!(TmsMod = "TMsMod" {
    Unchanged => "UNCHANGED",
    Random => "RANDOM",
});

java_enum!(TmsHmsCompatibilityMod = "TMsHMsCompatibilityMod" {
    Unchanged => "UNCHANGED",
    RandomPreferType => "RANDOM_PREFER_TYPE",
    CompletelyRandom => "COMPLETELY_RANDOM",
    Full => "FULL",
});

java_enum!(MoveTutorMovesMod = "MoveTutorMovesMod" {
    Unchanged => "UNCHANGED",
    Random => "RANDOM",
});

java_enum!(MoveTutorsCompatibilityMod = "MoveTutorsCompatibilityMod" {
    Unchanged => "UNCHANGED",
    RandomPreferType => "RANDOM_PREFER_TYPE",
    CompletelyRandom => "COMPLETELY_RANDOM",
    Full => "FULL",
});

java_enum!(InGameTradesMod = "InGameTradesMod" {
    Unchanged => "UNCHANGED",
    RandomizeGiven => "RANDOMIZE_GIVEN",
    RandomizeGivenAndRequested => "RANDOMIZE_GIVEN_AND_REQUESTED",
});

java_enum!(FieldItemsMod = "FieldItemsMod" {
    Unchanged => "UNCHANGED",
    Shuffle => "SHUFFLE",
    Random => "RANDOM",
    RandomEven => "RANDOM_EVEN",
});

java_enum!(ShopItemsMod = "ShopItemsMod" {
    Unchanged => "UNCHANGED",
    Shuffle => "SHUFFLE",
    Random => "RANDOM",
});

java_enum!(PickupItemsMod = "PickupItemsMod" {
    Unchanged => "UNCHANGED",
    Random => "RANDOM",
});

java_enum!(ExpCurve = "ExpCurve" {
    Slow => "SLOW",
    MediumSlow => "MEDIUM_SLOW",
    MediumFast => "MEDIUM_FAST",
    Fast => "FAST",
    Erratic => "ERRATIC",
    Fluctuating => "FLUCTUATING",
});

impl ExpCurve {
    pub fn from_byte(byte: i8) -> Option<Self> {
        match byte {
            0 => Some(Self::MediumFast),
            1 => Some(Self::Erratic),
            2 => Some(Self::Fluctuating),
            3 => Some(Self::MediumSlow),
            4 => Some(Self::Fast),
            5 => Some(Self::Slow),
            _ => None,
        }
    }

    pub fn to_byte(self) -> i8 {
        match self {
            Self::Slow => 5,
            Self::MediumSlow => 3,
            Self::MediumFast => 0,
            Self::Fast => 4,
            Self::Erratic => 1,
            Self::Fluctuating => 2,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::MediumFast => "Medium Fast",
            Self::Erratic => "Erratic",
            Self::Fluctuating => "Fluctuating",
            Self::MediumSlow => "Medium Slow",
            Self::Fast => "Fast",
            Self::Slow => "Slow",
        }
    }
}
