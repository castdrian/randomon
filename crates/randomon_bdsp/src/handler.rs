use std::collections::BTreeMap;
use std::path::Path;

use randomon_core::{HandlerCore, OutputFile, SwitchLayout, SwitchMod};
use randomon_data::constants::species;
use randomon_data::{GameData, ItemList, PokemonId};
use randomon_java::SharedRandomSource;

use crate::bundles::Bundles;
use crate::constants;
use crate::error::Result;
use crate::exefs::{Game, starter_patches};
use crate::statics::{ScriptAsset, load_battle_scripts};
use crate::tables::Tables;
use crate::text::{
    ABILITY_NAMES, ITEM_NAMES, MOVE_NAMES, MessageTable, SCENARIO_TABLES, TRAINER_CLASS_NAMES,
    TRAINER_NAMES,
};

pub const EXEFS_PATCH_FOLDER: &str = "randomon_bdsp";
pub const DEFAULT_STARTERS: [i32; 3] = [species::TURTWIG, species::CHIMCHAR, species::PIPLUP];

pub struct Messages {
    pub moves: MessageTable,
    pub items: MessageTable,
    pub abilities: MessageTable,
    pub trainer_names: MessageTable,
    pub trainer_classes: MessageTable,
    pub scenarios: Vec<MessageTable>,
}

impl Messages {
    fn load(bundles: &Bundles) -> Result<Self> {
        let mut names = vec![
            MOVE_NAMES,
            ITEM_NAMES,
            ABILITY_NAMES,
            TRAINER_NAMES,
            TRAINER_CLASS_NAMES,
        ];
        names.extend(SCENARIO_TABLES);
        let mut tables = MessageTable::load_all(bundles, &names)?.into_iter();
        let mut next = || tables.next().expect("one table per requested name");
        let moves = next();
        let items = next();
        let abilities = next();
        let trainer_names = next();
        let trainer_classes = next();
        let scenarios = tables.collect();
        Ok(Self {
            moves,
            items,
            abilities,
            trainer_names,
            trainer_classes,
            scenarios,
        })
    }

    pub fn scenario_with_label(&mut self, label: &str) -> Option<&mut MessageTable> {
        self.scenarios
            .iter_mut()
            .find(|table| table.has_label(label))
    }

    pub fn scenario_text(&self, label: &str) -> Option<String> {
        self.scenarios.iter().find_map(|table| table.text_of(label))
    }

    fn write_back(&self, bundles: &mut Bundles) -> Result<()> {
        for table in [
            &self.moves,
            &self.items,
            &self.abilities,
            &self.trainer_names,
            &self.trainer_classes,
        ]
        .into_iter()
        .chain(&self.scenarios)
        {
            table.write_back(bundles)?;
        }
        Ok(())
    }
}

pub struct BdspRomHandler {
    pub(crate) core: HandlerCore,
    pub(crate) bundles: Bundles,
    pub(crate) tables: Tables,
    pub(crate) messages: Messages,
    pub(crate) pokes: Vec<Option<PokemonId>>,
    pub(crate) first_forme: BTreeMap<i32, (i32, i32)>,
    pub(crate) item_names: Vec<String>,
    pub(crate) ability_names: Vec<String>,
    pub(crate) allowed_items: ItemList,
    pub(crate) non_bad_items: ItemList,
    pub(crate) field_item_rules: ItemList,
    pub(crate) tm_items: Vec<i32>,
    pub(crate) battle_scripts: Vec<ScriptAsset>,
    pub(crate) starters: [i32; 3],
    pub(crate) layout: SwitchLayout,
    pub(crate) file_name: String,
}

pub(crate) fn index_texts(table: &MessageTable, count: usize) -> Result<Vec<String>> {
    let by_index = table.by_index()?;
    Ok((0..count as i32)
        .map(|index| by_index.get(&index).cloned().unwrap_or_default())
        .collect())
}

impl BdspRomHandler {
    pub fn load(
        romfs: &Path,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> randomon_core::Result<Self> {
        let bundles = Bundles::load(romfs)?;
        let tables = Tables::load(&bundles)?;
        let messages = Messages::load(&bundles)?;
        let item_names = index_texts(&messages.items, constants::HIGHEST_ITEM_INDEX + 1)?;
        let ability_names = index_texts(
            &messages.abilities,
            constants::HIGHEST_ABILITY_INDEX as usize + 1,
        )?;
        let mut handler = Self {
            core: HandlerCore::new(random_source),
            bundles,
            tables,
            messages,
            pokes: Vec::new(),
            first_forme: BTreeMap::new(),
            item_names,
            ability_names,
            allowed_items: ItemList::new(constants::HIGHEST_ITEM_INDEX),
            non_bad_items: ItemList::new(constants::HIGHEST_ITEM_INDEX),
            field_item_rules: ItemList::new(constants::HIGHEST_ITEM_INDEX),
            tm_items: Vec::new(),
            battle_scripts: Vec::new(),
            starters: DEFAULT_STARTERS,
            layout: SwitchLayout::default(),
            file_name: file_name.to_owned(),
        };
        handler.core.data = GameData::new();
        handler.load_moves()?;
        handler.load_pokemon()?;
        handler.load_items()?;
        handler.battle_scripts = load_battle_scripts(&handler.bundles)?;
        Ok(handler)
    }

    pub(crate) fn pk(&self, number: i32) -> PokemonId {
        PokemonId::from_index(number as usize)
    }

    pub(crate) fn alt_forme_number(&self, species_number: i32, forme: i32) -> Option<i32> {
        let &(first, count) = self.first_forme.get(&species_number)?;
        (forme > 0 && forme < count).then_some(first + forme - 1)
    }

    pub(crate) fn pokemon_with_forme(&self, species_number: i32, forme: i32) -> PokemonId {
        let number = self
            .alt_forme_number(species_number, forme)
            .unwrap_or(species_number);
        self.pk(number)
    }

    pub(crate) fn alt_forme_of(&self, pokemon: PokemonId, forme: i32) -> PokemonId {
        let base = self.core.data.base_number(pokemon);
        let Some(number) = self.alt_forme_number(base, forme) else {
            return pokemon;
        };
        let alternate = self.core.data.pk(self.pk(number));
        if alternate.actually_cosmetic {
            alternate.base_forme.unwrap_or(pokemon)
        } else {
            self.pk(number)
        }
    }

    pub(crate) fn save_files(&mut self) -> randomon_core::Result<Vec<OutputFile>> {
        self.save_pokemon()?;
        self.save_moves()?;
        self.tables.write_back(&mut self.bundles)?;
        self.write_back_battle_scripts()?;
        self.messages.write_back(&mut self.bundles)?;
        let modified: Vec<_> = self.bundles.modified().collect();
        let mut romfs_files = Vec::new();
        for bundle in modified {
            romfs_files.push((
                format!("Data/{}", bundle.relative_path()),
                self.bundles.serialize(bundle)?,
            ));
        }
        let exefs_patches = if self.starters == DEFAULT_STARTERS {
            Vec::new()
        } else {
            let [grass, fire, water] = self.starters.map(|number| number as u16);
            starter_patches([grass, fire, water])
        };
        let switch_mod = SwitchMod {
            title_ids: Game::ALL
                .iter()
                .map(|game| game.title_id().to_owned())
                .collect(),
            exefs_patch_folder: EXEFS_PATCH_FOLDER.to_owned(),
            romfs_files,
            exefs_patches,
        };
        Ok(switch_mod.files(self.layout))
    }
}
