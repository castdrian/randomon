use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};

use randomon_core::{HandlerCore, RandomizationError, Result};
use randomon_data::constants::gen5;
use randomon_data::{ItemList, PokemonId};
use randomon_formats::bytes::crc32;
use randomon_formats::nds::NarcArchive;
use randomon_java::SharedRandomSource;
use randomon_nds::{Arm9Extension, DsRom, rom_code_from_bytes, version_from_bytes};

use crate::rom_entry::{RomEntry, entry_for};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FormeInfo {
    pub base_forme: i32,
    pub forme_number: i32,
    pub forme_sprite_offset: i32,
}

pub struct Gen5RomHandler {
    pub(crate) core: HandlerCore,
    pub(crate) rom: RefCell<DsRom>,
    pub(crate) file_name: String,
    pub(crate) entry: &'static RomEntry,
    pub(crate) arm9: Vec<u8>,
    pub(crate) arm9_extension: Arm9Extension,
    pub(crate) forme_mappings: BTreeMap<i32, FormeInfo>,
    pub(crate) pokemon_list: Vec<Option<PokemonId>>,
    pub(crate) pokemon_list_incl_formes: Vec<Option<PokemonId>>,
    pub(crate) ability_names: Vec<String>,
    pub(crate) item_names: Vec<String>,
    pub(crate) shop_names: &'static [&'static str],
    pub(crate) wild_map_names: RefCell<Option<HashMap<i32, String>>>,
    pub(crate) allowed_items: ItemList,
    pub(crate) non_bad_items: ItemList,
    pub(crate) regular_shop_items: Vec<i32>,
    pub(crate) op_shop_items: Vec<i32>,
    pub(crate) hidden_hollow_count: Cell<i32>,
    pub(crate) hidden_hollow_counted: Cell<bool>,
    pub(crate) original_double_trainers: RefCell<Vec<i32>>,
    pub(crate) effectiveness_updated: bool,
    pub(crate) pickup_items_table_offset: Cell<i32>,
    pub(crate) actual_arm9_crc32: i64,
    pub(crate) actual_overlay_crc32s: HashMap<i32, i64>,
    pub(crate) actual_file_crc32s: HashMap<String, i64>,
    pub(crate) poke_narc: NarcArchive,
    pub(crate) move_narc: NarcArchive,
    pub(crate) strings_narc: NarcArchive,
    pub(crate) story_text_narc: NarcArchive,
    pub(crate) script_narc: NarcArchive,
    pub(crate) shop_narc: Option<NarcArchive>,
}

pub fn detect_rom(rom: &[u8]) -> bool {
    match (rom_code_from_bytes(rom), version_from_bytes(rom)) {
        (Some(code), Some(version)) => entry_for(&code, version).is_some(),
        _ => false,
    }
}

pub(crate) fn file_path(entry: &'static RomEntry, key: &str) -> Result<&'static str> {
    entry
        .file(key)
        .ok_or(RandomizationError::NullPointer("romEntry.getFile"))
}

fn checksum(data: &[u8]) -> i64 {
    i64::from(crc32(data))
}

impl Gen5RomHandler {
    pub fn load(rom: Vec<u8>, file_name: &str, random_source: &SharedRandomSource) -> Result<Self> {
        let unsupported = || RandomizationError::UnsupportedRom(file_name.to_owned());
        let code = rom_code_from_bytes(&rom).ok_or_else(unsupported)?;
        let version = version_from_bytes(&rom).ok_or_else(unsupported)?;
        let entry = entry_for(&code, version).ok_or_else(unsupported)?;
        let mut ds_rom = DsRom::open(rom, file_name)?;
        let arm9 = ds_rom.read_arm9()?;
        let strings_narc = ds_rom.read_narc(file_path(entry, "TextStrings")?)?;
        let story_text_narc = ds_rom.read_narc(file_path(entry, "TextStory")?)?;
        let script_narc = ds_rom.read_narc(file_path(entry, "Scripts")?)?;
        let shop_narc = if entry.is_bw2() {
            Some(ds_rom.read_narc(file_path(entry, "ShopItems")?)?)
        } else {
            None
        };
        let poke_narc = ds_rom.read_narc(file_path(entry, "PokemonStats")?)?;
        let mut handler = Self {
            core: HandlerCore::new(random_source),
            rom: RefCell::new(ds_rom),
            file_name: file_name.to_owned(),
            entry,
            arm9,
            arm9_extension: Arm9Extension::default(),
            forme_mappings: BTreeMap::new(),
            pokemon_list: Vec::new(),
            pokemon_list_incl_formes: Vec::new(),
            ability_names: Vec::new(),
            item_names: Vec::new(),
            shop_names: if entry.is_bw2() {
                gen5::BW2_SHOP_NAMES
            } else {
                gen5::BW1_SHOP_NAMES
            },
            wild_map_names: RefCell::new(None),
            allowed_items: gen5::allowed_items(),
            non_bad_items: gen5::non_bad_items(entry.rom_type),
            regular_shop_items: gen5::regular_shop_items(),
            op_shop_items: gen5::op_shop_items(),
            hidden_hollow_count: Cell::new(0),
            hidden_hollow_counted: Cell::new(false),
            original_double_trainers: RefCell::new(Vec::new()),
            effectiveness_updated: false,
            pickup_items_table_offset: Cell::new(0),
            actual_arm9_crc32: 0,
            actual_overlay_crc32s: HashMap::new(),
            actual_file_crc32s: HashMap::new(),
            poke_narc,
            move_narc: NarcArchive::new(),
            strings_narc,
            story_text_narc,
            script_narc,
            shop_narc,
        };
        handler.load_pokemon_stats()?;
        handler.load_moves()?;
        handler.ability_names = handler.get_strings(false, entry.int("AbilityNamesTextOffset"))?;
        handler.item_names = handler.get_strings(false, entry.int("ItemNamesTextOffset"))?;
        handler.compute_crc32s_for_rom()?;
        let should_extend_arm9 = entry.tweak("ShedinjaEvolutionTweak").is_some()
            || entry.tweak("NewIndexToMusicTweak").is_some();
        if should_extend_arm9 {
            let arm9 = std::mem::take(&mut handler.arm9);
            handler.arm9 = handler.arm9_extension.extend(
                arm9,
                entry.int("Arm9ExtensionSize"),
                entry.string("TCMCopyingPrefix"),
                gen5::ARM9_OFFSET,
            );
        }
        Ok(handler)
    }

    fn compute_crc32s_for_rom(&mut self) -> Result<()> {
        self.actual_arm9_crc32 = checksum(&self.arm9);
        let overlays: Vec<i32> = self.entry.overlay_expected_crc32s.keys().copied().collect();
        for number in overlays {
            let overlay = self.read_overlay(number)?;
            self.actual_overlay_crc32s
                .insert(number, checksum(&overlay));
        }
        let files: Vec<(&String, &str)> = self
            .entry
            .files
            .iter()
            .map(|(key, file)| (key, file.path.as_str()))
            .collect();
        for (key, path) in files {
            let data = self.rom.borrow_mut().read_file(path)?;
            self.actual_file_crc32s.insert(key.clone(), checksum(&data));
        }
        Ok(())
    }

    pub(crate) fn rom_is_valid(&self) -> bool {
        if self.entry.arm9_expected_crc32 != self.actual_arm9_crc32 {
            return false;
        }
        let overlays_match = self
            .entry
            .overlay_expected_crc32s
            .iter()
            .all(|(number, expected)| self.actual_overlay_crc32s.get(number) == Some(expected));
        let files_match = self
            .entry
            .files
            .iter()
            .all(|(key, file)| self.actual_file_crc32s.get(key) == Some(&file.expected_crc32));
        overlays_match && files_match
    }

    pub(crate) fn pk(&self, number: i32) -> PokemonId {
        PokemonId::from_index(number as usize)
    }

    pub(crate) fn rom_type(&self) -> i32 {
        self.entry.rom_type
    }

    pub(crate) fn is_bw2(&self) -> bool {
        self.entry.is_bw2()
    }

    pub(crate) fn read_narc(&self, key: &str) -> Result<NarcArchive> {
        let path = file_path(self.entry, key)?;
        self.rom.borrow_mut().read_narc(path)
    }

    pub(crate) fn write_narc(&self, key: &str, narc: &NarcArchive) -> Result<()> {
        let path = file_path(self.entry, key)?;
        self.rom.borrow_mut().write_narc(path, narc)
    }

    pub(crate) fn read_overlay(&self, number: i32) -> Result<Vec<u8>> {
        self.rom.borrow_mut().read_overlay(number)
    }

    pub(crate) fn write_overlay(&self, number: i32, data: &[u8]) -> Result<()> {
        self.rom.borrow_mut().write_overlay(number, data)
    }

    pub(crate) fn generic_ips_patch(&self, data: &mut [u8], tweak: &str) -> Result<bool> {
        let Some(name) = self.entry.tweak(tweak) else {
            return Ok(false);
        };
        let patch = crate::patches::patch_bytes(name)
            .ok_or_else(|| RandomizationError::Io(format!("missing patch {name}")))?;
        randomon_formats::ips::apply_patch(data, patch).map_err(randomon_nds::format_error)?;
        Ok(true)
    }

    pub(crate) fn save_rom_bytes(&mut self) -> Result<Vec<u8>> {
        self.save_pokemon_stats()?;
        self.save_moves()?;
        self.rom.borrow_mut().write_arm9(&self.arm9)?;
        self.write_narc("TextStrings", &self.strings_narc)?;
        self.write_narc("TextStory", &self.story_text_narc)?;
        self.write_narc("Scripts", &self.script_narc)?;
        self.rom.borrow_mut().save()
    }
}
