use randomon_core::{HandlerCore, RandomizationError, Result};
use randomon_data::constants::gb;
use randomon_data::{
    Evolution, EvolutionType, ExpCurve, ItemList, Move, PokemonId, StatLayout, Type,
};
use randomon_gb::{
    HeaderIds, TextTables, bank_of, calculate_offset, crc32, length_of_string_at, read_word,
    rom_code_matches, write_word,
};
use randomon_java::SharedRandomSource;

use crate::assets::table_text;
use crate::constants::{self as c, POKEMON_COUNT};
use crate::items;
use crate::maps::{MapData, load_maps};
use crate::moves::{
    apply_misc_info_from_effect, apply_stat_changes_from_effect, apply_status_from_effect,
    hit_count, initial_category, priority,
};
use crate::rom_entry::{RomEntry, rom_entries};

pub struct Gen2RomHandler {
    pub(crate) core: HandlerCore,
    pub(crate) rom: Vec<u8>,
    pub(crate) original_rom: Vec<u8>,
    pub(crate) file_name: String,
    pub(crate) entry: &'static RomEntry,
    pub(crate) tables: TextTables,
    pub(crate) pokemon_list: Vec<Option<PokemonId>>,
    pub(crate) item_names: Vec<String>,
    pub(crate) maps: MapData,
    pub(crate) is_viet_crystal: bool,
    pub(crate) have_patched_fleeing: bool,
    pub(crate) allowed_items: ItemList,
    pub(crate) non_bad_items: ItemList,
    pub(crate) actual_crc32: i64,
    pub(crate) effectiveness_updated: bool,
}

pub fn check_rom_entry(rom: &[u8]) -> Option<&'static RomEntry> {
    let HeaderIds {
        version,
        non_japanese,
        crc_in_header,
    } = HeaderIds::read(rom)?;
    let matches = |entry: &RomEntry| {
        entry
            .rom_code
            .as_deref()
            .is_some_and(|code| rom_code_matches(rom, code))
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

fn is_viet_crystal(rom: &[u8], entry: &RomEntry) -> bool {
    entry.name == "Crystal (J)"
        && rom.get(c::VIET_CRYSTAL_CHECK_OFFSET) == Some(&c::VIET_CRYSTAL_CHECK_VALUE)
}

fn load_text_tables(rom: &[u8], entry: &RomEntry) -> TextTables {
    let mut tables = TextTables::default();
    tables.read_table(randomon_gb::GAMEBOY_JPN_TABLE);
    if let Some(extra) = entry.extra_table_file.as_deref()
        && !extra.eq_ignore_ascii_case("none")
        && let Some(text) = table_text(extra)
    {
        tables.read_table(text);
    }
    if is_viet_crystal(rom, entry)
        && let Some(text) = table_text("vietcrystal")
    {
        tables.read_table(text);
    }
    tables
}

pub(crate) fn pokemon(number: i32) -> PokemonId {
    PokemonId::from_index(number.max(0) as usize)
}

impl Gen2RomHandler {
    pub fn load(rom: Vec<u8>, file_name: &str, random_source: &SharedRandomSource) -> Result<Self> {
        if !detect_rom(&rom) {
            return Err(RandomizationError::UnsupportedRom(file_name.to_owned()));
        }
        let entry = check_rom_entry(&rom)
            .ok_or_else(|| RandomizationError::UnsupportedRom(file_name.to_owned()))?;
        let tables = load_text_tables(&rom, entry);
        let viet_crystal = is_viet_crystal(&rom, entry);
        let maps = load_maps(&rom, entry, &tables);
        let mut allowed_items = c::allowed_items();
        if viet_crystal {
            allowed_items.ban_singles(&[
                items::BURN_HEAL,
                items::CALCIUM,
                items::ELIXER,
                items::TWISTED_SPOON,
            ]);
        }
        let mut handler = Self {
            core: HandlerCore::new(random_source),
            original_rom: rom.clone(),
            actual_crc32: crc32(&rom),
            rom,
            file_name: file_name.to_owned(),
            entry,
            tables,
            pokemon_list: Vec::new(),
            item_names: Vec::new(),
            maps,
            is_viet_crystal: viet_crystal,
            have_patched_fleeing: false,
            allowed_items,
            non_bad_items: c::non_bad_items(),
            effectiveness_updated: false,
        };
        handler.load_pokemon_stats()?;
        handler.load_moves();
        handler.load_item_names();
        Ok(handler)
    }

    pub(crate) fn value(&self, key: &str) -> i32 {
        self.entry.value(key)
    }

    pub(crate) fn array(&self, key: &str) -> &'static [i32] {
        self.entry.array(key)
    }

    pub(crate) fn byte(&self, offset: i32) -> u8 {
        self.rom[offset as usize]
    }

    pub(crate) fn int(&self, offset: i32) -> i32 {
        i32::from(self.byte(offset))
    }

    pub(crate) fn signed_byte(&self, offset: i32) -> i32 {
        i32::from(self.byte(offset) as i8)
    }

    pub(crate) fn set_byte(&mut self, offset: i32, value: u8) {
        self.rom[offset as usize] = value;
    }

    pub(crate) fn word(&self, offset: i32) -> i32 {
        read_word(&self.rom, offset)
    }

    pub(crate) fn set_word(&mut self, offset: i32, value: i32) {
        write_word(&mut self.rom, offset, value);
    }

    pub(crate) fn pokemon_at(&self, offset: i32) -> PokemonId {
        pokemon(self.int(offset))
    }

    pub(crate) fn number_of(&self, id: PokemonId) -> u8 {
        self.core.data.pk(id).number as u8
    }

    pub(crate) fn read_string(&self, offset: i32, length: usize) -> String {
        self.tables.read_string(&self.rom, offset, length, false)
    }

    pub(crate) fn read_variable_string(&self, offset: i32) -> String {
        self.read_string(offset, usize::MAX)
    }

    pub(crate) fn string_length_at(&self, offset: i32) -> i32 {
        length_of_string_at(&self.rom, offset, false)
    }

    pub(crate) fn translate(&self, text: &str) -> Vec<u8> {
        self.tables.translate(text)
    }

    pub(crate) fn write_bytes(&mut self, offset: i32, bytes: &[u8]) {
        let start = offset as usize;
        self.rom[start..start + bytes.len()].copy_from_slice(bytes);
    }

    pub(crate) fn write_fixed_string(&mut self, text: &str, offset: i32, length: i32) {
        let translated = self.translate(text);
        let length = length.max(0) as usize;
        let copied = translated.len().min(length);
        let start = offset as usize;
        self.rom[start..start + copied].copy_from_slice(&translated[..copied]);
        self.rom[start + copied..start + length].fill(gb::STRING_TERMINATOR);
    }

    pub(crate) fn write_variable_string(
        &mut self,
        text: &str,
        offset: i32,
        already_terminated: bool,
    ) {
        let translated = self.translate(text);
        self.write_bytes(offset, &translated);
        if !already_terminated {
            self.set_byte(offset + translated.len() as i32, gb::STRING_TERMINATOR);
        }
    }

    pub(crate) fn find_unique(&self, hex: &str) -> Result<i32> {
        if hex.is_empty() {
            return Err(RandomizationError::IndexOutOfBounds("buildKMPSearchTable"));
        }
        Ok(randomon_gb::find_unique_hex(&self.rom, hex))
    }

    fn read_pokemon_names(&self) -> Vec<String> {
        let offset = self.value("PokemonNamesOffset");
        let length = self.value("PokemonNamesLength");
        (0..=POKEMON_COUNT)
            .map(|number| {
                if number == 0 {
                    String::new()
                } else {
                    self.read_string(offset + (number - 1) * length, length as usize)
                }
            })
            .collect()
    }

    pub(crate) fn stats_offset(&self, number: i32) -> i32 {
        self.value("PokemonStatsOffset") + (number - 1) * c::BASE_STATS_ENTRY_SIZE
    }

    fn load_pokemon_stats(&mut self) -> Result<()> {
        let names = self.read_pokemon_names();
        self.pokemon_list = vec![None];
        for number in 1..=POKEMON_COUNT {
            let id = self.core.data.create_pokemon(StatLayout::Standard, number);
            self.pokemon_list.push(Some(id));
            self.load_basic_stats(id, self.stats_offset(number));
            self.core.data.pk_mut(id).name = names[number as usize].clone();
        }
        self.populate_evolutions()
    }

    fn load_basic_stats(&mut self, id: PokemonId, offset: i32) {
        let read = |delta: i32| i32::from(self.rom[(offset + delta) as usize]);
        let primary = c::type_table(read(c::BS_PRIMARY_TYPE_OFFSET));
        let mut secondary = c::type_table(read(c::BS_SECONDARY_TYPE_OFFSET));
        if secondary == primary {
            secondary = None;
        }
        let growth =
            ExpCurve::from_byte(self.rom[(offset + c::BS_GROWTH_CURVE_OFFSET) as usize] as i8);
        let pk = self.core.data.pk_mut(id);
        pk.hp = read(c::BS_HP_OFFSET);
        pk.attack = read(c::BS_ATTACK_OFFSET);
        pk.defense = read(c::BS_DEFENSE_OFFSET);
        pk.speed = read(c::BS_SPEED_OFFSET);
        pk.spatk = read(c::BS_SP_ATK_OFFSET);
        pk.spdef = read(c::BS_SP_DEF_OFFSET);
        pk.primary_type = primary;
        pk.secondary_type = secondary;
        pk.catch_rate = read(c::BS_CATCH_RATE_OFFSET);
        pk.guaranteed_held_item = -1;
        pk.common_held_item = read(c::BS_COMMON_HELD_ITEM_OFFSET);
        pk.rare_held_item = read(c::BS_RARE_HELD_ITEM_OFFSET);
        pk.dark_grass_held_item = -1;
        pk.growth_curve = growth;
        pk.pic_dimensions = read(c::BS_PIC_DIMENSIONS_OFFSET);
    }

    pub(crate) fn moveset_pointer(&self, number: i32) -> i32 {
        let table = self.value("PokemonMovesetsTableOffset");
        calculate_offset(bank_of(table), self.word(table + (number - 1) * 2))
    }

    fn read_evolution(&self, from: PokemonId, pointer: i32) -> Result<Evolution> {
        let method = self.int(pointer);
        let target = self.pokemon_at(pointer + 2 + i32::from(method == 5));
        let mut kind = EvolutionType::from_index(2, method).ok_or_else(|| {
            RandomizationError::randomization(format!("unknown Gen 2 evolution method {method}"))
        })?;
        let mut extra = 0;
        match kind {
            EvolutionType::Trade => {
                let item = self.int(pointer + 1);
                if item != 0xFF {
                    kind = EvolutionType::TradeItem;
                    extra = item;
                }
            }
            EvolutionType::LevelAttackHigher => {
                kind = match self.int(pointer + 2) {
                    2 => EvolutionType::LevelDefenseHigher,
                    3 => EvolutionType::LevelAtkDefSame,
                    _ => kind,
                };
                extra = self.int(pointer + 1);
            }
            EvolutionType::Happiness => {
                kind = match self.int(pointer + 1) {
                    2 => EvolutionType::HappinessDay,
                    3 => EvolutionType::HappinessNight,
                    _ => kind,
                };
            }
            _ => extra = self.int(pointer + 1),
        }
        Ok(Evolution::new(from, target, true, kind, extra))
    }

    fn populate_evolutions(&mut self) -> Result<()> {
        for number in 1..=POKEMON_COUNT {
            let mut pointer = self.moveset_pointer(number);
            let from = pokemon(number);
            while self.byte(pointer) != 0 {
                let method = self.int(pointer);
                let evolution = self.read_evolution(from, pointer)?;
                let target = evolution.to;
                let data = &mut self.core.data;
                if !data.evolution_list_contains(data.evolutions_from(from), &evolution) {
                    let id = data.new_evolution(evolution);
                    data.evolutions_from_mut(from).push(id);
                    if !data.exists(target) {
                        return Err(RandomizationError::NullPointer("pokes[otherPoke]"));
                    }
                    data.evolutions_to_mut(target).push(id);
                }
                pointer += if method == 5 { 4 } else { 3 };
            }
            let data = &mut self.core.data;
            if data.evolutions_from(from).len() > 1 {
                for evo in data.evolutions_from(from).to_vec() {
                    data.evo_mut(evo).carry_stats = false;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn read_move_names(&self) -> Vec<String> {
        let mut offset = self.value("MoveNamesOffset");
        let mut names = vec![String::new(); (c::MOVE_COUNT + 1) as usize];
        for name in names.iter_mut().skip(1) {
            *name = self.read_variable_string(offset);
            offset += self.string_length_at(offset) + 1;
        }
        names
    }

    fn load_moves(&mut self) {
        let names = self.read_move_names();
        let moves_offset = self.value("MoveDataOffset");
        for number in 1..=c::MOVE_COUNT {
            let base = moves_offset + (number - 1) * 7;
            let mut mv = Move {
                name: names[number as usize].clone(),
                number,
                internal_id: number,
                effect_index: self.int(base + 1),
                hitratio: f64::from(self.byte(base + 4)) / 255.0 * 100.0,
                power: self.int(base + 2),
                pp: self.int(base + 5),
                kind: c::type_table(self.signed_byte(base + 3)),
                ..Move::default()
            };
            mv.category = initial_category(&mv);
            if number == randomon_data::constants::moves::SWIFT {
                self.core.engine.perfect_accuracy = mv.hitratio as i32;
            }
            mv.hit_count = hit_count(number);
            mv.priority = priority(mv.effect_index);
            let secondary_chance = f64::from(self.byte(base + 6)) / 255.0 * 100.0;
            apply_stat_changes_from_effect(&mut mv, secondary_chance);
            apply_status_from_effect(&mut mv, secondary_chance);
            apply_misc_info_from_effect(&mut mv, secondary_chance);
            self.core.data.create_move(mv);
        }
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
            names[index % 256] = self.read_string(start, 20);
        }
        self.item_names = names;
    }

    pub(crate) fn save_pokemon_stats(&mut self) -> Result<()> {
        let names_offset = self.value("PokemonNamesOffset");
        let name_length = self.value("PokemonNamesLength");
        for number in 1..=POKEMON_COUNT {
            let name = self.core.data.pk(pokemon(number)).name.clone();
            self.write_fixed_string(
                &name,
                names_offset + (number - 1) * name_length,
                name_length,
            );
        }
        for number in 1..=POKEMON_COUNT {
            self.save_basic_stats(pokemon(number), self.stats_offset(number))?;
        }
        self.write_evos_and_moves_learnt(true, None)
    }

    fn save_basic_stats(&mut self, id: PokemonId, offset: i32) -> Result<()> {
        let pk = self.core.data.pk(id).clone();
        let primary = c::type_to_byte(pk.primary_type);
        let secondary = if pk.secondary_type.is_none() {
            primary
        } else {
            c::type_to_byte(pk.secondary_type)
        };
        let growth = pk
            .growth_curve
            .ok_or(RandomizationError::NullPointer("growthCurve"))?;
        self.set_byte(offset + c::BS_HP_OFFSET, pk.hp as u8);
        self.set_byte(offset + c::BS_ATTACK_OFFSET, pk.attack as u8);
        self.set_byte(offset + c::BS_DEFENSE_OFFSET, pk.defense as u8);
        self.set_byte(offset + c::BS_SPEED_OFFSET, pk.speed as u8);
        self.set_byte(offset + c::BS_SP_ATK_OFFSET, pk.spatk as u8);
        self.set_byte(offset + c::BS_SP_DEF_OFFSET, pk.spdef as u8);
        self.set_byte(offset + c::BS_PRIMARY_TYPE_OFFSET, primary);
        self.set_byte(offset + c::BS_SECONDARY_TYPE_OFFSET, secondary);
        self.set_byte(offset + c::BS_CATCH_RATE_OFFSET, pk.catch_rate as u8);
        self.set_byte(
            offset + c::BS_COMMON_HELD_ITEM_OFFSET,
            pk.common_held_item as u8,
        );
        self.set_byte(
            offset + c::BS_RARE_HELD_ITEM_OFFSET,
            pk.rare_held_item as u8,
        );
        self.set_byte(offset + c::BS_GROWTH_CURVE_OFFSET, growth.to_byte() as u8);
        Ok(())
    }

    pub(crate) fn save_moves(&mut self) {
        let moves_offset = self.value("MoveDataOffset");
        for number in 1..=c::MOVE_COUNT {
            let Some(mv) = self.core.data.move_by_number(number).cloned() else {
                continue;
            };
            let base = moves_offset + (number - 1) * 7;
            self.set_byte(base + 1, mv.effect_index as u8);
            self.set_byte(base + 2, mv.power as u8);
            self.set_byte(base + 3, c::type_to_byte(mv.kind));
            let hitratio =
                (randomon_java::math::round_double(mv.hitratio * 2.55) as i32).clamp(0, 255);
            self.set_byte(base + 4, hitratio as u8);
            self.set_byte(base + 5, mv.pp as u8);
        }
    }

    pub(crate) fn patch_fleeing(&mut self) {
        self.have_patched_fleeing = true;
        let offset = self.value("FleeingDataOffset");
        self.set_byte(offset, 0xFF);
        self.set_byte(offset + c::FLEEING_SET_TWO_OFFSET, 0xFF);
        self.set_byte(offset + c::FLEEING_SET_THREE_OFFSET, 0xFF);
    }

    pub(crate) fn ensure_fleeing_patched(&mut self) {
        if !self.have_patched_fleeing {
            self.patch_fleeing();
        }
    }

    pub(crate) fn type_relationships(&self) -> Vec<(Option<Type>, Option<Type>, u8)> {
        let mut table = Vec::new();
        let mut offset = self.value("TypeEffectivenessOffset");
        while self.byte(offset) != 0xFE {
            let attacking = c::type_table(self.signed_byte(offset));
            let defending = c::type_table(self.signed_byte(offset + 1));
            let effectiveness = self.signed_byte(offset + 2);
            if matches!(effectiveness, 20 | 10 | 5 | 0) {
                table.push((attacking, defending, effectiveness as u8));
            }
            offset += 3;
        }
        table
    }
}
