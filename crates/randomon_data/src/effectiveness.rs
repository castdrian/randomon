use crate::types::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effectiveness {
    Zero,
    Half,
    Neutral,
    Double,
    Quarter,
    Quadruple,
}

use Effectiveness::{Double as D, Half as H, Neutral as N, Zero as Z};

const GEN1_TABLE: [[Effectiveness; 15]; 15] = [
    [N, N, N, N, N, N, H, N, N, N, N, N, Z, N, N],
    [D, N, H, N, N, N, D, N, N, H, N, N, Z, H, D],
    [N, D, N, D, N, N, H, N, N, D, N, H, N, N, N],
    [N, N, H, H, D, H, D, D, N, H, H, N, N, H, N],
    [N, N, N, H, H, D, D, D, N, N, H, N, N, N, N],
    [N, N, N, D, H, H, H, N, N, D, H, N, N, N, D],
    [N, H, D, N, N, D, N, H, N, D, N, N, N, N, D],
    [N, N, Z, H, N, D, D, N, N, H, N, D, N, D, N],
    [N, D, N, N, N, N, N, N, H, N, N, N, N, D, N],
    [N, H, H, D, N, H, N, N, D, N, N, N, H, D, N],
    [N, N, N, N, N, N, N, N, N, N, D, N, N, N, N],
    [N, N, D, H, D, N, N, Z, N, N, H, H, N, N, N],
    [Z, N, N, N, N, N, N, N, Z, N, N, N, D, N, N],
    [N, N, N, D, N, N, H, H, N, D, N, N, H, H, N],
    [N, N, D, D, H, N, N, D, N, N, D, N, N, N, H],
];

const GEN2_THROUGH_5_TABLE: [[Effectiveness; 17]; 17] = [
    [N, N, N, N, N, N, H, N, N, N, N, N, Z, N, N, H, N],
    [D, N, H, N, N, N, D, N, N, H, N, N, Z, H, D, D, D],
    [N, D, N, D, N, N, H, N, N, D, N, H, N, N, N, H, N],
    [N, N, H, H, D, H, D, D, N, H, H, N, N, H, N, H, N],
    [N, N, N, H, H, D, D, D, N, N, H, N, N, N, N, N, N],
    [N, N, N, D, H, H, H, N, N, D, H, N, N, N, D, D, N],
    [N, H, D, N, N, D, N, H, N, D, N, N, N, N, D, H, N],
    [N, N, Z, H, N, D, D, N, N, H, N, D, N, D, N, D, N],
    [N, D, N, N, N, N, N, N, H, N, N, N, N, D, N, H, Z],
    [N, H, H, D, N, H, N, N, D, N, N, N, H, H, N, H, D],
    [N, N, N, N, N, N, N, N, N, N, D, N, N, N, N, H, N],
    [N, N, D, H, D, N, N, Z, N, N, H, H, N, N, N, N, N],
    [Z, N, N, N, N, N, N, N, D, N, N, N, D, N, N, H, H],
    [N, N, N, D, N, N, H, H, N, N, N, N, H, H, N, Z, N],
    [N, N, D, D, H, H, N, D, N, N, D, N, N, N, H, H, N],
    [N, N, N, N, H, H, D, N, N, N, N, H, N, N, D, H, N],
    [N, H, N, N, N, N, N, N, D, N, N, N, D, N, N, H, H],
];

const GEN6_PLUS_TABLE: [[Effectiveness; 18]; 18] = [
    [N, N, N, N, N, N, H, N, N, N, N, N, Z, N, N, H, N, N],
    [D, N, H, N, N, N, D, N, N, H, N, N, Z, H, D, D, D, H],
    [N, D, N, D, N, N, H, N, N, D, N, H, N, N, N, H, N, N],
    [N, N, H, H, D, H, D, D, N, H, H, N, N, H, N, H, N, N],
    [N, N, N, H, H, D, D, D, N, N, H, N, N, N, N, N, N, N],
    [N, N, N, D, H, H, H, N, N, D, H, N, N, N, D, D, N, N],
    [N, H, D, N, N, D, N, H, N, D, N, N, N, N, D, H, N, N],
    [N, N, Z, H, N, D, D, N, N, H, N, D, N, D, N, D, N, N],
    [N, D, N, N, N, N, N, N, H, N, N, N, N, D, N, H, Z, N],
    [N, H, H, D, N, H, N, N, D, N, N, N, H, H, N, H, D, H],
    [N, N, N, N, N, N, N, N, N, N, D, N, N, N, N, H, N, Z],
    [N, N, D, H, D, N, N, Z, N, N, H, H, N, N, N, N, N, N],
    [Z, N, N, N, N, N, N, N, D, N, N, N, D, N, N, N, H, N],
    [N, N, N, D, N, N, H, H, N, N, N, N, H, H, N, Z, N, D],
    [N, N, D, D, H, H, N, D, N, N, D, N, N, N, H, H, N, N],
    [N, N, N, N, H, H, D, N, N, N, N, H, N, N, D, H, N, D],
    [N, H, N, N, N, N, N, N, D, N, N, N, D, N, N, N, H, H],
    [N, D, N, N, N, H, N, N, N, N, D, N, N, H, N, H, D, N],
];

#[derive(Clone, Copy)]
enum Table {
    Gen1,
    Gen2Through5,
    Gen6Plus,
}

impl Table {
    fn lookup(self, attacker: Type, defender: Type) -> Option<Effectiveness> {
        let (row, column) = (attacker.ordinal(), defender.ordinal());
        match self {
            Table::Gen1 => GEN1_TABLE.get(row)?.get(column).copied(),
            Table::Gen2Through5 => GEN2_THROUGH_5_TABLE.get(row)?.get(column).copied(),
            Table::Gen6Plus => GEN6_PLUS_TABLE.get(row)?.get(column).copied(),
        }
    }

    fn for_lists(generation: i32, effectiveness_updated: bool) -> Table {
        if generation == 1 {
            if effectiveness_updated {
                Table::Gen2Through5
            } else {
                Table::Gen1
            }
        } else if (2..=5).contains(&generation) {
            if effectiveness_updated {
                Table::Gen6Plus
            } else {
                Table::Gen2Through5
            }
        } else {
            Table::Gen6Plus
        }
    }
}

const COMBINE_TABLE: [[Effectiveness; 4]; 4] = [
    [Z, Z, Z, Z],
    [Z, Effectiveness::Quarter, H, N],
    [Z, H, N, D],
    [Z, N, D, Effectiveness::Quadruple],
];

impl Effectiveness {
    pub fn ordinal(self) -> usize {
        self as usize
    }

    fn combine(self, other: Effectiveness) -> Effectiveness {
        COMBINE_TABLE[self.ordinal()][other.ordinal()]
    }

    pub fn against(
        primary: Type,
        secondary: Option<Type>,
        generation: i32,
        effectiveness_updated: bool,
    ) -> Option<Vec<(Type, Effectiveness)>> {
        let (table, all_types) = if (2..=5).contains(&generation) {
            if effectiveness_updated {
                (Table::Gen6Plus, Type::all_types(2))
            } else {
                (Table::Gen2Through5, Type::all_types(2))
            }
        } else if generation >= 6 {
            (Table::Gen6Plus, Type::all_types(6))
        } else {
            return None;
        };
        Some(
            all_types
                .iter()
                .map(|&attacker| {
                    let mut effect = table.lookup(attacker, primary).unwrap_or(N);
                    if let Some(secondary) = secondary {
                        effect = effect.combine(table.lookup(attacker, secondary).unwrap_or(N));
                    }
                    (attacker, effect)
                })
                .collect(),
        )
    }

    pub fn not_very_effective(
        attacking: Type,
        generation: i32,
        effectiveness_updated: bool,
    ) -> Vec<Type> {
        let table = Table::for_lists(generation, effectiveness_updated);
        Type::all_types(generation)
            .iter()
            .copied()
            .filter(|&defending| matches!(table.lookup(attacking, defending), Some(H) | Some(Z)))
            .collect()
    }

    pub fn super_effective(
        attacking: Type,
        generation: i32,
        effectiveness_updated: bool,
    ) -> Vec<Type> {
        let table = Table::for_lists(generation, effectiveness_updated);
        Type::all_types(generation)
            .iter()
            .copied()
            .filter(|&defending| table.lookup(attacking, defending) == Some(D))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeRelationship {
    pub attacker: Type,
    pub defender: Type,
    pub effectiveness: Effectiveness,
}
