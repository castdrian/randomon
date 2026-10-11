use std::collections::BTreeMap;

use randomon_core::Result;
use randomon_data::constants::species;
use randomon_data::{PokemonId, StaticEncounter};

use crate::handler::Gen3RomHandler;
use crate::rom_entry::{RomType, StaticPokemonEntry};

const UNPATCHED_HARDCODED_SPECIES: i32 = 0xFFFF;
const MALE_ONLY: i32 = 0x00;
const GENDERLESS: i32 = 0xFF;
const FRLG_ROAMER_COUNT: usize = 3;
const MUSIC_POOL_STRIDE: i32 = 4;

impl Gen3RomHandler {
    fn static_species(&self, entry: &StaticPokemonEntry) -> PokemonId {
        let internal = entry
            .species_offsets
            .first()
            .map_or(0, |&offset| self.rom.word(offset));
        self.pokemon_or_fallback(internal)
    }

    fn static_level(&self, entry: &StaticPokemonEntry, index: usize) -> i32 {
        entry
            .level_offsets
            .get(index)
            .map_or(1, |&offset| self.rom.signed(offset))
    }

    fn set_static_species(&mut self, entry: &StaticPokemonEntry, pokemon: PokemonId) {
        let value = self.internal_of(pokemon);
        for &offset in &entry.species_offsets {
            self.rom.set_word(offset, value);
        }
    }

    fn set_static_levels(&mut self, entry: &StaticPokemonEntry, level: i32) {
        for &offset in &entry.level_offsets {
            self.rom.set_byte(offset, level as u8);
        }
    }

    fn read_static_entry(&self, entry: &StaticPokemonEntry) -> StaticEncounter {
        let mut encounter = StaticEncounter::new(self.static_species(entry));
        encounter.level = self.static_level(entry, 0);
        encounter
    }

    fn hardcoded_static_count(&self) -> usize {
        usize::from(
            self.entry.tweak("StaticFirstBattleTweak").is_some()
                || self.entry.tweak("GhostMarowakTweak").is_some(),
        )
    }

    fn read_first_battle(&self) -> StaticEncounter {
        let species_offset = self.value("StaticFirstBattleSpeciesOffset");
        let species = self.rom.word(species_offset);
        let mut encounter = StaticEncounter::new(self.pokemon_or_fallback(species));
        encounter.level = self.rom.signed(self.value("StaticFirstBattleLevelOffset"));
        encounter
    }

    fn ghost_marowak_species_offset(&self) -> i32 {
        self.array("GhostMarowakSpeciesOffsets")
            .first()
            .copied()
            .unwrap_or(-1)
    }

    fn read_ghost_marowak(&self) -> StaticEncounter {
        let species = self.rom.word(self.ghost_marowak_species_offset());
        let mut encounter = StaticEncounter::new(self.pokemon_or_fallback(species));
        let level_offset = self
            .array("GhostMarowakLevelOffsets")
            .first()
            .copied()
            .unwrap_or(-1);
        encounter.level = self.rom.signed(level_offset);
        encounter
    }

    pub(crate) fn prepare_hardcoded_statics(&mut self) -> Result<()> {
        if self.entry.tweak("StaticFirstBattleTweak").is_some() {
            let species_offset = self.value("StaticFirstBattleSpeciesOffset");
            if self.rom.word(species_offset) == UNPATCHED_HARDCODED_SPECIES {
                self.apply_tweak_patch("StaticFirstBattleTweak")?;
            }
        } else if self.entry.tweak("GhostMarowakTweak").is_some()
            && self.rom.word(self.ghost_marowak_species_offset()) == UNPATCHED_HARDCODED_SPECIES
        {
            self.apply_tweak_patch("GhostMarowakTweak")?;
        }
        self.prepare_roamers()
    }

    fn first_roamer_species(&self) -> Option<i32> {
        let offset = *self
            .entry
            .roaming_pokemon
            .first()?
            .species_offsets
            .first()?;
        Some(self.rom.word(offset))
    }

    fn prepare_roamers(&mut self) -> Result<()> {
        let Some(first_species) = self.first_roamer_species() else {
            return Ok(());
        };
        match self.rom_type() {
            RomType::Ruby if first_species == 0 => self.apply_ruby_roamer_patch(),
            RomType::FireRedLeafGreen
                if self.entry.tweak("RoamingPokemonTweak").is_some()
                    && first_species == UNPATCHED_HARDCODED_SPECIES =>
            {
                self.apply_tweak_patch("RoamingPokemonTweak")?;
            }
            RomType::Emerald if first_species >= self.pokemon_count() + 1 => {
                self.apply_emerald_roamer_patch();
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn read_static_pokemon(&self) -> Vec<StaticEncounter> {
        let egg_indexes = self.array("StaticEggPokemonOffsets");
        let entries = self.entry.static_pokemon.clone();
        let mut statics: Vec<StaticEncounter> = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let mut encounter = self.read_static_entry(entry);
                encounter.is_egg = egg_indexes.contains(&(index as i32));
                encounter
            })
            .collect();
        if self.entry.tweak("StaticFirstBattleTweak").is_some() {
            statics.push(self.read_first_battle());
        } else if self.entry.tweak("GhostMarowakTweak").is_some() {
            statics.push(self.read_ghost_marowak());
        }
        self.read_roamers(&mut statics);
        statics
    }

    fn read_roamers(&self, statics: &mut Vec<StaticEncounter>) {
        let roamers = &self.entry.roaming_pokemon;
        let Some(first) = roamers.first() else {
            return;
        };
        match self.rom_type() {
            RomType::Ruby | RomType::Sapphire => statics.push(self.read_static_entry(first)),
            RomType::FireRedLeafGreen => {
                if self.entry.tweak("RoamingPokemonTweak").is_some() {
                    statics.extend(roamers.iter().map(|roamer| self.read_static_entry(roamer)));
                }
            }
            RomType::Emerald => {
                let southern_island = self.array("StaticSouthernIslandOffsets");
                for (index, roamer) in roamers.iter().enumerate() {
                    let encounter = self.read_static_entry(roamer);
                    let Some(target) = southern_island
                        .get(index)
                        .and_then(|&slot| usize::try_from(slot).ok())
                        .and_then(|slot| statics.get_mut(slot))
                    else {
                        continue;
                    };
                    target.linked_encounters.push(encounter);
                }
            }
        }
    }

    fn roamer_static_count(&self) -> usize {
        let roamers = self.entry.roaming_pokemon.len();
        match self.rom_type() {
            RomType::Emerald => 0,
            RomType::Ruby | RomType::Sapphire => roamers.min(1),
            RomType::FireRedLeafGreen => {
                if self.entry.tweak("RoamingPokemonTweak").is_some() {
                    roamers
                } else {
                    0
                }
            }
        }
    }

    pub(crate) fn write_static_pokemon(&mut self, statics: &[StaticEncounter]) -> Result<bool> {
        self.attempt_obedience_evolution_patches();
        let entries = self.entry.static_pokemon.clone();
        let expected = entries.len() + self.hardcoded_static_count() + self.roamer_static_count();
        if statics.len() != expected {
            return Ok(false);
        }
        for (entry, encounter) in entries.iter().zip(statics) {
            self.set_static_species(entry, encounter.pkmn);
            if let Some(&offset) = entry.level_offsets.first() {
                self.rom.set_byte(offset, encounter.level as u8);
            }
        }
        if self.entry.tweak("StaticFirstBattleTweak").is_some() {
            self.write_first_battle(statics);
        } else if self.entry.tweak("GhostMarowakTweak").is_some() {
            self.write_ghost_marowak(statics);
        }
        self.write_roamers(statics);
        Ok(true)
    }

    fn write_first_battle(&mut self, statics: &[StaticEncounter]) {
        let Some(encounter) = usize::try_from(self.value("StaticFirstBattleOffset"))
            .ok()
            .and_then(|index| statics.get(index))
        else {
            return;
        };
        let species = self.internal_of(encounter.pkmn);
        self.rom
            .set_word(self.value("StaticFirstBattleSpeciesOffset"), species);
        self.rom.set_byte(
            self.value("StaticFirstBattleLevelOffset"),
            encounter.level as u8,
        );
    }

    fn write_ghost_marowak(&mut self, statics: &[StaticEncounter]) {
        let Some(encounter) = usize::try_from(self.value("GhostMarowakOffset"))
            .ok()
            .and_then(|index| statics.get(index))
        else {
            return;
        };
        let species = self.internal_of(encounter.pkmn);
        for offset in self.array("GhostMarowakSpeciesOffsets") {
            self.rom.set_word(offset, species);
        }
        for offset in self.array("GhostMarowakLevelOffsets") {
            self.rom.set_byte(offset, encounter.level as u8);
        }
        let gender_ratio = self.core.data.pk(encounter.pkmn).gender_ratio;
        if gender_ratio == MALE_ONLY || gender_ratio == GENDERLESS {
            self.rom
                .set_byte(self.value("GhostMarowakGenderOffset"), gender_ratio as u8);
        }
    }

    fn write_roamer(&mut self, roamer: &StaticPokemonEntry, encounter: &StaticEncounter) {
        self.set_static_species(roamer, encounter.pkmn);
        self.set_static_levels(roamer, encounter.level);
    }

    fn write_roamers(&mut self, statics: &[StaticEncounter]) {
        let roamers = self.entry.roaming_pokemon.clone();
        match self.rom_type() {
            RomType::Ruby | RomType::Sapphire => {
                if let (Some(roamer), Some(encounter)) = (roamers.first(), statics.last()) {
                    self.write_roamer(roamer, encounter);
                }
            }
            RomType::FireRedLeafGreen => {
                if self.entry.tweak("RoamingPokemonTweak").is_none() {
                    return;
                }
                let start = statics.len().saturating_sub(FRLG_ROAMER_COUNT);
                for (roamer, encounter) in roamers.iter().zip(&statics[start..]) {
                    self.write_roamer(roamer, encounter);
                }
            }
            RomType::Emerald => {
                let southern_island = self.array("StaticSouthernIslandOffsets");
                for (index, roamer) in roamers.iter().enumerate() {
                    let linked = southern_island
                        .get(index)
                        .and_then(|&slot| usize::try_from(slot).ok())
                        .and_then(|slot| statics.get(slot))
                        .and_then(|island| island.linked_encounters.first());
                    if let Some(encounter) = linked {
                        self.write_roamer(roamer, encounter);
                    }
                }
            }
        }
    }

    fn latios_internal(&self) -> i32 {
        self.internal_of_number(species::LATIOS)
    }

    fn apply_ruby_roamer_patch(&mut self) {
        let latios = self.latios_internal();
        let offset = self.value("FindMapsWithMonFunctionStartOffset");
        self.rom.set_byte(offset + 22, 0x57);
        self.rom.set_long(offset + 128, latios);
        self.rom.write_bytes(offset + 12, &[0x00, 0x00, 0x1C, 0x48]);
        let offset = self.value("CreateInitialRoamerMonFunctionStartOffset");
        self.rom.write_bytes(offset + 182, &[0x00, 0xBD]);
        self.rom.set_long(offset + 184, latios);
        self.rom.write_bytes(offset + 10, &[0x00, 0x00, 0x2A, 0x4E]);
    }

    fn apply_emerald_roamer_patch(&mut self) {
        let latios = self.latios_internal();
        let offset = self.value("CreateInitialRoamerMonFunctionStartOffset");
        self.rom.set_byte(offset + 14, 0x41);
        self.rom.set_long(offset + 28, latios);
        self.rom.write_bytes(offset + 8, &[0x00, 0x28, 0x04, 0x4B]);
        self.rom.write_bytes(offset + 48, &[0x1A, 0x46, 0x00, 0x00]);
    }

    pub(crate) fn apply_static_music(&mut self, changes: &BTreeMap<i32, i32>) -> Result<()> {
        if !self.apply_tweak_patch("NewIndexToMusicTweak")? {
            return Ok(());
        }
        let pool = self.value("NewIndexToMusicPoolOffset");
        if pool <= 0 {
            return Ok(());
        }
        let pool_size = self.array("SpecialMusicStatics").len().max(changes.len()) as i32;
        let mut replaced: Vec<i32> = Vec::new();
        for (&old_static, &new_static) in changes {
            let found = (0..pool_size)
                .map(|slot| pool + slot * MUSIC_POOL_STRIDE)
                .find(|&offset| {
                    !replaced.contains(&offset)
                        && self.dex_of_internal(self.rom.word(offset)) == old_static
                });
            if let Some(offset) = found {
                let value = self.internal_of_number(new_static);
                self.rom.set_word(offset, value);
                replaced.push(offset);
            }
        }
        Ok(())
    }
}
