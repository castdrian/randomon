use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use randomon_data::{EvolutionUpdate, GameData, MegaEvolutionId, MoveLearnt, PokemonId, Type};
use randomon_java::{
    SharedCosmeticRandomSourceInstance, SharedRandomSource, SharedRandomSourceInstance,
};
use randomon_log::LogWriter;

use crate::error::{RandomizationError, Result};
use crate::progress::ProgressPlan;

pub type EvolutionUpdates = BTreeMap<(i32, i32), EvolutionUpdate>;

#[derive(Clone, Debug, Default)]
pub struct PokemonPool {
    pub main: Vec<PokemonId>,
    pub main_incl_formes: Vec<PokemonId>,
    pub alt_formes: Vec<PokemonId>,
    pub mega_evolutions: Vec<MegaEvolutionId>,
    pub no_legendary: Vec<PokemonId>,
    pub only_legendary: Vec<PokemonId>,
    pub ultra_beasts: Vec<PokemonId>,
    pub no_legendary_incl_formes: Vec<PokemonId>,
    pub only_legendary_incl_formes: Vec<PokemonId>,
    pub no_legendary_alts: Vec<PokemonId>,
    pub only_legendary_alts: Vec<PokemonId>,
}

#[derive(Clone, Debug, Default)]
pub struct TrainerMovesetCaches {
    pub level_up_moves: Option<BTreeMap<i32, Vec<MoveLearnt>>>,
    pub egg_moves: Option<BTreeMap<i32, Vec<i32>>>,
    pub tm_compat: Option<BTreeMap<PokemonId, Vec<bool>>>,
    pub tutor_compat: Option<BTreeMap<PokemonId, Vec<bool>>>,
    pub tm_moves: Option<Vec<i32>>,
    pub tutor_moves: Option<Vec<i32>>,
}

#[derive(Debug)]
pub struct EngineState {
    pub random: SharedRandomSourceInstance,
    pub cosmetic_random: SharedCosmeticRandomSourceInstance,
    pub restrictions_set: bool,
    pub pool: PokemonPool,
    pub picked_starters: Vec<PokemonId>,
    pub already_picked: Vec<PokemonId>,
    pub placement_history: HashMap<PokemonId, i32>,
    pub item_placement_history: HashMap<i32, i32>,
    pub fully_evolved_random_seed: i32,
    pub is_oras: bool,
    pub is_sm: bool,
    pub perfect_accuracy: i32,
    pub two_evo_pokes: Option<Vec<PokemonId>>,
    pub cached_replacement_lists: Option<BTreeMap<Type, Vec<PokemonId>>>,
    pub cached_all_list: Vec<PokemonId>,
    pub banned_list: Vec<PokemonId>,
    pub used_as_unique_list: Vec<PokemonId>,
    pub type_weightings: BTreeMap<Type, i32>,
    pub total_type_weighting: i32,
    pub move_updates: Option<BTreeMap<i32, [bool; 5]>>,
    pub impossible_evolution_updates: EvolutionUpdates,
    pub time_based_evolution_updates: EvolutionUpdates,
    pub easier_evolution_updates: EvolutionUpdates,
    pub moveset_caches: TrainerMovesetCaches,
    pub log: LogWriter,
    pub handler_logging: bool,
}

impl EngineState {
    pub fn new(random_source: &SharedRandomSource) -> Self {
        Self {
            random: random_source.instance(),
            cosmetic_random: random_source.cosmetic_instance(),
            restrictions_set: false,
            pool: PokemonPool::default(),
            picked_starters: Vec::new(),
            already_picked: Vec::new(),
            placement_history: HashMap::new(),
            item_placement_history: HashMap::new(),
            fully_evolved_random_seed: -1,
            is_oras: false,
            is_sm: false,
            perfect_accuracy: 100,
            two_evo_pokes: None,
            cached_replacement_lists: None,
            cached_all_list: Vec::new(),
            banned_list: Vec::new(),
            used_as_unique_list: Vec::new(),
            type_weightings: BTreeMap::new(),
            total_type_weighting: 0,
            move_updates: None,
            impossible_evolution_updates: BTreeMap::new(),
            time_based_evolution_updates: BTreeMap::new(),
            easier_evolution_updates: BTreeMap::new(),
            moveset_caches: TrainerMovesetCaches::default(),
            log: LogWriter::default(),
            handler_logging: false,
        }
    }

    pub fn handler_log(&mut self, line: &str) {
        if self.handler_logging {
            self.log.println(line);
        }
    }

    pub fn handler_log_blank_line(&mut self) {
        if self.handler_logging {
            self.log.newline();
        }
    }
}

#[derive(Debug, Default)]
pub struct DeferredError {
    first: RefCell<Option<RandomizationError>>,
}

impl DeferredError {
    pub fn settle<T: Default>(&self, result: Result<T>) -> T {
        result.unwrap_or_else(|error| {
            self.record(error);
            T::default()
        })
    }

    pub fn record(&self, error: RandomizationError) {
        let mut first = self.first.borrow_mut();
        if first.is_none() {
            *first = Some(error);
        }
    }

    pub fn check(&self) -> Result<()> {
        match self.first.borrow_mut().take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

#[derive(Debug)]
pub struct HandlerCore {
    pub data: GameData,
    pub engine: EngineState,
    pub deferred: DeferredError,
    pub progress: Option<Arc<ProgressPlan>>,
}

impl HandlerCore {
    pub fn new(random_source: &SharedRandomSource) -> Self {
        Self {
            data: GameData::new(),
            engine: EngineState::new(random_source),
            deferred: DeferredError::default(),
            progress: None,
        }
    }

    pub fn report_detail(&self, done: usize, total: usize, describe: impl FnOnce() -> String) {
        if let Some(progress) = &self.progress {
            progress.detail(done, total, describe);
        }
    }
}
