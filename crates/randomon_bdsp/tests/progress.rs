use std::cell::RefCell;
use std::path::{Path, PathBuf};

use randomon_bdsp::{
    BdspSettings, FINISHED_LABEL, Game, LOG_FILE_NAME, ModTarget, OutputLayout, Progress,
    RandomizedMod, detect_games, randomize_romfs_to,
};

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("scratch folder");
    directory
}

#[test]
fn the_mod_folder_is_the_top_level_folder_each_layout_writes() {
    let target = |layout, games: &[Game]| ModTarget {
        directory: PathBuf::from("/out/Output"),
        layout,
        games: games.to_vec(),
    };
    let one = [Game::BrilliantDiamond];
    assert_eq!(
        target(OutputLayout::Atmosphere, &one).mod_folder(),
        PathBuf::from("/out/Output/atmosphere")
    );
    assert_eq!(
        target(OutputLayout::Ryubing, &one).mod_folder(),
        PathBuf::from("/out/Output/mods")
    );
    assert_eq!(
        target(OutputLayout::Eden, &one).mod_folder(),
        PathBuf::from("/out/Output/0100000011D90000")
    );
    assert_eq!(
        target(OutputLayout::Eden, &Game::ALL).mod_folder(),
        PathBuf::from("/out/Output")
    );
    assert_eq!(
        target(OutputLayout::Emulator, &one).log_path(),
        PathBuf::from("/out/Output").join(LOG_FILE_NAME)
    );
}

#[test]
fn written_files_land_under_the_reported_mod_folder_and_log_inside_the_output() {
    let directory = scratch("randomon-bdsp-layout-test");
    let randomized = RandomizedMod {
        romfs_files: vec![(
            "StreamingAssets/AssetAssistant/Dpr/masterdatas".to_owned(),
            vec![1],
        )],
        exefs_patches: vec![("0123".to_owned(), vec![2])],
        log: "log".to_owned(),
    };
    for layout in OutputLayout::ALL {
        let target = ModTarget {
            directory: directory.join(format!("{layout:?}")),
            layout,
            games: vec![Game::BrilliantDiamond],
        };
        let seen = RefCell::new(Vec::new());
        let files = randomized
            .write_to_with_progress(
                &target.directory,
                target.layout,
                &target.games,
                &|done, total, path: &Path| seen.borrow_mut().push((done, total, path.to_owned())),
            )
            .expect("writes");
        let written = target.written(files);
        assert!(written.log.is_file());
        assert_eq!(written.log.parent(), Some(target.directory.as_path()));
        for file in written.files.iter().filter(|file| **file != written.log) {
            assert!(
                file.starts_with(&written.mod_folder),
                "{layout:?}: {file:?}"
            );
        }
        let seen = seen.into_inner();
        assert_eq!(seen.len(), written.files.len());
        assert!(
            seen.iter()
                .enumerate()
                .all(|(index, (done, _, _))| *done == index)
        );
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn randomizing_a_real_romfs_reports_progress_to_completion() {
    let Some(romfs) = std::env::var_os("RANDOMON_BDSP_ROMFS").map(PathBuf::from) else {
        return;
    };
    let directory = scratch("randomon-bdsp-progress-test");
    let target = ModTarget {
        directory: directory.clone(),
        layout: OutputLayout::Atmosphere,
        games: detect_games(&romfs),
    };
    let seen: RefCell<Vec<Progress>> = RefCell::new(Vec::new());
    let written = randomize_romfs_to(&romfs, &BdspSettings::default(), 7, &target, &|progress| {
        seen.borrow_mut().push(progress)
    })
    .expect("randomizes");
    assert!(written.mod_folder.is_dir());
    let seen = seen.into_inner();
    for pair in seen.windows(2) {
        assert!(pair[1].fraction() >= pair[0].fraction(), "{pair:?}");
    }
    assert!(seen.iter().any(|progress| progress.detail.is_some()));
    let last = seen.last().expect("finished");
    assert_eq!(last.label, FINISHED_LABEL);
    assert_eq!(last.percent(), 100);
    let _ = std::fs::remove_dir_all(&directory);
}
