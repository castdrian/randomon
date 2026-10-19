use std::path::Path;

use randomon_core::RomHandlerFactory;
use randomon_swsh::{GENERATION, ROM_CODE, SwshFactory, romfs_root};

#[test]
fn ordinary_folders_and_files_are_not_swsh_games() {
    let folder = std::env::temp_dir().join("randomon-swsh-tests/not-a-romfs");
    std::fs::create_dir_all(&folder).expect("folder");
    assert!(SwshFactory.detect_file(&folder).is_none());
    assert!(!SwshFactory.is_loadable_file(&folder));
    assert!(romfs_root(&folder).is_none());
    assert!(
        SwshFactory
            .detect_file(Path::new("/nowhere/game.nds"))
            .is_none()
    );
    assert!(!SwshFactory.is_loadable(&[0; 16]));
}

#[test]
fn the_factory_reports_generation_eight() {
    assert_eq!(SwshFactory.generation(), GENERATION);
    assert_eq!(ROM_CODE, "SWSH");
    let source = randomon_java::RandomSource::with_seed(0).into_shared();
    assert!(SwshFactory.create(Vec::new(), "rom", &source).is_err());
}
