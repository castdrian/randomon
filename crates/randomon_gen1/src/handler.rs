use std::collections::BTreeMap;

use randomon_core::{
    Compatibility, EggMoves, HandlerCore, Movesets, RandomizationError, Result, RomHandler,
    TrainerNameMode,
};
use randomon_data::constants::global;
use randomon_data::constants::{gb, gen1, species};
use randomon_data::{
    EncounterSet, Evolution, EvolutionType, ExpCurve, IngameTrade, ItemList, MegaEvolutionId, Move,
    MoveLearnt, PokemonId, Shop, StatChange, StatLayout, StaticEncounter, TotemPokemon, Trainer,
    Type, stat,
};
use randomon_java::SharedRandomSource;
use randomon_log::LogWriter;
use randomon_settings::MiscTweak;

use randomon_gb::{
    HeaderIds, TextTables, bank_of, calculate_offset, length_of_string_at, make_gb_pointer,
    read_word, rom_signature_matches, write_word,
};

use crate::assets::{patch_bytes, table_text};
use crate::maps::{MapData, load_maps};
use crate::moves::{
    apply_misc_info_from_effect, apply_stat_changes_from_effect, apply_status_from_effect,
    hit_count_for_rom_id, initial_category,
};
use crate::rom_entry::{RomEntry, rom_entries};
use randomon_gb::crc32;

pub struct Gen1RomHandler {
    core: HandlerCore,
    rom: Vec<u8>,
    original_rom: Vec<u8>,
    file_name: String,
    entry: &'static RomEntry,
    tables: TextTables,
    poke_num_to_rby: [i32; 256],
    poke_rby_to_num: [i32; 256],
    move_num_to_rom: [i32; 256],
    move_rom_to_num: [i32; 256],
    pokedex_count: i32,
    pokemon_list: Vec<Option<PokemonId>>,
    item_names: Vec<String>,
    maps: MapData,
    x_acc_nerfed: bool,
    actual_crc32: i64,
    effectiveness_updated: bool,
    allowed_items: ItemList,
}

pub fn check_rom_entry(rom: &[u8]) -> Option<&'static RomEntry> {
    let HeaderIds {
        version,
        non_japanese,
        crc_in_header,
    } = HeaderIds::read(rom)?;
    let matches = |entry: &RomEntry| {
        entry
            .rom_name
            .as_deref()
            .is_some_and(|name| rom_signature_matches(rom, name))
            && entry.version == version
            && entry.non_japanese == non_japanese
    };
    rom_entries()
        .iter()
        .find(|entry| matches(entry) && entry.crc_in_header == crc_in_header)
        .or_else(|| {
            rom_entries()
                .iter()
                .find(|entry| matches(entry) && entry.crc_in_header == -1)
        })
}

pub fn detect_rom(rom: &[u8]) -> bool {
    rom.len() >= gb::MIN_ROM_SIZE && rom.len() <= gb::MAX_ROM_SIZE && check_rom_entry(rom).is_some()
}

impl Gen1RomHandler {
    pub fn load(rom: Vec<u8>, file_name: &str, random_source: &SharedRandomSource) -> Result<Self> {
        if !detect_rom(&rom) {
            return Err(RandomizationError::UnsupportedRom(file_name.to_owned()));
        }
        let entry = check_rom_entry(&rom)
            .ok_or_else(|| RandomizationError::UnsupportedRom(file_name.to_owned()))?;
        let mut tables = TextTables::default();
        tables.read_table(randomon_gb::GAMEBOY_JPN_TABLE);
        if let Some(extra) = entry.extra_table_file.as_deref()
            && !extra.eq_ignore_ascii_case("none")
            && let Some(text) = table_text(extra)
        {
            tables.read_table(text);
        }
        let maps = load_maps(&rom, entry, &tables);
        let mut handler = Self {
            core: HandlerCore::new(random_source),
            original_rom: rom.clone(),
            actual_crc32: crc32(&rom),
            rom,
            file_name: file_name.to_owned(),
            entry,
            tables,
            poke_num_to_rby: [0; 256],
            poke_rby_to_num: [0; 256],
            move_num_to_rom: [0; 256],
            move_rom_to_num: [0; 256],
            pokedex_count: 0,
            pokemon_list: Vec::new(),
            item_names: Vec::new(),
            maps,
            x_acc_nerfed: false,
            effectiveness_updated: false,
            allowed_items: gen1::allowed_items(),
        };
        handler.load_pokedex_order();
        handler.load_pokemon_stats()?;
        handler.load_moves();
        handler.load_item_names();
        Ok(handler)
    }

    fn value(&self, key: &str) -> i32 {
        self.entry.value(key)
    }

    fn byte(&self, offset: i32) -> u8 {
        self.rom[offset as usize]
    }

    fn signed_byte(&self, offset: i32) -> i32 {
        i32::from(self.rom[offset as usize] as i8)
    }

    fn set_byte(&mut self, offset: i32, value: u8) {
        self.rom[offset as usize] = value;
    }

    fn word(&self, offset: i32) -> i32 {
        read_word(&self.rom, offset)
    }

    fn read_fixed_string(&self, offset: i32, length: usize) -> String {
        self.tables.read_string(&self.rom, offset, length, false)
    }

    fn read_variable_string(&self, offset: i32) -> String {
        self.tables
            .read_string(&self.rom, offset, usize::MAX, false)
    }

    fn write_fixed_string(&mut self, text: &str, offset: i32, length: i32) {
        let translated = self.tables.translate(text);
        let length = length.max(0) as usize;
        let copied = translated.len().min(length);
        let start = offset as usize;
        self.rom[start..start + copied].copy_from_slice(&translated[..copied]);
        for byte in &mut self.rom[start + copied..start + length] {
            *byte = gb::STRING_TERMINATOR;
        }
    }

    fn write_variable_string(&mut self, text: &str, offset: i32, already_terminated: bool) {
        let translated = self.tables.translate(text);
        let start = offset as usize;
        self.rom[start..start + translated.len()].copy_from_slice(&translated);
        if !already_terminated {
            self.rom[start + translated.len()] = gb::STRING_TERMINATOR;
        }
    }

    fn id_to_type(&self, value: i32) -> Option<Type> {
        gen1::type_table(value).or_else(|| self.entry.extra_type_lookup.get(&value).copied())
    }

    fn type_to_byte(&self, kind: Option<Type>) -> u8 {
        let Some(kind) = kind else {
            return 0;
        };
        if let Some(&value) = self.entry.extra_type_reverse.get(&kind) {
            return value as u8;
        }
        gen1::type_to_byte(Some(kind))
    }

    fn pokemon_by_internal(&self, internal: u8) -> PokemonId {
        PokemonId::from_index(self.poke_rby_to_num[internal as usize] as usize)
    }

    fn loaded_pokemon_by_internal(&self, internal: u8) -> Result<PokemonId> {
        let pokemon = self.pokemon_by_internal(internal);
        if self.core.data.exists(pokemon) {
            Ok(pokemon)
        } else {
            Err(RandomizationError::NullPointer(
                "pokes[pokeRBYToNumTable[...]]",
            ))
        }
    }

    fn internal_of(&self, pokemon: PokemonId) -> u8 {
        self.poke_num_to_rby[self.core.data.pk(pokemon).number as usize] as u8
    }

    fn load_pokedex_order(&mut self) {
        let count = self.value("InternalPokemonCount");
        let order_offset = self.value("PokedexOrder");
        self.pokedex_count = 0;
        for internal in 1..=count {
            let dex = i32::from(self.byte(order_offset + internal - 1));
            self.poke_rby_to_num[internal as usize] = dex;
            if dex != 0 && self.poke_num_to_rby[dex as usize] == 0 {
                self.poke_num_to_rby[dex as usize] = internal;
            }
            self.pokedex_count = self.pokedex_count.max(dex);
        }
    }

    fn read_pokemon_names(&self) -> Vec<String> {
        let offset = self.value("PokemonNamesOffset");
        let length = self.value("PokemonNamesLength");
        let count = self.value("InternalPokemonCount");
        let mut names = vec![String::new(); (count + 1) as usize];
        for internal in 1..=count {
            names[internal as usize] =
                self.read_fixed_string(offset + (internal - 1) * length, length as usize);
        }
        names
    }

    fn load_pokemon_stats(&mut self) -> Result<()> {
        let names = self.read_pokemon_names();
        let stats_offset = self.value("PokemonStatsOffset");
        self.pokemon_list = vec![None];
        for number in 1..=self.pokedex_count {
            let id = self.core.data.create_pokemon(StatLayout::Gen1, number);
            self.pokemon_list.push(Some(id));
            if number != species::MEW || self.entry.is_yellow {
                self.load_basic_stats(
                    id,
                    stats_offset + (number - 1) * gen1::BASE_STATS_ENTRY_SIZE,
                );
            }
            let name = names
                .get(self.poke_num_to_rby[number as usize] as usize)
                .cloned()
                .unwrap_or_default();
            self.core.data.pk_mut(id).name = name;
        }
        if !self.entry.is_yellow {
            let mew = PokemonId::from_index(species::MEW as usize);
            self.load_basic_stats(mew, self.value("MewStatsOffset"));
        }
        self.populate_evolutions()
    }

    fn load_basic_stats(&mut self, id: PokemonId, offset: i32) {
        let read = |delta: i32| i32::from(self.rom[(offset + delta) as usize]);
        let primary = self.id_to_type(read(gen1::BS_PRIMARY_TYPE_OFFSET));
        let mut secondary = self.id_to_type(read(gen1::BS_SECONDARY_TYPE_OFFSET));
        if secondary == primary {
            secondary = None;
        }
        let front_sprite = self.word(offset + gen1::BS_FRONT_SPRITE_OFFSET);
        let growth =
            ExpCurve::from_byte(self.rom[(offset + gen1::BS_GROWTH_CURVE_OFFSET) as usize] as i8);
        let pk = self.core.data.pk_mut(id);
        pk.hp = read(gen1::BS_HP_OFFSET);
        pk.attack = read(gen1::BS_ATTACK_OFFSET);
        pk.defense = read(gen1::BS_DEFENSE_OFFSET);
        pk.speed = read(gen1::BS_SPEED_OFFSET);
        pk.special = read(gen1::BS_SPECIAL_OFFSET);
        pk.primary_type = primary;
        pk.secondary_type = secondary;
        pk.catch_rate = read(gen1::BS_CATCH_RATE_OFFSET);
        pk.exp_yield = read(gen1::BS_EXP_YIELD_OFFSET);
        pk.growth_curve = growth;
        pk.front_sprite_pointer = front_sprite;
        pk.guaranteed_held_item = -1;
        pk.common_held_item = -1;
        pk.rare_held_item = -1;
        pk.dark_grass_held_item = -1;
    }

    fn populate_evolutions(&mut self) -> Result<()> {
        let pointers_offset = self.value("PokemonMovesetsTableOffset");
        let count = self.value("InternalPokemonCount");
        for internal in 1..=count {
            let pointer = self.word(pointers_offset + (internal - 1) * 2);
            let mut real_pointer = calculate_offset(bank_of(pointers_offset), pointer);
            let number = self.poke_rby_to_num[internal as usize];
            if number == 0 {
                continue;
            }
            let pokemon = PokemonId::from_index(number as usize);
            while self.byte(real_pointer) != 0 {
                let method = self.signed_byte(real_pointer);
                let kind = EvolutionType::from_index(1, method).ok_or_else(|| {
                    RandomizationError::randomization(format!(
                        "unknown Gen 1 evolution method {method}"
                    ))
                })?;
                let stone = kind == EvolutionType::Stone;
                let target_internal = self.byte(real_pointer + 2 + i32::from(stone));
                let target = self.pokemon_by_internal(target_internal);
                let extra = i32::from(self.byte(real_pointer + 1));
                let evolution = Evolution::new(pokemon, target, true, kind, extra);
                let data = &mut self.core.data;
                if !data.evolution_list_contains(data.evolutions_from(pokemon), &evolution) {
                    let id = data.new_evolution(evolution);
                    data.evolutions_from_mut(pokemon).push(id);
                    if data.exists(target) {
                        data.evolutions_to_mut(target).push(id);
                    }
                }
                real_pointer += if stone { 4 } else { 3 };
            }
            let data = &mut self.core.data;
            if data.evolutions_from(pokemon).len() > 1 {
                for evo in data.evolutions_from(pokemon).to_vec() {
                    data.evo_mut(evo).carry_stats = false;
                }
            }
        }
        Ok(())
    }

    fn read_move_names(&self) -> Vec<String> {
        let count = self.value("MoveCount");
        let mut offset = self.value("MoveNamesOffset");
        let mut names = vec![String::new(); (count + 1) as usize];
        for index in 1..=count {
            names[index as usize] = self.read_variable_string(offset);
            offset += length_of_string_at(&self.rom, offset, false) + 1;
        }
        names
    }

    fn load_moves(&mut self) {
        let names = self.read_move_names();
        let count = self.value("MoveCount");
        let moves_offset = self.value("MoveDataOffset");
        let mut true_index = 0;
        for rom_id in 1..=count {
            let base = moves_offset + (rom_id - 1) * 6;
            let animation = self.byte(base);
            if animation == 0 || names[rom_id as usize] == "Nothing" {
                continue;
            }
            true_index += 1;
            self.move_num_to_rom[true_index as usize] = rom_id;
            self.move_rom_to_num[rom_id as usize] = true_index;
            let mut mv = Move {
                name: names[rom_id as usize].clone(),
                internal_id: rom_id,
                number: true_index,
                effect_index: i32::from(self.byte(base + 1)),
                hitratio: f64::from(self.byte(base + 4)) / 255.0 * 100.0,
                power: i32::from(self.byte(base + 2)),
                pp: i32::from(self.byte(base + 5)),
                kind: self.id_to_type(i32::from(self.byte(base + 3))),
                ..Move::default()
            };
            mv.category = initial_category(&mv);
            if mv.name == "Swift" {
                self.core.engine.perfect_accuracy = mv.hitratio as i32;
            }
            mv.hit_count = hit_count_for_rom_id(rom_id);
            apply_stat_changes_from_effect(&mut mv);
            apply_status_from_effect(&mut mv);
            apply_misc_info_from_effect(&mut mv);
            self.core.data.create_move(mv);
        }
        self.core.data.reserve_moves(true_index as usize + 1);
    }

    fn load_item_names(&mut self) {
        let mut names = vec![String::new(); 256];
        names[0] = "glitch".to_owned();
        let original = self.value("ItemNamesOffset");
        let mut offset = original;
        for index in 1..=0x100 {
            if offset / gb::BANK_SIZE > original / gb::BANK_SIZE {
                break;
            }
            let start = offset;
            while self.byte(offset) != gb::STRING_TERMINATOR {
                offset += 1;
            }
            offset += 1;
            names[index % 256] = self.read_fixed_string(start, 20);
        }
        for index in gen1::HMS_START_INDEX..gen1::TMS_START_INDEX {
            names[index as usize] = format!("HM{:02}", index - gen1::HMS_START_INDEX + 1);
        }
        for index in gen1::TMS_START_INDEX..0x100 {
            names[index as usize] = format!("TM{:02}", index - gen1::TMS_START_INDEX + 1);
        }
        self.item_names = names;
    }

    fn map_name(&self, map: usize) -> String {
        self.maps
            .names
            .get(map)
            .cloned()
            .flatten()
            .unwrap_or_else(|| "null".to_owned())
    }

    fn ghost_marowak(&self) -> PokemonId {
        if self.can_change_static_pokemon() {
            self.pokemon_by_internal(self.byte(self.entry.ghost_marowak_offsets[0]))
        } else {
            PokemonId::from_index(species::MAROWAK as usize)
        }
    }

    fn trainer_pointers(&self) -> Vec<i32> {
        let table = self.value("TrainerDataTableOffset");
        let mut pointers = vec![0; (gen1::TRAINER_CLASS_COUNT + 1) as usize];
        for class in 1..=gen1::TRAINER_CLASS_COUNT {
            let pointer = self.word(table + (class - 1) * 2);
            pointers[class as usize] = calculate_offset(bank_of(table), pointer);
        }
        pointers
    }

    fn trainer_class_names_for_text(&self) -> Vec<String> {
        let offsets = self.entry.array("TrainerClassNamesOffsets");
        let mut offset = offsets[offsets.len() - 1];
        let mut names = Vec::new();
        for _ in 0..gen1::TCLASSES_COUNTS[1] {
            names.push(self.read_variable_string(offset));
            offset += length_of_string_at(&self.rom, offset, false) + 1;
        }
        names
    }

    fn base_stats_offset(&self, number: i32) -> i32 {
        if self.entry.is_yellow || number != species::MEW {
            self.value("PokemonStatsOffset") + (number - 1) * gen1::BASE_STATS_ENTRY_SIZE
        } else {
            self.value("MewStatsOffset")
        }
    }

    fn save_pokemon_stats(&mut self) -> Result<()> {
        let names_offset = self.value("PokemonNamesOffset");
        let name_length = self.value("PokemonNamesLength");
        for number in 1..=self.pokedex_count {
            let internal = self.poke_num_to_rby[number as usize];
            let name = self
                .core
                .data
                .pk(PokemonId::from_index(number as usize))
                .name
                .clone();
            self.write_fixed_string(
                &name,
                names_offset + (internal - 1) * name_length,
                name_length,
            );
        }
        let stats_offset = self.value("PokemonStatsOffset");
        for number in 1..=self.pokedex_count {
            if number == species::MEW {
                continue;
            }
            self.save_basic_stats(
                PokemonId::from_index(number as usize),
                stats_offset + (number - 1) * gen1::BASE_STATS_ENTRY_SIZE,
            );
        }
        let mew_offset = if self.entry.is_yellow {
            stats_offset + (species::MEW - 1) * gen1::BASE_STATS_ENTRY_SIZE
        } else {
            self.value("MewStatsOffset")
        };
        self.save_basic_stats(PokemonId::from_index(species::MEW as usize), mew_offset);
        self.write_evos_and_moves_learnt(true, None)
    }

    fn save_basic_stats(&mut self, id: PokemonId, offset: i32) {
        let pk = self.core.data.pk(id).clone();
        let primary = self.type_to_byte(pk.primary_type);
        let secondary = if pk.secondary_type.is_none() {
            primary
        } else {
            self.type_to_byte(pk.secondary_type)
        };
        self.set_byte(offset + gen1::BS_HP_OFFSET, pk.hp as u8);
        self.set_byte(offset + gen1::BS_ATTACK_OFFSET, pk.attack as u8);
        self.set_byte(offset + gen1::BS_DEFENSE_OFFSET, pk.defense as u8);
        self.set_byte(offset + gen1::BS_SPEED_OFFSET, pk.speed as u8);
        self.set_byte(offset + gen1::BS_SPECIAL_OFFSET, pk.special as u8);
        self.set_byte(offset + gen1::BS_PRIMARY_TYPE_OFFSET, primary);
        self.set_byte(offset + gen1::BS_SECONDARY_TYPE_OFFSET, secondary);
        self.set_byte(offset + gen1::BS_CATCH_RATE_OFFSET, pk.catch_rate as u8);
        self.set_byte(
            offset + gen1::BS_GROWTH_CURVE_OFFSET,
            pk.growth_curve.map_or(0, ExpCurve::to_byte) as u8,
        );
        self.set_byte(offset + gen1::BS_EXP_YIELD_OFFSET, pk.exp_yield as u8);
    }

    fn save_moves(&mut self) {
        let moves_offset = self.value("MoveDataOffset");
        let moves: Vec<Move> = self
            .core
            .data
            .move_ids()
            .into_iter()
            .flatten()
            .map(|id| self.core.data.mv(id).clone())
            .collect();
        for mv in moves {
            let base = moves_offset + (mv.internal_id - 1) * 6;
            self.set_byte(base + 1, mv.effect_index as u8);
            self.set_byte(base + 2, mv.power as u8);
            let kind = self.type_to_byte(mv.kind);
            self.set_byte(base + 3, kind);
            let hitratio =
                (randomon_java::math::round_double(mv.hitratio * 2.55) as i32).clamp(0, 255);
            self.set_byte(base + 4, hitratio as u8);
            self.set_byte(base + 5, mv.pp as u8);
        }
    }

    fn evolution_bytes(&self, pokemon: PokemonId) -> Vec<u8> {
        let data = &self.core.data;
        let mut bytes = Vec::new();
        for &evo in data.evolutions_from(pokemon) {
            let evolution = data.evo(evo);
            bytes.push(evolution.kind.to_index(1) as u8);
            match evolution.kind {
                EvolutionType::Level => bytes.push(evolution.extra_info as u8),
                EvolutionType::Stone => {
                    bytes.push(evolution.extra_info as u8);
                    bytes.push(1);
                }
                EvolutionType::Trade => bytes.push(1),
                _ => {}
            }
            bytes.push(self.poke_num_to_rby[data.pk(evolution.to).number as usize] as u8);
        }
        bytes
    }

    fn copy_old_evolutions(&self, old_offset: i32, out: &mut Vec<u8>) {
        let mut offset = old_offset;
        while self.byte(offset) != 0 {
            let limiter = if self.byte(offset) == 2 { 4 } else { 3 };
            for _ in 0..limiter {
                out.push(self.byte(offset));
                offset += 1;
            }
        }
    }

    fn copy_old_moves(&self, old_offset: i32, out: &mut Vec<u8>) {
        let mut offset = old_offset;
        while self.byte(offset) != 0 {
            offset += if self.byte(offset) == 2 { 4 } else { 3 };
        }
        offset += 1;
        while self.byte(offset) != 0 {
            out.push(self.byte(offset));
            out.push(self.byte(offset + 1));
            offset += 2;
        }
    }

    fn write_new_moves(&mut self, pokemon: PokemonId, moves: &[MoveLearnt], out: &mut Vec<u8>) {
        let number = self.core.data.pk(pokemon).number;
        let stats_offset = self.base_stats_offset(number);
        let mut index = 0;
        while index < 4 && moves.len() > index && moves[index].level == 1 {
            let rom_move = self.move_num_to_rom[moves[index].move_number as usize] as u8;
            self.set_byte(
                stats_offset + gen1::BS_LEVEL1_MOVES_OFFSET + index as i32,
                rom_move,
            );
            index += 1;
        }
        for slot in index..4 {
            self.set_byte(stats_offset + gen1::BS_LEVEL1_MOVES_OFFSET + slot as i32, 0);
        }
        while index < moves.len() {
            out.push(moves[index].level as u8);
            out.push(self.move_num_to_rom[moves[index].move_number as usize] as u8);
            index += 1;
        }
    }

    fn write_evos_and_moves_learnt(
        &mut self,
        write_evos: bool,
        movesets: Option<&Movesets>,
    ) -> Result<()> {
        let start = self.value("PokemonMovesetsTableOffset");
        let bank = bank_of(start);
        let count = self.value("InternalPokemonCount");
        let mut pointer_table = vec![0u8; (count * 2) as usize];
        let main_size = self.value("PokemonMovesetsDataSize");
        let main_offset = start + pointer_table.len() as i32;
        let mut main_block = DataBlock::new(main_offset, main_size);
        let extra_offset = self.value("PokemonMovesetsExtraSpaceOffset");
        let mut extra_block = (bank == bank_of(extra_offset) && extra_offset != 0).then(|| {
            let next_bank = ((extra_offset / gb::BANK_SIZE) + 1) * gb::BANK_SIZE;
            DataBlock::new(extra_offset, next_bank - extra_offset)
        });
        let mut null_entry_pointer = -1;
        for internal in 1..=count {
            let old_offset = calculate_offset(bank, self.word(start + (internal - 1) * 2));
            let mut set_null_pointer_here = false;
            let number = self.poke_rby_to_num[internal as usize];
            let data = if number == 0 {
                if null_entry_pointer == -1 {
                    set_null_pointer_here = true;
                    Some(vec![0u8, 0u8])
                } else {
                    write_word(&mut pointer_table, (internal - 1) * 2, null_entry_pointer);
                    None
                }
            } else {
                let pokemon = PokemonId::from_index(number as usize);
                let mut bytes = Vec::new();
                if write_evos {
                    bytes.extend(self.evolution_bytes(pokemon));
                } else {
                    self.copy_old_evolutions(old_offset, &mut bytes);
                }
                bytes.push(0);
                match movesets {
                    None => self.copy_old_moves(old_offset, &mut bytes),
                    Some(movesets) => {
                        let moves = movesets
                            .get(&number)
                            .ok_or(RandomizationError::NullPointer("movesets.get"))?
                            .clone();
                        self.write_new_moves(pokemon, &moves, &mut bytes);
                    }
                }
                bytes.push(0);
                Some(bytes)
            };
            let Some(data) = data else {
                continue;
            };
            let pointer = match main_block.place(&data) {
                Some(pointer) => pointer,
                None => match extra_block.as_mut().and_then(|block| block.place(&data)) {
                    Some(pointer) => pointer,
                    None => {
                        return Err(RandomizationError::randomization(
                            "Unable to save moves/evolutions, out of space",
                        ));
                    }
                },
            };
            if pointer >= 0 {
                write_word(&mut pointer_table, (internal - 1) * 2, pointer);
                if set_null_pointer_here {
                    null_entry_pointer = pointer;
                }
            }
        }
        let start = start as usize;
        self.rom[start..start + pointer_table.len()].copy_from_slice(&pointer_table);
        main_block.write_into(&mut self.rom);
        if let Some(block) = extra_block {
            block.write_into(&mut self.rom);
        }
        Ok(())
    }

    fn update_type_effectiveness(&mut self) {
        let mut table = self.read_type_effectiveness_table();
        self.core
            .engine
            .handler_log("--Updating Type Effectiveness--");
        for relationship in &mut table {
            if relationship.attacker == Some(Type::Poison)
                && relationship.defender == Some(Type::Bug)
            {
                relationship.attacker = Some(Type::Ice);
                relationship.defender = Some(Type::Fire);
                relationship.effectiveness = 5;
                self.core.engine.handler_log(
                    "Replaced: Poison super effective vs Bug => Ice not very effective vs Fire",
                );
            } else if relationship.attacker == Some(Type::Bug)
                && relationship.defender == Some(Type::Poison)
            {
                relationship.effectiveness = 5;
                self.core.engine.handler_log(
                    "Changed: Bug super effective vs Poison => Bug not very effective vs Poison",
                );
            } else if relationship.attacker == Some(Type::Ghost)
                && relationship.defender == Some(Type::Psychic)
            {
                relationship.effectiveness = 20;
                self.core.engine.handler_log(
                    "Changed: Psychic immune to Ghost => Ghost super effective vs Psychic",
                );
            }
        }
        self.core.engine.handler_log_blank_line();
        let mut offset = self.value("TypeEffectivenessOffset");
        for relationship in table {
            self.set_byte(offset, gen1::type_to_byte(relationship.attacker));
            self.set_byte(offset + 1, gen1::type_to_byte(relationship.defender));
            self.set_byte(offset + 2, relationship.effectiveness);
            offset += 3;
        }
        self.effectiveness_updated = true;
    }

    fn read_type_effectiveness_table(&self) -> Vec<RawTypeRelationship> {
        let mut table = Vec::new();
        let mut offset = self.value("TypeEffectivenessOffset");
        let mut attacking = self.signed_byte(offset);
        while attacking != -1 {
            let defending = self.signed_byte(offset + 1);
            let effectiveness = self.signed_byte(offset + 2);
            if matches!(effectiveness, 20 | 10 | 5 | 0) {
                table.push(RawTypeRelationship {
                    attacker: gen1::type_table(attacking),
                    defender: gen1::type_table(defending),
                    effectiveness: effectiveness as u8,
                });
            }
            offset += 3;
            attacking = self.signed_byte(offset);
        }
        table
    }

    fn generic_ips_patch(&mut self, key: &str) -> Result<bool> {
        let Some(name) = self.entry.tweak_files.get(key) else {
            return Ok(false);
        };
        let patch = patch_bytes(name)
            .ok_or_else(|| RandomizationError::Io(format!("missing patch {name}")))?;
        randomon_gb::apply_ips_patch(&mut self.rom, patch).map_err(RandomizationError::Io)?;
        Ok(true)
    }

    fn item_offsets(&self) -> Vec<i32> {
        let mut offsets: Vec<i32> = self
            .maps
            .item_offsets
            .iter()
            .flatten()
            .flatten()
            .copied()
            .collect();
        let hidden_routine = self.value("HiddenItemRoutine");
        let special_table = self.value("SpecialMapPointerTable");
        let special_bank = bank_of(special_table);
        let collect_from = |mut special_offset: i32, offsets: &mut Vec<i32>| {
            while self.byte(special_offset) != 0xFF {
                let target = calculate_offset(
                    i32::from(self.byte(special_offset + 3)),
                    self.word(special_offset + 4),
                );
                if target == hidden_routine {
                    offsets.push(special_offset + 2);
                }
                special_offset += 6;
            }
        };
        if !self.is_yellow() {
            let mut list_offset = self.value("SpecialMapList");
            let mut index = 0;
            while self.byte(list_offset) != 0xFF {
                let special_offset =
                    calculate_offset(special_bank, self.word(special_table + index));
                collect_from(special_offset, &mut offsets);
                list_offset += 1;
                index += 2;
            }
        } else {
            let mut list_offset = special_table;
            while self.byte(list_offset) != 0xFF {
                let special_offset = calculate_offset(special_bank, self.word(list_offset + 1));
                collect_from(special_offset, &mut offsets);
                list_offset += 3;
            }
        }
        offsets
    }

    fn find_unique(&self, hex: &str) -> i32 {
        randomon_gb::find_unique_hex(&self.rom, hex)
    }

    fn trade_layout(&self) -> (i32, i32, i32, Vec<i32>) {
        let table = self.value("TradeTableOffset");
        let size = self.value("TradeTableSize");
        let name_length = self.value("TradeNameLength");
        (
            table,
            size,
            name_length,
            self.entry.array("TradesUnused").to_vec(),
        )
    }

    fn write_starter_pokedex_routine(&mut self, starters: &[PokemonId]) {
        let mut on_values: BTreeMap<i32, i32> = BTreeMap::new();
        for &starter in starters.iter().take(3) {
            let dex = self.core.data.pk(starter).number;
            let ram_offset = (dex - 1) / 8 + self.value("PokedexRamOffset");
            let write_value = 1 << ((dex - 1) % 8);
            *on_values.entry(ram_offset).or_insert(0) |= write_value;
        }
        let on_offset = self.value("StarterPokedexOnOffset");
        let off_offset = self.value("StarterPokedexOffOffset");
        let on_routine_size = 5 * on_values.len() as i32 + 3;
        let on_routine = self.value("StarterPokedexBranchOffset");
        let off_routine = on_routine + on_routine_size;
        let on_pointer = make_gb_pointer(on_routine);
        let off_pointer = make_gb_pointer(off_routine);
        let return_on = make_gb_pointer(on_offset + 5);
        let return_off = make_gb_pointer(off_offset + 4);
        self.set_byte(on_offset, gb::Z80_JUMP);
        write_word(&mut self.rom, on_offset + 1, on_pointer);
        self.set_byte(on_offset + 3, gb::Z80_NOP);
        self.set_byte(on_offset + 4, gb::Z80_NOP);
        self.set_byte(off_offset, gb::Z80_JUMP);
        write_word(&mut self.rom, off_offset + 1, off_pointer);
        self.set_byte(off_offset + 3, gb::Z80_NOP);
        self.set_byte(off_routine, gb::Z80_XOR_A);
        let mut turn_on = on_routine;
        let mut turn_off = off_routine + 1;
        for (&ram_offset, &value) in &on_values {
            for byte in [
                gb::Z80_LD_A,
                value as u8,
                gb::Z80_LD_A_TO_FAR,
                (ram_offset % 0x100) as u8,
                (ram_offset / 0x100) as u8,
            ] {
                self.set_byte(turn_on, byte);
                turn_on += 1;
            }
            for byte in [
                gb::Z80_LD_A_TO_FAR,
                (ram_offset % 0x100) as u8,
                (ram_offset / 0x100) as u8,
            ] {
                self.set_byte(turn_off, byte);
                turn_off += 1;
            }
        }
        self.set_byte(turn_on, gb::Z80_JUMP);
        write_word(&mut self.rom, turn_on + 1, return_on);
        self.set_byte(turn_off, gb::Z80_JUMP);
        write_word(&mut self.rom, turn_off + 1, return_off);
    }

    fn random_pokemon_internal(&mut self) -> Result<u8> {
        let pokemon = randomon_core::engine::pool::random_pokemon(self)?;
        Ok(self.internal_of(pokemon))
    }
}

struct RawTypeRelationship {
    attacker: Option<Type>,
    defender: Option<Type>,
    effectiveness: u8,
}

struct DataBlock {
    offset: i32,
    size: i32,
    bytes: Vec<u8>,
    used: i32,
}

impl DataBlock {
    fn new(offset: i32, size: i32) -> Self {
        Self {
            offset,
            size,
            bytes: vec![0; size.max(0) as usize],
            used: 0,
        }
    }

    fn place(&mut self, data: &[u8]) -> Option<i32> {
        let length = data.len() as i32;
        let merges_with_previous = data[0] == 0 && self.used > 0;
        let fits = self.used + length <= self.size
            || (merges_with_previous && self.used + length == self.size + 1);
        if !fits {
            return None;
        }
        if merges_with_previous {
            let pointer = make_gb_pointer(self.offset + self.used - 1);
            let start = self.used as usize;
            self.bytes[start..start + data.len() - 1].copy_from_slice(&data[1..]);
            self.used += length - 1;
            Some(pointer)
        } else {
            let pointer = make_gb_pointer(self.offset + self.used);
            let start = self.used as usize;
            self.bytes[start..start + data.len()].copy_from_slice(data);
            self.used += length;
            Some(pointer)
        }
    }

    fn write_into(&self, rom: &mut [u8]) {
        let start = self.offset as usize;
        rom[start..start + self.bytes.len()].copy_from_slice(&self.bytes);
    }
}

impl RomHandler for Gen1RomHandler {
    fn core(&self) -> &HandlerCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HandlerCore {
        &mut self.core
    }

    fn save_rom(&mut self) -> Result<Vec<u8>> {
        self.save_pokemon_stats()?;
        self.save_moves();
        Ok(self.rom.clone())
    }

    fn loaded_filename(&self) -> &str {
        &self.file_name
    }

    fn print_rom_diagnostics(&self, log: &mut LogWriter) {
        let file_name = std::path::Path::new(&self.file_name)
            .file_name()
            .map_or_else(
                || self.file_name.clone(),
                |name| name.to_string_lossy().into_owned(),
            );
        log.println(&format!("File name: {file_name}"));
        log.println(&format!(
            "Original ROM CRC32: {:08X}",
            crc32(&self.original_rom)
        ));
    }

    fn is_rom_valid(&self) -> bool {
        self.entry.expected_crc32 == self.actual_crc32
    }

    fn get_pokemon(&self) -> Vec<Option<PokemonId>> {
        self.pokemon_list.clone()
    }

    fn get_pokemon_incl_formes(&self) -> Vec<Option<PokemonId>> {
        self.pokemon_list.clone()
    }

    fn get_alt_formes(&self) -> Vec<PokemonId> {
        Vec::new()
    }

    fn get_mega_evolutions(&self) -> Vec<MegaEvolutionId> {
        Vec::new()
    }

    fn get_alt_forme_of_pokemon(&self, pokemon: PokemonId, _forme: i32) -> PokemonId {
        pokemon
    }

    fn get_irregular_formes(&self) -> Vec<PokemonId> {
        Vec::new()
    }

    fn remove_evos_for_pokemon_pool(&mut self) {}

    fn get_starters(&self) -> Vec<PokemonId> {
        let mut starters = vec![
            self.pokemon_by_internal(self.byte(self.entry.array("StarterOffsets1")[0])),
            self.pokemon_by_internal(self.byte(self.entry.array("StarterOffsets2")[0])),
        ];
        if !self.entry.is_yellow {
            starters
                .push(self.pokemon_by_internal(self.byte(self.entry.array("StarterOffsets3")[0])));
        }
        starters
    }

    fn set_starters(&mut self, starters: &[PokemonId]) -> Result<bool> {
        let amount = if self.entry.is_yellow { 2 } else { 3 };
        if starters.len() != amount {
            return Ok(false);
        }
        for (index, &starter) in starters.iter().enumerate() {
            let value = self.internal_of(starter);
            for &offset in self.entry.array(&format!("StarterOffsets{}", index + 1)) {
                self.set_byte(offset, value);
            }
        }
        if !self.entry.is_yellow {
            if self.value("CanChangeStarterText") > 0 {
                let text_offsets = self.entry.array("StarterTextOffsets").to_vec();
                for (index, &offset) in text_offsets.iter().enumerate().take(3) {
                    let name = self.core.data.pk(starters[index]).name.clone();
                    self.write_variable_string(&format!("So! You want\\n{name}?\\e"), offset, true);
                }
            }
            if self.value("PatchPokedex") > 0 {
                self.write_starter_pokedex_routine(starters);
            }
        }
        if self
            .entry
            .entries
            .contains_key("PikachuHappinessCheckOffset")
            && self.core.data.pk(starters[0]).number != species::PIKACHU
        {
            let offset = self.value("PikachuHappinessCheckOffset");
            self.set_byte(offset, gb::Z80_NOP);
            self.set_byte(offset + 1, gb::Z80_NOP);
        }
        Ok(true)
    }

    fn has_starter_alt_formes(&self) -> bool {
        false
    }

    fn starter_count(&self) -> i32 {
        if self.is_yellow() { 2 } else { 3 }
    }

    fn supports_starter_held_items(&self) -> bool {
        false
    }

    fn get_starter_held_items(&self) -> Vec<i32> {
        Vec::new()
    }

    fn set_starter_held_items(&mut self, _items: &[i32]) {}

    fn get_updated_pokemon_stats(&self, generation: i32) -> BTreeMap<i32, StatChange> {
        let mut map = global::stat_changes(generation);
        if generation == 6 {
            map.insert(12, StatChange::new(stat::SPECIAL, &[90]));
            map.insert(36, StatChange::new(stat::SPECIAL, &[95]));
            map.insert(45, StatChange::new(stat::SPECIAL, &[110]));
        }
        map
    }

    fn type_in_game(&self, kind: Type) -> bool {
        if !kind.is_hack_only() && kind != Type::Dark && kind != Type::Steel && kind != Type::Fairy
        {
            return true;
        }
        self.entry.extra_type_reverse.contains_key(&kind)
    }

    fn abilities_per_pokemon(&self) -> i32 {
        0
    }

    fn highest_ability_index(&self) -> i32 {
        0
    }

    fn get_ability_variations(&self) -> BTreeMap<i32, Vec<i32>> {
        BTreeMap::new()
    }

    fn has_mega_evolutions(&self) -> bool {
        false
    }

    fn get_encounters(&self, _use_time_of_day: bool) -> Vec<EncounterSet> {
        let mut encounters: Vec<EncounterSet> = Vec::new();
        let ghost_marowak = self.ghost_marowak();
        let mut used_offsets: Vec<i32> = Vec::new();
        let mut table_offset = self.value("WildPokemonTableOffset");
        let table_bank = bank_of(table_offset);
        let mut map_id: i32 = -1;
        while self.word(table_offset) != gen1::ENCOUNTER_TABLE_END {
            map_id += 1;
            let mut offset = calculate_offset(table_bank, self.word(table_offset));
            let root_offset = offset;
            if !used_offsets.contains(&offset) {
                used_offsets.push(offset);
                for area in 0..2 {
                    let rate = i32::from(self.byte(offset));
                    offset += 1;
                    if rate > 0 {
                        let mut set = EncounterSet {
                            rate,
                            offset: root_offset,
                            display_name: Some(format!(
                                "{} on {}",
                                if area == 1 { "Surfing" } else { "Grass/Cave" },
                                self.map_name(map_id as usize)
                            )),
                            ..EncounterSet::default()
                        };
                        if (gen1::TOWER_MAPS_START_INDEX..=gen1::TOWER_MAPS_END_INDEX)
                            .contains(&map_id)
                        {
                            set.ban(ghost_marowak);
                        }
                        for _ in 0..gen1::ENCOUNTER_TABLE_SIZE {
                            set.encounters.push(randomon_data::Encounter {
                                level: i32::from(self.byte(offset)),
                                pokemon: self.pokemon_by_internal(self.byte(offset + 1)),
                                ..randomon_data::Encounter::default()
                            });
                            offset += 2;
                        }
                        encounters.push(set);
                    }
                }
            } else {
                let suffix = format!(", {}", self.map_name(map_id as usize));
                for set in &mut encounters {
                    if set.offset == offset
                        && let Some(name) = set.display_name.as_mut()
                    {
                        name.push_str(&suffix);
                    }
                }
            }
            table_offset += 2;
        }
        let old_rod = self.value("OldRodOffset");
        let mut old_rod_set = EncounterSet {
            display_name: Some("Old Rod Fishing".to_owned()),
            ..EncounterSet::default()
        };
        old_rod_set.encounters.push(randomon_data::Encounter {
            level: i32::from(self.byte(old_rod + 2)),
            pokemon: self.pokemon_by_internal(self.byte(old_rod + 1)),
            ..randomon_data::Encounter::default()
        });
        old_rod_set.ban(ghost_marowak);
        encounters.push(old_rod_set);
        let good_rod = self.value("GoodRodOffset");
        let mut good_rod_set = EncounterSet {
            display_name: Some("Good Rod Fishing".to_owned()),
            ..EncounterSet::default()
        };
        for slot in 0..2 {
            good_rod_set.encounters.push(randomon_data::Encounter {
                level: i32::from(self.byte(good_rod + slot * 2)),
                pokemon: self.pokemon_by_internal(self.byte(good_rod + slot * 2 + 1)),
                ..randomon_data::Encounter::default()
            });
        }
        good_rod_set.ban(ghost_marowak);
        encounters.push(good_rod_set);
        let mut super_rod = self.value("SuperRodTableOffset");
        if self.entry.is_yellow {
            while self.byte(super_rod) != 0xFF {
                let map = self.byte(super_rod) as usize;
                super_rod += 1;
                let mut set = EncounterSet {
                    display_name: Some(format!("Super Rod Fishing on {}", self.map_name(map))),
                    ..EncounterSet::default()
                };
                for _ in 0..gen1::YELLOW_SUPER_ROD_TABLE_SIZE {
                    set.encounters.push(randomon_data::Encounter {
                        level: i32::from(self.byte(super_rod + 1)),
                        pokemon: self.pokemon_by_internal(self.byte(super_rod)),
                        ..randomon_data::Encounter::default()
                    });
                    super_rod += 2;
                }
                set.ban(ghost_marowak);
                encounters.push(set);
            }
        } else {
            let super_rod_bank = bank_of(super_rod);
            let mut used: Vec<i32> = Vec::new();
            while self.byte(super_rod) != 0xFF {
                let map = self.byte(super_rod) as usize;
                super_rod += 1;
                let mut set_offset = calculate_offset(super_rod_bank, self.word(super_rod));
                super_rod += 2;
                if !used.contains(&set_offset) {
                    used.push(set_offset);
                    let mut set = EncounterSet {
                        display_name: Some(format!("Super Rod Fishing on {}", self.map_name(map))),
                        offset: set_offset,
                        ..EncounterSet::default()
                    };
                    let count = self.byte(set_offset);
                    set_offset += 1;
                    for _ in 0..count {
                        set.encounters.push(randomon_data::Encounter {
                            level: i32::from(self.byte(set_offset)),
                            pokemon: self.pokemon_by_internal(self.byte(set_offset + 1)),
                            ..randomon_data::Encounter::default()
                        });
                        set_offset += 2;
                    }
                    set.ban(ghost_marowak);
                    encounters.push(set);
                } else {
                    let suffix = format!(", {}", self.map_name(map));
                    for set in &mut encounters {
                        if set.offset == set_offset
                            && let Some(name) = set.display_name.as_mut()
                        {
                            name.push_str(&suffix);
                        }
                    }
                }
            }
        }
        encounters
    }

    fn set_encounters(
        &mut self,
        _use_time_of_day: bool,
        encounters: &[EncounterSet],
    ) -> Result<()> {
        let mut sets = encounters.iter();
        let mut next_set = || {
            sets.next()
                .ok_or(RandomizationError::IndexOutOfBounds("encounter sets"))
        };
        let mut used_offsets: Vec<i32> = Vec::new();
        let mut table_offset = self.value("WildPokemonTableOffset");
        let table_bank = bank_of(table_offset);
        while self.word(table_offset) != gen1::ENCOUNTER_TABLE_END {
            let mut offset = calculate_offset(table_bank, self.word(table_offset));
            if !used_offsets.contains(&offset) {
                used_offsets.push(offset);
                for _ in 0..2 {
                    let rate = self.byte(offset);
                    offset += 1;
                    if rate > 0 {
                        let set = next_set()?;
                        for slot in 0..gen1::ENCOUNTER_TABLE_SIZE as usize {
                            let encounter = &set.encounters[slot];
                            self.set_byte(offset, encounter.level as u8);
                            let internal = self.internal_of(encounter.pokemon);
                            self.set_byte(offset + 1, internal);
                            offset += 2;
                        }
                    }
                }
            }
            table_offset += 2;
        }
        let old_rod = self.value("OldRodOffset");
        let old_rod_set = next_set()?;
        let encounter = &old_rod_set.encounters[0];
        self.set_byte(old_rod + 2, encounter.level as u8);
        let internal = self.internal_of(encounter.pokemon);
        self.set_byte(old_rod + 1, internal);
        let good_rod = self.value("GoodRodOffset");
        let good_rod_set = next_set()?;
        for slot in 0..2 {
            let encounter = &good_rod_set.encounters[slot as usize];
            self.set_byte(good_rod + slot * 2, encounter.level as u8);
            let internal = self.internal_of(encounter.pokemon);
            self.set_byte(good_rod + slot * 2 + 1, internal);
        }
        let mut super_rod = self.value("SuperRodTableOffset");
        if self.entry.is_yellow {
            while self.byte(super_rod) != 0xFF {
                super_rod += 1;
                let set = next_set()?;
                for slot in 0..gen1::YELLOW_SUPER_ROD_TABLE_SIZE as usize {
                    let encounter = &set.encounters[slot];
                    self.set_byte(super_rod + 1, encounter.level as u8);
                    let internal = self.internal_of(encounter.pokemon);
                    self.set_byte(super_rod, internal);
                    super_rod += 2;
                }
            }
        } else {
            let super_rod_bank = bank_of(super_rod);
            let mut used: Vec<i32> = Vec::new();
            while self.byte(super_rod) != 0xFF {
                super_rod += 1;
                let mut set_offset = calculate_offset(super_rod_bank, self.word(super_rod));
                super_rod += 2;
                if !used.contains(&set_offset) {
                    used.push(set_offset);
                    let count = self.byte(set_offset);
                    set_offset += 1;
                    let set = next_set()?;
                    for slot in 0..count as usize {
                        let encounter = &set.encounters[slot];
                        self.set_byte(set_offset, encounter.level as u8);
                        let internal = self.internal_of(encounter.pokemon);
                        self.set_byte(set_offset + 1, internal);
                        set_offset += 2;
                    }
                }
            }
        }
        Ok(())
    }

    fn has_wild_alt_formes(&self) -> bool {
        false
    }

    fn enable_guaranteed_pokemon_catching(&mut self) {
        let mut offset = self.find_unique(gen1::GUARANTEED_CATCH_PREFIX);
        if offset > 0 {
            offset += gen1::GUARANTEED_CATCH_PREFIX.len() as i32 / 2;
            if self.byte(offset) == 0xCA {
                self.set_byte(offset, 0xC3);
            }
        }
    }

    fn get_trainers(&self) -> Result<Vec<Trainer>> {
        let pointers = self.trainer_pointers();
        let class_limits = self.entry.array("TrainerDataClassCounts");
        let class_names = self.trainer_class_names_for_text();
        let mut trainers = Vec::new();
        let mut index = 0;
        for class in 1..=gen1::TRAINER_CLASS_COUNT {
            let mut offset = pointers[class as usize];
            let limit = class_limits[class as usize];
            let class_name = class_names[(class - 1) as usize].clone();
            for _ in 0..limit {
                index += 1;
                let mut trainer = Trainer::new();
                trainer.offset = offset;
                trainer.index = index;
                trainer.trainerclass = class;
                trainer.full_display_name = Some(class_name.clone());
                let data_type = self.byte(offset);
                offset += 1;
                if data_type == 0xFF {
                    trainer.poketype = 1;
                    while self.byte(offset) != 0 {
                        let pokemon = self.loaded_pokemon_by_internal(self.byte(offset + 1))?;
                        trainer.pokemon.push(randomon_data::TrainerPokemon::new(
                            pokemon,
                            i32::from(self.byte(offset)),
                        ));
                        offset += 2;
                    }
                } else {
                    trainer.poketype = 0;
                    while self.byte(offset) != 0 {
                        let pokemon = self.loaded_pokemon_by_internal(self.byte(offset))?;
                        trainer.pokemon.push(randomon_data::TrainerPokemon::new(
                            pokemon,
                            i32::from(data_type),
                        ));
                        offset += 1;
                    }
                }
                offset += 1;
                trainers.push(trainer);
            }
        }
        gen1::tag_trainers_universal(&mut trainers);
        if self.entry.is_yellow {
            gen1::tag_trainers_yellow(&mut trainers);
        } else {
            gen1::tag_trainers_red_blue(&mut trainers);
        }
        Ok(trainers)
    }

    fn get_main_playthrough_trainers(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_elite_four_trainers(&self, _is_challenge_mode: bool) -> Vec<i32> {
        Vec::new()
    }

    fn set_trainers(&mut self, trainers: &[Trainer], _double_battle_mode: bool) -> Result<()> {
        let pointers = self.trainer_pointers();
        let class_limits = self.entry.array("TrainerDataClassCounts").to_vec();
        let mut iterator = trainers.iter();
        for class in 1..=gen1::TRAINER_CLASS_COUNT {
            let mut offset = pointers[class as usize];
            for _ in 0..class_limits[class as usize] {
                let trainer = iterator
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("trainers"))?;
                if trainer.poketype == 0 {
                    let fixed_level = trainer
                        .pokemon
                        .first()
                        .ok_or(RandomizationError::IndexOutOfBounds("trainer pokemon"))?
                        .level;
                    self.set_byte(offset, fixed_level as u8);
                    offset += 1;
                    for tp in &trainer.pokemon {
                        let internal = self.internal_of(tp.pokemon);
                        self.set_byte(offset, internal);
                        offset += 1;
                    }
                } else {
                    self.set_byte(offset, 0xFF);
                    offset += 1;
                    for tp in &trainer.pokemon {
                        self.set_byte(offset, tp.level as u8);
                        let internal = self.internal_of(tp.pokemon);
                        self.set_byte(offset + 1, internal);
                        offset += 2;
                    }
                }
                self.set_byte(offset, 0);
                offset += 1;
            }
        }
        let extra_moves_table = self.value("ExtraTrainerMovesTableOffset");
        self.set_byte(extra_moves_table, 0xFF);
        if !self.is_yellow() {
            let champ_rival_jump = self.value("GymLeaderMovesTableOffset")
                - gen1::CHAMP_RIVAL_OFFSET_FROM_GYM_LEADER_MOVES;
            self.set_byte(champ_rival_jump, gb::Z80_NOP);
            self.set_byte(champ_rival_jump + 1, gb::Z80_NOP);
        }
        Ok(())
    }

    fn has_rival_final_battle(&self) -> bool {
        true
    }

    fn has_physical_special_split(&self) -> bool {
        false
    }

    fn get_moves_learnt(&self) -> Movesets {
        let mut movesets = Movesets::new();
        let pointers_offset = self.value("PokemonMovesetsTableOffset");
        let count = self.value("InternalPokemonCount");
        for internal in 1..=count {
            let pointer = self.word(pointers_offset + (internal - 1) * 2);
            let mut real_pointer = calculate_offset(bank_of(pointers_offset), pointer);
            let number = self.poke_rby_to_num[internal as usize];
            if number == 0 {
                continue;
            }
            let stats_offset = if number == species::MEW && !self.entry.is_yellow {
                self.value("MewStatsOffset")
            } else {
                (number - 1) * 0x1C + self.value("PokemonStatsOffset")
            };
            let mut moves = Vec::new();
            for delta in gen1::BS_LEVEL1_MOVES_OFFSET..gen1::BS_LEVEL1_MOVES_OFFSET + 4 {
                let value = self.byte(stats_offset + delta);
                if value != 0 {
                    moves.push(MoveLearnt {
                        level: 1,
                        move_number: self.move_rom_to_num[value as usize],
                    });
                }
            }
            while self.byte(real_pointer) != 0 {
                real_pointer += match self.byte(real_pointer) {
                    2 => 4,
                    _ => 3,
                };
            }
            real_pointer += 1;
            while self.byte(real_pointer) != 0 {
                moves.push(MoveLearnt {
                    level: i32::from(self.byte(real_pointer)),
                    move_number: self.move_rom_to_num[self.byte(real_pointer + 1) as usize],
                });
                real_pointer += 2;
            }
            movesets.insert(number, moves);
        }
        movesets
    }

    fn set_moves_learnt(&mut self, movesets: &Movesets) -> Result<()> {
        self.write_evos_and_moves_learnt(false, Some(movesets))
    }

    fn get_moves_banned_from_levelup(&self) -> Vec<i32> {
        gen1::BANNED_LEVELUP_MOVES.to_vec()
    }

    fn get_egg_moves(&self) -> EggMoves {
        EggMoves::new()
    }

    fn set_egg_moves(&mut self, _egg_moves: &EggMoves) -> Result<()> {
        Ok(())
    }

    fn supports_four_starting_moves(&self) -> bool {
        true
    }

    fn get_static_pokemon(&self) -> Vec<StaticEncounter> {
        if self.value("StaticPokemonSupport") <= 0 {
            return Vec::new();
        }
        self.entry
            .static_pokemon
            .iter()
            .map(|entry| {
                let mut encounter = StaticEncounter::new(
                    self.pokemon_by_internal(self.byte(entry.species_offsets[0])),
                );
                encounter.level = match entry.level_offsets.first() {
                    Some(&offset) => self.signed_byte(offset),
                    None => 1,
                };
                encounter
            })
            .collect()
    }

    fn set_static_pokemon(&mut self, statics: &[StaticEncounter]) -> Result<bool> {
        if self.value("StaticPokemonSupport") == 0 {
            return Ok(false);
        }
        for (index, entry) in self.entry.static_pokemon.iter().enumerate() {
            let encounter = statics
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds("static pokemon"))?;
            let internal = self.internal_of(encounter.pkmn);
            for &offset in &entry.species_offsets {
                self.rom[offset as usize] = internal;
            }
            let level_offset = *entry
                .level_offsets
                .first()
                .ok_or(RandomizationError::IndexOutOfBounds("static level offsets"))?;
            self.rom[level_offset as usize] = encounter.level as u8;
        }
        Ok(true)
    }

    fn can_change_static_pokemon(&self) -> bool {
        self.value("StaticPokemonSupport") > 0
    }

    fn has_static_alt_formes(&self) -> bool {
        false
    }

    fn has_main_game_legendaries(&self) -> bool {
        false
    }

    fn get_main_game_legendaries(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_special_music_statics(&self) -> Vec<i32> {
        Vec::new()
    }

    fn apply_correct_static_music(&mut self, _changes: &BTreeMap<i32, i32>) {}

    fn has_static_music_fix(&self) -> bool {
        false
    }

    fn get_totem_pokemon(&self) -> Vec<TotemPokemon> {
        Vec::new()
    }

    fn set_totem_pokemon(&mut self, _totems: &[TotemPokemon]) -> Result<()> {
        Ok(())
    }

    fn get_tm_moves(&self) -> Vec<i32> {
        let offset = self.value("TMMovesOffset");
        (0..gen1::TM_COUNT)
            .map(|index| self.move_rom_to_num[self.byte(offset + index) as usize])
            .collect()
    }

    fn get_hm_moves(&self) -> Vec<i32> {
        let offset = self.value("TMMovesOffset");
        (0..gen1::HM_COUNT)
            .map(|index| self.move_rom_to_num[self.byte(offset + gen1::TM_COUNT + index) as usize])
            .collect()
    }

    fn set_tm_moves(&mut self, moves: &[i32]) -> Result<()> {
        let offset = self.value("TMMovesOffset");
        for index in 0..gen1::TM_COUNT {
            let value = self.move_num_to_rom[moves[index as usize] as usize] as u8;
            self.set_byte(offset + index, value);
        }
        if !self.entry.is_yellow {
            let gym_leader_moves = self.value("GymLeaderMovesTableOffset");
            for (index, &tm) in gen1::GYM_LEADER_TMS.iter().enumerate() {
                let value = self.move_num_to_rom[moves[(tm - 1) as usize] as usize] as u8;
                self.set_byte(gym_leader_moves + index as i32 * 2, value);
            }
        }
        let names = self.read_move_names();
        for text in &self.entry.tm_texts {
            let move_name =
                &names[self.move_num_to_rom[moves[(text.number - 1) as usize] as usize] as usize];
            let rendered = text.template.replace("%m", move_name);
            self.write_variable_string(&rendered, text.offset, true);
        }
        Ok(())
    }

    fn get_tm_count(&self) -> i32 {
        gen1::TM_COUNT
    }

    fn get_hm_count(&self) -> i32 {
        gen1::HM_COUNT
    }

    fn get_tmhm_compatibility(&self) -> Compatibility {
        let mut compat = Compatibility::new();
        let flag_count = (gen1::TM_COUNT + gen1::HM_COUNT + 1) as usize;
        for number in 1..=self.pokedex_count {
            let offset = self.base_stats_offset(number);
            let mut flags = vec![false; flag_count];
            for byte_index in 0..7 {
                let value = self.byte(offset + gen1::BS_TMHM_COMPAT_OFFSET + byte_index);
                for bit in 0..8 {
                    let slot = (byte_index * 8 + 1 + bit) as usize;
                    if slot < flag_count {
                        flags[slot] = (value >> bit) & 1 == 1;
                    }
                }
            }
            compat.insert(PokemonId::from_index(number as usize), flags);
        }
        compat
    }

    fn set_tmhm_compatibility(&mut self, compatibility: &Compatibility) {
        for (&pokemon, flags) in compatibility {
            let offset = self.base_stats_offset(self.core.data.pk(pokemon).number);
            for byte_index in 0..7 {
                let mut value = 0u8;
                for bit in 0..8 {
                    let slot = (byte_index * 8 + 1 + bit) as usize;
                    if slot < flags.len() && flags[slot] {
                        value |= 1 << bit;
                    }
                }
                self.set_byte(offset + gen1::BS_TMHM_COMPAT_OFFSET + byte_index, value);
            }
        }
    }

    fn has_move_tutors(&self) -> bool {
        false
    }

    fn get_move_tutor_moves(&self) -> Vec<i32> {
        Vec::new()
    }

    fn set_move_tutor_moves(&mut self, _moves: &[i32]) -> Result<()> {
        Ok(())
    }

    fn get_move_tutor_compatibility(&self) -> Compatibility {
        Compatibility::new()
    }

    fn set_move_tutor_compatibility(&mut self, _compatibility: &Compatibility) {}

    fn can_change_trainer_text(&self) -> bool {
        self.value("CanChangeTrainerText") > 0
    }

    fn get_trainer_names(&self) -> Vec<String> {
        let offsets = self.entry.array("TrainerClassNamesOffsets");
        let mut offset = offsets[offsets.len() - 1];
        let mut names = Vec::new();
        for class in 0..gen1::TCLASSES_COUNTS[1] {
            let name = self.read_variable_string(offset);
            offset += length_of_string_at(&self.rom, offset, false) + 1;
            if gen1::SINGULAR_TRAINERS.contains(&class) {
                names.push(name);
            }
        }
        names
    }

    fn set_trainer_names(&mut self, names: &[String]) {
        if self.value("CanChangeTrainerText") <= 0 {
            return;
        }
        let offsets = self.entry.array("TrainerClassNamesOffsets").to_vec();
        let mut iterator = names.iter();
        let mut offset = offsets[offsets.len() - 1];
        for class in 0..gen1::TCLASSES_COUNTS[1] {
            let old_length = length_of_string_at(&self.rom, offset, false) + 1;
            if gen1::SINGULAR_TRAINERS.contains(&class)
                && let Some(name) = iterator.next()
            {
                self.write_fixed_string(name, offset, old_length);
            }
            offset += old_length;
        }
    }

    fn trainer_name_mode(&self) -> TrainerNameMode {
        TrainerNameMode::SameLength
    }

    fn get_tc_name_lengths_by_trainer(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_trainer_class_names(&self) -> Vec<String> {
        let offsets = self.entry.array("TrainerClassNamesOffsets");
        let mut names = Vec::new();
        if offsets.len() == 2 {
            for (group, &start) in offsets.iter().enumerate() {
                let mut offset = start;
                for class in 0..gen1::TCLASSES_COUNTS[group] {
                    let name = self.read_variable_string(offset);
                    offset += length_of_string_at(&self.rom, offset, false) + 1;
                    if group == 0 || !gen1::SINGULAR_TRAINERS.contains(&class) {
                        names.push(name);
                    }
                }
            }
        } else {
            let mut offset = offsets[0];
            for class in 0..gen1::TCLASSES_COUNTS[1] {
                let name = self.read_variable_string(offset);
                offset += length_of_string_at(&self.rom, offset, false) + 1;
                if !gen1::SINGULAR_TRAINERS.contains(&class) {
                    names.push(name);
                }
            }
        }
        names
    }

    fn set_trainer_class_names(&mut self, names: &[String]) {
        if self.value("CanChangeTrainerText") <= 0 {
            return;
        }
        let offsets = self.entry.array("TrainerClassNamesOffsets").to_vec();
        let mut iterator = names.iter();
        if offsets.len() == 2 {
            for (group, &start) in offsets.iter().enumerate() {
                let mut offset = start;
                for class in 0..gen1::TCLASSES_COUNTS[group] {
                    let old_length = length_of_string_at(&self.rom, offset, false) + 1;
                    if (group == 0 || !gen1::SINGULAR_TRAINERS.contains(&class))
                        && let Some(name) = iterator.next()
                    {
                        self.write_fixed_string(name, offset, old_length);
                    }
                    offset += old_length;
                }
            }
        } else {
            let mut offset = offsets[0];
            for class in 0..gen1::TCLASSES_COUNTS[1] {
                let old_length = length_of_string_at(&self.rom, offset, false) + 1;
                if !gen1::SINGULAR_TRAINERS.contains(&class)
                    && let Some(name) = iterator.next()
                {
                    self.write_fixed_string(name, offset, old_length);
                }
                offset += old_length;
            }
        }
    }

    fn fixed_trainer_class_names_length(&self) -> bool {
        true
    }

    fn get_doubles_trainer_classes(&self) -> Vec<i32> {
        Vec::new()
    }

    fn allowed_items_mut(&mut self) -> &mut ItemList {
        &mut self.allowed_items
    }

    fn non_bad_items_mut(&mut self) -> &mut ItemList {
        &mut self.allowed_items
    }

    fn get_evolution_items(&self) -> Option<Vec<i32>> {
        None
    }

    fn get_unique_no_sell_items(&self) -> Vec<i32> {
        Vec::new()
    }

    fn get_regular_shop_items(&self) -> Option<Vec<i32>> {
        None
    }

    fn get_op_shop_items(&self) -> Option<Vec<i32>> {
        None
    }

    fn get_item_names(&self) -> Vec<String> {
        self.item_names.clone()
    }

    fn get_required_field_tms(&self) -> Vec<i32> {
        gen1::REQUIRED_FIELD_TMS.to_vec()
    }

    fn get_current_field_tms(&self) -> Vec<i32> {
        let allowed = gen1::allowed_items();
        self.item_offsets()
            .into_iter()
            .map(|offset| i32::from(self.byte(offset)))
            .filter(|&item| allowed.is_tm(item))
            .map(|item| item - gen1::TMS_START_INDEX + 1)
            .collect()
    }

    fn set_field_tms(&mut self, tms: &[i32]) -> Result<()> {
        let allowed = gen1::allowed_items();
        let mut iterator = tms.iter();
        for offset in self.item_offsets() {
            if allowed.is_tm(i32::from(self.byte(offset))) {
                let tm = iterator
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("field TMs"))?;
                self.set_byte(offset, (tm + gen1::TMS_START_INDEX - 1) as u8);
            }
        }
        Ok(())
    }

    fn get_regular_field_items(&self) -> Vec<i32> {
        let allowed = gen1::allowed_items();
        self.item_offsets()
            .into_iter()
            .map(|offset| i32::from(self.byte(offset)))
            .filter(|&item| allowed.is_allowed(item) && !allowed.is_tm(item))
            .collect()
    }

    fn set_regular_field_items(&mut self, items: &[i32]) -> Result<()> {
        let allowed = gen1::allowed_items();
        let mut iterator = items.iter();
        for offset in self.item_offsets() {
            let here = i32::from(self.byte(offset));
            if allowed.is_allowed(here) && !allowed.is_tm(here) {
                let item = iterator
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("field items"))?;
                self.set_byte(offset, *item as u8);
            }
        }
        Ok(())
    }

    fn has_shop_randomization(&self) -> bool {
        false
    }

    fn get_shop_items(&self) -> Option<BTreeMap<i32, Shop>> {
        None
    }

    fn set_shop_items(&mut self, _shops: &BTreeMap<i32, Shop>) -> Result<()> {
        Ok(())
    }

    fn set_shop_prices(&mut self) {}

    fn get_ingame_trades(&self) -> Vec<IngameTrade> {
        let (table, size, name_length, unused) = self.trade_layout();
        let entry_length = name_length + 3;
        let mut unused_index = 0;
        let mut trades = Vec::new();
        for entry in 0..size {
            if unused_index < unused.len() && unused[unused_index] == entry {
                unused_index += 1;
                continue;
            }
            let entry_offset = table + entry * entry_length;
            trades.push(IngameTrade {
                requested_pokemon: Some(self.pokemon_by_internal(self.byte(entry_offset))),
                given_pokemon: self.pokemon_by_internal(self.byte(entry_offset + 1)),
                nickname: self.tables.read_string(
                    &self.rom,
                    entry_offset + 3,
                    name_length as usize,
                    false,
                ),
                ..IngameTrade::default()
            });
        }
        trades
    }

    fn set_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        let (table, size, name_length, unused) = self.trade_layout();
        let entry_length = name_length + 3;
        let mut unused_index = 0;
        let mut trade_index = 0;
        for entry in 0..size {
            if unused_index < unused.len() && unused[unused_index] == entry {
                unused_index += 1;
                continue;
            }
            let trade = trades
                .get(trade_index)
                .ok_or(RandomizationError::IndexOutOfBounds("trades"))?;
            trade_index += 1;
            let entry_offset = table + entry * entry_length;
            let requested = trade
                .requested_pokemon
                .ok_or(RandomizationError::NullPointer("requestedPokemon"))?;
            let requested_internal = self.internal_of(requested);
            let given_internal = self.internal_of(trade.given_pokemon);
            self.set_byte(entry_offset, requested_internal);
            self.set_byte(entry_offset + 1, given_internal);
            if self.value("CanChangeTrainerText") > 0 {
                self.write_fixed_string(&trade.nickname, entry_offset + 3, name_length);
            }
        }
        Ok(())
    }

    fn has_dvs(&self) -> bool {
        true
    }

    fn remove_impossible_evolutions(
        &mut self,
        _settings: &randomon_settings::Settings,
    ) -> Result<()> {
        let pokemon = self.pokemon_list.clone();
        let core = &mut self.core;
        for pk in pokemon.into_iter().flatten() {
            for evo in core.data.evolutions_from(pk).to_vec() {
                if core.data.evo(evo).kind == EvolutionType::Trade {
                    let evolution = core.data.evo_mut(evo);
                    evolution.kind = EvolutionType::Level;
                    evolution.extra_info = 37;
                    randomon_core::engine::evolutions::add_evo_update_level(
                        &core.data,
                        &mut core.engine.impossible_evolution_updates,
                        evo,
                    );
                }
            }
        }
        Ok(())
    }

    fn make_evolutions_easier(&mut self, _settings: &randomon_settings::Settings) -> Result<()> {
        Ok(())
    }

    fn remove_time_based_evolutions(&mut self) -> Result<()> {
        Ok(())
    }

    fn get_field_moves(&self) -> Vec<i32> {
        gen1::FIELD_MOVES.to_vec()
    }

    fn get_early_required_hm_moves(&self) -> Vec<i32> {
        gen1::EARLY_REQUIRED_HMS.to_vec()
    }

    fn get_game_breaking_moves(&self) -> Vec<i32> {
        if self.x_acc_nerfed {
            gen1::BANNED_MOVES_WITH_X_ACC_BANNED.to_vec()
        } else {
            gen1::BANNED_MOVES_WITHOUT_X_ACC_BANNED.to_vec()
        }
    }

    fn is_yellow(&self) -> bool {
        self.entry.is_yellow
    }

    fn rom_name(&self) -> String {
        format!("Pokemon {}", self.entry.name)
    }

    fn rom_code(&self) -> String {
        format!(
            "{} ({}/{})",
            self.entry.rom_name.as_deref().unwrap_or("null"),
            self.entry.version,
            self.entry.non_japanese
        )
    }

    fn support_level(&self) -> String {
        if self.value("StaticPokemonSupport") > 0 {
            "Complete".to_owned()
        } else {
            "No Static Pokemon".to_owned()
        }
    }

    fn default_extension(&self) -> String {
        "gbc".to_owned()
    }

    fn internal_string_length(&self, text: &str) -> i32 {
        self.tables.translate(text).len() as i32
    }

    fn randomize_intro_pokemon(&mut self) -> Result<()> {
        let intro = self.random_pokemon_internal()?;
        let pokemon_offset = self.value("IntroPokemonOffset");
        let cry_offset = self.value("IntroCryOffset");
        self.set_byte(pokemon_offset, intro);
        self.set_byte(cry_offset, intro);
        Ok(())
    }

    fn generation_of_pokemon(&self) -> i32 {
        1
    }

    fn misc_tweaks_available(&self) -> i32 {
        let mut available =
            MiscTweak::LowerCasePokemonNames.value() | MiscTweak::UpdateTypeEffectiveness.value();
        if self.entry.tweak_files.contains_key("BWXPTweak") {
            available |= MiscTweak::BwExpPatch.value();
        }
        if self.entry.tweak_files.contains_key("XAccNerfTweak") {
            available |= MiscTweak::NerfXAccuracy.value();
        }
        if self.entry.tweak_files.contains_key("CritRateTweak") {
            available |= MiscTweak::FixCritRate.value();
        }
        if self.value("TextDelayFunctionOffset") != 0 {
            available |= MiscTweak::FastestText.value();
        }
        if self.value("PCPotionOffset") != 0 {
            available |= MiscTweak::RandomizePcPotion.value();
        }
        if self.value("PikachuEvoJumpOffset") != 0 {
            available |= MiscTweak::AllowPikachuEvolution.value();
        }
        if self.value("CatchingTutorialMonOffset") != 0 {
            available |= MiscTweak::RandomizeCatchingTutorial.value();
        }
        available
    }

    fn apply_misc_tweak(&mut self, tweak: MiscTweak) -> Result<()> {
        match tweak {
            MiscTweak::BwExpPatch => {
                self.generic_ips_patch("BWXPTweak")?;
            }
            MiscTweak::NerfXAccuracy => {
                self.x_acc_nerfed = self.generic_ips_patch("XAccNerfTweak")?
            }
            MiscTweak::FixCritRate => {
                self.generic_ips_patch("CritRateTweak")?;
            }
            MiscTweak::FastestText => {
                let offset = self.value("TextDelayFunctionOffset");
                if offset != 0 {
                    self.set_byte(offset, gb::Z80_RET);
                }
            }
            MiscTweak::RandomizePcPotion => {
                let offset = self.value("PCPotionOffset");
                if offset != 0 {
                    let mut random = self.core.engine.random.clone();
                    let item = self.allowed_items.random_non_tm(&mut random);
                    self.set_byte(offset, item as u8);
                }
            }
            MiscTweak::AllowPikachuEvolution => {
                let offset = self.value("PikachuEvoJumpOffset");
                if offset != 0 {
                    self.set_byte(offset, gb::Z80_JUMP_RELATIVE);
                }
            }
            MiscTweak::LowerCasePokemonNames => {
                for pokemon in self.pokemon_list.clone().into_iter().flatten() {
                    let name = randomon_data::text::camel_case(&self.core.data.pk(pokemon).name);
                    self.core.data.pk_mut(pokemon).name = name;
                }
            }
            MiscTweak::UpdateTypeEffectiveness => self.update_type_effectiveness(),
            MiscTweak::RandomizeCatchingTutorial => {
                let offset = self.value("CatchingTutorialMonOffset");
                if offset != 0 {
                    let internal = self.random_pokemon_internal()?;
                    self.set_byte(offset, internal);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn is_effectiveness_updated(&self) -> bool {
        self.effectiveness_updated
    }

    fn has_functional_formes(&self) -> bool {
        false
    }
}
