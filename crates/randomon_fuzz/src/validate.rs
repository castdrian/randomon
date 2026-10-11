use std::collections::BTreeSet;

use randomon_core::RomHandler;
use randomon_data::{GameData, PokemonId};
use randomon_settings::{EvolutionsMod, Settings};

const MAX_LEVEL: i32 = 100;
const MAX_PARTY: usize = 6;

#[derive(Clone, Debug, Default)]
pub struct Baseline {
    pub trainer_count: usize,
    pub encounter_set_count: usize,
    pub seasonal_encounter_set_count: usize,
    pub static_count: usize,
    pub trade_count: usize,
    pub tm_count: usize,
    pub starter_count: usize,
    pub field_item_count: usize,
}

impl Baseline {
    pub fn capture(handler: &dyn RomHandler) -> Result<Self, String> {
        Ok(Self {
            trainer_count: handler
                .get_trainers()
                .map_err(|error| format!("baseline trainers: {error}"))?
                .len(),
            encounter_set_count: handler.get_encounters(false).len(),
            seasonal_encounter_set_count: handler.get_encounters(true).len(),
            static_count: handler.get_static_pokemon().len(),
            trade_count: handler.get_ingame_trades().len(),
            tm_count: handler.get_tm_moves().len(),
            starter_count: handler.get_starters().len(),
            field_item_count: handler.get_regular_field_items().len(),
        })
    }
}

struct Report {
    problems: Vec<String>,
}

impl Report {
    fn check(&mut self, ok: bool, problem: impl FnOnce() -> String) {
        if !ok && self.problems.len() < 40 {
            self.problems.push(problem());
        }
    }
}

pub fn validate(handler: &dyn RomHandler, settings: &Settings, baseline: &Baseline) -> Vec<String> {
    let mut report = Report {
        problems: Vec::new(),
    };
    let data = &handler.core().data;
    let move_ok = |number: i32| data.move_by_number(number).is_some();
    let item_count = handler.get_item_names().len() as i32;
    let item_ok = |item: i32| item == 0 || (item > 0 && item < item_count.max(1));
    let species_ok = |id: PokemonId| data.exists(id);

    check_pokemon(&mut report, handler);
    check_evolutions(&mut report, handler, settings);
    check_movesets(&mut report, handler, &move_ok);
    check_trainers(
        &mut report,
        handler,
        baseline,
        &species_ok,
        &move_ok,
        &item_ok,
    );

    let seasonal = settings.use_time_based_encounters;
    let expected_sets = if seasonal {
        baseline.seasonal_encounter_set_count
    } else {
        baseline.encounter_set_count
    };
    let encounters = handler.get_encounters(seasonal);
    report.check(encounters.len() == expected_sets, || {
        format!(
            "encounter sets changed from {expected_sets} to {}",
            encounters.len()
        )
    });
    for set in &encounters {
        for encounter in &set.encounters {
            report.check(species_ok(encounter.pokemon), || {
                format!("wild species {:?} missing", encounter.pokemon)
            });
            report.check((1..=MAX_LEVEL).contains(&encounter.level), || {
                format!("wild level {} out of range", encounter.level)
            });
        }
    }

    let statics = handler.get_static_pokemon();
    report.check(statics.len() == baseline.static_count, || {
        format!(
            "statics changed from {} to {}",
            baseline.static_count,
            statics.len()
        )
    });
    for encounter in &statics {
        report.check(species_ok(encounter.pkmn), || {
            format!("static species {:?} missing", encounter.pkmn)
        });
        report.check((0..=MAX_LEVEL).contains(&encounter.level), || {
            format!("static level {} out of range", encounter.level)
        });
    }

    let tms = handler.get_tm_moves();
    report.check(tms.len() == baseline.tm_count, || {
        format!(
            "TM count changed from {} to {}",
            baseline.tm_count,
            tms.len()
        )
    });
    for &tm in &tms {
        report.check(move_ok(tm), || format!("TM move {tm} missing"));
    }

    let trades = handler.get_ingame_trades();
    report.check(trades.len() == baseline.trade_count, || {
        format!(
            "trades changed from {} to {}",
            baseline.trade_count,
            trades.len()
        )
    });
    for trade in &trades {
        report.check(species_ok(trade.given_pokemon), || {
            format!("trade species {:?} missing", trade.given_pokemon)
        });
    }

    let starters = handler.get_starters();
    report.check(starters.len() == baseline.starter_count, || {
        format!(
            "starters changed from {} to {}",
            baseline.starter_count,
            starters.len()
        )
    });
    for &starter in &starters {
        report.check(species_ok(starter), || {
            format!("starter {starter:?} missing")
        });
    }

    let field_items = handler.get_regular_field_items();
    for &item in &field_items {
        report.check(item_ok(item), || format!("field item {item} invalid"));
    }
    report.problems
}

fn check_pokemon(report: &mut Report, handler: &dyn RomHandler) {
    let data = &handler.core().data;
    let generation = handler.generation_of_pokemon();
    for id in handler.get_pokemon_incl_formes().into_iter().flatten() {
        let pokemon = data.pk(id);
        let stats = if generation == 1 {
            vec![
                pokemon.hp,
                pokemon.attack,
                pokemon.defense,
                pokemon.speed,
                pokemon.special,
            ]
        } else {
            vec![
                pokemon.hp,
                pokemon.attack,
                pokemon.defense,
                pokemon.speed,
                pokemon.spatk,
                pokemon.spdef,
            ]
        };
        report.check(stats.iter().all(|stat| (1..=255).contains(stat)), || {
            format!("{} has stats {stats:?}", pokemon.name)
        });
        report.check(pokemon.primary_type.is_some(), || {
            format!("{} has no primary type", pokemon.name)
        });
        report.check(
            pokemon.secondary_type.is_none() || pokemon.secondary_type != pokemon.primary_type,
            || format!("{} has a duplicated type", pokemon.name),
        );
    }
}

fn check_evolutions(report: &mut Report, handler: &dyn RomHandler, settings: &Settings) {
    let data = &handler.core().data;
    let pokemon: Vec<PokemonId> = handler
        .get_pokemon_incl_formes()
        .into_iter()
        .flatten()
        .collect();
    for &id in &pokemon {
        for &evolution in data.evolutions_from(id) {
            let evolution = data.evo(evolution);
            report.check(data.exists(evolution.to), || {
                format!("{} evolves into a missing species", data.pk(id).name)
            });
            report.check((0..=MAX_LEVEL).contains(&evolution.level), || {
                format!("{} evolves at level {}", data.pk(id).name, evolution.level)
            });
        }
    }
    if settings.evolutions_mod != EvolutionsMod::RandomEveryLevel {
        for &id in &pokemon {
            report.check(!reaches_itself(data, id), || {
                format!("{} is in an evolution cycle", data.pk(id).name)
            });
        }
    }
}

fn reaches_itself(data: &GameData, start: PokemonId) -> bool {
    let mut seen = BTreeSet::new();
    let mut frontier = data.evolution_targets(start);
    while let Some(next) = frontier.pop() {
        if next == start {
            return true;
        }
        if data.exists(next) && seen.insert(next) {
            frontier.extend(data.evolution_targets(next));
        }
    }
    false
}

fn check_movesets(report: &mut Report, handler: &dyn RomHandler, move_ok: &dyn Fn(i32) -> bool) {
    for (species, moves) in handler.get_moves_learnt() {
        report.check(!moves.is_empty(), || {
            format!("species {species} learns nothing")
        });
        for learnt in &moves {
            report.check(move_ok(learnt.move_number), || {
                format!(
                    "species {species} learns missing move {}",
                    learnt.move_number
                )
            });
            report.check((0..=MAX_LEVEL).contains(&learnt.level), || {
                format!("species {species} learns at level {}", learnt.level)
            });
        }
    }
}

fn check_trainers(
    report: &mut Report,
    handler: &dyn RomHandler,
    baseline: &Baseline,
    species_ok: &dyn Fn(PokemonId) -> bool,
    move_ok: &dyn Fn(i32) -> bool,
    item_ok: &dyn Fn(i32) -> bool,
) {
    let trainers = match handler.get_trainers() {
        Ok(trainers) => trainers,
        Err(error) => {
            report.check(false, || format!("trainers unreadable: {error}"));
            return;
        }
    };
    report.check(trainers.len() == baseline.trainer_count, || {
        format!(
            "trainer count changed from {} to {}",
            baseline.trainer_count,
            trainers.len()
        )
    });
    for trainer in &trainers {
        let size = trainer.pokemon.len();
        report.check((1..=MAX_PARTY).contains(&size), || {
            format!("trainer {} has {size} Pokemon", trainer.index)
        });
        for member in &trainer.pokemon {
            report.check(species_ok(member.pokemon), || {
                format!("trainer {} uses a missing species", trainer.index)
            });
            report.check((1..=MAX_LEVEL).contains(&member.level), || {
                format!("trainer {} has level {}", trainer.index, member.level)
            });
            report.check(item_ok(member.held_item), || {
                format!("trainer {} holds item {}", trainer.index, member.held_item)
            });
            for &mv in &member.moves {
                report.check(mv == 0 || move_ok(mv), || {
                    format!("trainer {} knows missing move {mv}", trainer.index)
                });
            }
        }
    }
}
