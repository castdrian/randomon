use randomon_data::IngameTrade;

use crate::handler::SwshRomHandler;

const RANDOM_GENDER: i8 = 0;

impl SwshRomHandler {
    pub(crate) fn read_ingame_trades(&self) -> Vec<IngameTrade> {
        self.files
            .data
            .trades
            .trades
            .iter()
            .enumerate()
            .filter_map(|(index, trade)| {
                let given = self.pokemon_with_forme(trade.species, i32::from(trade.form))?;
                Some(IngameTrade {
                    id: index as i32,
                    requested_pokemon: self
                        .pokemon_with_forme(trade.required_species, i32::from(trade.required_form)),
                    given_pokemon: given,
                    nickname: String::new(),
                    ot_name: String::new(),
                    ot_id: trade.trainer_id,
                    ivs: Vec::new(),
                    item: trade.held_item,
                })
            })
            .collect()
    }

    pub(crate) fn write_ingame_trades(&mut self, trades: &[IngameTrade]) {
        for trade in trades {
            let Some(original) = usize::try_from(trade.id)
                .ok()
                .and_then(|index| self.files.data.trades.trades.get(index))
            else {
                continue;
            };
            let mut entry = *original;
            let (species, form) = self.placement(
                trade.given_pokemon,
                0,
                (entry.species, i32::from(entry.form)),
            );
            if entry.species != species || i32::from(entry.form) != form {
                entry.species = species;
                entry.form = form as u8;
                entry.can_gigantamax = false;
                entry.gender = RANDOM_GENDER;
                entry.relearn_1 = 0;
                entry.relearn_2 = 0;
                entry.relearn_3 = 0;
                entry.relearn_4 = 0;
            }
            if let Some(requested) = trade.requested_pokemon {
                let (species, form) = self.placement(
                    requested,
                    0,
                    (entry.required_species, i32::from(entry.required_form)),
                );
                entry.required_species = species;
                entry.required_form = form as u8;
            }
            if self.allowed_items.is_allowed(trade.item) || trade.item == 0 {
                entry.held_item = trade.item;
            }
            let index = trade.id as usize;
            if entry != self.files.data.trades.trades[index] {
                self.files.data.trades.trades[index] = entry;
            }
        }
    }
}
