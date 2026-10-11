use std::collections::BTreeSet;

use randomon_core::Result;
use randomon_data::EncounterSet;
use randomon_data::constants::gen4;
use randomon_formats::nds::NarcArchive;

use crate::bytes::{i32_at, u8_at, u16_at};
use crate::handler::Gen4RomHandler;

const TIMES: usize = 3;
const MORNING: usize = 0;
const DAY: usize = 1;
const NIGHT: usize = 2;

#[derive(Clone)]
struct AreaTable {
    by_time: Vec<[BTreeSet<i32>; TIMES]>,
}

impl AreaTable {
    fn new() -> Self {
        Self {
            by_time: vec![Default::default(); gen4::POKEMON_COUNT as usize + 1],
        }
    }

    fn add(&mut self, species: i32, time: usize, area: i32) {
        if let Some(slots) = usize::try_from(species)
            .ok()
            .filter(|&species| species >= 1)
            .and_then(|species| self.by_time.get_mut(species))
        {
            slots[time].insert(area);
        }
    }

    fn add_all_times(&mut self, species: i32, area: i32) {
        for time in 0..TIMES {
            self.add(species, time, area);
        }
    }
}

#[derive(Clone)]
struct SpecialTable {
    areas: Vec<BTreeSet<i32>>,
}

impl SpecialTable {
    fn new() -> Self {
        Self {
            areas: vec![BTreeSet::new(); gen4::POKEMON_COUNT as usize + 1],
        }
    }

    fn add(&mut self, species: i32, area: i32) {
        if let Some(areas) = usize::try_from(species)
            .ok()
            .filter(|&species| species >= 1)
            .and_then(|species| self.areas.get_mut(species))
        {
            areas.insert(area);
        }
    }
}

fn area_data_file(areas: &BTreeSet<i32>) -> Vec<u8> {
    let mut output = Vec::with_capacity(areas.len() * 4 + 4);
    for &area in areas {
        output.extend_from_slice(&area.to_le_bytes());
    }
    output.extend_from_slice(&[0; 4]);
    output
}

fn dex_map(maps: &[i32], index: usize) -> i32 {
    maps.get(index).copied().unwrap_or(-1)
}

fn put_file(narc: &mut NarcArchive, index: i32, areas: &BTreeSet<i32>) {
    if let Some(slot) = usize::try_from(index)
        .ok()
        .and_then(|index| narc.files.get_mut(index))
    {
        *slot = area_data_file(areas);
    }
}

enum Target {
    Overworld,
    Dungeon,
}

fn classify(overworld: &[i32], dungeon: &[i32], index: usize) -> Option<(Target, i32)> {
    let overworld_index = dex_map(overworld, index);
    if overworld_index != -1 {
        return Some((Target::Overworld, overworld_index));
    }
    let dungeon_index = dex_map(dungeon, index);
    (dungeon_index != -1).then_some((Target::Dungeon, dungeon_index))
}

struct DpptAreaData {
    dungeon: AreaTable,
    overworld: AreaTable,
    dungeon_pre: SpecialTable,
    dungeon_post: SpecialTable,
    overworld_pre: SpecialTable,
    overworld_post: SpecialTable,
}

impl DpptAreaData {
    fn new() -> Self {
        Self {
            dungeon: AreaTable::new(),
            overworld: AreaTable::new(),
            dungeon_pre: SpecialTable::new(),
            dungeon_post: SpecialTable::new(),
            overworld_pre: SpecialTable::new(),
            overworld_post: SpecialTable::new(),
        }
    }

    fn targets(&mut self, target: &Target) -> (&mut AreaTable, &mut SpecialTable) {
        match target {
            Target::Overworld => (&mut self.overworld, &mut self.overworld_post),
            Target::Dungeon => (&mut self.dungeon, &mut self.dungeon_post),
        }
    }
}

struct HgssAreaData {
    dungeon: AreaTable,
    overworld: AreaTable,
    dungeon_special: SpecialTable,
    overworld_special: SpecialTable,
}

impl HgssAreaData {
    fn targets(&mut self, target: &Target) -> (&mut AreaTable, &mut SpecialTable) {
        match target {
            Target::Overworld => (&mut self.overworld, &mut self.overworld_special),
            Target::Dungeon => (&mut self.dungeon, &mut self.dungeon_special),
        }
    }
}

fn collect_dppt_area(
    data: &[u8],
    table: &mut AreaTable,
    special: &mut SpecialTable,
    area: i32,
) -> Result<()> {
    if i32_at(data, 0)? != 0 {
        for slot in 0..12 {
            let species = i32_at(data, 8 + slot * 8)?;
            if slot == 2 || slot == 3 {
                table.add(species, MORNING, area);
            } else {
                table.add_all_times(species, area);
            }
        }
        for slot in 0..4 {
            let species = i32_at(data, 108 + 4 * slot)?;
            if (1..=gen4::POKEMON_COUNT).contains(&species) {
                table.add(species, if slot > 1 { NIGHT } else { DAY }, area);
            }
        }
        for slot in 6..10 {
            let species = i32_at(data, 100 + slot * 4)?;
            if (1..=gen4::POKEMON_COUNT).contains(&species) {
                special.add(species, area);
            }
        }
    }
    let mut offset = 204;
    for set in 0..5 {
        let rate = i32_at(data, offset)?;
        offset += 4;
        if rate != 0 && set != 1 {
            for slot in 0..5 {
                table.add_all_times(i32_at(data, offset + 4 + slot * 8)?, area);
            }
        }
        offset += 40;
    }
    Ok(())
}

fn collect_dppt_specials(
    handler: &Gen4RomHandler,
    sets: &[EncounterSet],
    areas: &mut DpptAreaData,
) {
    for set in sets {
        let species: Vec<i32> = set
            .encounters
            .iter()
            .map(|encounter| handler.number_of(encounter.pokemon))
            .collect();
        for &number in &species {
            if set.display_name_contains("Mt. Coronet Feebas Tiles") {
                areas
                    .dungeon_pre
                    .add(number, gen4::DPPT_MT_CORONET_DEX_INDEX);
                areas
                    .dungeon_post
                    .add(number, gen4::DPPT_MT_CORONET_DEX_INDEX);
            } else if set.display_name_contains("Honey Tree Group 1")
                || set.display_name_contains("Honey Tree Group 2")
            {
                areas
                    .dungeon_pre
                    .add(number, gen4::DPPT_FLOAROMA_MEADOW_DEX_INDEX);
                areas
                    .dungeon_post
                    .add(number, gen4::DPPT_FLOAROMA_MEADOW_DEX_INDEX);
                for &index in gen4::DPPT_OVERWORLD_HONEY_TREE_DEX_INDICES {
                    areas.overworld_pre.add(number, index);
                    areas.overworld_post.add(number, index);
                }
            } else if set.display_name_contains("Trophy Garden Rotating Pokemon") {
                areas
                    .dungeon_post
                    .add(number, gen4::DPPT_TROPHY_GARDEN_DEX_INDEX);
            } else if set.display_name_contains("Great Marsh Rotating Pokemon (Post-National Dex)")
            {
                areas
                    .dungeon_post
                    .add(number, gen4::DPPT_GREAT_MARSH_DEX_INDEX);
            } else if set.display_name_contains("Great Marsh Rotating Pokemon (Pre-National Dex)") {
                areas
                    .dungeon_pre
                    .add(number, gen4::DPPT_GREAT_MARSH_DEX_INDEX);
            }
        }
    }
}

fn hgss_rates(data: &[u8]) -> Result<Vec<i32>> {
    (0..6).map(|rate| u8_at(data, rate)).collect()
}

fn collect_hgss_area(
    data: &[u8],
    table: &mut AreaTable,
    special: &mut SpecialTable,
    area: i32,
) -> Result<()> {
    let rates = hgss_rates(data)?;
    if rates[0] != 0 {
        for time in 0..TIMES {
            for slot in 0..12 {
                table.add(u16_at(data, 20 + time as i32 * 24 + slot * 2)?, time, area);
            }
        }
    }
    for slot in 0..4 {
        let species = u16_at(data, 92 + slot * 2)?;
        if species != 0 {
            special.add(species, area);
        }
    }
    let amounts = [0, 5, 2, 5, 5, 5];
    let mut offset = 100;
    for set in 1..4 {
        if rates[set] != 0 {
            for slot in 0..amounts[set] {
                table.add_all_times(u16_at(data, offset + 2 + slot * 4)?, area);
            }
        }
        offset += 4 * amounts[set];
    }
    let night_replacement = u16_at(data, 192)?;
    for (rate, rod_offset, replaced_slot) in [
        (
            rates[4],
            offset,
            gen4::HGSS_GOOD_ROD_REPLACEMENT_INDEX as i32,
        ),
        (
            rates[5],
            offset + 20,
            gen4::HGSS_SUPER_ROD_REPLACEMENT_INDEX as i32,
        ),
    ] {
        if rate == 0 {
            continue;
        }
        for slot in 0..5 {
            let species = u16_at(data, rod_offset + 2 + slot * 4)?;
            table.add(species, MORNING, area);
            table.add(species, DAY, area);
            let night_species = if slot == replaced_slot && night_replacement != 0 {
                night_replacement
            } else {
                species
            };
            table.add(night_species, NIGHT, area);
        }
    }
    Ok(())
}

impl Gen4RomHandler {
    pub(crate) fn update_pokedex_area_data_dppt(&self, sets: &[EncounterSet]) -> Result<()> {
        let wild = self.read_narc("WildPokemon")?;
        let mut areas = DpptAreaData::new();
        for (index, data) in wild.files.iter().enumerate() {
            let Some((target, area)) = classify(
                gen4::DPPT_OVERWORLD_DEX_MAPS,
                gen4::DPPT_DUNGEON_DEX_MAPS,
                index,
            ) else {
                continue;
            };
            let (table, special) = areas.targets(&target);
            collect_dppt_area(data, table, special, area)?;
        }
        collect_dppt_specials(self, sets, &mut areas);
        let mut narc = self.read_narc("PokedexAreaData")?;
        let index = |key: &str| self.entry.int(key);
        for species in 1..=gen4::POKEMON_COUNT {
            let slot = species as usize;
            for time in 0..TIMES {
                let shift = species + time as i32 * gen4::POKEDEX_AREA_DATA_SIZE;
                put_file(
                    &mut narc,
                    index("PokedexAreaDataDungeonIndex") + shift,
                    &areas.dungeon.by_time[slot][time],
                );
                put_file(
                    &mut narc,
                    index("PokedexAreaDataOverworldIndex") + shift,
                    &areas.overworld.by_time[slot][time],
                );
            }
            put_file(
                &mut narc,
                index("PokedexAreaDataDungeonSpecialPreNationalIndex") + species,
                &areas.dungeon_pre.areas[slot],
            );
            put_file(
                &mut narc,
                index("PokedexAreaDataDungeonSpecialPostNationalIndex") + species,
                &areas.dungeon_post.areas[slot],
            );
            put_file(
                &mut narc,
                index("PokedexAreaDataOverworldSpecialPreNationalIndex") + species,
                &areas.overworld_pre.areas[slot],
            );
            put_file(
                &mut narc,
                index("PokedexAreaDataOverworldSpecialPostNationalIndex") + species,
                &areas.overworld_post.areas[slot],
            );
        }
        self.write_narc("PokedexAreaData", &narc)
    }

    fn collect_hgss_headbutts(&self, sets: &[EncounterSet], areas: &mut HgssAreaData) {
        let Some(start) = sets
            .iter()
            .position(|set| set.display_name_contains("Route 1 Headbutt"))
        else {
            return;
        };
        for (offset, &overworld) in gen4::HGSS_HEADBUTT_OVERWORLD_DEX_MAPS.iter().enumerate() {
            let Some(set) = sets.get(start + offset) else {
                break;
            };
            let dungeon = dex_map(gen4::HGSS_HEADBUTT_DUNGEON_DEX_MAPS, offset);
            for encounter in &set.encounters {
                let number = self.number_of(encounter.pokemon);
                if overworld != -1 {
                    areas.overworld_special.add(number, overworld);
                } else if dungeon != -1 {
                    areas.dungeon_special.add(number, dungeon);
                }
            }
        }
    }

    pub(crate) fn update_pokedex_area_data_hgss(&self, sets: &[EncounterSet]) -> Result<()> {
        let wild = self.read_narc("WildPokemon")?;
        let mut areas = HgssAreaData {
            dungeon: AreaTable::new(),
            overworld: AreaTable::new(),
            dungeon_special: SpecialTable::new(),
            overworld_special: SpecialTable::new(),
        };
        for (index, data) in wild.files.iter().enumerate() {
            let Some((target, area)) = classify(
                gen4::HGSS_OVERWORLD_DEX_MAPS,
                gen4::HGSS_DUNGEON_DEX_MAPS,
                index,
            ) else {
                continue;
            };
            let (table, special) = areas.targets(&target);
            collect_hgss_area(data, table, special, area)?;
        }
        self.collect_hgss_headbutts(sets, &mut areas);
        let mut narc = self.read_narc("PokedexAreaData")?;
        let index = |key: &str| self.entry.int(key);
        for species in 1..=gen4::POKEMON_COUNT {
            let slot = species as usize;
            for time in 0..TIMES {
                let shift = species + time as i32 * gen4::POKEDEX_AREA_DATA_SIZE;
                put_file(
                    &mut narc,
                    index("PokedexAreaDataDungeonIndex") + shift,
                    &areas.dungeon.by_time[slot][time],
                );
                put_file(
                    &mut narc,
                    index("PokedexAreaDataOverworldIndex") + shift,
                    &areas.overworld.by_time[slot][time],
                );
            }
            put_file(
                &mut narc,
                index("PokedexAreaDataDungeonSpecialIndex") + species,
                &areas.dungeon_special.areas[slot],
            );
            put_file(
                &mut narc,
                index("PokedexAreaDataOverworldSpecialIndex") + species,
                &areas.overworld_special.areas[slot],
            );
        }
        self.write_narc("PokedexAreaData", &narc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_files_are_sorted_longs_with_a_terminator() {
        let areas = BTreeSet::from([7, 2]);
        assert_eq!(
            area_data_file(&areas),
            vec![2, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(area_data_file(&BTreeSet::new()), vec![0; 4]);
    }

    #[test]
    fn out_of_range_maps_are_skipped() {
        assert!(classify(&[1], &[-1], 5).is_none());
        assert!(matches!(
            classify(&[-1], &[4], 0),
            Some((Target::Dungeon, 4))
        ));
    }
}
