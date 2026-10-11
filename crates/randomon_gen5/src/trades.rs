use std::collections::BTreeMap;

use randomon_core::{RandomizationError, Result};
use randomon_data::IngameTrade;
use randomon_nds::{read_long, read_word, write_long, write_word};

use crate::handler::Gen5RomHandler;

const PERSON_TEXT_TRADE_OFFSET: usize = 24;
const REGEX_METACHARACTERS: &[char] = &[
    '\\', '^', '$', '|', '?', '*', '+', '(', ')', '[', ']', '{', '}',
];
const LINE_TERMINATORS: &[char] = &['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}'];

fn unsupported_regex(pattern: &str) -> RandomizationError {
    RandomizationError::randomization(format!(
        "unsupported regular expression in replaceAll: {pattern}"
    ))
}

fn pattern_matches_at(text: &[char], start: usize, pattern: &[char]) -> bool {
    start + pattern.len() <= text.len()
        && pattern.iter().enumerate().all(|(index, &expected)| {
            let actual = text[start + index];
            if expected == '.' {
                !LINE_TERMINATORS.contains(&actual)
            } else {
                actual == expected
            }
        })
}

fn literal_replacement(replacement: &str) -> Result<String> {
    if replacement.contains('$') {
        return Err(unsupported_regex(replacement));
    }
    let mut literal = String::with_capacity(replacement.len());
    let mut characters = replacement.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            let escaped = characters.next().ok_or_else(|| {
                RandomizationError::randomization("character to be escaped is missing")
            })?;
            literal.push(escaped);
        } else {
            literal.push(character);
        }
    }
    Ok(literal)
}

pub(crate) fn java_replace_all(text: &str, pattern: &str, replacement: &str) -> Result<String> {
    if pattern.contains(REGEX_METACHARACTERS) || pattern.is_empty() {
        return Err(unsupported_regex(pattern));
    }
    let replacement = literal_replacement(replacement)?;
    let characters: Vec<char> = text.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    let mut output = String::with_capacity(text.len());
    let mut position = 0;
    while position < characters.len() {
        if pattern_matches_at(&characters, position, &pattern) {
            output.push_str(&replacement);
            position += pattern.len();
        } else {
            output.push(characters[position]);
            position += 1;
        }
    }
    Ok(output)
}

impl Gen5RomHandler {
    fn trades_unused(&self) -> Result<&'static [i32]> {
        self.entry
            .array("TradesUnused")
            .ok_or(RandomizationError::NullPointer(
                "arrayEntries.get(TradesUnused)",
            ))
    }

    fn trade_string(strings: &[String], index: usize) -> Result<String> {
        strings
            .get(index)
            .cloned()
            .ok_or(RandomizationError::IndexOutOfBounds("tradeStrings"))
    }

    pub(crate) fn ingame_trades(&self) -> Result<Vec<IngameTrade>> {
        let narc = self.read_narc("InGameTrades")?;
        let strings = self.get_strings(false, self.entry.int("IngameTradesTextOffset"))?;
        let unused = self.trades_unused()?;
        let mut unused_offset = 0;
        let mut trades = Vec::new();
        for (entry, file) in narc.files.iter().enumerate() {
            if unused_offset < unused.len() && unused[unused_offset] == entry as i32 {
                unused_offset += 1;
                continue;
            }
            trades.push(IngameTrade {
                nickname: Self::trade_string(&strings, entry * 2)?,
                given_pokemon: self.pk(read_long(file, 4)),
                ivs: (0..6).map(|iv| read_long(file, 0x10 + iv * 4)).collect(),
                ot_id: read_word(file, 0x34),
                item: read_long(file, 0x4C),
                ot_name: Self::trade_string(&strings, entry * 2 + 1)?,
                requested_pokemon: Some(self.pk(read_long(file, 0x5C))),
                ..IngameTrade::default()
            });
        }
        Ok(trades)
    }

    fn apply_trade_script(
        &mut self,
        script_index: usize,
        requested: i32,
        given: i32,
    ) -> Result<()> {
        let script = self
            .entry
            .trade_scripts
            .get(script_index)
            .ok_or(RandomizationError::IndexOutOfBounds("tradeScripts"))?;
        let file = self
            .script_narc
            .files
            .get_mut(script.file_number as usize)
            .ok_or(RandomizationError::IndexOutOfBounds("scriptNARC.files"))?;
        for (&requested_offset, &given_offset) in
            script.requested_offsets.iter().zip(&script.given_offsets)
        {
            write_word(file, requested_offset, requested);
            write_word(file, given_offset, given);
        }
        Ok(())
    }

    fn update_trade_person_texts(
        &mut self,
        old_trades: &[IngameTrade],
        trades: &[IngameTrade],
    ) -> Result<()> {
        let Some(text_offsets) = self.entry.array("IngameTradePersonTextOffsets") else {
            return Ok(());
        };
        let unused = self.trades_unused()?;
        let mut unused_offset = 0;
        for (index, &text_offset) in text_offsets.iter().enumerate() {
            let trade_index = index + PERSON_TEXT_TRADE_OFFSET;
            if unused_offset < unused.len() && unused[unused_offset] == trade_index as i32 {
                unused_offset += 1;
                continue;
            }
            if text_offset <= 0 {
                continue;
            }
            if trade_index >= old_trades.len() || trade_index >= trades.len() {
                break;
            }
            let old_trade = &old_trades[trade_index];
            let new_trade = &trades[trade_index];
            let name = |id| self.core.data.pk(id).name.clone();
            let mut replacements = BTreeMap::new();
            replacements.insert(name(old_trade.given_pokemon), name(new_trade.given_pokemon));
            if old_trade.requested_pokemon != new_trade.requested_pokemon {
                let requested = |trade: &IngameTrade| {
                    trade
                        .requested_pokemon
                        .ok_or(RandomizationError::NullPointer("requestedPokemon"))
                };
                replacements.insert(name(requested(old_trade)?), name(requested(new_trade)?));
            }
            self.replace_all_strings_in_entry(text_offset, &replacements)?;
        }
        Ok(())
    }

    pub(crate) fn set_ingame_trades_impl(&mut self, trades: &[IngameTrade]) -> Result<()> {
        let old_trades = self.ingame_trades()?;
        let mut narc = self.read_narc("InGameTrades")?;
        let strings_index = self.entry.int("IngameTradesTextOffset");
        let mut strings = self.get_strings(false, strings_index)?;
        let unused = self.trades_unused()?;
        let mut unused_offset = 0;
        let mut trade_offset = 0;
        for index in 0..narc.files.len() {
            if unused_offset < unused.len() && unused[unused_offset] == index as i32 {
                unused_offset += 1;
                continue;
            }
            let trade = trades
                .get(trade_offset)
                .ok_or(RandomizationError::IndexOutOfBounds("trades"))?;
            trade_offset += 1;
            let mut set_string = |slot: usize, value: &str| -> Result<()> {
                *strings
                    .get_mut(slot)
                    .ok_or(RandomizationError::IndexOutOfBounds("tradeStrings"))? =
                    value.to_owned();
                Ok(())
            };
            set_string(index * 2, &trade.nickname)?;
            set_string(index * 2 + 1, &trade.ot_name)?;
            let given = self.core.data.pk(trade.given_pokemon).number;
            let requested = self
                .core
                .data
                .pk(trade
                    .requested_pokemon
                    .ok_or(RandomizationError::NullPointer("requestedPokemon"))?)
                .number;
            let file = &mut narc.files[index];
            write_long(file, 4, given);
            write_long(file, 8, 0);
            for (iv, &value) in trade.ivs.iter().enumerate().take(6) {
                write_long(file, 0x10 + iv as i32 * 4, value);
            }
            write_long(file, 0x2C, 0xFF);
            write_word(file, 0x34, trade.ot_id);
            write_long(file, 0x4C, trade.item);
            write_long(file, 0x5C, requested);
            if !self.entry.trade_scripts.is_empty() {
                self.apply_trade_script(index - unused_offset, requested, given)?;
            }
        }
        self.write_narc("InGameTrades", &narc)?;
        self.set_strings(false, strings_index, &strings)?;
        self.update_trade_person_texts(&old_trades, trades)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_names_replace_every_occurrence() {
        assert_eq!(
            java_replace_all("Petilil and Petilil", "Petilil", "Cottonee").unwrap(),
            "Cottonee and Cottonee"
        );
    }

    #[test]
    fn dots_match_any_character_like_java_regex() {
        assert_eq!(
            java_replace_all("Mr. Mime Mr! Mime", "Mr. Mime", "Ditto").unwrap(),
            "Ditto Ditto"
        );
    }
}
