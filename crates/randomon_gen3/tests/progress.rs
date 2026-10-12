use std::sync::{Arc, Mutex};

use randomon_core::{HandlerRegistry, Progress, ProgressReporter, RandomizationOptions};
use randomon_gen3::{Gen3Factory, synthetic};
use randomon_settings::{ExpCurve, Settings, TrainersMod, WildPokemonMod};

fn recorded_run(rom: &[u8], settings: &Settings) -> Vec<Progress> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let options = RandomizationOptions {
        file_name: "synthetic.gba".to_owned(),
        progress: Some(ProgressReporter::new(move |progress| {
            sink.lock().expect("progress lock").push(progress);
        })),
        ..RandomizationOptions::default()
    };
    HandlerRegistry::new()
        .with(Gen3Factory)
        .randomize(rom, settings, 42, &options)
        .expect("synthetic ROM randomizes");
    seen.lock().expect("progress lock").clone()
}

#[test]
fn progress_moves_forward_and_finishes_for_a_gba_rom() {
    let entry = "Emerald (U)";
    let settings = Settings {
        rom_name: Some(entry.to_owned()),
        selected_exp_curve: Some(ExpCurve::MediumFast),
        wild_pokemon_mod: WildPokemonMod::Random,
        trainers_mod: TrainersMod::Random,
        ..Settings::default()
    };
    let seen = recorded_run(&synthetic::build(entry), &settings);
    assert!(seen.len() > 2);
    for pair in seen.windows(2) {
        assert!(
            pair[1].fraction() >= pair[0].fraction(),
            "{pair:?} moved backwards"
        );
    }
    let labels: Vec<&str> = seen.iter().map(|progress| progress.label).collect();
    assert!(labels.contains(&"Randomizing wild Pokémon"));
    assert!(labels.contains(&"Randomizing trainers"));
    assert!(labels.contains(&"Saving the game"));
    let last = seen.last().expect("finished");
    assert!(last.is_finished());
    assert_eq!(last.percent(), 100);
}
