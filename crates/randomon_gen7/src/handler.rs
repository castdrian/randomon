use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

use randomon_core::{HandlerCore, RandomizationError, Result};
use randomon_ctr::rom::BoxedSource;
use randomon_ctr::strings::{read_strings, write_strings};
use randomon_ctr::{CtrIdentity, CtrRom, CtrRomEntry, TYPE_SM, TYPE_USUM};
use randomon_data::{GameData, ItemList, MegaEvolutionId, PokemonId};
use randomon_formats::ctr::{Decompression, GarcArchive};
use randomon_java::SharedRandomSource;

use crate::constants;
use crate::rom_entry::{entry_for, rom_type};

#[derive(Clone, Copy, Debug)]
pub struct FormeInfo {
    pub base_forme: i32,
    pub forme_number: i32,
}

#[derive(Clone, Debug, Default)]
pub struct AreaData {
    pub file_number: usize,
    pub has_tables: bool,
    pub encounter_tables: Vec<Vec<u8>>,
    pub name: String,
}

pub struct Gen7RomHandler {
    pub(crate) core: HandlerCore,
    pub(crate) rom: RefCell<CtrRom>,
    pub(crate) entry: &'static CtrRomEntry,
    pub(crate) rom_type: i32,
    pub(crate) code: Vec<u8>,
    pub(crate) poke_garc: GarcArchive,
    pub(crate) move_garc: GarcArchive,
    pub(crate) encounter_garc: GarcArchive,
    pub(crate) strings_garc: GarcArchive,
    pub(crate) story_text_garc: GarcArchive,
    pub(crate) area_data: Vec<AreaData>,
    pub(crate) pokes: Vec<Option<PokemonId>>,
    pub(crate) forme_mappings: BTreeMap<i32, FormeInfo>,
    pub(crate) absolute_poke_num_by_base_forme: HashMap<i32, HashMap<i32, i32>>,
    pub(crate) mega_evolutions: Vec<MegaEvolutionId>,
    pub(crate) ability_names: Vec<String>,
    pub(crate) item_names: Vec<String>,
    pub(crate) shop_names: Vec<String>,
    pub(crate) allowed_items: ItemList,
    pub(crate) non_bad_items: ItemList,
    pub(crate) actual_code_crc32: i64,
    pub(crate) actual_file_crc32s: HashMap<String, i64>,
    pub(crate) file_name: String,
}

pub(crate) fn io(error: impl std::fmt::Display) -> RandomizationError {
    RandomizationError::Io(error.to_string())
}

pub(crate) fn crc32(data: &[u8]) -> i64 {
    i64::from(randomon_formats::bytes::crc32(data))
}

impl Gen7RomHandler {
    pub fn load(
        source: BoxedSource,
        file_name: &str,
        random_source: &SharedRandomSource,
    ) -> Result<Self> {
        let rom = CtrRom::open(source, file_name)?;
        let identity = CtrIdentity {
            product_code: rom.product_code().map(str::to_owned),
            title_id: rom.title_id().map(str::to_owned),
        };
        let entry = entry_for(&identity)
            .ok_or_else(|| RandomizationError::UnsupportedRom(file_name.to_owned()))?;
        let rom_type = rom_type(entry);
        let mut handler = Self {
            core: HandlerCore::new(random_source),
            rom: RefCell::new(rom),
            entry,
            rom_type,
            code: Vec::new(),
            poke_garc: GarcArchive::default(),
            move_garc: GarcArchive::default(),
            encounter_garc: GarcArchive::default(),
            strings_garc: GarcArchive::default(),
            story_text_garc: GarcArchive::default(),
            area_data: Vec::new(),
            pokes: Vec::new(),
            forme_mappings: BTreeMap::new(),
            absolute_poke_num_by_base_forme: HashMap::new(),
            mega_evolutions: Vec::new(),
            ability_names: Vec::new(),
            item_names: Vec::new(),
            shop_names: Vec::new(),
            allowed_items: constants::allowed_items(rom_type),
            non_bad_items: constants::non_bad_items(),
            actual_code_crc32: 0,
            actual_file_crc32s: HashMap::new(),
            file_name: file_name.to_owned(),
        };
        handler.loaded_rom()?;
        Ok(handler)
    }

    pub(crate) fn loaded_rom(&mut self) -> Result<()> {
        self.core.data = GameData::new();
        self.code = self.rom.get_mut().read_code()?;
        self.strings_garc = self.read_garc(self.file("TextStrings"), Decompression::Skip)?;
        self.story_text_garc = self.read_garc(self.file("StoryText"), Decompression::Skip)?;
        self.area_data = self.read_area_data()?;
        self.load_pokemon_stats()?;
        self.load_moves()?;
        self.item_names = self.strings(false, self.entry.int("ItemNamesTextOffset"))?;
        self.ability_names = self.strings(false, self.entry.int("AbilityNamesTextOffset"))?;
        self.shop_names = constants::shop_names(self.rom_type);
        self.allowed_items = constants::allowed_items(self.rom_type);
        self.non_bad_items = constants::non_bad_items();
        if self.rom_type == TYPE_SM {
            self.core.engine.is_sm = true;
        }
        self.compute_crc32s()
    }

    pub(crate) fn is_sm(&self) -> bool {
        self.rom_type == TYPE_SM
    }

    pub(crate) fn is_usum(&self) -> bool {
        self.rom_type == TYPE_USUM
    }

    pub(crate) fn pokemon_count(&self) -> i32 {
        constants::pokemon_count(self.rom_type)
    }

    pub(crate) fn total_pokemon(&self) -> i32 {
        self.pokemon_count() + constants::forme_count(self.rom_type)
    }

    pub(crate) fn file(&self, key: &str) -> &'static str {
        self.entry.file(key).unwrap_or("")
    }

    pub(crate) fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        self.rom.borrow_mut().read_file(path)
    }

    pub(crate) fn write_file(&self, path: &str, data: &[u8]) -> Result<()> {
        self.rom.borrow_mut().write_file(path, data)
    }

    pub(crate) fn read_garc(
        &self,
        path: &str,
        decompression: Decompression,
    ) -> Result<GarcArchive> {
        self.rom.borrow_mut().read_garc(path, decompression)
    }

    pub(crate) fn write_garc(&self, path: &str, garc: &GarcArchive) -> Result<()> {
        self.rom.borrow_mut().write_garc(path, garc)
    }

    pub(crate) fn pk(&self, number: i32) -> PokemonId {
        PokemonId::from_index(number as usize)
    }

    pub(crate) fn absolute_poke_num(&self, species: i32, forme: i32) -> i32 {
        self.absolute_poke_num_by_base_forme
            .get(&species)
            .and_then(|formes| formes.get(&forme).copied())
            .unwrap_or(0)
    }

    pub(crate) fn pokemon_with_forme(&self, species: i32, forme: i32) -> PokemonId {
        let pokemon = self.pk(species);
        if !self.core.data.exists(pokemon) {
            return pokemon;
        }
        let cosmetic_forms = self.core.data.pk(pokemon).cosmetic_forms;
        if forme <= cosmetic_forms || forme == 30 || forme == 31 {
            return pokemon;
        }
        let alternate = self.pk(self.absolute_poke_num(species, forme));
        if self.core.data.exists(alternate) && alternate.index() != 0 {
            alternate
        } else {
            pokemon
        }
    }

    pub(crate) fn alt_forme_of(&self, pokemon: PokemonId, forme: i32) -> PokemonId {
        let number = self.absolute_poke_num(pokemon.number(), forme);
        if number == 0 {
            return pokemon;
        }
        let alternate = self.core.data.pk(self.pk(number));
        if alternate.actually_cosmetic {
            alternate.base_forme.unwrap_or(pokemon)
        } else {
            self.pk(number)
        }
    }

    pub(crate) fn strings(&self, story_text: bool, index: i32) -> Result<Vec<String>> {
        let garc = if story_text {
            &self.story_text_garc
        } else {
            &self.strings_garc
        };
        read_strings(garc, index as usize)
    }

    pub(crate) fn set_strings(
        &mut self,
        story_text: bool,
        index: i32,
        strings: &[String],
    ) -> Result<()> {
        let garc = if story_text {
            &mut self.story_text_garc
        } else {
            &mut self.strings_garc
        };
        write_strings(garc, index as usize, strings)
    }

    pub(crate) fn write_strings_for_all_languages(
        &self,
        strings: &[String],
        index: i32,
    ) -> Result<()> {
        for language in [
            "JaKana",
            "JaKanji",
            "Fr",
            "It",
            "De",
            "Es",
            "Ko",
            "ZhSimplified",
            "ZhTraditional",
        ] {
            let path = self.file(&format!("TextStrings{language}"));
            let mut garc = self.read_garc(path, Decompression::Skip)?;
            write_strings(&mut garc, index as usize, strings)?;
            self.write_garc(path, &garc)?;
        }
        Ok(())
    }

    fn compute_crc32s(&mut self) -> Result<()> {
        self.actual_code_crc32 = crc32(&self.code);
        let mut crcs = HashMap::new();
        for (key, file) in &self.entry.files {
            let data = self.read_file(&file.path)?;
            crcs.insert(key.clone(), crc32(&data));
        }
        self.actual_file_crc32s = crcs;
        Ok(())
    }

    pub(crate) fn rom_is_valid(&self) -> bool {
        let index = usize::from(self.rom.borrow().has_update());
        if self.entry.expected_code_crc32s[index] != self.actual_code_crc32 {
            return false;
        }
        self.entry.files.iter().all(|(key, file)| {
            self.actual_file_crc32s.get(key).copied() == Some(file.expected_crc32s[index])
        })
    }

    pub(crate) fn game_version(&self) -> String {
        let Ok(title_screen) = self.strings(false, self.entry.int("TitleScreenTextOffset")) else {
            return "Unpatched".to_owned();
        };
        let index = self.entry.int("UpdateStringOffset");
        if title_screen.len() as i32 > index {
            return title_screen[index as usize].clone();
        }
        "Unpatched".to_owned()
    }
}
