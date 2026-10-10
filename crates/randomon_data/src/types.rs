use std::fmt;

use randomon_java::JavaRng;

macro_rules! java_enum {
    ($name:ident { $($variant:ident => $java:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name {
            $($variant,)+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant,)+];

            pub fn java_name(self) -> &'static str {
                match self {
                    $($name::$variant => $java,)+
                }
            }

            pub fn ordinal(self) -> usize {
                self as usize
            }

            pub fn from_ordinal(ordinal: usize) -> Option<Self> {
                Self::ALL.get(ordinal).copied()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.java_name())
            }
        }

        impl randomon_java::JavaEnum for $name {
            fn ordinal(&self) -> usize {
                *self as usize
            }
        }
    };
}

java_enum!(Type {
    Normal => "NORMAL",
    Fighting => "FIGHTING",
    Flying => "FLYING",
    Grass => "GRASS",
    Water => "WATER",
    Fire => "FIRE",
    Rock => "ROCK",
    Ground => "GROUND",
    Psychic => "PSYCHIC",
    Bug => "BUG",
    Dragon => "DRAGON",
    Electric => "ELECTRIC",
    Ghost => "GHOST",
    Poison => "POISON",
    Ice => "ICE",
    Steel => "STEEL",
    Dark => "DARK",
    Fairy => "FAIRY",
    Gas => "GAS",
    Wood => "WOOD",
    Abnormal => "ABNORMAL",
    Wind => "WIND",
    Sound => "SOUND",
    Light => "LIGHT",
    Tri => "TRI",
});

impl Type {
    pub fn is_hack_only(self) -> bool {
        self.ordinal() > Type::Fairy.ordinal()
    }

    pub fn all_types(generation: i32) -> &'static [Type] {
        match generation {
            1 => &Type::ALL[..=Type::Ice as usize],
            2..=5 => &Type::ALL[..=Type::Dark as usize],
            _ => &Type::ALL[..=Type::Fairy as usize],
        }
    }

    pub fn random_type(random: &mut impl JavaRng) -> Type {
        Type::ALL[random.next_int_bounded(Type::ALL.len() as i32) as usize]
    }

    pub fn camel_case(self) -> String {
        crate::text::camel_case(self.java_name())
    }

    pub fn from_java_name(name: &str) -> Option<Type> {
        Type::ALL
            .iter()
            .copied()
            .find(|kind| kind.java_name() == name)
    }
}

java_enum!(MoveCategory {
    Physical => "PHYSICAL",
    Special => "SPECIAL",
    Status => "STATUS",
});

java_enum!(StatChangeType {
    None => "NONE",
    Attack => "ATTACK",
    Defense => "DEFENSE",
    SpecialAttack => "SPECIAL_ATTACK",
    SpecialDefense => "SPECIAL_DEFENSE",
    Speed => "SPEED",
    Accuracy => "ACCURACY",
    Evasion => "EVASION",
    All => "ALL",
    Special => "SPECIAL",
});

java_enum!(StatChangeMoveType {
    NoneOrUnknown => "NONE_OR_UNKNOWN",
    NoDamageTarget => "NO_DAMAGE_TARGET",
    NoDamageUser => "NO_DAMAGE_USER",
    NoDamageAll => "NO_DAMAGE_ALL",
    NoDamageAlly => "NO_DAMAGE_ALLY",
    DamageTarget => "DAMAGE_TARGET",
    DamageUser => "DAMAGE_USER",
});

java_enum!(StatusMoveType {
    NoneOrUnknown => "NONE_OR_UNKNOWN",
    NoDamage => "NO_DAMAGE",
    Damage => "DAMAGE",
});

java_enum!(StatusType {
    None => "NONE",
    Paralyze => "PARALYZE",
    Sleep => "SLEEP",
    Freeze => "FREEZE",
    Burn => "BURN",
    Poison => "POISON",
    Confusion => "CONFUSION",
    ToxicPoison => "TOXIC_POISON",
});

java_enum!(CriticalChance {
    None => "NONE",
    Normal => "NORMAL",
    Increased => "INCREASED",
    Guaranteed => "GUARANTEED",
});

java_enum!(SosType {
    Generic => "GENERIC",
    Rain => "RAIN",
    Hail => "HAIL",
    Sand => "SAND",
});

pub mod stat {
    pub const HP: i32 = 1;
    pub const ATK: i32 = 1 << 1;
    pub const DEF: i32 = 1 << 2;
    pub const SPATK: i32 = 1 << 3;
    pub const SPDEF: i32 = 1 << 4;
    pub const SPEED: i32 = 1 << 5;
    pub const SPECIAL: i32 = 1 << 6;
    pub const POWER: i32 = 1 << 7;
    pub const ACCURACY: i32 = 1 << 8;
    pub const PP: i32 = 1 << 9;
    pub const TYPE: i32 = 1 << 10;
    pub const CATEGORY: i32 = 1 << 11;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatChange {
    pub stat: i32,
    pub values: Vec<i32>,
}

impl StatChange {
    pub fn new(stat: i32, values: &[i32]) -> Self {
        Self {
            stat,
            values: values.to_vec(),
        }
    }
}
