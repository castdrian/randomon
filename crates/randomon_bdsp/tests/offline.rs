use std::path::{Path, PathBuf};

use randomon_bdsp::{
    BdspSettings, EvolutionMode, Game, Ips32Patch, LearnsetMode, OutputLayout, RandomizedMod,
    STARTER_OFFSETS, SettingsError, StarterMode, TmCompatibilityMode, TrainerMode, WildMode,
    decode_item_argument, detect_games, encode_item_argument, movz_w, starter_patch,
};

#[test]
fn default_settings_round_trip() {
    let settings = BdspSettings::default();
    let text = settings.to_settings_string();
    assert!(text.starts_with("BDSP"));
    assert_eq!(
        BdspSettings::from_settings_string(&text).expect("parses"),
        settings
    );
}

#[test]
fn customized_settings_round_trip() {
    let settings = BdspSettings {
        starters: StarterMode::Any,
        starters_distinct_types: false,
        wild: WildMode::GlobalMapping,
        wild_generations: [true, false, true, false],
        wild_level_percent: -25,
        trainers: TrainerMode::Random,
        trainer_level_percent: 150,
        learnsets: LearnsetMode::RandomPreferSameType,
        tms: true,
        tm_compatibility: TmCompatibilityMode::Full,
        abilities: true,
        evolutions: EvolutionMode::SimilarStrength,
        shops: true,
        sixty_fps: true,
        speed_multiplier_percent: 300,
        ..BdspSettings::default()
    };
    let text = settings.to_settings_string();
    assert_eq!(
        BdspSettings::from_settings_string(&text).expect("parses"),
        settings
    );
}

#[test]
fn corrupt_settings_are_rejected() {
    let text = BdspSettings::default().to_settings_string();
    let mut corrupted = text.clone();
    corrupted.pop();
    corrupted.push(if text.ends_with('A') { 'B' } else { 'A' });
    assert!(BdspSettings::from_settings_string(&corrupted).is_err());
    assert_eq!(
        BdspSettings::from_settings_string("322abc"),
        Err(SettingsError::WrongPrefix)
    );
}

#[test]
fn movz_matches_the_assembler() {
    assert_eq!(movz_w(8, 0x1ed), [0xa8, 0x3d, 0x80, 0x52]);
    assert_eq!(movz_w(0, 1), [0x20, 0x00, 0x80, 0x52]);
}

#[test]
fn starter_patches_target_every_supported_build() {
    assert_eq!(STARTER_OFFSETS.len(), 14);
    let offsets = STARTER_OFFSETS[0];
    let patch = starter_patch(&offsets, [1, 4, 7]);
    let bytes = patch.to_bytes();
    assert!(bytes.starts_with(b"IPS32") && bytes.ends_with(b"EEOF"));
    let parsed = Ips32Patch::parse(&bytes).expect("parses");
    assert_eq!(parsed, patch);
    let (first_offset, first_bytes) = &parsed.records()[0];
    assert_eq!(*first_offset, 0x1BB_BBB0 + 0xC8 + 0x100);
    assert_eq!(first_bytes.as_slice(), movz_w(0, 1));
}

#[test]
fn field_item_arguments_are_float_bits() {
    assert_eq!(decode_item_argument(1_136_033_792), Some(365));
    assert_eq!(decode_item_argument(1_065_353_216), Some(1));
    assert_eq!(encode_item_argument(365), 1_136_033_792);
}

#[test]
fn layouts_place_files_where_loaders_expect_them() {
    let randomized = RandomizedMod {
        romfs_files: vec![(
            "StreamingAssets/AssetAssistant/Dpr/masterdatas".into(),
            vec![1],
        )],
        exefs_patches: vec![("ABC".into(), vec![2])],
        log: "log".into(),
    };
    let paths = |layout| -> Vec<PathBuf> {
        randomized
            .files(layout, &[Game::BrilliantDiamond])
            .into_iter()
            .map(|file| file.path)
            .collect()
    };
    let atmosphere = paths(OutputLayout::Atmosphere);
    assert!(atmosphere.contains(&PathBuf::from(
        "atmosphere/contents/0100000011D90000/romfs/Data/StreamingAssets/AssetAssistant/Dpr/masterdatas"
    )));
    assert!(atmosphere.contains(&PathBuf::from(
        "atmosphere/exefs_patches/randomon_bdsp/ABC.ips"
    )));
    let yuzu = paths(OutputLayout::Yuzu);
    assert!(yuzu.contains(&PathBuf::from("0100000011D90000/randomon/exefs/ABC.ips")));
    let ryujinx = paths(OutputLayout::Ryujinx);
    assert!(ryujinx.contains(&PathBuf::from(
        "mods/contents/0100000011D90000/randomon/romfs/Data/StreamingAssets/AssetAssistant/Dpr/masterdatas"
    )));
    let both = randomized.files(OutputLayout::Atmosphere, &Game::ALL);
    assert_eq!(
        both.iter()
            .filter(|file| file.path.ends_with("masterdatas"))
            .count(),
        2
    );
    assert_eq!(
        both.iter()
            .filter(|file| file.path.ends_with("ABC.ips"))
            .count(),
        1
    );
}

#[test]
fn games_are_detected_from_the_dump_folder() {
    assert_eq!(
        detect_games(Path::new("/dumps/010018E011D92000/romfs")),
        vec![Game::ShiningPearl]
    );
    assert_eq!(detect_games(Path::new("/dumps/romfs")), Game::ALL.to_vec());
}
