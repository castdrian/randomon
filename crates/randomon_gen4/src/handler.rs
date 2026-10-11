use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use randomon_core::{HandlerCore, RandomizationError, Result};
use randomon_data::constants::gen4;
use randomon_data::{ItemList, PokemonId};
use randomon_formats::bytes::crc32;
use randomon_formats::nds::NarcArchive;
use randomon_java::SharedRandomSource;
use randomon_nds::{Arm9Extension, DsRom, rom_code_from_bytes, version_from_bytes};

use crate::bytes::{after_prefix, i32_at};
use crate::rom_entry::{RomEntry, entry_for};

#[derive(Clone, Debug, Default)]
pub(crate) struct MapNames {
    pub wild: HashMap<i32, String>,
    pub headbutt: HashMap<i32, String>,
}

pub struct Gen4RomHandler {
    pub(crate) core: HandlerCore,
    pub(crate) rom: RefCell<DsRom>,
    pub(crate) file_name: String,
    pub(crate) entry: &'static RomEntry,
    pub(crate) arm9: Vec<u8>,
    pub(crate) arm9_extension: Arm9Extension,
    pub(crate) pokemon_list: Vec<Option<PokemonId>>,
    pub(crate) pokemon_list_incl_formes: Vec<Option<PokemonId>>,
    pub(crate) ability_names: Vec<String>,
    pub(crate) item_names: Vec<String>,
    pub(crate) map_names: RefCell<Option<MapNames>>,
    pub(crate) allowed_items: ItemList,
    pub(crate) non_bad_items: ItemList,
    pub(crate) regular_shop_items: Vec<i32>,
    pub(crate) op_shop_items: Vec<i32>,
    pub(crate) roamer_randomization_enabled: bool,
    pub(crate) effectiveness_updated: bool,
    pub(crate) pickup_items_table_offset: Cell<i32>,
    pub(crate) rare_pickup_items_table_offset: Cell<i32>,
    pub(crate) actual_arm9_crc32: i64,
    pub(crate) actual_overlay_crc32s: HashMap<i32, i64>,
    pub(crate) actual_file_crc32s: HashMap<String, i64>,
    pub(crate) poke_narc: NarcArchive,
    pub(crate) move_narc: NarcArchive,
    pub(crate) msg_narc: NarcArchive,
    pub(crate) script_narc: NarcArchive,
    pub(crate) event_narc: NarcArchive,
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
        .filter(|path| !path.is_empty())
        .ok_or(RandomizationError::NullPointer("romEntry.getFile"))
}

fn checksum(data: &[u8]) -> i64 {
    i64::from(crc32(data))
}

fn roamer_randomization_enabled(entry: &RomEntry) -> bool {
    match entry.rom_type {
        gen4::TYPE_DP => !entry.roaming_pokemon.is_empty(),
        gen4::TYPE_PLAT | gen4::TYPE_HGSS => entry.tweak("NewRoamerSubroutineTweak").is_some(),
        _ => false,
    }
}

impl Gen4RomHandler {
    pub fn load(rom: Vec<u8>, file_name: &str, random_source: &SharedRandomSource) -> Result<Self> {
        let unsupported = || RandomizationError::UnsupportedRom(file_name.to_owned());
        let code = rom_code_from_bytes(&rom).ok_or_else(unsupported)?;
        let version = version_from_bytes(&rom).ok_or_else(unsupported)?;
        let entry = entry_for(&code, version).ok_or_else(unsupported)?;
        let mut ds_rom = DsRom::open(rom, file_name)?;
        let arm9 = ds_rom.read_arm9()?;
        let msg_narc = ds_rom.read_narc(file_path(entry, "Text")?)?;
        let script_narc = ds_rom.read_narc(file_path(entry, "Scripts")?)?;
        let event_narc = ds_rom.read_narc(file_path(entry, "Events")?)?;
        let poke_narc = ds_rom.read_narc(file_path(entry, "PokemonStats")?)?;
        let mut handler = Self {
            core: HandlerCore::new(random_source),
            rom: RefCell::new(ds_rom),
            file_name: file_name.to_owned(),
            entry,
            arm9,
            arm9_extension: Arm9Extension::default(),
            pokemon_list: Vec::new(),
            pokemon_list_incl_formes: Vec::new(),
            ability_names: Vec::new(),
            item_names: Vec::new(),
            map_names: RefCell::new(None),
            allowed_items: gen4::allowed_items(),
            non_bad_items: gen4::non_bad_items(),
            regular_shop_items: gen4::regular_shop_items(),
            op_shop_items: gen4::op_shop_items(),
            roamer_randomization_enabled: roamer_randomization_enabled(entry),
            effectiveness_updated: false,
            pickup_items_table_offset: Cell::new(0),
            rare_pickup_items_table_offset: Cell::new(0),
            actual_arm9_crc32: 0,
            actual_overlay_crc32s: HashMap::new(),
            actual_file_crc32s: HashMap::new(),
            poke_narc,
            move_narc: NarcArchive::new(),
            msg_narc,
            script_narc,
            event_narc,
        };
        handler.load_pokemon_stats()?;
        handler.load_moves()?;
        handler.ability_names = handler.get_strings(entry.int("AbilityNamesTextOffset"))?;
        handler.item_names = handler.get_strings(entry.int("ItemNamesTextOffset"))?;
        handler.compute_crc32s_for_rom();
        handler.apply_catching_tutorial_fix();
        if entry.static_pokemon_support && handler.roamer_randomization_enabled {
            handler.prepare_roamer_code();
        }
        Ok(handler)
    }

    pub(crate) fn has_catching_tutorial_fix(&self) -> bool {
        self.entry.is_hgss()
            && self
                .entry
                .tweak("NewCatchingTutorialSubroutineTweak")
                .is_some()
    }

    fn apply_catching_tutorial_fix(&mut self) {
        if !self.has_catching_tutorial_fix() {
            return;
        }
        let applied = self
            .extend_arm9()
            .and_then(|()| self.patch_arm9("NewCatchingTutorialSubroutineTweak"));
        if let Err(error) = applied {
            log::warn!("catching tutorial fix not applied: {error}");
        }
    }

    fn compute_crc32s_for_rom(&mut self) {
        self.actual_arm9_crc32 = checksum(&self.arm9);
        let overlays: Vec<i32> = self.entry.overlay_expected_crc32s.keys().copied().collect();
        for number in overlays {
            if let Ok(overlay) = self.read_overlay(number) {
                self.actual_overlay_crc32s
                    .insert(number, checksum(&overlay));
            }
        }
        for (key, file) in &self.entry.files {
            if let Ok(data) = self.rom.borrow_mut().read_file(&file.path) {
                self.actual_file_crc32s.insert(key.clone(), checksum(&data));
            }
        }
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

    pub(crate) fn pokemon_by_number(&self, number: i32) -> Result<PokemonId> {
        usize::try_from(number)
            .ok()
            .and_then(|index| self.pokemon_list_incl_formes.get(index).copied().flatten())
            .ok_or(RandomizationError::NullPointer("pokes[species]"))
    }

    pub(crate) fn pk(&self, number: i32) -> PokemonId {
        PokemonId::from_index(number as usize)
    }

    pub(crate) fn number_of(&self, pokemon: PokemonId) -> i32 {
        self.core.data.pk(pokemon).number
    }

    pub(crate) fn rom_type(&self) -> i32 {
        self.entry.rom_type
    }

    pub(crate) fn read_narc(&self, key: &str) -> Result<NarcArchive> {
        let path = file_path(self.entry, key)?;
        self.rom.borrow_mut().read_narc(path)
    }

    pub(crate) fn write_narc(&self, key: &str, narc: &NarcArchive) -> Result<()> {
        let path = file_path(self.entry, key)?;
        self.rom.borrow_mut().write_narc(path, narc)
    }

    pub(crate) fn read_file(&self, key: &str) -> Result<Vec<u8>> {
        let path = file_path(self.entry, key)?;
        self.rom.borrow_mut().read_file(path)
    }

    pub(crate) fn write_file(&self, key: &str, data: &[u8]) -> Result<()> {
        let path = file_path(self.entry, key)?;
        self.rom.borrow_mut().write_file(path, data)
    }

    pub(crate) fn read_overlay(&self, number: i32) -> Result<Vec<u8>> {
        self.rom.borrow_mut().read_overlay(number)
    }

    pub(crate) fn write_overlay(&self, number: i32, data: &[u8]) -> Result<()> {
        self.rom.borrow_mut().write_overlay(number, data)
    }

    pub(crate) fn ips_patch(&self, data: &mut [u8], tweak: &str) -> Result<bool> {
        let Some(name) = self.entry.tweak(tweak) else {
            return Ok(false);
        };
        let patch = crate::patches::patch_bytes(name)
            .ok_or_else(|| RandomizationError::Io(format!("missing patch {name}")))?;
        let mut patched = data.to_vec();
        randomon_formats::ips::apply_patch(&mut patched, patch)
            .map_err(randomon_nds::format_error)?;
        data.copy_from_slice(&patched);
        Ok(true)
    }

    pub(crate) fn patch_arm9(&mut self, tweak: &str) -> Result<bool> {
        let mut arm9 = std::mem::take(&mut self.arm9);
        let patched = self.ips_patch(&mut arm9, tweak);
        self.arm9 = arm9;
        patched
    }

    fn arm9_can_be_extended(&self, prefix: &str) -> bool {
        let Some(pointers) = after_prefix(&self.arm9, prefix) else {
            return false;
        };
        let pointer = |delta: i32| {
            i32_at(&self.arm9, pointers + delta)
                .ok()
                .map(|value| value.wrapping_sub(gen4::ARM9_OFFSET))
        };
        let (Some(destinations), Some(itcm_source)) = (pointer(0), pointer(8)) else {
            return false;
        };
        let Ok(itcm_size) = i32_at(&self.arm9, destinations.wrapping_add(4)) else {
            return false;
        };
        let dtcm_start = itcm_source.wrapping_add(itcm_size);
        destinations >= 0
            && itcm_source >= 0
            && itcm_size >= 0
            && dtcm_start >= 0
            && (dtcm_start as usize) <= self.arm9.len()
            && i32_at(&self.arm9, pointers + 4).is_ok()
    }

    pub(crate) fn extend_arm9(&mut self) -> Result<()> {
        if self.arm9_extension.is_extended() {
            return Ok(());
        }
        let prefix = self.entry.string("TCMCopyingPrefix");
        let extend_by = self.entry.int("Arm9ExtensionSize");
        if extend_by <= 0 || !self.arm9_can_be_extended(prefix) {
            return Err(RandomizationError::randomization(
                "The ARM9 of this ROM cannot be extended",
            ));
        }
        let arm9 = std::mem::take(&mut self.arm9);
        self.arm9 = self
            .arm9_extension
            .extend(arm9, extend_by, prefix, gen4::ARM9_OFFSET);
        Ok(())
    }

    pub(crate) fn save_rom_bytes(&mut self) -> Result<Vec<u8>> {
        self.save_pokemon_stats()?;
        self.save_moves()?;
        self.rom.borrow_mut().write_arm9(&self.arm9)?;
        self.write_narc("Text", &self.msg_narc)?;
        self.write_narc("Scripts", &self.script_narc)?;
        self.write_narc("Events", &self.event_narc)?;
        self.rom.borrow_mut().save()
    }
}
