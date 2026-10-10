use crate::evolution::{Evolution, MegaEvolution};
use crate::ids::{EvolutionId, EvolutionListId, MegaEvolutionId, MoveId, PokemonId};
use crate::moves::Move;
use crate::pokemon::{Pokemon, StatLayout, is_legendary_number, is_strong_legendary_number};

#[derive(Clone, Debug, Default)]
pub struct GameData {
    pokemon: Vec<Option<Pokemon>>,
    evolutions: Vec<Evolution>,
    evolution_lists: Vec<Vec<EvolutionId>>,
    mega_evolutions: Vec<MegaEvolution>,
    moves: Vec<Option<Move>>,
    temporary_evolution: Option<EvolutionId>,
}

impl GameData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_pokemon(&mut self, layout: StatLayout, number: i32) -> PokemonId {
        let index = usize::try_from(number).expect("Pokemon numbers are not negative");
        if self.pokemon.len() <= index {
            self.pokemon.resize_with(index + 1, || None);
        }
        let evolutions_from = self.new_evolution_list();
        let evolutions_to = self.new_evolution_list();
        self.pokemon[index] = Some(Pokemon::new(layout, number, evolutions_from, evolutions_to));
        PokemonId::from_index(index)
    }

    fn new_evolution_list(&mut self) -> EvolutionListId {
        self.evolution_lists.push(Vec::new());
        EvolutionListId::from_index(self.evolution_lists.len() - 1)
    }

    pub fn pokemon_slots(&self) -> usize {
        self.pokemon.len()
    }

    pub fn exists(&self, id: PokemonId) -> bool {
        self.pokemon.get(id.index()).is_some_and(Option::is_some)
    }

    pub fn pk(&self, id: PokemonId) -> &Pokemon {
        self.pokemon[id.index()]
            .as_ref()
            .expect("Pokemon id refers to a loaded Pokemon")
    }

    pub fn pk_mut(&mut self, id: PokemonId) -> &mut Pokemon {
        self.pokemon[id.index()]
            .as_mut()
            .expect("Pokemon id refers to a loaded Pokemon")
    }

    pub fn is_legendary(&self, id: PokemonId) -> bool {
        is_legendary_number(self.base_number_for_legendary_check(id))
    }

    pub fn is_strong_legendary(&self, id: PokemonId) -> bool {
        is_strong_legendary_number(self.base_number_for_legendary_check(id))
    }

    fn base_number_for_legendary_check(&self, id: PokemonId) -> i32 {
        let pokemon = self.pk(id);
        match (pokemon.forme_number, pokemon.base_forme) {
            (0, _) | (_, None) => pokemon.number,
            (_, Some(base)) => self.pk(base).number,
        }
    }

    pub fn base_number(&self, id: PokemonId) -> i32 {
        let mut current = id;
        while let Some(base) = self.pk(current).base_forme {
            current = base;
        }
        self.pk(current).number
    }

    pub fn full_name(&self, id: PokemonId) -> String {
        self.pk(id).full_name()
    }

    pub fn create_move(&mut self, mv: Move) -> MoveId {
        let index = usize::try_from(mv.number).expect("move numbers are not negative");
        if self.moves.len() <= index {
            self.moves.resize_with(index + 1, || None);
        }
        self.moves[index] = Some(mv);
        MoveId::from_index(index)
    }

    pub fn reserve_moves(&mut self, count: usize) {
        if self.moves.len() < count {
            self.moves.resize_with(count, || None);
        }
    }

    pub fn move_slots(&self) -> usize {
        self.moves.len()
    }

    pub fn move_ids(&self) -> Vec<Option<MoveId>> {
        self.moves
            .iter()
            .enumerate()
            .map(|(index, mv)| mv.as_ref().map(|_| MoveId::from_index(index)))
            .collect()
    }

    pub fn mv(&self, id: MoveId) -> &Move {
        self.moves[id.index()]
            .as_ref()
            .expect("move id refers to a loaded move")
    }

    pub fn mv_mut(&mut self, id: MoveId) -> &mut Move {
        self.moves[id.index()]
            .as_mut()
            .expect("move id refers to a loaded move")
    }

    pub fn move_by_number(&self, number: i32) -> Option<&Move> {
        usize::try_from(number)
            .ok()
            .and_then(|index| self.moves.get(index))
            .and_then(Option::as_ref)
    }

    pub fn move_name(&self, number: i32) -> Option<&str> {
        self.move_by_number(number).map(|mv| mv.name.as_str())
    }

    pub fn evo(&self, id: EvolutionId) -> &Evolution {
        &self.evolutions[id.index()]
    }

    pub fn evo_mut(&mut self, id: EvolutionId) -> &mut Evolution {
        &mut self.evolutions[id.index()]
    }

    pub fn new_evolution(&mut self, evolution: Evolution) -> EvolutionId {
        self.evolutions.push(evolution);
        EvolutionId::from_index(self.evolutions.len() - 1)
    }

    pub fn temporary_evolution(&mut self, evolution: Evolution) -> EvolutionId {
        match self.temporary_evolution {
            Some(id) => {
                self.evolutions[id.index()] = evolution;
                id
            }
            None => {
                let id = self.new_evolution(evolution);
                self.temporary_evolution = Some(id);
                id
            }
        }
    }

    pub fn evolutions_from(&self, id: PokemonId) -> &[EvolutionId] {
        &self.evolution_lists[self.pk(id).evolutions_from.index()]
    }

    pub fn evolutions_to(&self, id: PokemonId) -> &[EvolutionId] {
        &self.evolution_lists[self.pk(id).evolutions_to.index()]
    }

    pub fn evolutions_from_mut(&mut self, id: PokemonId) -> &mut Vec<EvolutionId> {
        let list = self.pk(id).evolutions_from;
        &mut self.evolution_lists[list.index()]
    }

    pub fn evolutions_to_mut(&mut self, id: PokemonId) -> &mut Vec<EvolutionId> {
        let list = self.pk(id).evolutions_to;
        &mut self.evolution_lists[list.index()]
    }

    pub fn evolution_targets(&self, id: PokemonId) -> Vec<PokemonId> {
        self.evolutions_from(id)
            .iter()
            .map(|&evo| self.evo(evo).to)
            .collect()
    }

    pub fn evolution_sources(&self, id: PokemonId) -> Vec<PokemonId> {
        self.evolutions_to(id)
            .iter()
            .map(|&evo| self.evo(evo).from)
            .collect()
    }

    pub fn first_pre_evolution(&self, id: PokemonId) -> Option<PokemonId> {
        self.evolutions_to(id)
            .first()
            .map(|&evo| self.evo(evo).from)
    }

    pub fn evolution_list_contains(&self, list: &[EvolutionId], candidate: &Evolution) -> bool {
        list.iter().any(|&evo| self.evo(evo).java_equals(candidate))
    }

    pub fn remove_first_equal_from(&mut self, id: PokemonId, candidate: EvolutionId) {
        let target = self.evo(candidate).clone();
        let position = self
            .evolutions_from(id)
            .iter()
            .position(|&evo| self.evo(evo).java_equals(&target));
        if let Some(position) = position {
            self.evolutions_from_mut(id).remove(position);
        }
    }

    pub fn remove_first_equal_to(&mut self, id: PokemonId, candidate: EvolutionId) {
        let target = self.evo(candidate).clone();
        let position = self
            .evolutions_to(id)
            .iter()
            .position(|&evo| self.evo(evo).java_equals(&target));
        if let Some(position) = position {
            self.evolutions_to_mut(id).remove(position);
        }
    }

    pub fn share_evolutions_from(&mut self, target: PokemonId, source: PokemonId) {
        let list = self.pk(source).evolutions_from;
        self.pk_mut(target).evolutions_from = list;
    }

    pub fn new_mega_evolution(&mut self, mega: MegaEvolution) -> MegaEvolutionId {
        self.mega_evolutions.push(mega);
        MegaEvolutionId::from_index(self.mega_evolutions.len() - 1)
    }

    pub fn mega(&self, id: MegaEvolutionId) -> &MegaEvolution {
        &self.mega_evolutions[id.index()]
    }
}
