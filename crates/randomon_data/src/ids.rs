use std::cmp::Ordering;

use randomon_java::JavaHash;

macro_rules! arena_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);

        impl $name {
            pub fn index(self) -> usize {
                self.0 as usize
            }

            pub fn from_index(index: usize) -> Self {
                Self(index as u32)
            }
        }
    };
}

arena_id!(PokemonId);
arena_id!(MoveId);
arena_id!(EvolutionId);
arena_id!(EvolutionListId);
arena_id!(MegaEvolutionId);

impl PokemonId {
    pub fn number(self) -> i32 {
        self.0 as i32
    }
}

impl MoveId {
    pub fn number(self) -> i32 {
        self.0 as i32
    }
}

impl JavaHash for PokemonId {
    fn java_hash_code(&self) -> i32 {
        31i32.wrapping_add(self.number())
    }

    fn java_compare_to(&self, other: &Self) -> Option<Ordering> {
        Some(self.number().cmp(&other.number()))
    }
}
