use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use randomon_bdsp::{
    BROKEN_ABILITIES, BdspSettings, BundleId, Bundles, EvolutionMode, GameData, Ips32Patch,
    LearnsetMode, MAX_ABILITY, OutputLayout, RandomizedMod, STARTER_OFFSETS, TmCompatibilityMode,
    TrainerMode, WildMode, decode_item_argument, detect_games, is_legendary, is_randomizable_item,
    learned_moves, path_ids, randomize_bundles,
};
use randomon_unity::{AssetFile, Value};

const ROMFS_ENV: &str = "RANDOMON_BDSP_ROMFS";
const SEEDS: [u64; 3] = [1, 424_242, 9_876_543_210];

fn romfs() -> Option<PathBuf> {
    std::env::var(ROMFS_ENV).ok().map(PathBuf::from)
}

fn everything_on() -> BdspSettings {
    BdspSettings {
        learnsets: LearnsetMode::RandomPreferSameType,
        tms: true,
        tm_compatibility: TmCompatibilityMode::TypeAware,
        abilities: true,
        evolutions: EvolutionMode::SimilarStrength,
        shops: true,
        trainer_max_ivs: true,
        trainer_level_percent: 20,
        wild_level_percent: 10,
        hgss_following: true,
        true_size: true,
        remove_weather: true,
        bike_everywhere: true,
        sixty_fps: true,
        speed_multiplier_percent: 150,
        ..BdspSettings::default()
    }
}

fn setting_grid() -> Vec<(&'static str, BdspSettings)> {
    vec![
        ("defaults", BdspSettings::default()),
        ("everything", everything_on()),
        (
            "chaos",
            BdspSettings {
                wild: WildMode::PerSlot,
                wild_similar_strength: false,
                wild_allow_legendaries: true,
                wild_generations: [false, false, false, true],
                trainers: TrainerMode::Random,
                trainer_type_themes: false,
                learnsets: LearnsetMode::Random,
                tm_compatibility: TmCompatibilityMode::Random,
                abilities_follow_evolutions: false,
                evolutions: EvolutionMode::Random,
                shops: true,
                shops_keep_basics: false,
                ..BdspSettings::default()
            },
        ),
        (
            "global",
            BdspSettings {
                wild: WildMode::GlobalMapping,
                tm_compatibility: TmCompatibilityMode::Full,
                ..BdspSettings::default()
            },
        ),
    ]
}

struct Outputs {
    assets: BTreeMap<BundleId, AssetFile>,
}

fn load_outputs(romfs: &Path, output: &RandomizedMod) -> Outputs {
    let data = randomon_bdsp::data_directory(romfs);
    let mut assets = BTreeMap::new();
    for bundle in BundleId::ALL {
        let edited = output
            .romfs_files
            .iter()
            .find(|(path, _)| path == bundle.relative_path());
        let bytes = match edited {
            Some((_, bytes)) => bytes.clone(),
            None => match std::fs::read(data.join(bundle.relative_path())) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            },
        };
        let asset = AssetFile::parse(&bytes)
            .unwrap_or_else(|error| panic!("{bundle:?} output re-opens: {error}"));
        if edited.is_some() {
            for file in asset.serialized_files() {
                for object in &file.objects {
                    if file.type_tree(object).is_some() {
                        file.decode(object).unwrap_or_else(|error| {
                            panic!("{bundle:?} object {} decodes: {error}", object.path_id)
                        });
                    }
                }
            }
        }
        assets.insert(bundle, asset);
    }
    Outputs { assets }
}

fn read(outputs: &Outputs, bundle: BundleId, path_id: i64) -> Value {
    outputs.assets[&bundle]
        .read_value(path_id)
        .expect("object reads")
}

fn array<'a>(value: &'a Value, name: &str) -> &'a [Value] {
    value.field(name).expect("field").as_array().expect("array")
}

fn int(value: &Value, name: &str) -> i64 {
    value.int_field(name).expect("int field")
}

fn check_encounters(outputs: &Outputs, data: &GameData, settings: &BdspSettings) {
    for path_id in [
        path_ids::FIELD_ENCOUNT_TABLE_D,
        path_ids::FIELD_ENCOUNT_TABLE_P,
    ] {
        let table = read(outputs, BundleId::Gamesettings, path_id);
        for area in array(&table, "table") {
            for (_, field) in area.as_struct().expect("area").iter() {
                let Some(slots) = field.as_array() else {
                    continue;
                };
                for slot in slots.iter().filter_map(Value::as_struct) {
                    let Some(species) = slot.get("monsNo") else {
                        continue;
                    };
                    let species = species.int().expect("species");
                    if species == 0 {
                        continue;
                    }
                    assert!(data.valid_species(species as u16), "wild species {species}");
                    if !settings.wild_allow_legendaries && !settings.wild_keep_legendaries {
                        assert!(!is_legendary(species as u16));
                    }
                    let min = slot.get("minlv").and_then(Value::as_i64).expect("minlv");
                    let max = slot.get("maxlv").and_then(Value::as_i64).expect("maxlv");
                    assert!((1..=100).contains(&min) && (1..=100).contains(&max) && min <= max);
                }
            }
        }
        for entry in array(&table, "safari") {
            assert!(data.valid_species(int(entry, "MonsNo") as u16));
        }
    }
}

fn check_underground(outputs: &Outputs, data: &GameData) {
    for path_id in path_ids::UG_ENCOUNT_TABLES {
        let table = read(outputs, BundleId::UgData, path_id);
        let species: Vec<i64> = array(&table, "table")
            .iter()
            .map(|entry| int(entry, "monsno"))
            .collect();
        for &id in &species {
            assert!(
                data.valid_species(id as u16) && !is_legendary(id as u16),
                "underground {id}"
            );
        }
    }
}

fn learnsets(outputs: &Outputs) -> BTreeMap<u16, Vec<u16>> {
    let table = read(
        outputs,
        BundleId::PersonalMasterdatas,
        path_ids::WAZA_OBOE_TABLE,
    );
    array(&table, "WazaOboe")
        .iter()
        .map(|entry| {
            let moves = array(entry, "ar")
                .iter()
                .map(|value| value.int().expect("int") as u16)
                .collect();
            (int(entry, "id") as u16, moves)
        })
        .collect()
}

fn check_learnsets(outputs: &Outputs, data: &GameData, settings: &BdspSettings) {
    for (id, learnset) in learnsets(outputs) {
        let moves: Vec<u16> = learnset
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| pair[1])
            .collect();
        for &move_id in &moves {
            assert!(
                data.valid_move(move_id),
                "species {id} learns invalid move {move_id}"
            );
        }
        if settings.learnsets != LearnsetMode::Unchanged && !moves.is_empty() {
            let distinct: BTreeSet<u16> = moves.iter().copied().collect();
            assert_eq!(
                distinct.len(),
                moves.len(),
                "species {id} learnset has duplicates"
            );
            assert!(
                data.moves[&moves[0]].is_damaging(),
                "species {id} starts with a damaging move"
            );
        }
    }
}

fn check_trainers(outputs: &Outputs, originals: &Outputs, data: &GameData) {
    let learnsets = learnsets(outputs);
    let table = read(outputs, BundleId::Masterdatas, path_ids::TRAINER_TABLE);
    let original_table = read(originals, BundleId::Masterdatas, path_ids::TRAINER_TABLE);
    for (trainer, original) in array(&table, "TrainerPoke")
        .iter()
        .zip(array(&original_table, "TrainerPoke"))
    {
        for slot in 1..=6 {
            let level = int(trainer, &format!("P{slot}Level"));
            if level <= 0 {
                continue;
            }
            assert!((1..=100).contains(&level));
            let species = int(trainer, &format!("P{slot}MonsNo")) as u16;
            assert!(data.valid_species(species), "trainer species {species}");
            let learnable: BTreeSet<u16> = learnsets
                .get(&species)
                .map(|set| set.as_chunks::<2>().0.iter().map(|pair| pair[1]).collect())
                .unwrap_or_default();
            let expected = learned_moves(
                learnsets.get(&species).map_or(&[][..], Vec::as_slice),
                level,
            );
            for index in 1..=4 {
                let move_id = int(trainer, &format!("P{slot}Waza{index}")) as u16;
                let original_move = int(original, &format!("P{slot}Waza{index}")) as u16;
                if move_id != 0 && move_id != original_move {
                    assert!(data.valid_move(move_id), "trainer move {move_id}");
                    assert!(
                        learnable.contains(&move_id) || expected.is_empty(),
                        "trainer {} species {species} knows {move_id} outside its learnset",
                        int(trainer, "ID")
                    );
                }
            }
            let ability = int(trainer, &format!("P{slot}Tokusei")) as u16;
            let abilities = data.species(species).expect("species").abilities;
            let original_ability = int(original, &format!("P{slot}Tokusei")) as u16;
            assert!(
                abilities.contains(&ability) || ability == original_ability,
                "trainer ability {ability} for {species}"
            );
        }
    }
}

fn check_species_tables(outputs: &Outputs, data: &GameData) {
    for species in data.species.values() {
        for ability in species.abilities {
            assert!((1..=MAX_ABILITY).contains(&ability));
        }
    }
    let items = read(outputs, BundleId::PersonalMasterdatas, path_ids::ITEM_TABLE);
    let tm_moves: Vec<u16> = array(&items, "WazaMachine")
        .iter()
        .filter(|machine| (1..=100).contains(&int(machine, "machineNo")))
        .map(|machine| int(machine, "wazaNo") as u16)
        .collect();
    assert_eq!(tm_moves.len(), 100);
    let distinct: BTreeSet<u16> = tm_moves.iter().copied().collect();
    assert_eq!(distinct.len(), 100, "TM moves are distinct");
    assert!(tm_moves.iter().all(|&id| data.valid_move(id)));
    for targets in data.evolutions.values() {
        assert!(targets.iter().all(|&target| data.valid_species(target)));
    }
}

fn check_items(outputs: &Outputs, original: &GameData, data: &GameData) {
    let scripts = read(outputs, BundleId::EvScript, path_ids::FLD_ITEM);
    for script in array(&scripts, "Scripts") {
        let Some(command) = array(script, "Commands").first() else {
            continue;
        };
        let arguments = array(command, "Arg");
        if arguments.len() < 2 || int(&arguments[1], "argType") != 1 {
            continue;
        }
        if let Some(item) = decode_item_argument(int(&arguments[1], "data")) {
            assert!(data.items.contains_key(&item), "field item {item} exists");
        }
    }
    let shops = read(outputs, BundleId::Masterdatas, path_ids::SHOP_TABLE);
    for shop in ["FS", "FixedShop"] {
        for entry in array(&shops, shop) {
            let item = int(entry, "ItemNo") as u16;
            assert!(data.items.contains_key(&item), "shop item {item}");
        }
    }
    let marts = array(&shops, "FS");
    let basics: Vec<u16> = marts
        .iter()
        .filter(|entry| int(entry, "ZoneID") == -1)
        .map(|entry| int(entry, "ItemNo") as u16)
        .collect();
    assert!(
        basics
            .iter()
            .all(|&item| original.items.contains_key(&item))
    );
    assert!(
        basics
            .iter()
            .all(|&item| is_randomizable_item(original, item) || item == 0)
    );
}

fn check_exefs(output: &RandomizedMod) {
    assert_eq!(output.exefs_patches.len(), STARTER_OFFSETS.len());
    for (build_id, bytes) in &output.exefs_patches {
        let patch = Ips32Patch::parse(bytes).expect("IPS32 parses");
        assert_eq!(patch.records().len(), 6, "{build_id}");
        for (_, record) in patch.records() {
            let word = u32::from_le_bytes([record[0], record[1], record[2], record[3]]);
            assert_eq!(word & 0xFFE0_0000, 0x5280_0000, "MOVZ instruction");
            let species = (word >> 5) & 0xFFFF;
            assert!((1..=493).contains(&species));
        }
    }
}

fn check_engine(outputs: &Outputs) {
    let asset = &outputs.assets[&BundleId::GlobalGameManagers];
    let time = asset
        .object_data(path_ids::TIME_MANAGER)
        .expect("TimeManager");
    assert_eq!(time.len(), 16);
    let fixed = f32::from_le_bytes([time[0], time[1], time[2], time[3]]);
    assert!(fixed > 0.0 && fixed < 0.1);
}

fn validate(romfs: &Path, name: &str, settings: &BdspSettings, seed: u64, original: &GameData) {
    let bundles = Bundles::load(romfs).expect("romfs loads");
    let output = randomize_bundles(bundles, settings, seed)
        .unwrap_or_else(|error| panic!("{name} seed {seed}: {error}"));
    let again = randomize_bundles(Bundles::load(romfs).expect("romfs loads"), settings, seed)
        .expect("second run");
    assert!(output == again, "{name} seed {seed} is reproducible");
    let outputs = load_outputs(romfs, &output);
    let reloaded = Bundles::from_assets(outputs.assets.clone());
    let data = GameData::load(&reloaded).expect("randomized game data loads");
    check_encounters(&outputs, &data, settings);
    check_underground(&outputs, &data);
    check_learnsets(&outputs, &data, settings);
    let originals = load_outputs(romfs, &RandomizedMod::default());
    check_trainers(&outputs, &originals, &data);
    check_species_tables(&outputs, &data);
    check_items(&outputs, original, &data);
    if settings.starters != randomon_bdsp::StarterMode::Unchanged {
        check_exefs(&output);
        let english = read(&outputs, BundleId::English, path_ids::ENGLISH_SCENARIO1);
        assert!(format!("{english:?}").contains("8-EV_POKESELECT_02"));
    }
    if settings.speed_multiplier_percent != 100 {
        check_engine(&outputs);
    }
    for ability in &BROKEN_ABILITIES {
        if settings.abilities {
            assert!(
                data.species
                    .values()
                    .all(|species| !species.abilities.contains(ability))
            );
        }
    }
    for layout in OutputLayout::ALL {
        let files = output.files(layout, &detect_games(romfs));
        assert!(
            files
                .iter()
                .any(|file| file.path.ends_with(randomon_bdsp::LOG_FILE_NAME))
        );
        assert!(files.iter().any(|file| {
            file.path
                .to_string_lossy()
                .contains("romfs/Data/StreamingAssets")
        }));
    }
    assert!(output.log.contains(&format!("Seed: {seed}")));
}

#[test]
fn randomized_outputs_are_valid() {
    let Some(romfs) = romfs() else {
        eprintln!("{ROMFS_ENV} is not set; skipping BDSP validation");
        return;
    };
    let original = GameData::load(&Bundles::load(&romfs).expect("romfs loads")).expect("data");
    for (name, settings) in setting_grid() {
        for seed in SEEDS {
            validate(&romfs, name, &settings, seed, &original);
        }
    }
}

#[test]
fn different_seeds_differ() {
    let Some(romfs) = romfs() else {
        return;
    };
    let settings = BdspSettings::default();
    let first =
        randomize_bundles(Bundles::load(&romfs).expect("loads"), &settings, 1).expect("runs");
    let second =
        randomize_bundles(Bundles::load(&romfs).expect("loads"), &settings, 2).expect("runs");
    assert!(first.romfs_files != second.romfs_files);
}

#[test]
fn dump_everything_output_when_requested() {
    let (Some(romfs), Ok(directory)) = (romfs(), std::env::var("RANDOMON_BDSP_DUMP")) else {
        return;
    };
    let output = randomize_bundles(
        Bundles::load(&romfs).expect("loads"),
        &everything_on(),
        31_337,
    )
    .expect("runs");
    output
        .write_to(
            &PathBuf::from(directory),
            OutputLayout::Emulator,
            &detect_games(&romfs),
        )
        .expect("writes");
}
