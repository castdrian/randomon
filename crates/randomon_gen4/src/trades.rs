use std::collections::BTreeMap;

use randomon_core::{RandomizationError, Result};
use randomon_data::IngameTrade;

use crate::bytes::{file, file_mut, i32_at, set_i32, set_u16, u16_at};
use crate::handler::Gen4RomHandler;

const HGSS_DUPLICATED_TRADE_TEXT: usize = 6;

fn string_at(strings: &[String], index: usize) -> Result<String> {
    strings
        .get(index)
        .cloned()
        .ok_or(RandomizationError::IndexOutOfBounds("tradeStrings"))
}

impl Gen4RomHandler {
    fn regular_trade_indices(&self, trade_count: usize) -> Vec<usize> {
        let static_trades = self.entry.array_or_empty("StaticPokemonTrades");
        (0..trade_count)
            .filter(|&index| !static_trades.contains(&(index as i32)))
            .collect()
    }

    pub(crate) fn ingame_trades(&self) -> Result<Vec<IngameTrade>> {
        let narc = self.read_narc("InGameTrades")?;
        let strings = self.get_strings(self.entry.int("IngameTradesTextOffset"))?;
        let trade_count = narc.files.len();
        let mut trades = Vec::new();
        for index in self.regular_trade_indices(trade_count) {
            let data = file(&narc, index as i32)?;
            let mut ivs = [0; 6];
            for (slot, iv) in ivs.iter_mut().enumerate() {
                *iv = i32_at(data, 4 + slot as i32 * 4)?;
            }
            trades.push(IngameTrade {
                nickname: string_at(&strings, index)?,
                given_pokemon: self.pokemon_by_number(i32_at(data, 0)?)?,
                ivs: ivs.to_vec(),
                ot_id: u16_at(data, 0x20)?,
                ot_name: string_at(&strings, index + trade_count)?,
                item: i32_at(data, 0x3C)?,
                requested_pokemon: Some(self.pokemon_by_number(i32_at(data, 0x4C)?)?),
                ..IngameTrade::default()
            });
        }
        Ok(trades)
    }

    pub(crate) fn set_ingame_trades_impl(&mut self, trades: &[IngameTrade]) -> Result<()> {
        let old_trades = self.ingame_trades()?;
        let mut narc = self.read_narc("InGameTrades")?;
        let strings_index = self.entry.int("IngameTradesTextOffset");
        let mut strings = self.get_strings(strings_index)?;
        let trade_count = narc.files.len();
        for (index, trade) in self
            .regular_trade_indices(trade_count)
            .into_iter()
            .zip(trades)
        {
            if let Some(slot) = strings.get_mut(index) {
                *slot = trade.nickname.clone();
            }
            if let Some(slot) = strings.get_mut(index + trade_count) {
                *slot = trade.ot_name.clone();
            }
            let given = self.number_of(trade.given_pokemon);
            let requested = trade
                .requested_pokemon
                .map(|pokemon| self.number_of(pokemon));
            let data = file_mut(&mut narc, index as i32)?;
            set_i32(data, 0, given)?;
            for (slot, &iv) in trade.ivs.iter().take(6).enumerate() {
                set_i32(data, 4 + slot as i32 * 4, iv)?;
            }
            set_u16(data, 0x20, trade.ot_id)?;
            set_i32(data, 0x3C, trade.item)?;
            if let Some(requested) = requested {
                set_i32(data, 0x4C, requested)?;
            }
            if data.len() > 0x50 {
                set_i32(data, 0x50, 0)?;
            }
        }
        self.write_narc("InGameTrades", &narc)?;
        self.set_strings(strings_index, &strings)?;
        self.update_trade_person_texts(&old_trades, trades)
    }

    fn update_trade_person_texts(
        &mut self,
        old_trades: &[IngameTrade],
        trades: &[IngameTrade],
    ) -> Result<()> {
        let Some(text_offsets) = self.entry.array("IngameTradePersonTextOffsets") else {
            return Ok(());
        };
        for (trade, &text_offset) in text_offsets.iter().enumerate() {
            if text_offset <= 0 {
                continue;
            }
            let (Some(old), Some(new)) = (old_trades.get(trade), trades.get(trade)) else {
                break;
            };
            let name = |pokemon| self.core.data.pk(pokemon).name.clone();
            let mut replacements = BTreeMap::new();
            replacements.insert(
                name(old.given_pokemon).to_uppercase(),
                name(new.given_pokemon),
            );
            if let (Some(old_requested), Some(new_requested)) =
                (old.requested_pokemon, new.requested_pokemon)
                && old_requested != new_requested
            {
                replacements.insert(name(old_requested).to_uppercase(), name(new_requested));
            }
            self.replace_all_strings_in_entry(text_offset, &replacements)?;
            if self.entry.is_hgss() && trade == HGSS_DUPLICATED_TRADE_TEXT {
                self.replace_all_strings_in_entry(text_offset + 1, &replacements)?;
            }
        }
        Ok(())
    }
}
