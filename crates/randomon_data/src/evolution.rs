use randomon_java::{FormatArg, java_format};

use crate::evolution_type::EvolutionType;
use crate::ids::PokemonId;

#[derive(Clone, Debug)]
pub struct Evolution {
    pub from: PokemonId,
    pub to: PokemonId,
    pub carry_stats: bool,
    pub kind: EvolutionType,
    pub extra_info: i32,
    pub forme: i32,
    pub forme_suffix: String,
    pub level: i32,
}

impl Evolution {
    pub fn new(
        from: PokemonId,
        to: PokemonId,
        carry_stats: bool,
        kind: EvolutionType,
        extra_info: i32,
    ) -> Self {
        Self {
            from,
            to,
            carry_stats,
            kind,
            extra_info,
            forme: 0,
            forme_suffix: String::new(),
            level: 0,
        }
    }

    pub fn java_equals(&self, other: &Evolution) -> bool {
        self.from == other.from && self.to == other.to && self.kind == other.kind
    }

    pub fn java_hash_code(&self) -> i32 {
        let mut result = 1i32;
        result = result.wrapping_mul(31).wrapping_add(self.from.number());
        result = result.wrapping_mul(31).wrapping_add(self.to.number());
        result
            .wrapping_mul(31)
            .wrapping_add(self.kind.ordinal() as i32)
    }
}

#[derive(Clone, Debug)]
pub struct MegaEvolution {
    pub from: PokemonId,
    pub to: PokemonId,
    pub method: i32,
    pub argument: i32,
    pub carry_stats: bool,
}

impl MegaEvolution {
    pub fn new(from: PokemonId, to: PokemonId, method: i32, argument: i32) -> Self {
        Self {
            from,
            to,
            method,
            argument,
            carry_stats: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EvolutionUpdate {
    pub from: PokemonId,
    pub to: PokemonId,
    pub from_name: String,
    pub to_name: String,
    pub kind: EvolutionType,
    pub extra_info: String,
    pub condensed: bool,
    pub additional: bool,
}

impl EvolutionUpdate {
    pub fn sort_key(&self) -> (i32, i32) {
        (self.from.number(), self.to.number())
    }

    pub fn describe(&self) -> String {
        let from = FormatArg::from(&self.from_name);
        let to = FormatArg::from(&self.to_name);
        let extra = FormatArg::from(&self.extra_info);
        let formatted = match self.kind {
            EvolutionType::Level if self.condensed => {
                let target_width = if self.additional { "%-15s" } else { "%-20s" };
                let also = if self.additional { " also" } else { "" };
                java_format(
                    &format!("%-15s now%s evolves into {target_width} at minimum level %s"),
                    &[from, also.into(), to, extra],
                )
            }
            EvolutionType::Level => java_format("%-15s -> %-15s at level %s", &[from, to, extra]),
            EvolutionType::Stone => java_format("%-15s -> %-15s using a %s", &[from, to, extra]),
            EvolutionType::Happiness => {
                java_format("%-15s -> %-15s by reaching high happiness", &[from, to])
            }
            EvolutionType::LevelItemDay => java_format(
                "%-15s -> %-15s by leveling up holding %s",
                &[from, to, extra],
            ),
            EvolutionType::LevelWithOther => java_format(
                "%-15s -> %-15s by leveling up with %s in the party",
                &[from, to, extra],
            ),
            _ => Ok(String::new()),
        };
        formatted.unwrap_or_default()
    }
}
