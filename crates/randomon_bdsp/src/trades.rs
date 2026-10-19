use randomon_data::{IngameTrade, moves_at_level};
use randomon_unity::Value;

use crate::constants;
use crate::error::Result;
use crate::handler::BdspRomHandler;
use crate::tables::{Table, int, set_int};

fn label(row: &Value, field: &str) -> String {
    row.field(field)
        .ok()
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

impl BdspRomHandler {
    pub(crate) fn read_ingame_trades(&self) -> Result<Vec<IngameTrade>> {
        let mut trades = Vec::new();
        let mut seen = Vec::new();
        for (index, row) in self.tables.rows(Table::Trades, "data")?.iter().enumerate() {
            let nickname_label = label(row, "nickname_label");
            if seen.contains(&nickname_label) {
                continue;
            }
            seen.push(nickname_label);
            let given = int(row, "monsno")?;
            if !(1..=constants::SPECIES_COUNT).contains(&given) {
                continue;
            }
            let requested = int(row, "target")?;
            trades.push(IngameTrade {
                id: index as i32,
                requested_pokemon: (1..=constants::SPECIES_COUNT)
                    .contains(&requested)
                    .then(|| self.pk(requested)),
                given_pokemon: self.pk(given),
                nickname: self
                    .messages
                    .scenario_text(&label(row, "nickname_label"))
                    .unwrap_or_default(),
                ot_name: self
                    .messages
                    .scenario_text(&label(row, "name_label"))
                    .unwrap_or_default(),
                ot_id: int(row, "trainerid")?,
                ivs: Vec::new(),
                item: int(row, "itemno")?,
            });
        }
        Ok(trades)
    }

    pub(crate) fn write_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        let movesets = self.read_movesets()?;
        let mut rows = self.tables.rows(Table::Trades, "data")?.to_vec();
        let mut texts = Vec::new();
        for trade in trades {
            let Some(shared_label) = rows
                .get(trade.id as usize)
                .map(|row| label(row, "nickname_label"))
            else {
                continue;
            };
            for row in rows
                .iter_mut()
                .filter(|row| label(row, "nickname_label") == shared_label)
            {
                let given = self.core.data.base_number(trade.given_pokemon);
                set_int(row, "monsno", given)?;
                if let Some(requested) = trade.requested_pokemon {
                    set_int(row, "target", self.core.data.base_number(requested))?;
                }
                set_int(row, "itemno", trade.item)?;
                set_int(row, "tokusei", 0)?;
                let level = int(row, "level")?;
                let moves = movesets
                    .get(&given)
                    .map_or([0; 4], |movepool| moves_at_level(movepool, level, 0));
                if let Some(waza) = row.field_mut("waza")?.as_array_mut() {
                    for (slot, move_number) in waza.iter_mut().zip(moves) {
                        slot.set_int(i128::from(move_number))?;
                    }
                }
                texts.push((label(row, "nickname_label"), trade.nickname.clone()));
                texts.push((label(row, "name_label"), trade.ot_name.clone()));
            }
        }
        *self.tables.rows_mut(Table::Trades, "data")? = rows;
        for (label, text) in texts {
            if label.is_empty() || text.is_empty() {
                continue;
            }
            if let Some(table) = self.messages.scenario_with_label(&label)
                && table.text_of(&label).as_deref() != Some(text.as_str())
            {
                table.set_label_text(&label, &text)?;
            }
        }
        Ok(())
    }
}
