use randomon_data::constants::moves;
use randomon_gen7::constants::{move_category_to_byte, type_to_byte};
use randomon_gen7::parse_move;
use randomon_java::math::round_double;
use randomon_unity::Value;

use crate::constants;
use crate::error::{Result, data_error};
use crate::handler::{BdspRomHandler, index_texts};
use crate::tables::{Table, int, set_int};

const RECORD_LENGTH: usize = 40;

fn byte(row: &Value, field: &str) -> Result<u8> {
    Ok(int(row, field)? as u8)
}

fn packed_record(row: &Value) -> Result<[u8; RECORD_LENGTH]> {
    let mut data = [0u8; RECORD_LENGTH];
    data[0] = byte(row, "type")?;
    data[1] = byte(row, "category")?;
    data[2] = byte(row, "damageType")?;
    data[3] = byte(row, "power")?;
    data[4] = byte(row, "hitPer")?;
    data[5] = byte(row, "basePP")?;
    data[6] = byte(row, "priority")?;
    data[7] = byte(row, "hitCountMin")? | (byte(row, "hitCountMax")? << 4);
    data[8..10].copy_from_slice(&(int(row, "sickID")? as u16).to_le_bytes());
    data[10] = byte(row, "sickPer")?;
    data[11] = byte(row, "sickCont")?;
    data[12] = byte(row, "sickTurnMin")?;
    data[13] = byte(row, "sickTurnMax")?;
    data[14] = byte(row, "criticalRank")?;
    data[15] = byte(row, "shrinkPer")?;
    data[16..18].copy_from_slice(&(int(row, "aiSeqNo")? as u16).to_le_bytes());
    data[18] = byte(row, "damageRecoverRatio")?;
    data[19] = byte(row, "hpRecoverRatio")?;
    data[20] = byte(row, "target")?;
    for index in 0..3 {
        data[21 + index] = byte(row, &format!("rankEffType{}", index + 1))?;
        data[24 + index] = byte(row, &format!("rankEffValue{}", index + 1))?;
        data[27 + index] = byte(row, &format!("rankEffPer{}", index + 1))?;
    }
    data[36..40].copy_from_slice(&(row.int_field("flags")? as u32).to_le_bytes());
    Ok(data)
}

impl BdspRomHandler {
    pub(crate) fn load_moves(&mut self) -> Result<()> {
        let names = index_texts(&self.messages.moves, constants::MOVE_COUNT as usize + 1)?;
        let rows: Vec<Value> = self.tables.rows(Table::Waza, "Waza")?.to_vec();
        self.core
            .data
            .reserve_moves(constants::MOVE_COUNT as usize + 1);
        for row in &rows {
            let number = int(row, "wazaNo")?;
            if !(1..=constants::MOVE_COUNT).contains(&number) || int(row, "isValid")? != 1 {
                continue;
            }
            let name = names.get(number as usize).cloned().unwrap_or_default();
            let mv = parse_move(number, name, &packed_record(row)?)
                .map_err(|error| data_error(format!("move {number}: {error}")))?;
            if number == moves::SWIFT {
                self.core.engine.perfect_accuracy = mv.hitratio as i32;
            }
            self.core.data.create_move(mv);
        }
        Ok(())
    }

    pub(crate) fn save_moves(&mut self) -> Result<()> {
        let mut rows = self.tables.rows(Table::Waza, "Waza")?.to_vec();
        for row in rows.iter_mut() {
            let number = int(row, "wazaNo")?;
            let Some(mv) = self.core.data.move_by_number(number) else {
                continue;
            };
            let rounded = round_double(mv.hitratio) as i32;
            let hitratio = if rounded > 101 { 100 } else { rounded.max(0) };
            set_int(row, "type", i32::from(type_to_byte(mv.kind)))?;
            set_int(
                row,
                "damageType",
                i32::from(move_category_to_byte(mv.category)),
            )?;
            set_int(row, "power", mv.power)?;
            set_int(row, "hitPer", hitratio)?;
            set_int(row, "basePP", mv.pp)?;
        }
        *self.tables.rows_mut(Table::Waza, "Waza")? = rows;
        Ok(())
    }
}
