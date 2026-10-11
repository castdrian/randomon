use std::collections::BTreeMap;

use randomon_core::{RandomizationError, Result};
use randomon_data::constants::gen4;
use randomon_data::{PickupItem, Shop};

use crate::bytes::{after_prefix, file, file_mut, found_at, relative_pointer_at, set_u16, u16_at};
use crate::handler::Gen4RomHandler;

const SHOP_TERMINATOR: i32 = 0xFFFF;
const MAX_SCRIPT_POINTERS: i32 = 0x10000;
const LEVEL_RANGES: usize = 10;
const RARE_PICKUP_START: usize = 18;

#[derive(Clone, Copy)]
struct FieldItemSlot {
    in_arm9: bool,
    offset: i32,
}

fn ran_out(what: &'static str) -> RandomizationError {
    RandomizationError::IndexOutOfBounds(what)
}

impl Gen4RomHandler {
    fn set_var_script(&self) -> i32 {
        if self.entry.is_hgss() {
            gen4::HGSS_SET_VAR_SCRIPT
        } else {
            gen4::DPPT_SET_VAR_SCRIPT
        }
    }

    fn field_item_slots(&self) -> Result<Vec<FieldItemSlot>> {
        let scripts = file(&self.script_narc, self.entry.int("ItemBallsScriptOffset"))?;
        let skip_table = self.entry.array_or_empty("ItemBallsSkip");
        let set_var = self.set_var_script();
        let mut slots = Vec::new();
        let mut offset = 0;
        let mut skip_index = 0;
        for _ in 0..MAX_SCRIPT_POINTERS {
            if u16_at(scripts, offset)? == gen4::SCRIPT_LIST_TERMINATOR {
                break;
            }
            let offset_in_file = relative_pointer_at(scripts, offset)?;
            offset += 4;
            if skip_table.get(skip_index) == Some(&(offset / 4 - 1)) {
                skip_index += 1;
                continue;
            }
            let command = u16_at(scripts, offset_in_file);
            let variable = u16_at(scripts, offset_in_file + 2);
            if command.ok() == Some(set_var) && variable.ok() == Some(gen4::ITEM_SCRIPT_VARIABLE) {
                slots.push(FieldItemSlot {
                    in_arm9: false,
                    offset: offset_in_file + 4,
                });
            }
        }
        let hidden_table = self.entry.int("HiddenItemTableOffset");
        for index in 0..self.entry.int("HiddenItemCount") {
            slots.push(FieldItemSlot {
                in_arm9: true,
                offset: hidden_table + index * 8,
            });
        }
        Ok(slots)
    }

    fn field_items(&self) -> Result<Vec<i32>> {
        let scripts = file(&self.script_narc, self.entry.int("ItemBallsScriptOffset"))?;
        self.field_item_slots()?
            .into_iter()
            .map(|slot| {
                let data = if slot.in_arm9 {
                    &self.arm9[..]
                } else {
                    &scripts[..]
                };
                u16_at(data, slot.offset)
            })
            .collect()
    }

    fn set_field_items(&mut self, items: &[i32]) -> Result<()> {
        let slots = self.field_item_slots()?;
        if slots.len() > items.len() {
            return Err(ran_out("fieldItems"));
        }
        let script_index = self.entry.int("ItemBallsScriptOffset");
        let mut arm9 = self.arm9.clone();
        let mut scripts = file(&self.script_narc, script_index)?.clone();
        for (slot, &item) in slots.iter().zip(items) {
            let data = if slot.in_arm9 {
                &mut arm9
            } else {
                &mut scripts
            };
            set_u16(data, slot.offset, item)?;
        }
        self.arm9 = arm9;
        *file_mut(&mut self.script_narc, script_index)? = scripts;
        Ok(())
    }

    pub(crate) fn current_field_tms(&self) -> Result<Vec<i32>> {
        let allowed = gen4::allowed_items();
        Ok(self
            .field_items()?
            .into_iter()
            .filter(|&item| allowed.is_tm(item))
            .map(|item| item - gen4::TM_ITEM_OFFSET + 1)
            .collect())
    }

    pub(crate) fn set_field_tms_impl(&mut self, tms: &[i32]) -> Result<()> {
        let allowed = gen4::allowed_items();
        let mut field_items = self.field_items()?;
        let mut new_tms = tms.iter();
        for item in &mut field_items {
            if allowed.is_tm(*item) {
                *item =
                    new_tms.next().ok_or_else(|| ran_out("fieldTMs"))? + gen4::TM_ITEM_OFFSET - 1;
            }
        }
        self.set_field_items(&field_items)
    }

    pub(crate) fn regular_field_items(&self) -> Result<Vec<i32>> {
        let allowed = gen4::allowed_items();
        Ok(self
            .field_items()?
            .into_iter()
            .filter(|&item| allowed.is_allowed(item) && !allowed.is_tm(item))
            .collect())
    }

    pub(crate) fn set_regular_field_items_impl(&mut self, items: &[i32]) -> Result<()> {
        let allowed = gen4::allowed_items();
        let mut field_items = self.field_items()?;
        let mut new_items = items.iter();
        for item in &mut field_items {
            if allowed.is_allowed(*item) && !allowed.is_tm(*item) {
                *item = *new_items.next().ok_or_else(|| ran_out("fieldItems"))?;
            }
        }
        self.set_field_items(&field_items)
    }

    fn shop_data_offset(&self) -> Option<i32> {
        after_prefix(&self.arm9, self.entry.string("ShopDataPrefix"))
    }

    fn shop_item_offsets(&self, offset: &mut i32) -> Result<Vec<i32>> {
        let mut offsets = Vec::new();
        loop {
            let value = u16_at(&self.arm9, *offset)?;
            if value == SHOP_TERMINATOR {
                *offset += 2;
                return Ok(offsets);
            }
            if value != 0 {
                offsets.push(*offset);
            }
            *offset += 2;
        }
    }

    pub(crate) fn shop_items(&self) -> Result<BTreeMap<i32, Shop>> {
        let names = gen4::shop_names(self.rom_type());
        let main_game = self.entry.array_or_empty("MainGameShops");
        let skip = self.entry.array_or_empty("SkipShops");
        let mut shops = BTreeMap::new();
        let Some(mut offset) = self.shop_data_offset() else {
            return Ok(shops);
        };
        for index in 0..self.entry.int("ShopCount") {
            let item_offsets = self.shop_item_offsets(&mut offset)?;
            if skip.contains(&index) {
                continue;
            }
            let items = item_offsets
                .iter()
                .map(|&item| u16_at(&self.arm9, item))
                .collect::<Result<Vec<_>>>()?;
            shops.insert(
                index,
                Shop {
                    items,
                    name: names
                        .get(index as usize)
                        .copied()
                        .unwrap_or("Unknown Shop")
                        .to_owned(),
                    is_main_game: main_game.contains(&index),
                },
            );
        }
        Ok(shops)
    }

    pub(crate) fn set_shop_items_impl(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()> {
        let Some(mut offset) = self.shop_data_offset() else {
            return Ok(());
        };
        let mut arm9 = self.arm9.clone();
        for index in 0..self.entry.int("ShopCount") {
            let item_offsets = self.shop_item_offsets(&mut offset)?;
            let Some(shop) = shops.get(&index) else {
                continue;
            };
            for (&item_offset, &item) in item_offsets.iter().zip(&shop.items) {
                set_u16(&mut arm9, item_offset, item)?;
            }
        }
        self.arm9 = arm9;
        Ok(())
    }

    pub(crate) fn set_shop_prices_impl(&mut self) -> Result<()> {
        let start_of_unused_ids = if self.entry.is_dp() { 112 } else { 113 };
        let mut prices = self.read_narc("ItemData")?;
        let mut item_id = 1;
        for index in 1..prices.files.len() as i32 {
            if let Some(price) = gen4::balanced_item_price(item_id) {
                set_u16(file_mut(&mut prices, index)?, 0, price * 10)?;
            }
            item_id += 1;
            if item_id == start_of_unused_ids {
                item_id = 135;
            }
        }
        self.write_narc("ItemData", &prices)
    }

    fn locate_pickup_tables(&self, overlay: &[u8]) {
        if self.pickup_items_table_offset.get() == 0
            && let Some(offset) = found_at(overlay, gen4::PICKUP_TABLE_LOCATOR)
        {
            self.pickup_items_table_offset.set(offset);
        }
        if self.rare_pickup_items_table_offset.get() == 0
            && let Some(offset) = found_at(overlay, gen4::RARE_PICKUP_TABLE_LOCATOR)
        {
            self.rare_pickup_items_table_offset.set(offset);
        }
    }

    fn pickup_tables_found(&self) -> bool {
        self.pickup_items_table_offset.get() > 0 && self.rare_pickup_items_table_offset.get() > 0
    }

    fn pickup_slots(&self) -> Vec<i32> {
        let common = (0..gen4::NUMBER_OF_COMMON_PICKUP_ITEMS)
            .map(|index| self.pickup_items_table_offset.get() + 2 * index);
        let rare = (0..gen4::NUMBER_OF_RARE_PICKUP_ITEMS)
            .map(|index| self.rare_pickup_items_table_offset.get() + 2 * index);
        common.chain(rare).collect()
    }

    pub(crate) fn pickup_items(&self) -> Result<Vec<PickupItem>> {
        let overlay = self.read_overlay(self.entry.int("BattleOvlNumber"))?;
        self.locate_pickup_tables(&overlay);
        if !self.pickup_tables_found() {
            return Ok(Vec::new());
        }
        let mut items = self
            .pickup_slots()
            .into_iter()
            .map(|offset| u16_at(&overlay, offset).map(PickupItem::new))
            .collect::<Result<Vec<_>>>()?;
        for level_range in 0..LEVEL_RANGES {
            items[level_range].probabilities[level_range] = 30;
            for slot in 1..7 {
                items[level_range + slot].probabilities[level_range] = 10;
            }
            items[level_range + 7].probabilities[level_range] = 4;
            items[level_range + 8].probabilities[level_range] = 4;
            items[RARE_PICKUP_START + level_range].probabilities[level_range] = 1;
            items[RARE_PICKUP_START + level_range + 1].probabilities[level_range] = 1;
        }
        Ok(items)
    }

    pub(crate) fn set_pickup_items_impl(&mut self, items: &[PickupItem]) -> Result<()> {
        if !self.pickup_tables_found() {
            return Ok(());
        }
        let number = self.entry.int("BattleOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        let slots = self.pickup_slots();
        if items.len() < slots.len() {
            return Err(ran_out("pickupItems"));
        }
        for (offset, item) in slots.into_iter().zip(items) {
            set_u16(&mut overlay, offset, item.item)?;
        }
        self.write_overlay(number, &overlay)
    }
}
