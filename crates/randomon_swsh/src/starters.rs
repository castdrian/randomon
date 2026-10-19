use randomon_data::PokemonId;

use crate::formats::placement::{Critter, STARTER_AREA_FILE};
use crate::handler::SwshRomHandler;

const RANDOM_GENDER: u8 = 0;

impl SwshRomHandler {
    pub(crate) fn starter_gift_indices(&self) -> Vec<usize> {
        self.files.original().gifts.starter_indices()
    }

    pub(crate) fn read_starters(&self) -> Vec<PokemonId> {
        let gifts = &self.files.data.gifts.gifts;
        self.starter_gift_indices()
            .into_iter()
            .filter_map(|index| {
                let gift = gifts.get(index)?;
                self.pokemon_with_forme(gift.species, i32::from(gift.form))
            })
            .collect()
    }

    pub(crate) fn write_starters(&mut self, starters: &[PokemonId]) {
        let indices = self.starter_gift_indices();
        let original_species: Vec<i32> = indices
            .iter()
            .map(|&index| self.files.original().gifts.gifts[index].species)
            .collect();
        let mut placed = Vec::new();
        for (&index, &starter) in indices.iter().zip(starters) {
            let mut gift = self.files.data.gifts.gifts[index];
            let (species, form) = self.placement(starter, 0, (gift.species, i32::from(gift.form)));
            if gift.species != species || i32::from(gift.form) != form {
                gift.species = species;
                gift.form = form as u8;
                gift.can_gigantamax = false;
                gift.gender = RANDOM_GENDER;
                gift.special_move = 0;
                self.files.data.gifts.gifts[index] = gift;
            }
            placed.push((species, form));
        }
        self.update_starter_critters(&original_species, &placed);
    }

    fn original_starter_critters(&self) -> Option<Vec<Vec<Critter>>> {
        self.files
            .original()
            .placement
            .iter()
            .find(|area| area.file_name == STARTER_AREA_FILE)
            .map(|area| {
                area.archive
                    .zones
                    .iter()
                    .map(|zone| zone.critters.clone())
                    .collect()
            })
    }

    fn update_starter_critters(&mut self, original_species: &[i32], placed: &[(i32, i32)]) {
        let Some(original) = self.original_starter_critters() else {
            return;
        };
        let Some(area) = self
            .files
            .data
            .placement
            .iter_mut()
            .find(|area| area.file_name == STARTER_AREA_FILE)
        else {
            return;
        };
        let archive = &mut area.archive;
        for (zone, critters) in original.iter().enumerate() {
            for (critter, before) in critters.iter().enumerate() {
                let Some(position) = original_species
                    .iter()
                    .position(|&species| species == before.species as i32)
                else {
                    continue;
                };
                let Some(&(species, form)) = placed.get(position) else {
                    continue;
                };
                let can_set_form = archive.can_set_critter_form(zone, critter);
                let mut updated = *before;
                updated.species = species as u32;
                if can_set_form {
                    updated.form = form as u32;
                }
                let entry = &mut archive.zones[zone].critters[critter];
                if *entry != updated {
                    *entry = updated;
                }
            }
        }
    }
}

impl SwshRomHandler {
    pub(crate) fn read_starter_held_items(&self) -> Vec<i32> {
        let gifts = &self.files.data.gifts.gifts;
        self.starter_gift_indices()
            .into_iter()
            .filter_map(|index| gifts.get(index).map(|gift| gift.held_item))
            .collect()
    }

    pub(crate) fn write_starter_held_items(&mut self, items: &[i32]) {
        for (index, &item) in self.starter_gift_indices().iter().zip(items) {
            if item != 0 && !self.allowed_items.is_allowed(item) {
                continue;
            }
            let gift = &mut self.files.data.gifts.gifts[*index];
            if gift.held_item != item {
                gift.held_item = item;
            }
        }
    }
}
