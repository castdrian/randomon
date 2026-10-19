use std::path::{Path, PathBuf};

use randomon_swsh::formats::ahtb::HashTable;
use randomon_swsh::formats::egg_moves::EggMoves;
use randomon_swsh::formats::encounters::{self, EncounterArchive, EncounterSlot, NestArchive};
use randomon_swsh::formats::evolution::{EvolutionSet, method};
use randomon_swsh::formats::flatbuffer::TableRef;
use randomon_swsh::formats::gfpak::{Compression, GfPack};
use randomon_swsh::formats::gifts::{GiftEncounter, GiftEncounterArchive};
use randomon_swsh::formats::items::{ItemHashTable, ItemTable, pouch};
use randomon_swsh::formats::learnset;
use randomon_swsh::formats::moves::{Move, category};
use randomon_swsh::formats::personal::{BASE_RECORD_COUNT, EXPANSION_RECORD_COUNT, PersonalTable};
use randomon_swsh::formats::placement::{self, PlacementArchive};
use randomon_swsh::formats::shops::ShopInventory;
use randomon_swsh::formats::statics::{self, StaticEncounter, StaticEncounterArchive};
use randomon_swsh::formats::text::{TextFile, encode_line};
use randomon_swsh::formats::trades::{TradeEncounter, TradeEncounterArchive};
use randomon_swsh::formats::trainers::{
    TrainerClass, TrainerData, TrainerPokemon, parse_team, write_team,
};
use randomon_swsh::formats::underground::{UndergroundArchive, UndergroundEncounter};
use randomon_swsh::{DataVersion, Game, GameFiles, NameLists, detect, paths};

const ROMFS_ENV: &str = "RANDOMON_SWSH_ROMFS";

fn romfs() -> Option<PathBuf> {
    std::env::var(ROMFS_ENV).ok().map(PathBuf::from)
}

fn read(root: &Path, relative: &str) -> Vec<u8> {
    std::fs::read(root.join(relative)).unwrap_or_else(|error| panic!("{relative}: {error}"))
}

fn version_of(root: &Path) -> DataVersion {
    PersonalTable::parse(&read(root, paths::PERSONAL))
        .expect("personal")
        .data_version()
}

fn indexed_files(root: &Path, file_for: fn(usize) -> String) -> Vec<(usize, Vec<u8>)> {
    (0..)
        .map_while(|index| {
            let path = root.join(file_for(index));
            std::fs::read(path).ok().map(|data| (index, data))
        })
        .collect()
}

fn assert_no_slack(table: &TableRef<'_>, size_of: impl Fn(usize) -> Option<usize>, context: &str) {
    let suspicious = table.fields_with_nonzero_slack(size_of).expect("slack");
    assert!(
        suspicious.is_empty(),
        "{context}: fields {suspicious:?} are wider than declared"
    );
}

fn expected_record_count(version: DataVersion) -> usize {
    match version {
        DataVersion::Base => BASE_RECORD_COUNT,
        DataVersion::Expansion => EXPANSION_RECORD_COUNT,
    }
}

#[test]
fn personal_table_round_trips() {
    let Some(root) = romfs() else { return };
    let original = read(&root, paths::PERSONAL);
    let mut table = PersonalTable::parse(&original).expect("parse");
    assert_eq!(table.len(), expected_record_count(table.data_version()));
    let bulbasaur = &table.entries[1];
    assert_eq!(
        bulbasaur.base_stats.to_hp_atk_def_spe_spa_spd(),
        [45, 49, 49, 45, 65, 65]
    );
    assert_eq!(bulbasaur.types, [11, 3]);
    assert_eq!(bulbasaur.abilities, [65, 65, 34]);
    assert_eq!(bulbasaur.national_dex, 1);
    let grookey = &table.entries[810];
    assert!(grookey.is_present_in_game);
    assert_eq!(grookey.galar_dex, 1);
    assert_eq!(table.entries[52].form_count, 3);
    assert!(table.form_entry(52, 2).is_some());
    let present = table
        .entries
        .iter()
        .filter(|entry| entry.is_present_in_game)
        .count();
    assert!(present > 400, "{present} species present");
    if table.data_version() == DataVersion::Expansion {
        assert!(
            table
                .entries
                .iter()
                .any(|entry| entry.armor_dex.unwrap_or(0) != 0)
        );
        assert!(
            table
                .entries
                .iter()
                .any(|entry| entry.crown_dex.unwrap_or(0) != 0)
        );
    }
    assert_eq!(table.to_bytes().expect("write"), original);
    table.entries[1].base_stats.attack = 123;
    table.entries[1].types = [9, 9];
    let learns_tm_5 = table.entries[1].tm.get(5);
    table.entries[1].tm.set(5, !learns_tm_5);
    let written = table.to_bytes().expect("write");
    let reparsed = PersonalTable::parse(&written).expect("reparse");
    assert_eq!(reparsed.to_bytes().expect("rewrite"), written);
    assert_eq!(reparsed.entries[1].types, [9, 9]);
    assert_eq!(reparsed.entries[1].base_stats.attack, 123);
}

#[test]
fn learnsets_round_trip() {
    let Some(root) = romfs() else { return };
    let original = read(&root, paths::LEARNSETS);
    let mut learnsets = learnset::parse_all(&original).expect("parse");
    assert_eq!(learnsets.len(), expected_record_count(version_of(&root)));
    assert_eq!(learnsets[1].moves[0].move_id, 33);
    let longest = learnsets
        .iter()
        .map(|set| set.moves.len())
        .max()
        .unwrap_or(0);
    assert!(
        longest <= learnset::MAX_MOVES,
        "longest learnset has {longest} moves"
    );
    assert_eq!(learnset::write_all(&learnsets).expect("write"), original);
    learnsets[1].moves.push(learnset::LevelUpMove {
        move_id: 500,
        level: 99,
    });
    let reparsed =
        learnset::parse_all(&learnset::write_all(&learnsets).expect("write")).expect("reparse");
    assert_eq!(
        reparsed[1].moves.last().map(|entry| entry.move_id),
        Some(500)
    );
}

#[test]
fn evolutions_round_trip() {
    let Some(root) = romfs() else { return };
    let files = indexed_files(&root, paths::evolution_file);
    assert_eq!(files.len(), expected_record_count(version_of(&root)));
    for (index, data) in &files {
        let set = EvolutionSet::parse(data).unwrap_or_else(|error| panic!("evo {index}: {error}"));
        assert_eq!(&set.to_bytes().expect("write"), data, "evo {index}");
    }
    let bulbasaur = EvolutionSet::parse(&files[1].1).expect("parse");
    let first = bulbasaur.evolutions[0];
    assert_eq!(
        (first.method, first.species, first.level),
        (method::LEVEL_UP, 2, 16)
    );
    let eevee = EvolutionSet::parse(&files[133].1).expect("parse");
    assert!(eevee.active().count() >= 8);
}

#[test]
fn egg_moves_round_trip() {
    let Some(root) = romfs() else { return };
    let files = indexed_files(&root, paths::egg_move_file);
    assert!(files.len() > expected_record_count(version_of(&root)));
    for (index, data) in &files {
        let moves = EggMoves::parse(data).unwrap_or_else(|error| panic!("egg {index}: {error}"));
        assert_eq!(&moves.to_bytes().expect("write"), data, "egg {index}");
    }
    let bulbasaur = EggMoves::parse(&files[1].1).expect("parse");
    assert_eq!(bulbasaur.moves, vec![130, 80, 174, 275, 267, 92]);
}

#[test]
fn moves_reencode_losslessly() {
    let Some(root) = romfs() else { return };
    let files = indexed_files(&root, paths::move_file);
    let expected = match version_of(&root) {
        DataVersion::Base => 797,
        DataVersion::Expansion => 827,
    };
    assert_eq!(files.len(), expected);
    for (index, data) in &files {
        let parsed = Move::parse(data).unwrap_or_else(|error| panic!("move {index}: {error}"));
        let table = TableRef::root(data).expect("root");
        assert_no_slack(&table, Move::field_size, &format!("move {index}"));
        assert_eq!(Move::parse(&parsed.to_bytes()), Ok(parsed), "move {index}");
    }
    let tackle = Move::parse(&files[33].1).expect("tackle");
    assert_eq!(
        (tackle.move_id, tackle.power, tackle.accuracy, tackle.pp),
        (33, 40, 100, 35)
    );
    assert_eq!((tackle.move_type, tackle.category), (0, category::PHYSICAL));
    assert!(tackle.makes_contact);
    let mut edited = tackle;
    edited.move_type = 9;
    edited.category = category::SPECIAL;
    edited.power = 95;
    edited.accuracy = 90;
    edited.pp = 5;
    edited.priority = 1;
    assert_eq!(Move::parse(&edited.to_bytes()), Ok(edited));
}

#[test]
fn trainers_round_trip() {
    let Some(root) = romfs() else { return };
    let data_files = indexed_files(&root, paths::trainer_data_file);
    let poke_files = indexed_files(&root, paths::trainer_poke_file);
    let expected = match version_of(&root) {
        DataVersion::Base => 315,
        DataVersion::Expansion => 437,
    };
    assert_eq!(data_files.len(), expected);
    assert_eq!(poke_files.len(), expected);
    let mut mismatched_counts = Vec::new();
    for ((index, data), (_, team)) in data_files.iter().zip(&poke_files) {
        let trainer =
            TrainerData::parse(data).unwrap_or_else(|error| panic!("trainer {index}: {error}"));
        assert_eq!(&trainer.to_bytes(), data, "trainer {index}");
        let party = parse_team(team).unwrap_or_else(|error| panic!("team {index}: {error}"));
        assert_eq!(&write_team(&party), team, "team {index}");
        assert!(party.len() <= 6);
        if usize::from(trainer.party_count) != party.len() {
            mismatched_counts.push(*index);
        }
    }
    assert_eq!(mismatched_counts, vec![0]);
    let first = parse_team(&poke_files[1].1).expect("team");
    assert_eq!((first[0].species, first[0].level), (819, 5));
    let mut edited = first[0];
    edited.species = 25;
    edited.ivs.speed = 31;
    edited.shiny = true;
    edited.can_dynamax = true;
    assert_eq!(TrainerPokemon::parse(&edited.to_bytes()), Ok(edited));
    let classes = indexed_files(&root, paths::trainer_class_file);
    assert!(!classes.is_empty());
    for (index, data) in &classes {
        let class =
            TrainerClass::parse(data).unwrap_or_else(|error| panic!("class {index}: {error}"));
        assert_eq!(&class.to_bytes(), data);
    }
}

#[test]
fn items_round_trip() {
    let Some(root) = romfs() else { return };
    let original = read(&root, paths::ITEMS);
    let mut items = ItemTable::parse(&original).expect("parse");
    assert!(items.item_count() > 1500);
    assert_eq!(items.price(4), Some(200));
    assert_eq!(items.price(17), Some(200));
    assert_eq!(items.pouch(4), Some(pouch::BALLS));
    assert_eq!(items.pouch(1230), Some(pouch::MACHINES));
    assert_eq!(items.machines[0].item, 1230);
    assert_eq!(items.machines[0].move_id, 5);
    assert_eq!(items.machines[1].item, 328);
    assert_eq!(items.machines[100].item, 1130);
    assert_eq!(items.machines[199].item, 1229);
    assert!(items.is_shared_record(0));
    assert_eq!(items.to_bytes().expect("write"), original);
    items.set_price(4, 1).expect("price");
    let mut tms = items.tm_moves();
    tms[0] = 33;
    items.set_tm_moves(&tms).expect("tms");
    let reparsed = ItemTable::parse(&items.to_bytes().expect("write")).expect("reparse");
    assert_eq!(reparsed.price(4), Some(1));
    assert_eq!(reparsed.tm_moves()[0], 33);
    assert_eq!(
        reparsed.to_bytes().expect("rewrite"),
        items.to_bytes().expect("write")
    );

    let hashes_original = read(&root, paths::ITEM_HASHES);
    let hashes = ItemHashTable::parse(&hashes_original).expect("hashes");
    assert_eq!(hashes.to_bytes().expect("write"), hashes_original);
    let poke_ball = hashes.hash_of(4).expect("poke ball hash");
    assert_eq!(hashes.item_of(poke_ball), Some(4));
}

fn data_table(root: &Path) -> (Vec<u8>, GfPack) {
    let original = read(root, paths::DATA_TABLE);
    let pack = GfPack::parse(&original).expect("data table");
    (original, pack)
}

#[test]
fn packs_round_trip_exactly() {
    let Some(root) = romfs() else { return };
    for relative in [paths::DATA_TABLE, paths::PLACEMENT] {
        let original = read(&root, relative);
        let pack = GfPack::parse(&original).expect("pack");
        assert!(
            pack.files()
                .iter()
                .all(|file| file.compression == Compression::Lz4)
        );
        assert_eq!(pack.to_bytes().expect("write"), original, "{relative}");
    }
    let (_, mut pack) = data_table(&root);
    for name in [
        encounters::SWORD_HIDDEN_FILE,
        encounters::SWORD_SYMBOL_FILE,
        encounters::SHIELD_HIDDEN_FILE,
        encounters::SHIELD_SYMBOL_FILE,
        encounters::NEST_FILE,
    ] {
        assert!(pack.contains(name), "{name}");
    }
    let replacement = vec![0x5A; 1000];
    pack.replace_file(encounters::NEST_FILE, replacement.clone())
        .expect("replace");
    let reparsed = GfPack::parse(&pack.to_bytes().expect("write")).expect("reparse");
    assert_eq!(
        reparsed.file(encounters::NEST_FILE).expect("nest"),
        replacement
    );
    assert_eq!(
        reparsed
            .file(encounters::SHIELD_HIDDEN_FILE)
            .expect("hidden"),
        pack.file(encounters::SHIELD_HIDDEN_FILE).expect("hidden")
    );
}

fn check_encounter_slack(data: &[u8], context: &str) {
    let root = TableRef::root(data).expect("root");
    for table in root.tables(1).expect("tables") {
        for sub_table in table.tables(1).expect("sub tables") {
            assert_no_slack(&sub_table, |index| (index < 2).then_some(1), context);
            for slot in sub_table.tables(2).expect("slots") {
                assert_no_slack(&slot, EncounterSlot::field_size, context);
            }
        }
        assert_no_slack(&table, |index| (index == 0).then_some(8), context);
    }
}

#[test]
fn wild_encounters_reencode_losslessly() {
    let Some(root) = romfs() else { return };
    let (_, pack) = data_table(&root);
    for (name, tables, sub_tables) in [
        (encounters::SWORD_HIDDEN_FILE, 40, 11),
        (encounters::SHIELD_HIDDEN_FILE, 40, 11),
        (encounters::SWORD_SYMBOL_FILE, 70, 9),
        (encounters::SHIELD_SYMBOL_FILE, 70, 9),
    ] {
        let data = pack.file(name).expect("archive");
        let archive = EncounterArchive::parse(data).expect("parse");
        check_encounter_slack(data, name);
        assert!(
            archive.tables.len() >= tables,
            "{name}: {} tables",
            archive.tables.len()
        );
        assert!(
            archive
                .tables
                .iter()
                .all(|table| table.sub_tables.len() == sub_tables),
            "{name}"
        );
        assert!(archive.slots().any(|slot| slot.species == 819));
        assert!(
            archive
                .slots()
                .all(|slot| slot.species >= 0 && slot.species < 1000)
        );
        let mut edited = archive.clone();
        assert_eq!(EncounterArchive::parse(&edited.to_bytes()), Ok(archive));
        for slot in edited.slots_mut() {
            slot.species = 133;
            slot.form = 0;
        }
        edited.tables[0].sub_tables[0].slots[0].form = 1;
        let reparsed = EncounterArchive::parse(&edited.to_bytes()).expect("reparse");
        assert_eq!(reparsed, edited);
    }
    let nests = NestArchive::parse(pack.file(encounters::NEST_FILE).expect("nest")).expect("parse");
    assert!(!nests.tables.is_empty());
    assert_eq!(NestArchive::parse(&nests.to_bytes()), Ok(nests));
}

#[test]
fn script_encounters_reencode_losslessly() {
    let Some(root) = romfs() else { return };
    let version = version_of(&root);
    let data = read(&root, paths::STATICS);
    let statics = StaticEncounterArchive::parse(&data, version).expect("statics");
    let map = statics::field_map(version);
    let size_of = |file_index: usize| {
        StaticEncounter::FIELD_SIZES
            .iter()
            .find(|(schema, _)| map(*schema) == Some(file_index))
            .map(|(_, size)| *size)
    };
    for entry in TableRef::root(&data)
        .expect("root")
        .tables(0)
        .expect("tables")
    {
        assert_no_slack(&entry, size_of, "statics");
    }
    assert!(statics.encounters.len() >= 257);
    let zacian = statics
        .encounters
        .iter()
        .find(|entry| entry.species == 888)
        .expect("zacian");
    assert_eq!(zacian.held_item, 1103);
    assert_eq!(zacian.level, 70);
    assert!(
        statics
            .encounters
            .iter()
            .all(|entry| entry.level <= 100 && entry.species < 1000)
    );
    assert_eq!(
        StaticEncounterArchive::parse(&statics.to_bytes(version).expect("write"), version),
        Ok(statics.clone())
    );
    let mut edited = statics.clone();
    edited.encounters[0].species = 25;
    edited.encounters[0].form = 8;
    edited.encounters[0].set_moves([1, 2, 3, 4]);
    let reparsed =
        StaticEncounterArchive::parse(&edited.to_bytes(version).expect("write"), version)
            .expect("reparse");
    assert_eq!(reparsed, edited);

    let data = read(&root, paths::GIFTS);
    let gifts = GiftEncounterArchive::parse(&data).expect("gifts");
    for entry in TableRef::root(&data)
        .expect("root")
        .tables(0)
        .expect("tables")
    {
        assert_no_slack(&entry, GiftEncounter::field_size, "gifts");
    }
    assert_eq!(gifts.starter_indices().len(), 3);
    assert_eq!(gifts.gifts[gifts.starter_indices()[0]].level, 5);
    let mut edited = gifts.clone();
    assert_eq!(GiftEncounterArchive::parse(&edited.to_bytes()), Ok(gifts));
    edited.gifts[0].species = 1;
    edited.gifts[0].held_item = 4;
    assert_eq!(GiftEncounterArchive::parse(&edited.to_bytes()), Ok(edited));

    let data = read(&root, paths::TRADES);
    let trades = TradeEncounterArchive::parse(&data).expect("trades");
    for entry in TableRef::root(&data)
        .expect("root")
        .tables(0)
        .expect("tables")
    {
        assert_no_slack(&entry, TradeEncounter::field_size, "trades");
    }
    assert!(trades.trades.len() >= 11);
    assert!(
        trades
            .trades
            .iter()
            .all(|trade| trade.species > 0 && trade.required_species > 0)
    );
    let mut edited = trades.clone();
    assert_eq!(TradeEncounterArchive::parse(&edited.to_bytes()), Ok(trades));
    edited.trades[0].required_species = 133;
    edited.trades[0].required_form = 0;
    assert_eq!(TradeEncounterArchive::parse(&edited.to_bytes()), Ok(edited));
}

#[test]
fn underground_reencodes_when_present() {
    let Some(root) = romfs() else { return };
    let Ok(data) = std::fs::read(root.join(paths::UNDERGROUND)) else {
        assert_eq!(version_of(&root), DataVersion::Base);
        return;
    };
    let archive = UndergroundArchive::parse(&data).expect("underground");
    for entry in TableRef::root(&data)
        .expect("root")
        .tables(0)
        .expect("tables")
    {
        assert_no_slack(&entry, UndergroundEncounter::field_size, "underground");
    }
    assert!(archive.encounters.len() > 200);
    assert!(
        archive
            .encounters
            .iter()
            .all(|entry| entry.species > 0 && entry.level > 0)
    );
    let mut edited = archive.clone();
    assert_eq!(UndergroundArchive::parse(&edited.to_bytes()), Ok(archive));
    edited.encounters[0].species = 150;
    edited.encounters[0].set_moves([94, 0, 0, 0]);
    assert_eq!(UndergroundArchive::parse(&edited.to_bytes()), Ok(edited));
}

#[test]
fn shops_round_trip_in_place() {
    let Some(root) = romfs() else { return };
    let original = read(&root, paths::SHOPS);
    let mut shops = ShopInventory::parse(&original).expect("shops");
    assert!(shops.single.len() >= 150);
    assert!(!shops.multi.is_empty());
    assert_eq!(shops.multi[0].inventories[0][0], 4);
    assert_eq!(shops.to_bytes().expect("write"), original);
    shops.single[0].items[0] = 50;
    shops.multi[0].inventories[0][0] = 1;
    let reparsed = ShopInventory::parse(&shops.to_bytes().expect("write")).expect("reparse");
    assert_eq!(reparsed.single[0].items[0], 50);
    assert_eq!(reparsed.multi[0].inventories[0][0], 1);
}

#[test]
fn placement_round_trips_in_place() {
    let Some(root) = romfs() else { return };
    let pack = GfPack::parse(&read(&root, paths::PLACEMENT)).expect("placement");
    let names =
        HashTable::parse(pack.file(placement::AREA_NAME_TABLE).expect("names")).expect("ahtb");
    assert_eq!(
        HashTable::parse(&names.to_bytes().expect("write")),
        Ok(names.clone())
    );
    let mut areas = 0;
    let mut field_items = 0;
    let mut hidden_items = 0;
    for file_name in placement::area_file_names(&names) {
        let Some(index) = pack.index_of(&file_name) else {
            continue;
        };
        let data = pack.data(index).expect("data");
        let archive =
            PlacementArchive::parse(data).unwrap_or_else(|error| panic!("{file_name}: {error}"));
        assert_eq!(archive.to_bytes().expect("write"), data, "{file_name}");
        areas += 1;
        field_items += archive
            .zones
            .iter()
            .map(|zone| zone.field_items.len())
            .sum::<usize>();
        hidden_items += archive
            .zones
            .iter()
            .map(|zone| zone.hidden_items.len())
            .sum::<usize>();
    }
    assert!(areas > 50, "{areas} areas");
    assert!(field_items > 100, "{field_items} field items");
    assert!(hidden_items > 100, "{hidden_items} hidden items");

    let mut start =
        PlacementArchive::parse(pack.file(placement::STARTER_AREA_FILE).expect("a_0101"))
            .expect("parse");
    let critters: Vec<u32> = start.zones[0]
        .critters
        .iter()
        .map(|critter| critter.species)
        .collect();
    assert_eq!(&critters[1..4], &[813, 816, 810]);
    start.zones[0].critters[1].species = 1;
    if let Some(item) = start
        .zones
        .iter_mut()
        .flat_map(|zone| zone.field_items.iter_mut())
        .find(|item| !item.item_hashes.is_empty())
    {
        item.item_hashes[0] = 0x1234;
    }
    let reparsed = PlacementArchive::parse(&start.to_bytes().expect("write")).expect("reparse");
    assert_eq!(reparsed.zones, start.zones);
}

fn text_files(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for folder in ["common", "script"] {
        let directory = root
            .join(paths::MESSAGES)
            .join(paths::DEFAULT_LANGUAGE)
            .join(folder);
        let entries = std::fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("{}: {error}", directory.display()));
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|value| value == extension) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn text_files_round_trip() {
    let Some(root) = romfs() else { return };
    let files = text_files(&root, "dat");
    assert!(files.len() > 300);
    let mut lines = 0;
    let mut lossy_lines = Vec::new();
    for path in &files {
        let data = std::fs::read(path).expect("read");
        let text =
            TextFile::parse(&data).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(text.to_bytes().expect("write"), data, "{}", path.display());
        for (index, line) in text.lines.iter().enumerate() {
            lines += 1;
            let reencoded = encode_line(&line.text())
                .unwrap_or_else(|error| panic!("{} line {index}: {error}", path.display()));
            if reencoded != line.units {
                lossy_lines.push(format!("{}:{index}", path.display()));
            }
        }
    }
    assert!(lines > 10_000);
    assert!(
        lossy_lines.is_empty(),
        "{} lines do not survive decode/encode: {:?}",
        lossy_lines.len(),
        &lossy_lines[..lossy_lines.len().min(10)]
    );
    for path in text_files(&root, "tbl") {
        let data = std::fs::read(&path).expect("read");
        let table =
            HashTable::parse(&data).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(table.to_bytes().expect("write"), data, "{}", path.display());
    }
}

#[test]
fn name_lists_load_by_role() {
    let Some(root) = romfs() else { return };
    let names = NameLists::load_english(&root).expect("names");
    let version = version_of(&root);
    assert_eq!(names.species[1], "Bulbasaur");
    assert_eq!(names.species[810], "Grookey");
    assert_eq!(names.moves[33], "Tackle");
    assert_eq!(names.items[4], "Poké Ball");
    assert_eq!(names.abilities[65], "Overgrow");
    assert_eq!(names.types[9], "Fire");
    assert_eq!(names.natures[0], "Hardy");
    assert!(names.trainer_names.len() > 100);
    assert!(names.trainer_classes.len() > 50);
    let species = if version == DataVersion::Expansion {
        899
    } else {
        891
    };
    assert!(names.species.len() >= species);
    let mut text = TextFile::parse(&read(
        &root,
        &randomon_swsh::TextRole::SpeciesNames.path("English"),
    ))
    .expect("text");
    text.set_line(1, "Bulbasaurus Maximus").expect("set");
    let reparsed = TextFile::parse(&text.to_bytes().expect("write")).expect("reparse");
    assert_eq!(reparsed.line(1).as_deref(), Some("Bulbasaurus Maximus"));
    assert_eq!(reparsed.line(2).as_deref(), Some("Ivysaur"));
}

#[test]
fn game_files_track_modifications() {
    let Some(root) = romfs() else { return };
    let mut files = GameFiles::load(&root).expect("load");
    assert_eq!(files.version(), version_of(&root));
    assert!(!files.is_modified());
    assert_eq!(files.modified_files().expect("unmodified"), Vec::new());
    files.data.personal.entries[1].base_stats.hp = 99;
    files.data.moves[33].power = 50;
    let lead = files.data.trainers[1].team[0];
    files.data.trainers[1].team.push(lead);
    files.data.wild.shield_hidden.tables[0].sub_tables[0].slots[0].species = 25;
    files.data.gifts.gifts[0].species = 1;
    files.data.placement[0].archive.zones[0].critters[0].species = 4;
    files
        .data
        .texts
        .get_mut(&randomon_swsh::TextRole::SpeciesNames)
        .expect("species")
        .set_line(1, "Bulba")
        .expect("set");
    let modified = files.modified_files().expect("modified");
    let paths: Vec<&str> = modified
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect();
    assert_eq!(
        paths,
        vec![
            paths::PERSONAL.to_string(),
            paths::move_file(33),
            paths::trainer_data_file(1),
            paths::trainer_poke_file(1),
            paths::DATA_TABLE.to_string(),
            paths::GIFTS.to_string(),
            paths::PLACEMENT.to_string(),
            randomon_swsh::TextRole::SpeciesNames.path("English"),
        ]
    );
    let by_path = |path: &str| {
        modified
            .iter()
            .find(|file| file.relative_path == path)
            .map(|file| file.data.clone())
            .expect("file")
    };
    let pack = GfPack::parse(&by_path(paths::DATA_TABLE)).expect("pack");
    let wild = EncounterArchive::parse(pack.file(encounters::SHIELD_HIDDEN_FILE).expect("hidden"))
        .expect("wild");
    assert_eq!(wild.tables[0].sub_tables[0].slots[0].species, 25);
    let (_, original_pack) = data_table(&root);
    assert_eq!(
        pack.file(encounters::SWORD_HIDDEN_FILE).expect("sword"),
        original_pack
            .file(encounters::SWORD_HIDDEN_FILE)
            .expect("sword")
    );
    let trainer = TrainerData::parse(&by_path(&paths::trainer_data_file(1))).expect("trainer");
    assert_eq!(trainer.party_count, 2);
    assert_eq!(
        parse_team(&by_path(&paths::trainer_poke_file(1)))
            .expect("team")
            .len(),
        2
    );
    let personal = PersonalTable::parse(&by_path(paths::PERSONAL)).expect("personal");
    assert_eq!(personal.entries[1].base_stats.hp, 99);
}

#[test]
fn romfs_is_detected() {
    let Some(root) = romfs() else { return };
    let detection = detect(&root).expect("detected");
    assert_eq!(detection.version, version_of(&root));
    if detection.title_id.is_some() {
        assert!(detection.game.is_some());
        assert!(detection.build_id.is_some());
    }
    if let Some(game) = detection.game {
        assert!(Game::ALL.contains(&game));
    }
}
