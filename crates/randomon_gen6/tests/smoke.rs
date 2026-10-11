use std::path::PathBuf;
use std::time::Instant;

use randomon_core::{HandlerRegistry, RomHandler};
use randomon_gen6::Gen6Factory;
use randomon_java::RandomSource;

const ROM_ENV: &str = "RANDOMON_GEN6_ROM";
const UPDATE_ENV: &str = "RANDOMON_GEN6_UPDATE";

#[test]
fn loads_a_gen6_rom() {
    let Some(path) = std::env::var_os(ROM_ENV).map(PathBuf::from) else {
        eprintln!("{ROM_ENV} is not set; skipping");
        return;
    };
    let registry = HandlerRegistry::new().with(Gen6Factory);
    let detected = registry.detect_file(&path).expect("detected as Gen 6");
    eprintln!("detected {detected:?}");
    let start = Instant::now();
    let source = RandomSource::with_seed(0).into_shared();
    let mut handler = registry
        .load_file(&path, &path.to_string_lossy(), &source)
        .expect("loads");
    eprintln!("loaded in {:?}", start.elapsed());
    report(handler.as_ref());
    if let Some(update) = std::env::var_os(UPDATE_ENV).map(PathBuf::from) {
        let start = Instant::now();
        handler.load_game_update(&update).expect("update loads");
        eprintln!("update loaded in {:?}", start.elapsed());
        report(handler.as_ref());
    }
}

fn report(handler: &dyn RomHandler) {
    eprintln!(
        "{} {} valid={} update={:?}",
        handler.rom_name(),
        handler.rom_code(),
        handler.is_rom_valid(),
        handler.game_update_version()
    );
    let pokemon = handler.get_pokemon_incl_formes();
    eprintln!("{} pokemon slots", pokemon.len());
    let data = &handler.core().data;
    for number in [1, 6, 382, 383, 384, 721] {
        let id = pokemon[number].expect("pokemon exists");
        eprintln!("{} {}", number, data.full_name(id));
    }
    eprintln!(
        "starters {:?}",
        handler
            .get_starters()
            .iter()
            .map(|id| data.full_name(*id))
            .collect::<Vec<_>>()
    );
    eprintln!("encounter sets {}", handler.get_encounters(false).len());
    eprintln!(
        "trainers {}",
        handler.get_trainers().expect("trainers").len()
    );
    eprintln!("statics {}", handler.get_static_pokemon().len());
    eprintln!("tm moves {:?}", &handler.get_tm_moves()[..5]);
    eprintln!("tutor moves {}", handler.get_move_tutor_moves().len());
    eprintln!(
        "shops {}",
        handler.get_shop_items().map_or(0, |shops| shops.len())
    );
    eprintln!("trades {}", handler.get_ingame_trades().len());
    eprintln!("field items {}", handler.get_regular_field_items().len());
    eprintln!("megas {}", handler.get_mega_evolutions().len());
}
