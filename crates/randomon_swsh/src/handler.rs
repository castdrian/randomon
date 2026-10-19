use std::path::{Path, PathBuf};

use randomon_core::{HandlerCore, OutputFile, RandomizationError, SwitchLayout, SwitchMod};
use randomon_data::{GameData, ItemList, PokemonId};
use randomon_java::SharedRandomSource;

use crate::constants;
use crate::evolutions::RawEvolution;
use crate::{DataVersion, Detection, Game, GameFiles, RomfsLayers, TextRole, detect};

pub const EXEFS_PATCH_FOLDER: &str = "randomon_swsh";

pub struct SwshRomHandler {
    pub(crate) core: HandlerCore,
    pub(crate) files: GameFiles,
    pub(crate) random_source: SharedRandomSource,
    pub(crate) detection: Option<Detection>,
    pub(crate) species_count: i32,
    pub(crate) pokes: Vec<Option<PokemonId>>,
    pub(crate) item_names: Vec<String>,
    pub(crate) ability_names: Vec<String>,
    pub(crate) allowed_items: ItemList,
    pub(crate) non_bad_items: ItemList,
    pub(crate) tm_items: Vec<i32>,
    pub(crate) raw_evolutions: Vec<Vec<RawEvolution>>,
    pub(crate) dropped_evolutions: usize,
    pub(crate) update_loaded: bool,
    pub(crate) layout: SwitchLayout,
    pub(crate) file_name: String,
}

pub(crate) fn swsh_error(error: crate::SwshError) -> RandomizationError {
    RandomizationError::Io(error.to_string())
}

impl SwshRomHandler {
    pub fn load(
        romfs: &Path,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> randomon_core::Result<Self> {
        Self::load_layers(&[romfs.to_path_buf()], file_name, random_source)
    }

    pub fn load_layers(
        roots: &[PathBuf],
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> randomon_core::Result<Self> {
        let layers = RomfsLayers::from_roots(roots)
            .ok_or_else(|| RandomizationError::UnsupportedRom(file_name.to_owned()))?;
        let detection = detect(layers.base());
        let files =
            GameFiles::load_layered(layers, crate::paths::DEFAULT_LANGUAGE).map_err(swsh_error)?;
        let names = files.names();
        let mut handler = Self {
            core: HandlerCore::new(random_source),
            species_count: constants::species_count(files.version()),
            files,
            random_source: random_source.clone(),
            detection,
            pokes: Vec::new(),
            item_names: names.items,
            ability_names: names.abilities,
            allowed_items: ItemList::new(constants::HIGHEST_ITEM_INDEX),
            non_bad_items: ItemList::new(constants::HIGHEST_ITEM_INDEX),
            tm_items: Vec::new(),
            raw_evolutions: Vec::new(),
            dropped_evolutions: 0,
            update_loaded: roots.len() > 1,
            layout: SwitchLayout::default(),
            file_name: file_name.to_owned(),
        };
        handler.core.data = GameData::new();
        handler.load_moves();
        handler.load_pokemon(&names.species);
        handler.load_items();
        Ok(handler)
    }

    pub fn files(&self) -> &GameFiles {
        &self.files
    }

    pub fn version(&self) -> DataVersion {
        self.files.version()
    }

    pub fn game(&self) -> Option<Game> {
        self.detection.as_ref().and_then(|detection| detection.game)
    }

    pub fn reload_with_update(&mut self, update: &Path) -> randomon_core::Result<()> {
        let mut roots = self.files.layers().roots().to_vec();
        roots.push(update.to_path_buf());
        let reloaded = Self::load_layers(&roots, &self.file_name, &self.random_source)?;
        let layout = self.layout;
        *self = reloaded;
        self.layout = layout;
        Ok(())
    }

    pub(crate) fn existing(&self, number: i32) -> Option<PokemonId> {
        self.pokes
            .get(usize::try_from(number).ok()?)
            .copied()
            .flatten()
    }

    pub(crate) fn alt_forme_number(&self, species: i32, forme: i32) -> Option<i32> {
        let entry = self
            .files
            .data
            .personal
            .get(usize::try_from(species).ok()?)?;
        let index = entry.alternate_form_index(u16::try_from(forme).ok()?)? as i32;
        self.existing(index).map(|_| index)
    }

    pub(crate) fn pokemon_with_forme(&self, species: i32, forme: i32) -> Option<PokemonId> {
        if species < 1 || species > self.species_count {
            return None;
        }
        let base = self.existing(species)?;
        Some(
            self.alt_forme_number(species, forme)
                .and_then(|number| self.existing(number))
                .unwrap_or(base),
        )
    }

    pub(crate) fn alt_forme_of(&self, pokemon: PokemonId, forme: i32) -> PokemonId {
        let base = self.core.data.base_number(pokemon);
        let Some(alternate) = self
            .alt_forme_number(base, forme)
            .and_then(|number| self.existing(number))
        else {
            return pokemon;
        };
        let entry = self.core.data.pk(alternate);
        if entry.actually_cosmetic {
            entry.base_forme.unwrap_or(pokemon)
        } else {
            alternate
        }
    }

    pub(crate) fn cannot_dynamax(&self, species: i32, form: i32) -> bool {
        let personal = &self.files.data.personal;
        u16::try_from(species)
            .ok()
            .zip(u16::try_from(form).ok())
            .and_then(|(species, form)| personal.form_entry(species, form))
            .is_none_or(|entry| entry.cannot_dynamax())
    }

    pub(crate) fn dynamax_banned(&self) -> Vec<PokemonId> {
        self.all_pokemon()
            .into_iter()
            .filter(|&pokemon| {
                let (species, form) = self.species_and_form(pokemon, 0);
                self.cannot_dynamax(species, form)
            })
            .collect()
    }

    pub(crate) fn placement(
        &self,
        pokemon: PokemonId,
        forme: i32,
        original: (i32, i32),
    ) -> (i32, i32) {
        let placed = self.species_and_form(pokemon, forme);
        let unchanged = self
            .pokemon_with_forme(original.0, original.1)
            .is_some_and(|before| self.species_and_form(before, original.1) == placed);
        if unchanged { original } else { placed }
    }

    pub(crate) fn species_and_form(&self, pokemon: PokemonId, forme: i32) -> (i32, i32) {
        let data = &self.core.data;
        let species = data.base_number(pokemon);
        let entry = data.pk(pokemon);
        let form = if entry.base_forme.is_some() {
            entry.forme_number
        } else {
            forme.max(0)
        };
        let valid = form == 0 || self.alt_forme_number(species, form).is_some();
        (species, if valid { form } else { 0 })
    }

    pub(crate) fn title_ids(&self) -> Vec<String> {
        match self.game() {
            Some(game) => vec![game.title_id_text()],
            None => Game::ALL.iter().map(|game| game.title_id_text()).collect(),
        }
    }

    pub(crate) fn text_lines(&self, role: TextRole) -> Vec<String> {
        self.files
            .data
            .texts
            .get(&role)
            .map(|text| text.strings())
            .unwrap_or_default()
    }

    pub(crate) fn set_text_line(&mut self, role: TextRole, index: usize, text: &str) {
        let Some(file) = self.files.data.texts.get_mut(&role) else {
            return;
        };
        if file.line(index).as_deref() == Some(text) {
            return;
        }
        if let Err(error) = file.set_line(index, text) {
            self.core.deferred.record(swsh_error(error));
        }
    }

    pub(crate) fn save_files(&mut self) -> randomon_core::Result<Vec<OutputFile>> {
        self.save_pokemon();
        self.save_moves();
        let romfs_files = self
            .files
            .modified_files()
            .map_err(swsh_error)?
            .into_iter()
            .map(|file| (file.relative_path, file.data))
            .collect();
        let switch_mod = SwitchMod {
            title_ids: self.title_ids(),
            exefs_patch_folder: EXEFS_PATCH_FOLDER.to_owned(),
            romfs_files,
            exefs_patches: Vec::new(),
        };
        Ok(switch_mod.files(self.layout))
    }
}
