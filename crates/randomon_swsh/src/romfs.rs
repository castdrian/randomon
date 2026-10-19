use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{Result, SwshError, invalid};
use crate::formats::ahtb::HashTable;
use crate::formats::egg_moves::EggMoves;
use crate::formats::encounters::{self, EncounterArchive, NestArchive};
use crate::formats::evolution::EvolutionSet;
use crate::formats::gfpak::GfPack;
use crate::formats::gifts::GiftEncounterArchive;
use crate::formats::items::{ItemHashTable, ItemTable};
use crate::formats::learnset::{self, Learnset};
use crate::formats::moves::Move;
use crate::formats::personal::PersonalTable;
use crate::formats::placement::{self, PlacementArchive};
use crate::formats::shops::ShopInventory;
use crate::formats::statics::StaticEncounterArchive;
use crate::formats::text::TextFile;
use crate::formats::trades::TradeEncounterArchive;
use crate::formats::trainers::{Trainer, TrainerClass};
use crate::formats::underground::UndergroundArchive;
use crate::io::RomfsLayers;
use crate::names::{NameLists, TextRole, load_texts};
use crate::paths;
use crate::version::DataVersion;

const CONTEXT: &str = "romfs";

#[derive(Debug, Clone, PartialEq)]
pub struct WildEncounters {
    pub sword_hidden: EncounterArchive,
    pub sword_symbol: EncounterArchive,
    pub shield_hidden: EncounterArchive,
    pub shield_symbol: EncounterArchive,
}

impl WildEncounters {
    fn load(pack: &GfPack) -> Result<Self> {
        let archive = |name: &str| EncounterArchive::parse(pack.file(name)?);
        Ok(WildEncounters {
            sword_hidden: archive(encounters::SWORD_HIDDEN_FILE)?,
            sword_symbol: archive(encounters::SWORD_SYMBOL_FILE)?,
            shield_hidden: archive(encounters::SHIELD_HIDDEN_FILE)?,
            shield_symbol: archive(encounters::SHIELD_SYMBOL_FILE)?,
        })
    }

    pub fn by_file(&self) -> [(&'static str, &EncounterArchive); 4] {
        [
            (encounters::SWORD_HIDDEN_FILE, &self.sword_hidden),
            (encounters::SWORD_SYMBOL_FILE, &self.sword_symbol),
            (encounters::SHIELD_HIDDEN_FILE, &self.shield_hidden),
            (encounters::SHIELD_SYMBOL_FILE, &self.shield_symbol),
        ]
    }

    pub fn all_mut(&mut self) -> [&mut EncounterArchive; 4] {
        [
            &mut self.sword_hidden,
            &mut self.sword_symbol,
            &mut self.shield_hidden,
            &mut self.shield_symbol,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacementArea {
    pub file_name: String,
    pub archive: PlacementArchive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameData {
    pub personal: PersonalTable,
    pub learnsets: Vec<Learnset>,
    pub evolutions: Vec<EvolutionSet>,
    pub egg_moves: Vec<EggMoves>,
    pub moves: Vec<Move>,
    pub trainers: Vec<Trainer>,
    pub trainer_classes: Vec<TrainerClass>,
    pub items: ItemTable,
    pub item_hashes: ItemHashTable,
    pub wild: WildEncounters,
    pub nests: NestArchive,
    pub statics: StaticEncounterArchive,
    pub gifts: GiftEncounterArchive,
    pub trades: TradeEncounterArchive,
    pub shops: ShopInventory,
    pub underground: Option<UndergroundArchive>,
    pub placement: Vec<PlacementArea>,
    pub texts: BTreeMap<TextRole, TextFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifiedFile {
    pub relative_path: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GameFiles {
    layers: RomfsLayers,
    language: String,
    version: DataVersion,
    data_table: GfPack,
    placement_pack: GfPack,
    pristine: Box<GameData>,
    pub data: GameData,
}

fn parse_relative<T>(
    root: &RomfsLayers,
    relative: &str,
    parse: impl FnOnce(&[u8]) -> Result<T>,
) -> Result<T> {
    let bytes = root.read(relative)?;
    parse(&bytes).map_err(|error| error.in_file(root.path_of(relative)))
}

fn parse_indexed<T>(
    root: &RomfsLayers,
    count: usize,
    file_for: impl Fn(usize) -> String,
    parse: impl Fn(&[u8]) -> Result<T>,
) -> Result<Vec<T>> {
    (0..count)
        .map(|index| parse_relative(root, &file_for(index), &parse))
        .collect()
}

fn require_count(root: &RomfsLayers, relative: &str, count: usize) -> Result<usize> {
    if count == 0 {
        return Err(SwshError::MissingFile(root.base().join(relative)));
    }
    Ok(count)
}

fn load_trainers(root: &RomfsLayers) -> Result<Vec<Trainer>> {
    let count = require_count(
        root,
        paths::TRAINER_DATA,
        root.count_indexed_files(paths::trainer_data_file),
    )?;
    (0..count)
        .map(|index| {
            let data = root.read(&paths::trainer_data_file(index))?;
            let team = root.read(&paths::trainer_poke_file(index))?;
            Trainer::parse(&data, &team)
                .map_err(|error| error.in_file(root.path_of(&paths::trainer_data_file(index))))
        })
        .collect()
}

fn load_placement(pack: &GfPack) -> Result<Vec<PlacementArea>> {
    let names = HashTable::parse(pack.file(placement::AREA_NAME_TABLE)?)?;
    placement::area_file_names(&names)
        .into_iter()
        .filter(|file_name| pack.contains(file_name))
        .map(|file_name| {
            let archive = PlacementArchive::parse(pack.file(&file_name)?)
                .map_err(|error| error.in_file(format!("{}:{file_name}", paths::PLACEMENT)))?;
            Ok(PlacementArea { file_name, archive })
        })
        .collect()
}

fn optional<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(SwshError::MissingFile(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

impl GameFiles {
    pub fn load(root: impl Into<PathBuf>) -> Result<Self> {
        GameFiles::load_with_language(root, paths::DEFAULT_LANGUAGE)
    }

    pub fn load_with_language(root: impl Into<PathBuf>, language: &str) -> Result<Self> {
        GameFiles::load_layered(RomfsLayers::new(root), language)
    }

    pub fn load_layered(root: RomfsLayers, language: &str) -> Result<Self> {
        let personal = parse_relative(&root, paths::PERSONAL, PersonalTable::parse)?;
        let version = personal.data_version();
        let learnsets = parse_relative(&root, paths::LEARNSETS, learnset::parse_all)?;
        if learnsets.len() != personal.len() {
            return Err(invalid(
                CONTEXT,
                format!(
                    "{} learnsets for {} personal records",
                    learnsets.len(),
                    personal.len()
                ),
            ));
        }
        let evolution_count = require_count(
            &root,
            paths::EVOLUTIONS,
            root.count_indexed_files(paths::evolution_file),
        )?;
        let egg_move_count = require_count(
            &root,
            paths::EGG_MOVES,
            root.count_indexed_files(paths::egg_move_file),
        )?;
        let move_count = require_count(
            &root,
            paths::MOVES,
            root.count_indexed_files(paths::move_file),
        )?;
        let class_count = root.count_indexed_files(paths::trainer_class_file);
        let data_table = parse_relative(&root, paths::DATA_TABLE, GfPack::parse)?;
        let placement_pack = parse_relative(&root, paths::PLACEMENT, GfPack::parse)?;
        let data = GameData {
            learnsets,
            evolutions: parse_indexed(
                &root,
                evolution_count,
                paths::evolution_file,
                EvolutionSet::parse,
            )?,
            egg_moves: parse_indexed(&root, egg_move_count, paths::egg_move_file, EggMoves::parse)?,
            moves: parse_indexed(&root, move_count, paths::move_file, Move::parse)?,
            trainers: load_trainers(&root)?,
            trainer_classes: parse_indexed(
                &root,
                class_count,
                paths::trainer_class_file,
                TrainerClass::parse,
            )?,
            items: parse_relative(&root, paths::ITEMS, ItemTable::parse)?,
            item_hashes: parse_relative(&root, paths::ITEM_HASHES, ItemHashTable::parse)?,
            wild: WildEncounters::load(&data_table)
                .map_err(|error| error.in_file(root.path_of(paths::DATA_TABLE)))?,
            nests: NestArchive::parse(data_table.file(encounters::NEST_FILE)?)
                .map_err(|error| error.in_file(root.path_of(paths::DATA_TABLE)))?,
            statics: parse_relative(&root, paths::STATICS, |bytes| {
                StaticEncounterArchive::parse(bytes, version)
            })?,
            gifts: parse_relative(&root, paths::GIFTS, GiftEncounterArchive::parse)?,
            trades: parse_relative(&root, paths::TRADES, TradeEncounterArchive::parse)?,
            shops: parse_relative(&root, paths::SHOPS, ShopInventory::parse)?,
            underground: optional(parse_relative(
                &root,
                paths::UNDERGROUND,
                UndergroundArchive::parse,
            ))?,
            placement: load_placement(&placement_pack)?,
            texts: load_texts(&root, language)?,
            personal,
        };
        Ok(GameFiles {
            layers: root,
            language: language.to_string(),
            version,
            data_table,
            placement_pack,
            pristine: Box::new(data.clone()),
            data,
        })
    }

    pub fn root(&self) -> &Path {
        self.layers.base()
    }

    pub fn layers(&self) -> &RomfsLayers {
        &self.layers
    }

    pub fn version(&self) -> DataVersion {
        self.version
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn original(&self) -> &GameData {
        &self.pristine
    }

    pub fn names(&self) -> NameLists {
        NameLists::from_texts(&self.data.texts)
    }

    pub fn is_modified(&self) -> bool {
        self.data != *self.pristine
    }

    pub fn modified_files(&self) -> Result<Vec<ModifiedFile>> {
        let mut files = Vec::new();
        let current = &self.data;
        let original = &*self.pristine;
        let mut push = |relative_path: String, data: Vec<u8>| {
            files.push(ModifiedFile {
                relative_path,
                data,
            })
        };
        if current.personal != original.personal {
            push(paths::PERSONAL.to_string(), current.personal.to_bytes()?);
        }
        if current.learnsets != original.learnsets {
            push(
                paths::LEARNSETS.to_string(),
                learnset::write_all(&current.learnsets)?,
            );
        }
        for (index, set) in changed(&current.evolutions, &original.evolutions)? {
            push(paths::evolution_file(index), set.to_bytes()?);
        }
        for (index, moves) in changed(&current.egg_moves, &original.egg_moves)? {
            push(paths::egg_move_file(index), moves.to_bytes()?);
        }
        for (index, entry) in changed(&current.moves, &original.moves)? {
            push(paths::move_file(index), entry.to_bytes());
        }
        for (index, trainer) in changed(&current.trainers, &original.trainers)? {
            let before = &original.trainers[index];
            let (data, team) = trainer.to_bytes()?;
            if trainer.data != before.data || trainer.team.len() != before.team.len() {
                push(paths::trainer_data_file(index), data);
            }
            if trainer.team != before.team {
                push(paths::trainer_poke_file(index), team);
            }
        }
        for (index, class) in changed(&current.trainer_classes, &original.trainer_classes)? {
            push(paths::trainer_class_file(index), class.to_bytes());
        }
        if current.items != original.items {
            push(paths::ITEMS.to_string(), current.items.to_bytes()?);
        }
        if current.item_hashes != original.item_hashes {
            push(
                paths::ITEM_HASHES.to_string(),
                current.item_hashes.to_bytes()?,
            );
        }
        if let Some(pack) = self.updated_data_table()? {
            push(paths::DATA_TABLE.to_string(), pack);
        }
        if current.statics != original.statics {
            push(
                paths::STATICS.to_string(),
                current.statics.to_bytes(self.version)?,
            );
        }
        if current.gifts != original.gifts {
            push(paths::GIFTS.to_string(), current.gifts.to_bytes());
        }
        if current.trades != original.trades {
            push(paths::TRADES.to_string(), current.trades.to_bytes());
        }
        if current.shops != original.shops {
            push(paths::SHOPS.to_string(), current.shops.to_bytes()?);
        }
        match (&current.underground, &original.underground) {
            (Some(now), Some(before)) if now != before => {
                push(paths::UNDERGROUND.to_string(), now.to_bytes())
            }
            (Some(_), None) | (None, Some(_)) => {
                return Err(invalid(
                    CONTEXT,
                    "the underground archive cannot be added or removed",
                ));
            }
            _ => {}
        }
        if let Some(pack) = self.updated_placement()? {
            push(paths::PLACEMENT.to_string(), pack);
        }
        for (role, text) in &current.texts {
            if original.texts.get(role) != Some(text) {
                push(role.path(&self.language), text.to_bytes()?);
            }
        }
        Ok(files)
    }

    fn updated_data_table(&self) -> Result<Option<Vec<u8>>> {
        let current = &self.data;
        let original = &*self.pristine;
        if current.wild == original.wild && current.nests == original.nests {
            return Ok(None);
        }
        let mut pack = self.data_table.clone();
        for ((file_name, now), (_, before)) in current
            .wild
            .by_file()
            .into_iter()
            .zip(original.wild.by_file())
        {
            if now != before {
                pack.replace_file(file_name, now.to_bytes())?;
            }
        }
        if current.nests != original.nests {
            pack.replace_file(encounters::NEST_FILE, current.nests.to_bytes())?;
        }
        Ok(Some(pack.to_bytes()?))
    }

    fn updated_placement(&self) -> Result<Option<Vec<u8>>> {
        let current = &self.data.placement;
        let original = &self.pristine.placement;
        if current == original {
            return Ok(None);
        }
        if current.len() != original.len() {
            return Err(invalid(
                CONTEXT,
                "placement areas cannot be added or removed",
            ));
        }
        let mut pack = self.placement_pack.clone();
        for (area, before) in current.iter().zip(original) {
            if area.file_name != before.file_name {
                return Err(invalid(CONTEXT, "placement areas cannot be renamed"));
            }
            if area.archive != before.archive {
                pack.replace_file(&area.file_name, area.archive.to_bytes()?)?;
            }
        }
        Ok(Some(pack.to_bytes()?))
    }

    pub fn write_modified(&self, output_romfs: &Path) -> Result<Vec<PathBuf>> {
        let mut written = Vec::new();
        for file in self.modified_files()? {
            let path = output_romfs.join(&file.relative_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| SwshError::Io(parent.to_path_buf(), error.to_string()))?;
            }
            std::fs::write(&path, &file.data)
                .map_err(|error| SwshError::Io(path.clone(), error.to_string()))?;
            written.push(path);
        }
        Ok(written)
    }
}

fn changed<'a, T: PartialEq>(current: &'a [T], original: &[T]) -> Result<Vec<(usize, &'a T)>> {
    if current.len() != original.len() {
        return Err(invalid(CONTEXT, "indexed files cannot be added or removed"));
    }
    Ok(current
        .iter()
        .zip(original)
        .enumerate()
        .filter(|(_, (now, before))| now != before)
        .map(|(index, (now, _))| (index, now))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_reports_only_differing_indices() {
        let before = [1, 2, 3];
        let after = [1, 5, 3];
        assert_eq!(changed(&after, &before), Ok(vec![(1, &5)]));
        assert!(changed(&after[..2], &before).is_err());
    }

    #[test]
    fn missing_romfs_is_reported() {
        assert!(matches!(
            GameFiles::load("/nonexistent/romfs"),
            Err(SwshError::MissingFile(_))
        ));
    }

    #[test]
    fn optional_files_tolerate_absence_only() {
        assert_eq!(
            optional::<u8>(Err(SwshError::MissingFile(PathBuf::from("x")))),
            Ok(None)
        );
        assert!(optional::<u8>(Err(invalid("test", "bad"))).is_err());
    }
}
