use std::collections::{BTreeMap, HashMap};

use randomon_data::{ItemList, PickupItem, Shop};
use randomon_unity::Value;

use crate::constants;
use crate::error::{Result, data_error};
use crate::handler::BdspRomHandler;
use crate::tables::{Table, int, ints, set_int};

const ITEM_COMMANDS: [i64; 3] = [41, 187, 194];
const FLOAT_ARGUMENT: i64 = 1;
const COMMON_MART_ZONE: i32 = -1;

pub fn decode_item_argument(raw: i64) -> Option<i32> {
    let value = f32::from_bits(raw as u32);
    (value.fract() == 0.0 && (0.0..=f32::from(u16::MAX)).contains(&value)).then_some(value as i32)
}

pub fn encode_item_argument(item: i32) -> i128 {
    i128::from((item as f32).to_bits() as i32)
}

fn field_item_argument(script: &Value) -> Option<&Value> {
    let first = script.field("Commands").ok()?.as_array()?.first()?;
    let arguments = first.field("Arg").ok()?.as_array()?;
    if arguments.len() < 2 || !ITEM_COMMANDS.contains(&arguments[0].int_field("data").ok()?) {
        return None;
    }
    if arguments[1].int_field("argType").ok()? != FLOAT_ARGUMENT {
        return None;
    }
    arguments[1].field("data").ok()
}

fn field_item_argument_mut(script: &mut Value) -> Option<&mut Value> {
    field_item_argument(script)?;
    script
        .field_mut("Commands")
        .ok()?
        .as_array_mut()?
        .first_mut()?
        .field_mut("Arg")
        .ok()?
        .as_array_mut()?
        .get_mut(1)?
        .field_mut("data")
        .ok()
}

fn shop_key(group: usize, value: i32) -> String {
    match group {
        0 if value == COMMON_MART_ZONE => "Poké Mart".to_owned(),
        0 => format!("Zone {value} Mart"),
        _ => format!("Shop {value}"),
    }
}

impl BdspRomHandler {
    pub(crate) fn load_items(&mut self) -> Result<()> {
        let mut allowed = ItemList::new(constants::HIGHEST_ITEM_INDEX);
        let rows = self.tables.rows(Table::Item, "Item")?;
        let mut valid = vec![false; constants::HIGHEST_ITEM_INDEX + 1];
        for row in rows {
            let number = int(row, "no")?;
            let Some(slot) = valid.get_mut(number.max(0) as usize) else {
                continue;
            };
            let pocket = int(row, "fld_pocket")?;
            let price = int(row, "price")?;
            let name = self.item_names.get(number as usize).map(String::as_str);
            *slot = number > 0
                && constants::RANDOMIZABLE_POCKETS.contains(&pocket)
                && price > 0
                && !constants::UNRANDOMIZABLE_ITEMS.contains(&number)
                && name.is_some_and(|name| !name.is_empty() && name != "???");
        }
        for (number, &is_valid) in valid.iter().enumerate() {
            if !is_valid {
                allowed.ban_singles(&[number as i32]);
            }
        }
        self.tm_items = Vec::new();
        for machine in self.tables.rows(Table::Item, "WazaMachine")? {
            let number = int(machine, "machineNo")?;
            if (1..=constants::TM_COUNT).contains(&number) {
                self.tm_items.push(int(machine, "itemNo")?);
            }
        }
        for &item in &self.tm_items {
            allowed.tm_range(item, 1);
        }
        let mut non_bad = allowed.clone();
        constants::non_bad_bans(&mut non_bad);
        self.field_item_rules = allowed.clone();
        self.allowed_items = allowed;
        self.non_bad_items = non_bad;
        Ok(())
    }

    fn machine_rows(&self) -> Result<Vec<usize>> {
        Ok(self
            .tables
            .rows(Table::Item, "WazaMachine")?
            .iter()
            .enumerate()
            .filter(|(_, machine)| {
                int(machine, "machineNo")
                    .is_ok_and(|number| (1..=constants::TM_COUNT).contains(&number))
            })
            .map(|(index, _)| index)
            .collect())
    }

    pub(crate) fn read_tm_moves(&self) -> Result<Vec<i32>> {
        let machines = self.tables.rows(Table::Item, "WazaMachine")?;
        let mut moves = vec![0; constants::TM_COUNT as usize];
        for index in self.machine_rows()? {
            let number = int(&machines[index], "machineNo")?;
            moves[number as usize - 1] = int(&machines[index], "wazaNo")?;
        }
        Ok(moves)
    }

    pub(crate) fn write_tm_moves(&mut self, moves: &[i32]) -> Result<()> {
        let rows = self.machine_rows()?;
        let machines = self.tables.rows_mut(Table::Item, "WazaMachine")?;
        for index in rows {
            let number = int(&machines[index], "machineNo")?;
            if let Some(&move_number) = moves.get(number as usize - 1) {
                set_int(&mut machines[index], "wazaNo", move_number)?;
            }
        }
        Ok(())
    }

    fn tm_number(&self, item: i32) -> Option<i32> {
        self.tm_items
            .iter()
            .position(|&candidate| candidate == item)
            .map(|index| index as i32 + 1)
    }

    fn field_items(&self) -> Result<Vec<i32>> {
        Ok(self
            .tables
            .rows(Table::FieldItems, "Scripts")?
            .iter()
            .filter_map(field_item_argument)
            .filter_map(|argument| argument.int().ok().and_then(decode_item_argument))
            .collect())
    }

    fn write_field_items(&mut self, items: &[i32]) -> Result<()> {
        let mut iterator = items.iter();
        for script in self.tables.rows_mut(Table::FieldItems, "Scripts")? {
            let Some(argument) = field_item_argument_mut(script) else {
                continue;
            };
            if decode_item_argument(argument.int()?).is_none() {
                continue;
            }
            let item = iterator
                .next()
                .ok_or_else(|| data_error("fewer field items than item scripts"))?;
            argument.set_int(encode_item_argument(*item))?;
        }
        Ok(())
    }

    fn is_randomizable_field_item(&self, item: i32) -> bool {
        self.field_item_rules.is_allowed(item) && !self.field_item_rules.is_tm(item)
    }

    pub(crate) fn current_field_tms(&self) -> Result<Vec<i32>> {
        let mut tms = Vec::new();
        for item in self.field_items()? {
            if let Some(tm) = self.tm_number(item)
                && !tms.contains(&tm)
            {
                tms.push(tm);
            }
        }
        Ok(tms)
    }

    pub(crate) fn write_field_tms(&mut self, field_tms: &[i32]) -> Result<()> {
        let mut items = self.field_items()?;
        let mut iterator = field_tms.iter();
        let mut replacements: HashMap<i32, i32> = HashMap::new();
        for item in items.iter_mut() {
            if self.tm_number(*item).is_none() {
                continue;
            }
            if let Some(&replacement) = replacements.get(item) {
                *item = replacement;
                continue;
            }
            let Some(&tm) = iterator.next() else {
                continue;
            };
            let replacement = self.tm_items[tm as usize - 1];
            replacements.insert(*item, replacement);
            *item = replacement;
        }
        self.write_field_items(&items)
    }

    pub(crate) fn regular_field_items(&self) -> Result<Vec<i32>> {
        Ok(self
            .field_items()?
            .into_iter()
            .filter(|&item| self.is_randomizable_field_item(item))
            .collect())
    }

    pub(crate) fn write_regular_field_items(&mut self, new_items: &[i32]) -> Result<()> {
        let mut items = self.field_items()?;
        let mut iterator = new_items.iter();
        for item in items.iter_mut() {
            if self.is_randomizable_field_item(*item)
                && let Some(&replacement) = iterator.next()
            {
                *item = replacement;
            }
        }
        self.write_field_items(&items)
    }

    fn shop_groups(&self) -> Result<Vec<(String, Vec<usize>, &'static str)>> {
        let mut groups: Vec<(String, Vec<usize>, &'static str)> = Vec::new();
        for (group, (field, key)) in [("FS", "ZoneID"), ("FixedShop", "ShopID")]
            .into_iter()
            .enumerate()
        {
            let mut by_key: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
            for (index, row) in self.tables.rows(Table::Shop, field)?.iter().enumerate() {
                by_key.entry(int(row, key)?).or_default().push(index);
            }
            for (value, indices) in by_key {
                groups.push((shop_key(group, value), indices, field));
            }
        }
        Ok(groups)
    }

    pub(crate) fn read_shop_items(&self) -> Result<BTreeMap<i32, Shop>> {
        let mut shops = BTreeMap::new();
        for (index, (name, rows, field)) in self.shop_groups()?.into_iter().enumerate() {
            let table = self.tables.rows(Table::Shop, field)?;
            let items = rows
                .iter()
                .map(|&row| int(&table[row], "ItemNo"))
                .collect::<Result<Vec<_>>>()?;
            shops.insert(
                index as i32,
                Shop {
                    items,
                    name,
                    is_main_game: true,
                },
            );
        }
        Ok(shops)
    }

    pub(crate) fn write_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()> {
        for (index, (_, rows, field)) in self.shop_groups()?.into_iter().enumerate() {
            let Some(shop) = shops.get(&(index as i32)) else {
                continue;
            };
            let table = self.tables.rows_mut(Table::Shop, field)?;
            for (&row, &item) in rows.iter().zip(&shop.items) {
                set_int(&mut table[row], "ItemNo", item)?;
            }
        }
        Ok(())
    }

    pub(crate) fn write_shop_prices(&mut self) -> Result<()> {
        for row in self.tables.rows_mut(Table::Item, "Item")? {
            let number = int(row, "no")?;
            if let Some(price) = randomon_gen7::constants::balanced_item_price(number)
                && int(row, "price")? > 0
            {
                set_int(row, "price", price)?;
            }
        }
        Ok(())
    }

    pub(crate) fn read_pickup_items(&self) -> Result<Vec<PickupItem>> {
        self.tables
            .rows(Table::Pickup, "MonoHiroi")?
            .iter()
            .map(|row| {
                let mut pickup = PickupItem::new(int(row, "ID")?);
                pickup.probabilities = ints(row, "Ratios")?;
                Ok(pickup)
            })
            .collect()
    }

    pub(crate) fn write_pickup_items(&mut self, items: &[PickupItem]) -> Result<()> {
        for (row, pickup) in self
            .tables
            .rows_mut(Table::Pickup, "MonoHiroi")?
            .iter_mut()
            .zip(items)
        {
            set_int(row, "ID", pickup.item)?;
        }
        Ok(())
    }
}
