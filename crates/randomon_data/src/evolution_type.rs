use std::fmt;

macro_rules! evolution_types {
    ($($variant:ident => $java:literal [$($index:literal),+]),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum EvolutionType {
            $($variant,)+
        }

        impl EvolutionType {
            pub const ALL: &'static [EvolutionType] = &[$(EvolutionType::$variant,)+];

            pub fn java_name(self) -> &'static str {
                match self {
                    $(EvolutionType::$variant => $java,)+
                }
            }

            fn index_numbers(self) -> [i32; 7] {
                match self {
                    $(EvolutionType::$variant => [$($index),+],)+
                }
            }
        }
    };
}

evolution_types! {
    Level => "LEVEL" [1, 1, 4, 4, 4, 4, 4],
    Stone => "STONE" [2, 2, 7, 7, 8, 8, 8],
    Trade => "TRADE" [3, 3, 5, 5, 5, 5, 5],
    TradeItem => "TRADE_ITEM" [-1, 3, 6, 6, 6, 6, 6],
    Happiness => "HAPPINESS" [-1, 4, 1, 1, 1, 1, 1],
    HappinessDay => "HAPPINESS_DAY" [-1, 4, 2, 2, 2, 2, 2],
    HappinessNight => "HAPPINESS_NIGHT" [-1, 4, 3, 3, 3, 3, 3],
    LevelAttackHigher => "LEVEL_ATTACK_HIGHER" [-1, 5, 8, 8, 9, 9, 9],
    LevelDefenseHigher => "LEVEL_DEFENSE_HIGHER" [-1, 5, 10, 10, 11, 11, 11],
    LevelAtkDefSame => "LEVEL_ATK_DEF_SAME" [-1, 5, 9, 9, 10, 10, 10],
    LevelLowPv => "LEVEL_LOW_PV" [-1, -1, 11, 11, 12, 12, 12],
    LevelHighPv => "LEVEL_HIGH_PV" [-1, -1, 12, 12, 13, 13, 13],
    LevelCreateExtra => "LEVEL_CREATE_EXTRA" [-1, -1, 13, 13, 14, 14, 14],
    LevelIsExtra => "LEVEL_IS_EXTRA" [-1, -1, 14, 14, 15, 15, 15],
    LevelHighBeauty => "LEVEL_HIGH_BEAUTY" [-1, -1, 15, 15, 16, 16, 16],
    StoneMaleOnly => "STONE_MALE_ONLY" [-1, -1, -1, 16, 17, 17, 17],
    StoneFemaleOnly => "STONE_FEMALE_ONLY" [-1, -1, -1, 17, 18, 18, 18],
    LevelItemDay => "LEVEL_ITEM_DAY" [-1, -1, -1, 18, 19, 19, 19],
    LevelItemNight => "LEVEL_ITEM_NIGHT" [-1, -1, -1, 19, 20, 20, 20],
    LevelWithMove => "LEVEL_WITH_MOVE" [-1, -1, -1, 20, 21, 21, 21],
    LevelWithOther => "LEVEL_WITH_OTHER" [-1, -1, -1, 21, 22, 22, 22],
    LevelMaleOnly => "LEVEL_MALE_ONLY" [-1, -1, -1, 22, 23, 23, 23],
    LevelFemaleOnly => "LEVEL_FEMALE_ONLY" [-1, -1, -1, 23, 24, 24, 24],
    LevelElectrifiedArea => "LEVEL_ELECTRIFIED_AREA" [-1, -1, -1, 24, 25, 25, 25],
    LevelMossRock => "LEVEL_MOSS_ROCK" [-1, -1, -1, 25, 26, 26, 26],
    LevelIcyRock => "LEVEL_ICY_ROCK" [-1, -1, -1, 26, 27, 27, 27],
    TradeSpecial => "TRADE_SPECIAL" [-1, -1, -1, -1, 7, 7, 7],
    FairyAffection => "FAIRY_AFFECTION" [-1, -1, -1, -1, -1, 29, 29],
    LevelWithDark => "LEVEL_WITH_DARK" [-1, -1, -1, -1, -1, 30, 30],
    LevelUpsideDown => "LEVEL_UPSIDE_DOWN" [-1, -1, -1, -1, -1, 28, 28],
    LevelRain => "LEVEL_RAIN" [-1, -1, -1, -1, -1, 31, 31],
    LevelDay => "LEVEL_DAY" [-1, -1, -1, -1, -1, 32, 32],
    LevelNight => "LEVEL_NIGHT" [-1, -1, -1, -1, -1, 33, 33],
    LevelFemaleEspurr => "LEVEL_FEMALE_ESPURR" [-1, -1, -1, -1, -1, 34, 34],
    LevelGame => "LEVEL_GAME" [-1, -1, -1, -1, -1, -1, 36],
    LevelDayGame => "LEVEL_DAY_GAME" [-1, -1, -1, -1, -1, -1, 37],
    LevelNightGame => "LEVEL_NIGHT_GAME" [-1, -1, -1, -1, -1, -1, 38],
    LevelSnowy => "LEVEL_SNOWY" [-1, -1, -1, -1, -1, -1, 39],
    LevelDusk => "LEVEL_DUSK" [-1, -1, -1, -1, -1, -1, 40],
    LevelNightUltra => "LEVEL_NIGHT_ULTRA" [-1, -1, -1, -1, -1, -1, 41],
    StoneUltra => "STONE_ULTRA" [-1, -1, -1, -1, -1, -1, 42],
    None => "NONE" [-1, -1, -1, -1, -1, -1, -1],
}

impl EvolutionType {
    pub fn ordinal(self) -> usize {
        self as usize
    }

    pub fn to_index(self, generation: i32) -> i32 {
        self.index_numbers()[(generation - 1) as usize]
    }

    pub fn from_index(generation: i32, index: i32) -> Option<EvolutionType> {
        if !(0..50).contains(&index) {
            return Option::None;
        }
        EvolutionType::ALL
            .iter()
            .copied()
            .find(|kind| index > 0 && kind.to_index(generation) == index)
    }

    pub fn uses_level(self) -> bool {
        use EvolutionType::*;
        matches!(
            self,
            Level
                | LevelAttackHigher
                | LevelDefenseHigher
                | LevelAtkDefSame
                | LevelLowPv
                | LevelHighPv
                | LevelCreateExtra
                | LevelIsExtra
                | LevelMaleOnly
                | LevelFemaleOnly
                | LevelWithDark
                | LevelUpsideDown
                | LevelRain
                | LevelDay
                | LevelNight
                | LevelFemaleEspurr
                | LevelGame
                | LevelDayGame
                | LevelNightGame
                | LevelSnowy
                | LevelDusk
                | LevelNightUltra
        )
    }

    pub fn skip_split_evo(self) -> bool {
        matches!(
            self,
            EvolutionType::LevelHighBeauty
                | EvolutionType::LevelNightUltra
                | EvolutionType::StoneUltra
        )
    }
}

impl fmt::Display for EvolutionType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.java_name())
    }
}
