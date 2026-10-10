use crate::ids::PokemonId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MultiBattleStatus {
    #[default]
    Never,
    Potential,
    Always,
}

#[derive(Clone, Debug, Default)]
pub struct TrainerPokemon {
    pub pokemon: PokemonId,
    pub level: i32,
    pub moves: [i32; 4],
    pub held_item: i32,
    pub has_mega_stone: bool,
    pub has_z_crystal: bool,
    pub ability_slot: i32,
    pub forme: i32,
    pub forme_suffix: String,
    pub forced_gender_flag: i32,
    pub nature: i8,
    pub hp_evs: i8,
    pub atk_evs: i8,
    pub def_evs: i8,
    pub spatk_evs: i8,
    pub spdef_evs: i8,
    pub speed_evs: i8,
    pub ivs: i32,
    pub strength: i32,
    pub reset_moves: bool,
}

impl TrainerPokemon {
    pub fn new(pokemon: PokemonId, level: i32) -> Self {
        Self {
            pokemon,
            level,
            ..Self::default()
        }
    }

    pub fn copy(&self) -> TrainerPokemon {
        TrainerPokemon {
            pokemon: self.pokemon,
            level: self.level,
            moves: self.moves,
            held_item: self.held_item,
            has_mega_stone: false,
            has_z_crystal: false,
            ability_slot: self.ability_slot,
            forme: self.forme,
            forme_suffix: self.forme_suffix.clone(),
            forced_gender_flag: self.forced_gender_flag,
            nature: self.nature,
            hp_evs: self.hp_evs,
            atk_evs: self.atk_evs,
            def_evs: self.def_evs,
            spatk_evs: self.spatk_evs,
            spdef_evs: self.spdef_evs,
            speed_evs: self.speed_evs,
            ivs: self.ivs,
            strength: self.strength,
            reset_moves: self.reset_moves,
        }
    }

    pub fn description_template(&self, pokemon_name: &str) -> String {
        let mut description = format!("{pokemon_name}{}", self.forme_suffix);
        if self.held_item != 0 {
            description.push_str("@%s");
        }
        description.push_str(&format!(" Lv{}", self.level));
        description
    }
}

#[derive(Clone, Debug, Default)]
pub struct Trainer {
    pub offset: i32,
    pub index: i32,
    pub pokemon: Vec<TrainerPokemon>,
    pub tag: Option<String>,
    pub important_trainer: bool,
    pub poketype: i32,
    pub name: Option<String>,
    pub trainerclass: i32,
    pub full_display_name: Option<String>,
    pub multi_battle_status: MultiBattleStatus,
    pub force_starter_position: i32,
    pub requires_unique_held_items: bool,
}

impl Trainer {
    pub fn new() -> Self {
        Self {
            force_starter_position: -1,
            ..Self::default()
        }
    }

    fn tag_str(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    pub fn is_boss(&self) -> bool {
        self.tag_str().is_some_and(|tag| {
            tag.starts_with("ELITE")
                || tag.starts_with("CHAMPION")
                || tag.starts_with("UBER")
                || tag.ends_with("LEADER")
        })
    }

    pub fn is_important(&self) -> bool {
        self.tag_str().is_some_and(|tag| {
            tag.starts_with("RIVAL") || tag.starts_with("FRIEND") || tag.ends_with("STRONG")
        })
    }

    pub fn skip_important(&self) -> bool {
        self.tag_str().is_some_and(|tag| {
            tag.starts_with("RIVAL1-") || tag.starts_with("FRIEND1-") || tag.ends_with("NOTSTRONG")
        })
    }

    pub fn set_pokemon_have_items(&mut self, have_items: bool) {
        if have_items {
            self.poketype |= 2;
        } else {
            self.poketype &= !2;
        }
    }

    pub fn pokemon_have_items(&self) -> bool {
        (self.poketype & 2) == 2
    }

    pub fn set_pokemon_have_custom_moves(&mut self, have_custom_moves: bool) {
        if have_custom_moves {
            self.poketype |= 1;
        } else {
            self.poketype &= !1;
        }
    }

    pub fn pokemon_have_custom_moves(&self) -> bool {
        (self.poketype & 1) == 1
    }

    pub fn pokemon_have_unique_held_items(&self) -> bool {
        let mut seen = Vec::new();
        for poke in &self.pokemon {
            if poke.held_item > 0 {
                if seen.contains(&poke.held_item) {
                    return false;
                }
                seen.push(poke.held_item);
            }
        }
        true
    }

    pub fn current_name(&self) -> &str {
        self.full_display_name
            .as_deref()
            .or(self.name.as_deref())
            .unwrap_or("")
    }
}
