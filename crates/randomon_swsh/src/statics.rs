use randomon_core::Movesets;
use randomon_data::{PokemonId, StaticEncounter, moves_at_level};

use crate::constants;
use crate::handler::SwshRomHandler;

const RANDOM_GENDER: u8 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StaticSource {
    Event(usize),
    Gift(usize),
}

impl SwshRomHandler {
    fn static_sources(&self) -> Vec<(StaticSource, PokemonId)> {
        let data = &self.files.data;
        let original = self.files.original();
        let starters = self.starter_gift_indices();
        let events = data
            .statics
            .encounters
            .iter()
            .zip(&original.statics.encounters)
            .enumerate()
            .filter(|(_, (_, before))| !constants::STATIC_SPECIES_KEPT.contains(&before.species))
            .filter_map(|(index, (entry, _))| {
                self.pokemon_with_forme(entry.species, i32::from(entry.form))
                    .map(|pokemon| (StaticSource::Event(index), pokemon))
            });
        let gifts = data
            .gifts
            .gifts
            .iter()
            .zip(&original.gifts.gifts)
            .enumerate()
            .filter(|(index, _)| !starters.contains(index))
            .filter(|(_, (_, before))| !constants::GIFT_SPECIES_KEPT.contains(&before.species))
            .filter_map(|(index, (gift, _))| {
                self.pokemon_with_forme(gift.species, i32::from(gift.form))
                    .map(|pokemon| (StaticSource::Gift(index), pokemon))
            });
        events.chain(gifts).collect()
    }

    fn static_forme(&self, pokemon: PokemonId, form: i32) -> i32 {
        if self.core.data.pk(pokemon).base_forme.is_some() {
            form
        } else {
            0
        }
    }

    pub(crate) fn read_static_pokemon(&self) -> Vec<StaticEncounter> {
        let data = &self.files.data;
        self.static_sources()
            .into_iter()
            .map(|(source, pokemon)| {
                let base = self.core.data.pk(pokemon).base_forme.unwrap_or(pokemon);
                let mut encounter = StaticEncounter::new(base);
                match source {
                    StaticSource::Event(index) => {
                        let entry = &data.statics.encounters[index];
                        encounter.forme = self.static_forme(pokemon, i32::from(entry.form));
                        encounter.level = i32::from(entry.level);
                        encounter.held_item = entry.held_item;
                    }
                    StaticSource::Gift(index) => {
                        let gift = &data.gifts.gifts[index];
                        encounter.forme = self.static_forme(pokemon, i32::from(gift.form));
                        encounter.level = i32::from(gift.level);
                        encounter.held_item = gift.held_item;
                        encounter.is_egg = gift.is_egg != 0;
                    }
                }
                encounter
            })
            .collect()
    }

    fn level_up_moves(
        &self,
        pokemon: PokemonId,
        form: i32,
        level: i32,
        movesets: &Movesets,
    ) -> [i32; 4] {
        let shown = self.alt_forme_of(pokemon, form);
        movesets
            .get(&shown.number())
            .map_or([0; 4], |movepool| moves_at_level(movepool, level, 0))
    }

    pub(crate) fn write_static_pokemon(&mut self, statics: &[StaticEncounter]) {
        let movesets = self.read_movesets();
        let sources = self.static_sources();
        for ((source, _), encounter) in sources.into_iter().zip(statics) {
            let original = match source {
                StaticSource::Event(index) => {
                    let entry = &self.files.data.statics.encounters[index];
                    (entry.species, i32::from(entry.form))
                }
                StaticSource::Gift(index) => {
                    let gift = &self.files.data.gifts.gifts[index];
                    (gift.species, i32::from(gift.form))
                }
            };
            let (species, form) = self.placement(encounter.pkmn, encounter.forme, original);
            let level = encounter.level.clamp(1, 100);
            match source {
                StaticSource::Event(index) => {
                    let mut entry = self.files.data.statics.encounters[index];
                    if entry.species != species || i32::from(entry.form) != form {
                        entry.species = species;
                        entry.form = form as u8;
                        entry.can_gigantamax = false;
                        entry.gender = RANDOM_GENDER as i8;
                        entry.set_moves(self.level_up_moves(
                            encounter.pkmn,
                            form,
                            level,
                            &movesets,
                        ));
                    }
                    entry.level = level as u8;
                    if entry != self.files.data.statics.encounters[index] {
                        self.files.data.statics.encounters[index] = entry;
                    }
                }
                StaticSource::Gift(index) => {
                    let mut gift = self.files.data.gifts.gifts[index];
                    if gift.species != species || i32::from(gift.form) != form {
                        gift.species = species;
                        gift.form = form as u8;
                        gift.can_gigantamax = false;
                        gift.gender = RANDOM_GENDER;
                        gift.special_move = 0;
                    }
                    gift.level = level as u8;
                    if gift != self.files.data.gifts.gifts[index] {
                        self.files.data.gifts.gifts[index] = gift;
                    }
                }
            }
        }
    }
}
