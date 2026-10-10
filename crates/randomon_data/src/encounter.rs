use std::collections::BTreeMap;

use randomon_java::JavaRng;

use crate::ids::PokemonId;
use crate::text::camel_case;
use crate::types::SosType;

#[derive(Clone, Debug, Default)]
pub struct Encounter {
    pub level: i32,
    pub max_level: i32,
    pub pokemon: PokemonId,
    pub forme_number: i32,
    pub is_sos: bool,
    pub sos_type: Option<SosType>,
}

#[derive(Clone, Debug, Default)]
pub struct EncounterSet {
    pub rate: i32,
    pub encounters: Vec<Encounter>,
    pub banned_pokemon: Vec<PokemonId>,
    pub display_name: Option<String>,
    pub offset: i32,
}

impl EncounterSet {
    pub fn ban(&mut self, pokemon: PokemonId) {
        if !self.banned_pokemon.contains(&pokemon) {
            self.banned_pokemon.push(pokemon);
        }
    }

    pub fn is_banned(&self, pokemon: PokemonId) -> bool {
        self.banned_pokemon.contains(&pokemon)
    }

    pub fn display_name_contains(&self, needle: &str) -> bool {
        self.display_name
            .as_deref()
            .is_some_and(|name| name.contains(needle))
    }
}

#[derive(Clone, Debug, Default)]
pub struct StaticEncounter {
    pub pkmn: PokemonId,
    pub forme: i32,
    pub level: i32,
    pub max_level: i32,
    pub held_item: i32,
    pub is_egg: bool,
    pub reset_moves: bool,
    pub restricted_pool: bool,
    pub restricted_list: Vec<PokemonId>,
    pub linked_encounters: Vec<StaticEncounter>,
}

impl StaticEncounter {
    pub fn new(pkmn: PokemonId) -> Self {
        Self {
            pkmn,
            ..Self::default()
        }
    }

    pub fn describe(&self, full_name: &str, print_level: bool) -> String {
        if self.is_egg {
            return format!("{full_name} (egg)");
        }
        if !print_level {
            return full_name.to_owned();
        }
        let mut levels = format!("Lv{}", self.level);
        if self.max_level > 0 {
            levels.push_str(&format!("-{}", self.max_level));
        }
        let linked_levels_differ = self
            .linked_encounters
            .iter()
            .any(|linked| linked.level != self.level);
        if linked_levels_differ {
            for linked in &self.linked_encounters {
                levels.push_str(&format!(" / Lv{}", linked.level));
            }
        }
        format!("{full_name}, {levels}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuraStat {
    None,
    Attack,
    Defense,
    SpecialAttack,
    SpecialDefense,
    Speed,
    All,
}

impl AuraStat {
    const ALL: [AuraStat; 7] = [
        AuraStat::None,
        AuraStat::Attack,
        AuraStat::Defense,
        AuraStat::SpecialAttack,
        AuraStat::SpecialDefense,
        AuraStat::Speed,
        AuraStat::All,
    ];

    fn java_name(self) -> &'static str {
        match self {
            AuraStat::None => "NONE",
            AuraStat::Attack => "ATTACK",
            AuraStat::Defense => "DEFENSE",
            AuraStat::SpecialAttack => "SPECIAL_ATTACK",
            AuraStat::SpecialDefense => "SPECIAL_DEFENSE",
            AuraStat::Speed => "SPEED",
            AuraStat::All => "ALL",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aura {
    pub stat: AuraStat,
    pub stages: i32,
}

impl Aura {
    pub fn from_byte(byte: i8) -> Self {
        let value = i32::from(byte);
        if value == 0 {
            return Self {
                stat: AuraStat::None,
                stages: 0,
            };
        }
        Self {
            stat: AuraStat::ALL[(((value - 1) / 3) + 1) as usize],
            stages: ((value - 1) % 3) + 1,
        }
    }

    pub fn to_byte(self) -> i8 {
        if self.stat == AuraStat::None {
            0
        } else {
            (((self.stat as i32 - 1) * 3) + self.stages) as i8
        }
    }

    pub fn random(random: &mut impl JavaRng) -> Self {
        Self::from_byte((random.next_int_bounded(18) + 1) as i8)
    }

    pub fn random_similar_strength(random: &mut impl JavaRng, old: Aura) -> Self {
        if old.stat == AuraStat::None || old.stat == AuraStat::All {
            old
        } else {
            Self {
                stat: AuraStat::ALL[(random.next_int_bounded(5) + 1) as usize],
                stages: old.stages,
            }
        }
    }

    pub fn describe(self) -> String {
        let name = camel_case(self.stat.java_name()).replace('_', " ");
        if self.stat == AuraStat::None {
            name
        } else {
            format!("{name} +{}", self.stages)
        }
    }
}

#[derive(Clone, Debug)]
pub struct TotemPokemon {
    pub encounter: StaticEncounter,
    pub aura: Aura,
    pub ally1_offset: i32,
    pub ally2_offset: i32,
    pub allies: BTreeMap<i32, StaticEncounter>,
    pub unused: bool,
}

#[derive(Clone, Debug, Default)]
pub struct IngameTrade {
    pub id: i32,
    pub requested_pokemon: Option<PokemonId>,
    pub given_pokemon: PokemonId,
    pub nickname: String,
    pub ot_name: String,
    pub ot_id: i32,
    pub ivs: Vec<i32>,
    pub item: i32,
}
