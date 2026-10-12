use randomon_data::{EncounterSet, PokemonId, StaticEncounter, Trainer};

use crate::handler::RomHandler;

pub fn pokemon<H: RomHandler + ?Sized>(handler: &H, done: usize, total: usize, id: PokemonId) {
    let core = handler.core();
    core.report_detail(done, total, || {
        format!("{} · {} of {total}", core.data.full_name(id), done + 1)
    });
}

pub fn area<H: RomHandler + ?Sized>(handler: &H, done: usize, total: usize, set: &EncounterSet) {
    handler.core().report_detail(done, total, || {
        let encounters = set.encounters.len();
        match set.display_name.as_deref().filter(|name| !name.is_empty()) {
            Some(name) => format!("{name} · {encounters} encounters"),
            None => format!("Area {} of {total} · {encounters} encounters", done + 1),
        }
    });
}

pub fn trainer<H: RomHandler + ?Sized>(handler: &H, done: usize, total: usize, trainer: &Trainer) {
    handler.core().report_detail(done, total, || {
        let name = trainer
            .full_display_name
            .as_deref()
            .or(trainer.name.as_deref())
            .filter(|name| !name.trim().is_empty());
        match name {
            Some(name) => format!("Trainer {} of {total} · {}", done + 1, name.trim()),
            None => format!("Trainer {} of {total}", done + 1),
        }
    });
}

pub fn static_encounter<H: RomHandler + ?Sized>(
    handler: &H,
    done: usize,
    total: usize,
    encounter: &StaticEncounter,
) {
    let core = handler.core();
    core.report_detail(done, total, || {
        format!(
            "{} · {} of {total}",
            core.data.full_name(encounter.pkmn),
            done + 1
        )
    });
}

pub fn counted<H: RomHandler + ?Sized>(handler: &H, done: usize, total: usize, noun: &'static str) {
    handler
        .core()
        .report_detail(done, total, || format!("{noun} {} of {total}", done + 1));
}
