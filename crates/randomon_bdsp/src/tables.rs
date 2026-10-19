use std::collections::BTreeSet;

use randomon_unity::Value;

use crate::assets::{BundleId, path_ids};
use crate::bundles::Bundles;
use crate::error::{Result, data_error};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Table {
    Personal,
    Waza,
    WazaOboe,
    Evolve,
    TamagoWaza,
    Item,
    Trainer,
    EncounterDiamond,
    EncounterPearl,
    Shop,
    FieldItems,
    Trades,
    Pickup,
    MapInfo,
    PokemonInfo,
}

impl Table {
    pub const ALL: [Table; 15] = [
        Table::Personal,
        Table::Waza,
        Table::WazaOboe,
        Table::Evolve,
        Table::TamagoWaza,
        Table::Item,
        Table::Trainer,
        Table::EncounterDiamond,
        Table::EncounterPearl,
        Table::Shop,
        Table::FieldItems,
        Table::Trades,
        Table::Pickup,
        Table::MapInfo,
        Table::PokemonInfo,
    ];

    pub fn location(self) -> (BundleId, i64) {
        match self {
            Table::Personal => (BundleId::PersonalMasterdatas, path_ids::PERSONAL_TABLE),
            Table::Waza => (BundleId::PersonalMasterdatas, path_ids::WAZA_TABLE),
            Table::WazaOboe => (BundleId::PersonalMasterdatas, path_ids::WAZA_OBOE_TABLE),
            Table::Evolve => (BundleId::PersonalMasterdatas, path_ids::EVOLVE_TABLE),
            Table::TamagoWaza => (BundleId::PersonalMasterdatas, path_ids::TAMAGO_WAZA_TABLE),
            Table::Item => (BundleId::PersonalMasterdatas, path_ids::ITEM_TABLE),
            Table::Trainer => (BundleId::Masterdatas, path_ids::TRAINER_TABLE),
            Table::EncounterDiamond => (BundleId::Gamesettings, path_ids::FIELD_ENCOUNT_TABLE_D),
            Table::EncounterPearl => (BundleId::Gamesettings, path_ids::FIELD_ENCOUNT_TABLE_P),
            Table::Shop => (BundleId::Masterdatas, path_ids::SHOP_TABLE),
            Table::FieldItems => (BundleId::EvScript, path_ids::FLD_ITEM),
            Table::Trades => (BundleId::Masterdatas, path_ids::LOCAL_KOUKAN_DATA),
            Table::Pickup => (BundleId::Masterdatas, path_ids::MONOHIROI_TABLE),
            Table::MapInfo => (BundleId::Gamesettings, path_ids::MAP_INFO),
            Table::PokemonInfo => (BundleId::Masterdatas, path_ids::POKEMON_INFO),
        }
    }
}

pub struct Tables {
    values: Vec<Value>,
    underground: Vec<Value>,
    dirty: BTreeSet<Table>,
    dirty_underground: bool,
}

impl Tables {
    pub fn load(bundles: &Bundles) -> Result<Self> {
        let values = Table::ALL
            .iter()
            .map(|table| {
                let (bundle, path_id) = table.location();
                bundles.read(bundle, path_id)
            })
            .collect::<Result<Vec<_>>>()?;
        let underground = path_ids::UG_ENCOUNT_TABLES
            .iter()
            .map(|&path_id| bundles.read(BundleId::UgData, path_id))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            values,
            underground,
            dirty: BTreeSet::new(),
            dirty_underground: false,
        })
    }

    fn index(table: Table) -> usize {
        Table::ALL
            .iter()
            .position(|&candidate| candidate == table)
            .unwrap_or_default()
    }

    pub fn get(&self, table: Table) -> &Value {
        &self.values[Self::index(table)]
    }

    pub fn get_mut(&mut self, table: Table) -> &mut Value {
        self.dirty.insert(table);
        &mut self.values[Self::index(table)]
    }

    pub fn rows(&self, table: Table, field: &str) -> Result<&[Value]> {
        self.get(table)
            .field(field)?
            .as_array()
            .ok_or_else(|| data_error(format!("{field} is not an array")))
    }

    pub fn rows_mut(&mut self, table: Table, field: &str) -> Result<&mut Vec<Value>> {
        self.get_mut(table)
            .field_mut(field)?
            .as_array_mut()
            .ok_or_else(|| data_error(format!("{field} is not an array")))
    }

    pub fn underground(&self) -> &[Value] {
        &self.underground
    }

    pub fn underground_mut(&mut self) -> &mut [Value] {
        self.dirty_underground = true;
        &mut self.underground
    }

    pub fn write_back(&self, bundles: &mut Bundles) -> Result<()> {
        for &table in &self.dirty {
            let (bundle, path_id) = table.location();
            bundles.write(bundle, path_id, self.get(table))?;
        }
        if self.dirty_underground {
            for (&path_id, value) in path_ids::UG_ENCOUNT_TABLES.iter().zip(&self.underground) {
                bundles.write(BundleId::UgData, path_id, value)?;
            }
        }
        Ok(())
    }
}

pub fn int(value: &Value, field: &str) -> Result<i32> {
    let number = value.int_field(field)?;
    i32::try_from(number).map_err(|_| data_error(format!("{field} = {number} is out of range")))
}

pub fn set_int(value: &mut Value, field: &str, number: i32) -> Result<()> {
    value.set_int_field(field, i128::from(number))?;
    Ok(())
}

pub fn ints(value: &Value, field: &str) -> Result<Vec<i32>> {
    value
        .field(field)?
        .as_array()
        .ok_or_else(|| data_error(format!("{field} is not an array")))?
        .iter()
        .map(|item| {
            let number = item.int()?;
            i32::try_from(number)
                .map_err(|_| data_error(format!("{field} entry {number} is out of range")))
        })
        .collect()
}

pub fn set_u16s(value: &mut Value, field: &str, numbers: &[i32]) -> Result<()> {
    let array = value
        .field_mut(field)?
        .as_array_mut()
        .ok_or_else(|| data_error(format!("{field} is not an array")))?;
    *array = numbers
        .iter()
        .map(|&number| {
            u16::try_from(number)
                .map(Value::U16)
                .map_err(|_| data_error(format!("{field} entry {number} does not fit in 16 bits")))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(())
}
