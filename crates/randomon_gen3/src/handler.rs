use randomon_core::{HandlerCore, RandomizationError, Result};
use randomon_data::constants::species;
use randomon_data::text::java_lower_case;
use randomon_data::{ExpCurve, GameData, ItemList, Pokemon, PokemonId, StatLayout};
use randomon_java::SharedRandomSource;

use crate::assets::table_text;
use crate::constants as c;
use crate::detect::{detect_rom, matching_entry, normalize_unofficial_emerald};
use crate::rom::GbaRom;
use crate::rom_entry::{RomEntry, RomType};
use crate::text::{TEXT_TERMINATOR, TextTable, length_of_string_at};

const LARGEST_SANE_POKEMON_COUNT: i32 = 2048;
const LARGEST_SANE_MOVE_COUNT: i32 = 2048;
const LARGEST_SANE_ITEM_COUNT: i32 = 4096;
const LARGEST_SANE_NAME_LENGTH: i32 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InternalSlot {
    Missing,
    Main(PokemonId),
    Shadow(PokemonId),
}

pub struct Gen3RomHandler {
    pub(crate) core: HandlerCore,
    pub(crate) rom: GbaRom,
    pub(crate) original_crc32: i64,
    pub(crate) actual_crc32: i64,
    pub(crate) file_name: String,
    pub(crate) entry: RomEntry,
    pub(crate) table: TextTable,
    pub(crate) pokemon_list: Vec<Option<PokemonId>>,
    pub(crate) internal_to_dex: Vec<i32>,
    pub(crate) dex_to_internal: Vec<i32>,
    pub(crate) internal_slots: Vec<InternalSlot>,
    pub(crate) shadow: GameData,
    pub(crate) pokedex_count: i32,
    pub(crate) is_rom_hack: bool,
    pub(crate) jambo_moveset_hack: bool,
    pub(crate) have_patched_obedience: bool,
    pub(crate) item_offsets: Vec<i32>,
    pub(crate) map_names: Vec<Vec<String>>,
    pub(crate) ability_names: Vec<String>,
    pub(crate) item_names: Vec<String>,
    pub(crate) allowed_items: ItemList,
    pub(crate) non_bad_items: ItemList,
    pub(crate) pickup_items_table_offset: i32,
    pub(crate) effectiveness_updated: bool,
}

fn unsupported(file_name: &str, reason: &str) -> RandomizationError {
    RandomizationError::UnsupportedRom(format!("{file_name}: {reason}"))
}

impl Gen3RomHandler {
    pub fn load(rom: Vec<u8>, file_name: &str, random_source: &SharedRandomSource) -> Result<Self> {
        let mut bytes = rom;
        normalize_unofficial_emerald(&mut bytes);
        if !detect_rom(&bytes) {
            return Err(RandomizationError::UnsupportedRom(file_name.to_owned()));
        }
        let entry = matching_entry(&bytes)
            .ok_or_else(|| RandomizationError::UnsupportedRom(file_name.to_owned()))?
            .clone();
        let table_name = entry.table_file.clone().unwrap_or_default();
        let table = TextTable::from_text(
            table_text(&table_name).ok_or_else(|| unsupported(file_name, "missing text table"))?,
        );
        let crc = randomon_gb::crc32(&bytes);
        let rom_type = entry.rom_type;
        let mut handler = Self {
            core: HandlerCore::new(random_source),
            rom: GbaRom::new(bytes),
            original_crc32: crc,
            actual_crc32: crc,
            file_name: file_name.to_owned(),
            entry,
            table,
            pokemon_list: Vec::new(),
            internal_to_dex: Vec::new(),
            dex_to_internal: Vec::new(),
            internal_slots: Vec::new(),
            shadow: GameData::new(),
            pokedex_count: 0,
            is_rom_hack: false,
            jambo_moveset_hack: false,
            have_patched_obedience: false,
            item_offsets: Vec::new(),
            map_names: Vec::new(),
            ability_names: Vec::new(),
            item_names: Vec::new(),
            allowed_items: c::allowed_items(),
            non_bad_items: c::non_bad_items(rom_type),
            pickup_items_table_offset: 0,
            effectiveness_updated: false,
        };
        handler.loaded_rom()?;
        Ok(handler)
    }

    fn loaded_rom(&mut self) -> Result<()> {
        self.locate_tables()?;
        if self.entry.rom_code == "BPRE" && self.entry.version == 0 {
            self.basic_bpre10_hack_support();
        }
        self.validate_core_tables()?;
        let names = self.read_pokemon_names();
        self.load_pokedex(&names);
        self.load_pokemon_stats(&names);
        self.construct_pokemon_list();
        self.populate_evolutions();
        self.load_moves();
        self.locate_world_tables()?;
        self.determine_map_bank_sizes();
        self.locate_map_labels();
        self.load_ability_names();
        self.load_item_names();
        self.preprocess_maps();
        self.locate_pickup_table();
        if self.value("StaticPokemonSupport") > 0 {
            self.prepare_hardcoded_statics()?;
        }
        Ok(())
    }

    fn locate_tables(&mut self) -> Result<()> {
        let prefixes = self.rom.find_multiple(c::POKEDEX_ORDER_POINTER_PREFIX);
        let order_prefix = *prefixes
            .get(1)
            .ok_or_else(|| unsupported(&self.file_name, "no Pokédex order table"))?;
        let order = self.rom.pointer(order_prefix + 16);
        self.entry.set_value("PokedexOrder", order);
        if self.rom_type().is_ruby_or_sapphire() {
            self.locate_rs_tables()
        } else {
            self.locate_efrlg_tables();
            Ok(())
        }
    }

    fn locate_rs_tables(&mut self) -> Result<()> {
        let names_suffix = self.rom.find(c::RS_POKEMON_NAMES_POINTER_SUFFIX);
        if names_suffix < 4 {
            return Err(unsupported(&self.file_name, "no Pokémon names table"));
        }
        let names = self.rom.pointer(names_suffix - 4);
        self.entry.set_value("PokemonNames", names);
        let sprites = self.find_pointer_prefix_and_suffix(
            c::RS_FRONT_SPRITES_POINTER_PREFIX,
            c::RS_FRONT_SPRITES_POINTER_SUFFIX,
        );
        let palettes = self.find_pointer_prefix_and_suffix(
            c::RS_POKEMON_PALETTES_POINTER_PREFIX,
            c::RS_POKEMON_PALETTES_POINTER_SUFFIX,
        );
        self.entry
            .set_value("FrontSprites", self.pointer_or_invalid(sprites));
        self.entry
            .set_value("PokemonPalettes", self.pointer_or_invalid(palettes));
        Ok(())
    }

    fn pointer_or_invalid(&self, location: i32) -> i32 {
        if location < 0 {
            -1
        } else {
            self.rom.pointer(location)
        }
    }

    fn locate_efrlg_tables(&mut self) {
        let pointers = [
            ("PokemonNames", c::EFRLG_POKEMON_NAMES_POINTER),
            ("MoveNames", c::EFRLG_MOVE_NAMES_POINTER),
            ("AbilityNames", c::EFRLG_ABILITY_NAMES_POINTER),
            ("ItemData", c::EFRLG_ITEM_DATA_POINTER),
            ("MoveData", c::EFRLG_MOVE_DATA_POINTER),
            ("PokemonStats", c::EFRLG_POKEMON_STATS_POINTER),
            ("FrontSprites", c::EFRLG_FRONT_SPRITES_POINTER),
            ("PokemonPalettes", c::EFRLG_POKEMON_PALETTES_POINTER),
        ];
        for (key, location) in pointers {
            let pointer = self.rom.pointer(location);
            self.entry.set_value(key, pointer);
        }
        let tutor_compatibility = self.value("MoveTutorData") + self.value("MoveTutorMoves") * 2;
        self.entry
            .set_value("MoveTutorCompatibility", tutor_compatibility);
    }

    fn locate_world_tables(&mut self) -> Result<()> {
        let wild = self.rom.find_multiple(c::WILD_POKEMON_POINTER_PREFIX);
        let maps = self.rom.find_multiple(c::MAP_BANKS_POINTER_PREFIX);
        let (Some(&wild), Some(&maps)) = (wild.first(), maps.first()) else {
            return Err(unsupported(&self.file_name, "no wild or map tables"));
        };
        let wild_pokemon = self.rom.pointer(wild + 12);
        let map_headers = self.rom.pointer(maps + 12);
        self.entry.set_value("WildPokemon", wild_pokemon);
        self.entry.set_value("MapHeaders", map_headers);
        Ok(())
    }

    fn locate_map_labels(&mut self) {
        let prefix = if self.rom_type().is_frlg() {
            c::FRLG_MAP_LABELS_POINTER_PREFIX
        } else {
            c::RSE_MAP_LABELS_POINTER_PREFIX
        };
        let base = self.rom.find(prefix);
        let labels = self.rom.pointer(base + 12);
        self.entry.set_value("MapLabels", labels);
    }

    fn require_table(&self, key: &str, entries: i32, entry_size: i32) -> Result<()> {
        let length = i64::from(entries) * i64::from(entry_size);
        let start = self.value(key);
        let fits = start > 0 && i64::from(start) + length <= i64::from(self.rom.len());
        if fits {
            Ok(())
        } else {
            Err(unsupported(
                &self.file_name,
                &format!("{key} is out of range"),
            ))
        }
    }

    fn require_count(&self, key: &str, largest: i32) -> Result<i32> {
        let count = self.value(key);
        if (1..=largest).contains(&count) {
            Ok(count)
        } else {
            Err(unsupported(&self.file_name, &format!("{key} is not sane")))
        }
    }

    fn validate_core_tables(&self) -> Result<()> {
        let pokemon = self.require_count("PokemonCount", LARGEST_SANE_POKEMON_COUNT)?;
        let moves = self.require_count("MoveCount", LARGEST_SANE_MOVE_COUNT)?;
        let items = self.require_count("ItemCount", LARGEST_SANE_ITEM_COUNT)?;
        let name_length = self.require_count("PokemonNameLength", LARGEST_SANE_NAME_LENGTH)?;
        let move_name_length = self.require_count("MoveNameLength", LARGEST_SANE_NAME_LENGTH)?;
        self.require_count("AbilityNameLength", LARGEST_SANE_NAME_LENGTH)?;
        let item_size = self.require_count("ItemEntrySize", LARGEST_SANE_NAME_LENGTH)?;
        self.require_table("PokedexOrder", pokemon, 2)?;
        self.require_table("PokemonNames", pokemon + 1, name_length)?;
        self.require_table("PokemonStats", pokemon + 1, c::BASE_STATS_ENTRY_SIZE)?;
        self.require_table("PokemonEvolutions", pokemon + 1, c::EVOLUTION_ENTRY_SIZE)?;
        self.require_table("PokemonMovesets", pokemon + 1, 4)?;
        self.require_table("PokemonTMHMCompat", pokemon + 1, c::TMHM_COMPAT_ENTRY_SIZE)?;
        self.require_table("MoveData", moves + 1, c::MOVE_DATA_ENTRY_SIZE)?;
        self.require_table("MoveNames", moves + 1, move_name_length)?;
        self.require_table("AbilityNames", c::HIGHEST_ABILITY_INDEX + 1, 1)?;
        self.require_table("ItemData", items + 1, item_size)?;
        self.require_table("TmMoves", c::TM_COUNT, 2)?;
        Ok(())
    }

    pub(crate) fn rom_type(&self) -> RomType {
        self.entry.rom_type
    }

    pub(crate) fn value(&self, key: &str) -> i32 {
        self.entry.value(key)
    }

    pub(crate) fn array(&self, key: &str) -> Vec<i32> {
        self.entry.array(key).to_vec()
    }

    pub(crate) fn read_string(&self, offset: i32, max_length: i32) -> String {
        self.table
            .read_string(&self.rom, offset, max_length.max(0) as usize)
    }

    pub(crate) fn read_variable_string(&self, offset: i32) -> String {
        self.table.read_string(&self.rom, offset, usize::MAX)
    }

    pub(crate) fn translate(&self, text: &str) -> Vec<u8> {
        self.table.translate(text)
    }

    pub(crate) fn string_length_at(&self, offset: i32) -> i32 {
        length_of_string_at(&self.rom, offset)
    }

    pub(crate) fn write_fixed_string(&mut self, text: &str, offset: i32, length: i32) {
        let translated = self.translate(text);
        let length = length.max(0);
        let copied = (translated.len() as i32).min(length);
        self.rom.write_bytes(offset, &translated[..copied as usize]);
        let mut written = copied;
        if written < length {
            self.rom.set_byte(offset + written, TEXT_TERMINATOR);
            written += 1;
        }
        while written < length {
            self.rom.set_byte(offset + written, 0);
            written += 1;
        }
    }

    pub(crate) fn write_variable_string(&mut self, text: &str, offset: i32) {
        let translated = self.translate(text);
        self.rom.write_bytes(offset, &translated);
        self.rom
            .set_byte(offset + translated.len() as i32, TEXT_TERMINATOR);
    }

    pub(crate) fn find_pointer_prefix_and_suffix(&self, prefix: &str, suffix: &str) -> i32 {
        if !prefix.len().is_multiple_of(2) || !suffix.len().is_multiple_of(2) {
            return -1;
        }
        let (Ok(prefix), Ok(suffix)) = (
            randomon_formats::search::parse_hex_bytes(prefix),
            randomon_formats::search::parse_hex_bytes(suffix),
        ) else {
            return -1;
        };
        let length = self.rom.len();
        if prefix.len() >= suffix.len() {
            for start in self.rom.search(&prefix) {
                if start + 4 + suffix.len() as i32 > length {
                    continue;
                }
                let location = start + prefix.len() as i32;
                if self.rom.valid_pointer(location).is_some()
                    && self.rom.matches(location + 4, &suffix)
                {
                    return location;
                }
            }
        } else {
            for start in self.rom.search(&suffix) {
                if start - 4 - (prefix.len() as i32) < 0 {
                    continue;
                }
                let location = start - 4;
                if self.rom.valid_pointer(location).is_some()
                    && self.rom.matches(location - prefix.len() as i32, &prefix)
                {
                    return location;
                }
            }
        }
        -1
    }

    pub(crate) fn pokemon_count(&self) -> i32 {
        self.value("PokemonCount")
    }

    pub(crate) fn pokemon_from_internal(&self, internal: i32) -> Option<PokemonId> {
        let dex = *self.internal_to_dex.get(usize::try_from(internal).ok()?)?;
        let id = PokemonId::from_index(usize::try_from(dex).ok()?);
        (dex > 0 && self.core.data.exists(id)).then_some(id)
    }

    pub(crate) fn fallback_pokemon(&self) -> PokemonId {
        self.pokemon_list
            .iter()
            .flatten()
            .next()
            .copied()
            .unwrap_or_else(|| PokemonId::from_index(1))
    }

    pub(crate) fn pokemon_or_fallback(&self, internal: i32) -> PokemonId {
        self.pokemon_from_internal(internal)
            .unwrap_or_else(|| self.fallback_pokemon())
    }

    pub(crate) fn internal_of_number(&self, number: i32) -> i32 {
        usize::try_from(number)
            .ok()
            .and_then(|index| self.dex_to_internal.get(index))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn internal_of(&self, id: PokemonId) -> i32 {
        self.internal_of_number(self.core.data.pk(id).number)
    }

    pub(crate) fn dex_of_internal(&self, internal: i32) -> i32 {
        usize::try_from(internal)
            .ok()
            .and_then(|index| self.internal_to_dex.get(index))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn real_pokemon(&self) -> Vec<PokemonId> {
        self.pokemon_list
            .iter()
            .skip(1)
            .flatten()
            .copied()
            .collect()
    }

    pub(crate) fn loaded_pokemon(&self) -> Vec<PokemonId> {
        (1..self.core.data.pokemon_slots())
            .map(PokemonId::from_index)
            .filter(|&id| self.core.data.exists(id))
            .collect()
    }

    fn read_pokemon_names(&self) -> Vec<String> {
        let offset = self.value("PokemonNames");
        let length = self.value("PokemonNameLength");
        (0..=self.pokemon_count())
            .map(|index| {
                if index == 0 {
                    String::new()
                } else {
                    self.read_string(offset + index * length, length)
                }
            })
            .collect()
    }

    fn set_dex_to_internal(&mut self, dex: i32, internal: i32) {
        let index = dex.max(0) as usize;
        if self.dex_to_internal.len() <= index {
            self.dex_to_internal.resize(index + 1, 0);
        }
        self.dex_to_internal[index] = internal;
    }

    fn load_pokedex(&mut self, names: &[String]) {
        let order = self.value("PokedexOrder");
        let count = self.pokemon_count();
        self.internal_to_dex = vec![0; count as usize + 1];
        self.dex_to_internal = vec![0; count as usize + 1];
        let mut max_pokedex = 0;
        for internal in 1..=count {
            let dex = self.rom.word(order + (internal - 1) * 2);
            if dex != 0 {
                self.internal_to_dex[internal as usize] = dex;
                if self.internal_of_number(dex) == 0 {
                    self.set_dex_to_internal(dex, internal);
                }
                max_pokedex = max_pokedex.max(dex);
            }
        }
        if max_pokedex == c::UNHACKED_MAX_POKEDEX {
            self.relocate_johto_hoenn_gap(names);
        } else {
            self.is_rom_hack = true;
            self.pokedex_count = max_pokedex;
        }
    }

    fn relocate_johto_hoenn_gap(&mut self, names: &[String]) {
        let stats = self.value("PokemonStats");
        let gap = c::UNHACKED_MAX_POKEDEX - c::UNHACKED_REAL_POKEDEX;
        let mut used_slots = 0;
        for index in 0..gap {
            let slot = c::HOENN_POKES_START + index;
            let offset = stats + slot * c::BASE_STATS_ENTRY_SIZE;
            let lower_name = names
                .get(slot as usize)
                .map(|name| java_lower_case(name))
                .unwrap_or_default();
            let is_real = !self.rom.matches(offset, &c::EMPTY_POKEMON_SIG)
                && !lower_name.contains("unused")
                && lower_name != "?"
                && lower_name != "-";
            let Some(slot_dex) = self.internal_to_dex.get_mut(slot as usize) else {
                continue;
            };
            if is_real {
                used_slots += 1;
                *slot_dex = c::UNHACKED_REAL_POKEDEX + used_slots;
                self.set_dex_to_internal(c::UNHACKED_REAL_POKEDEX + used_slots, slot);
            } else {
                *slot_dex = 0;
            }
        }
        for index in used_slots + 1..=gap {
            self.set_dex_to_internal(c::UNHACKED_REAL_POKEDEX + index, 0);
        }
        if used_slots > 0 {
            self.is_rom_hack = true;
        }
        self.pokedex_count = c::UNHACKED_REAL_POKEDEX + used_slots;
    }

    fn load_pokemon_stats(&mut self, names: &[String]) {
        let count = self.pokemon_count();
        let stats = self.value("PokemonStats");
        let mut last_slot_for_dex = vec![0; self.dex_to_internal.len().max(1)];
        for internal in 1..=count {
            let dex = self.internal_to_dex[internal as usize];
            if dex > 0 {
                if last_slot_for_dex.len() <= dex as usize {
                    last_slot_for_dex.resize(dex as usize + 1, 0);
                }
                last_slot_for_dex[dex as usize] = internal;
            }
        }
        self.internal_slots = vec![InternalSlot::Missing; count as usize + 1];
        for internal in 1..=count {
            let dex = self.internal_to_dex[internal as usize];
            let is_main = dex > 0 && last_slot_for_dex[dex as usize] == internal;
            let slot = if is_main {
                let id = self.core.data.create_pokemon(StatLayout::Standard, dex);
                InternalSlot::Main(id)
            } else {
                let id = self.shadow.create_pokemon(StatLayout::Standard, internal);
                self.shadow.pk_mut(id).number = dex;
                InternalSlot::Shadow(id)
            };
            self.internal_slots[internal as usize] = slot;
            let offset = stats + internal * c::BASE_STATS_ENTRY_SIZE;
            let name = names.get(internal as usize).cloned().unwrap_or_default();
            let rom = &self.rom;
            let pokemon = match slot {
                InternalSlot::Main(id) => self.core.data.pk_mut(id),
                InternalSlot::Shadow(id) => self.shadow.pk_mut(id),
                InternalSlot::Missing => continue,
            };
            pokemon.name = name;
            load_basic_stats(rom, pokemon, offset);
        }
        if matches!(
            self.rom_type(),
            RomType::FireRedLeafGreen | RomType::Emerald
        ) {
            self.load_deoxys_hardcoded_stats();
        }
    }

    fn deoxys_stats_offset(&self) -> Option<i32> {
        let prefix = self.entry.string("DeoxysStatPrefix")?;
        let offset = self.rom.find(prefix);
        (offset > 0).then(|| offset + prefix.len() as i32 / 2)
    }

    fn deoxys(&self) -> Option<PokemonId> {
        let id = PokemonId::from_index(species::DEOXYS as usize);
        self.core.data.exists(id).then_some(id)
    }

    fn load_deoxys_hardcoded_stats(&mut self) {
        let (Some(offset), Some(deoxys)) = (self.deoxys_stats_offset(), self.deoxys()) else {
            return;
        };
        let stats: Vec<i32> = (0..6)
            .map(|index| self.rom.word(offset + index * 2))
            .collect();
        let pk = self.core.data.pk_mut(deoxys);
        pk.hp = stats[0];
        pk.attack = stats[1];
        pk.defense = stats[2];
        pk.speed = stats[3];
        pk.spatk = stats[4];
        pk.spdef = stats[5];
    }

    fn save_deoxys_hardcoded_stats(&mut self) {
        let (Some(offset), Some(deoxys)) = (self.deoxys_stats_offset(), self.deoxys()) else {
            return;
        };
        let pk = self.core.data.pk(deoxys);
        let stats = [pk.hp, pk.attack, pk.defense, pk.speed, pk.spatk, pk.spdef];
        for (index, stat) in stats.into_iter().enumerate() {
            self.rom.set_word(offset + index as i32 * 2, stat);
        }
    }

    fn construct_pokemon_list(&mut self) {
        let mut list = vec![None];
        for number in 1..=self.pokedex_count.max(0) {
            let id = PokemonId::from_index(number as usize);
            let exists = self.core.data.exists(id);
            if !self.is_rom_hack {
                list.push(exists.then_some(id));
                continue;
            }
            if exists {
                let lower_name = java_lower_case(&self.core.data.pk(id).name);
                if !lower_name.contains("unused") && lower_name != "?" {
                    list.push(Some(id));
                }
            }
        }
        self.pokemon_list = list;
    }

    pub(crate) fn slot_pokemon(&self, internal: i32) -> Option<&Pokemon> {
        match self.internal_slots.get(usize::try_from(internal).ok()?)? {
            InternalSlot::Main(id) => Some(self.core.data.pk(*id)),
            InternalSlot::Shadow(id) => Some(self.shadow.pk(*id)),
            InternalSlot::Missing => None,
        }
    }

    pub(crate) fn save_pokemon_stats(&mut self) {
        let names = self.value("PokemonNames");
        let name_length = self.value("PokemonNameLength");
        let stats = self.value("PokemonStats");
        for internal in 1..=self.pokemon_count() {
            let Some(pokemon) = self.slot_pokemon(internal).cloned() else {
                continue;
            };
            self.write_fixed_string(&pokemon.name, names + internal * name_length, name_length);
            save_basic_stats(
                &mut self.rom,
                &pokemon,
                stats + internal * c::BASE_STATS_ENTRY_SIZE,
            );
        }
        if matches!(
            self.rom_type(),
            RomType::FireRedLeafGreen | RomType::Emerald
        ) {
            self.save_deoxys_hardcoded_stats();
        }
        self.write_evolutions();
    }

    fn load_ability_names(&mut self) {
        let offset = self.value("AbilityNames");
        let length = self.value("AbilityNameLength");
        self.ability_names = (0..=c::HIGHEST_ABILITY_INDEX)
            .map(|index| self.read_string(offset + length * index, length))
            .collect();
    }

    fn load_item_names(&mut self) {
        let offset = self.value("ItemData");
        let size = self.value("ItemEntrySize");
        self.item_names = (0..=self.value("ItemCount"))
            .map(|index| self.read_variable_string(offset + size * index))
            .collect();
    }

    pub(crate) fn log(&mut self, line: &str) {
        self.core.engine.handler_log(line);
    }

    pub(crate) fn apply_tweak_patch(&mut self, key: &str) -> Result<bool> {
        let Some(name) = self.entry.tweak(key).map(str::to_owned) else {
            return Ok(false);
        };
        let patch = crate::assets::patch_bytes(&name)
            .ok_or_else(|| RandomizationError::Io(format!("missing patch {name}")))?;
        self.rom
            .apply_ips(patch)
            .map_err(|error| RandomizationError::Io(format!("{name}: {error}")))?;
        Ok(true)
    }
}

pub(crate) fn load_basic_stats(rom: &GbaRom, pokemon: &mut Pokemon, offset: i32) {
    let read = |delta: i32| rom.int(offset + delta);
    pokemon.hp = read(c::BS_HP_OFFSET);
    pokemon.attack = read(c::BS_ATTACK_OFFSET);
    pokemon.defense = read(c::BS_DEFENSE_OFFSET);
    pokemon.speed = read(c::BS_SPEED_OFFSET);
    pokemon.spatk = read(c::BS_SP_ATK_OFFSET);
    pokemon.spdef = read(c::BS_SP_DEF_OFFSET);
    pokemon.primary_type = c::type_table(read(c::BS_PRIMARY_TYPE_OFFSET));
    pokemon.secondary_type = c::type_table(read(c::BS_SECONDARY_TYPE_OFFSET));
    if pokemon.secondary_type == pokemon.primary_type {
        pokemon.secondary_type = None;
    }
    pokemon.catch_rate = read(c::BS_CATCH_RATE_OFFSET);
    pokemon.growth_curve = ExpCurve::from_byte(rom.byte(offset + c::BS_GROWTH_CURVE_OFFSET) as i8);
    pokemon.ability1 = read(c::BS_ABILITY1_OFFSET);
    pokemon.ability2 = read(c::BS_ABILITY2_OFFSET);
    let common = rom.word(offset + c::BS_COMMON_HELD_ITEM_OFFSET);
    let rare = rom.word(offset + c::BS_RARE_HELD_ITEM_OFFSET);
    if common == rare {
        pokemon.guaranteed_held_item = common;
        pokemon.common_held_item = 0;
        pokemon.rare_held_item = 0;
    } else {
        pokemon.guaranteed_held_item = 0;
        pokemon.common_held_item = common;
        pokemon.rare_held_item = rare;
    }
    pokemon.dark_grass_held_item = -1;
    pokemon.gender_ratio = read(c::BS_GENDER_RATIO_OFFSET);
}

pub(crate) fn save_basic_stats(rom: &mut GbaRom, pokemon: &Pokemon, offset: i32) {
    rom.set_byte(offset + c::BS_HP_OFFSET, pokemon.hp as u8);
    rom.set_byte(offset + c::BS_ATTACK_OFFSET, pokemon.attack as u8);
    rom.set_byte(offset + c::BS_DEFENSE_OFFSET, pokemon.defense as u8);
    rom.set_byte(offset + c::BS_SPEED_OFFSET, pokemon.speed as u8);
    rom.set_byte(offset + c::BS_SP_ATK_OFFSET, pokemon.spatk as u8);
    rom.set_byte(offset + c::BS_SP_DEF_OFFSET, pokemon.spdef as u8);
    let primary = c::type_to_byte(pokemon.primary_type);
    rom.set_byte(offset + c::BS_PRIMARY_TYPE_OFFSET, primary);
    let secondary = if pokemon.secondary_type.is_none() {
        primary
    } else {
        c::type_to_byte(pokemon.secondary_type)
    };
    rom.set_byte(offset + c::BS_SECONDARY_TYPE_OFFSET, secondary);
    rom.set_byte(offset + c::BS_CATCH_RATE_OFFSET, pokemon.catch_rate as u8);
    if let Some(growth) = pokemon.growth_curve {
        rom.set_byte(offset + c::BS_GROWTH_CURVE_OFFSET, growth.to_byte() as u8);
    }
    rom.set_byte(offset + c::BS_ABILITY1_OFFSET, pokemon.ability1 as u8);
    let ability2 = if pokemon.ability2 == 0 {
        pokemon.ability1
    } else {
        pokemon.ability2
    };
    rom.set_byte(offset + c::BS_ABILITY2_OFFSET, ability2 as u8);
    if pokemon.guaranteed_held_item > 0 {
        rom.set_word(
            offset + c::BS_COMMON_HELD_ITEM_OFFSET,
            pokemon.guaranteed_held_item,
        );
        rom.set_word(
            offset + c::BS_RARE_HELD_ITEM_OFFSET,
            pokemon.guaranteed_held_item,
        );
    } else {
        rom.set_word(
            offset + c::BS_COMMON_HELD_ITEM_OFFSET,
            pokemon.common_held_item,
        );
        rom.set_word(offset + c::BS_RARE_HELD_ITEM_OFFSET, pokemon.rare_held_item);
    }
    rom.set_byte(
        offset + c::BS_GENDER_RATIO_OFFSET,
        pokemon.gender_ratio as u8,
    );
}
