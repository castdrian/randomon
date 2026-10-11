use std::collections::BTreeMap;

use randomon_core::{RandomizationError, Result};
use randomon_data::constants::gen5;
use randomon_data::{PickupItem, Shop};
use randomon_nds::{find, read_byte, read_relative_pointer, read_word, write_word};

use crate::handler::Gen5RomHandler;

const PICKUP_LEVEL_RANGES: usize = 10;
const PICKUP_COMMON_START: usize = 11;

struct ItemScript {
    file: i32,
    skip_bound: usize,
    skip_table: &'static [i32],
    set_var_command: i32,
    var_set: i32,
}

fn visit_item_offsets(
    script: &ItemScript,
    data: &mut [u8],
    mut visit: impl FnMut(&mut [u8], i32) -> Result<()>,
) -> Result<()> {
    let mut offset = 0;
    let mut skip_offset = 0;
    loop {
        if read_word(data, offset) == gen5::SCRIPT_LIST_TERMINATOR {
            return Ok(());
        }
        let offset_in_file = read_relative_pointer(data, offset);
        offset += 4;
        if offset_in_file > data.len() as i32 {
            return Ok(());
        }
        if skip_offset < script.skip_bound && script.skip_table[skip_offset] == offset / 4 - 1 {
            skip_offset += 1;
            continue;
        }
        let command = read_word(data, offset_in_file + 2);
        let variable = read_word(data, offset_in_file + 4);
        if command == script.set_var_command && variable == script.var_set {
            visit(data, offset_in_file + 6)?;
        }
    }
}

impl Gen5RomHandler {
    fn item_scripts(&self) -> [ItemScript; 2] {
        let normal_skip = self.entry.array("ItemBallsSkip").unwrap_or(&[]);
        let hidden_skip = self.entry.array("HiddenItemsSkip").unwrap_or(&[]);
        [
            ItemScript {
                file: self.entry.int("ItemBallsScriptOffset"),
                skip_bound: normal_skip.len(),
                skip_table: normal_skip,
                set_var_command: gen5::NORMAL_ITEM_SET_VAR_COMMAND,
                var_set: gen5::NORMAL_ITEM_VAR_SET,
            },
            ItemScript {
                file: self.entry.int("HiddenItemsScriptOffset"),
                skip_bound: normal_skip.len(),
                skip_table: hidden_skip,
                set_var_command: gen5::HIDDEN_ITEM_SET_VAR_COMMAND,
                var_set: gen5::HIDDEN_ITEM_VAR_SET,
            },
        ]
    }

    fn field_items(&self) -> Vec<i32> {
        let mut items = Vec::new();
        for script in self.item_scripts() {
            let mut data = self.script_narc.files[script.file as usize].clone();
            let read = visit_item_offsets(&script, &mut data, |data, offset| {
                items.push(read_word(data, offset));
                Ok(())
            });
            read.expect("reading field items has no failure path");
        }
        items
    }

    fn set_field_items(&mut self, items: &[i32]) -> Result<()> {
        let mut values = items.iter();
        for script in self.item_scripts() {
            let data = self
                .script_narc
                .files
                .get_mut(script.file as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("scriptNARC.files"))?;
            visit_item_offsets(&script, data, |data, offset| {
                let item = values
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("field items iterator"))?;
                write_word(data, offset, *item);
                Ok(())
            })?;
        }
        Ok(())
    }

    fn tm_from_index(index: i32) -> i32 {
        if (gen5::TM_BLOCK_ONE_OFFSET..gen5::TM_BLOCK_ONE_OFFSET + gen5::TM_BLOCK_ONE_COUNT)
            .contains(&index)
        {
            index - (gen5::TM_BLOCK_ONE_OFFSET - 1)
        } else {
            (index + gen5::TM_BLOCK_ONE_COUNT) - (gen5::TM_BLOCK_TWO_OFFSET - 1)
        }
    }

    fn index_from_tm(tm: i32) -> i32 {
        if (1..=gen5::TM_BLOCK_ONE_COUNT).contains(&tm) {
            tm + (gen5::TM_BLOCK_ONE_OFFSET - 1)
        } else {
            tm + (gen5::TM_BLOCK_TWO_OFFSET - 1 - gen5::TM_BLOCK_ONE_COUNT)
        }
    }

    pub(crate) fn current_field_tms(&self) -> Vec<i32> {
        let allowed = gen5::allowed_items();
        self.field_items()
            .into_iter()
            .filter(|&item| allowed.is_tm(item))
            .map(Self::tm_from_index)
            .collect()
    }

    pub(crate) fn set_field_tms_impl(&mut self, tms: &[i32]) -> Result<()> {
        let allowed = gen5::allowed_items();
        let mut items = self.field_items();
        let mut new_tms = tms.iter();
        for item in &mut items {
            if allowed.is_tm(*item) {
                let tm = new_tms
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("field TMs iterator"))?;
                *item = Self::index_from_tm(*tm);
            }
        }
        self.set_field_items(&items)
    }

    pub(crate) fn regular_field_items(&self) -> Vec<i32> {
        let allowed = gen5::allowed_items();
        self.field_items()
            .into_iter()
            .filter(|&item| allowed.is_allowed(item) && !allowed.is_tm(item))
            .collect()
    }

    pub(crate) fn set_regular_field_items_impl(&mut self, new_items: &[i32]) -> Result<()> {
        let allowed = gen5::allowed_items();
        let mut items = self.field_items();
        let mut replacements = new_items.iter();
        for item in &mut items {
            if !allowed.is_tm(*item) && allowed.is_allowed(*item) {
                *item = *replacements
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("field items iterator"))?;
            }
        }
        self.set_field_items(&items)
    }

    fn is_bad_shop(&self, shop: i32) -> bool {
        let tm_shops = self.entry.array("TMShops").unwrap_or(&[]);
        let regular_shops = self.entry.array("RegularShops").unwrap_or(&[]);
        tm_shops.contains(&shop) || regular_shops.contains(&shop)
    }

    fn shop_array(&self, key: &str) -> Result<&'static [i32]> {
        self.entry
            .array(key)
            .ok_or(RandomizationError::NullPointer("arrayEntries.get"))
    }

    pub(crate) fn shop_items(&self) -> Result<BTreeMap<i32, Shop>> {
        let offsets = self.shop_array("ShopItemOffsets")?;
        let sizes = self.shop_array("ShopItemSizes")?;
        self.shop_array("TMShops")?;
        self.shop_array("RegularShops")?;
        let overlay = self.read_overlay(self.entry.int("ShopItemOvlNumber"))?;
        let mut shops = BTreeMap::new();
        for index in 0..self.entry.int("ShopCount") {
            if self.is_bad_shop(index) {
                continue;
            }
            let slot = index as usize;
            let items = if self.is_bw2() {
                let shop = self
                    .shop_narc
                    .as_ref()
                    .and_then(|narc| narc.files.get(slot))
                    .ok_or(RandomizationError::IndexOutOfBounds("shopNarc.files"))?;
                (0..shop.len() as i32)
                    .step_by(2)
                    .map(|position| read_word(shop, position))
                    .collect()
            } else {
                let size = *sizes
                    .get(slot)
                    .ok_or(RandomizationError::IndexOutOfBounds("shopItemSizes"))?;
                let offset = *offsets
                    .get(slot)
                    .ok_or(RandomizationError::IndexOutOfBounds("shopItemOffsets"))?;
                (0..size)
                    .map(|item| read_word(&overlay, offset + item * 2))
                    .collect()
            };
            let name = self
                .shop_names
                .get(slot)
                .ok_or(RandomizationError::IndexOutOfBounds("shopNames"))?;
            shops.insert(
                index,
                Shop {
                    items,
                    name: (*name).to_owned(),
                    is_main_game: gen5::main_game_shops(self.rom_type()).contains(&index),
                },
            );
        }
        Ok(shops)
    }

    pub(crate) fn set_shop_items_impl(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()> {
        let offsets = self.shop_array("ShopItemOffsets")?;
        let sizes = self.shop_array("ShopItemSizes")?;
        self.shop_array("TMShops")?;
        self.shop_array("RegularShops")?;
        let overlay_number = self.entry.int("ShopItemOvlNumber");
        let mut overlay = self.read_overlay(overlay_number)?;
        for index in 0..self.entry.int("ShopCount") {
            if self.is_bad_shop(index) {
                continue;
            }
            let contents = &shops
                .get(&index)
                .ok_or(RandomizationError::NullPointer("shopItems.get"))?
                .items;
            let mut items = contents.iter();
            let mut next = || {
                items
                    .next()
                    .copied()
                    .ok_or(RandomizationError::IndexOutOfBounds("shop items iterator"))
            };
            let slot = index as usize;
            if self.is_bw2() {
                let shop = self
                    .shop_narc
                    .as_mut()
                    .and_then(|narc| narc.files.get_mut(slot))
                    .ok_or(RandomizationError::IndexOutOfBounds("shopNarc.files"))?;
                for position in (0..shop.len() as i32).step_by(2) {
                    write_word(shop, position, next()?);
                }
            } else {
                let size = *sizes
                    .get(slot)
                    .ok_or(RandomizationError::IndexOutOfBounds("shopItemSizes"))?;
                let offset = *offsets
                    .get(slot)
                    .ok_or(RandomizationError::IndexOutOfBounds("shopItemOffsets"))?;
                for item in 0..size {
                    write_word(&mut overlay, offset + item * 2, next()?);
                }
            }
        }
        if self.is_bw2() {
            let narc = self
                .shop_narc
                .as_ref()
                .ok_or(RandomizationError::NullPointer("shopNarc"))?;
            self.write_narc("ShopItems", narc)
        } else {
            self.write_overlay(overlay_number, &overlay)
        }
    }

    pub(crate) fn set_shop_prices_impl(&mut self) -> Result<()> {
        let mut narc = self.read_narc("ItemData")?;
        for index in 1..narc.files.len() {
            let price = gen5::balanced_item_price(index as i32)
                .ok_or(RandomizationError::NullPointer("balancedItemPrices.get"))?;
            write_word(&mut narc.files[index], 0, price);
        }
        self.write_narc("ItemData", &narc)
    }

    pub(crate) fn pickup_items(&self) -> Result<Vec<PickupItem>> {
        let overlay = self.read_overlay(self.entry.int("PickupOvlNumber"))?;
        if self.pickup_items_table_offset.get() == 0 {
            let offset = find(&overlay, gen5::PICKUP_TABLE_LOCATOR);
            if offset > 0 {
                self.pickup_items_table_offset.set(offset);
            }
        }
        let table = self.pickup_items_table_offset.get();
        let mut items = Vec::new();
        if table > 0 {
            for index in 0..gen5::NUMBER_OF_PICKUP_ITEMS {
                let offset = table + 2 * index;
                items.push(PickupItem::new(
                    read_byte(&overlay, offset) | (read_byte(&overlay, offset + 1) << 8),
                ));
            }
        }
        if !items.is_empty() {
            for level_range in 0..PICKUP_LEVEL_RANGES {
                let rare = level_range;
                let common = PICKUP_COMMON_START + level_range;
                items[common].probabilities[level_range] = 30;
                for offset in 1..7 {
                    items[common + offset].probabilities[level_range] = 10;
                }
                items[common + 7].probabilities[level_range] = 4;
                items[common + 8].probabilities[level_range] = 4;
                items[rare].probabilities[level_range] = 1;
                items[rare + 1].probabilities[level_range] = 1;
            }
        }
        Ok(items)
    }

    pub(crate) fn set_pickup_items_impl(&mut self, items: &[PickupItem]) -> Result<()> {
        let table = self.pickup_items_table_offset.get();
        if table <= 0 {
            return Ok(());
        }
        let number = self.entry.int("PickupOvlNumber");
        let mut overlay = self.read_overlay(number)?;
        for index in 0..gen5::NUMBER_OF_PICKUP_ITEMS {
            let item = items
                .get(index as usize)
                .ok_or(RandomizationError::IndexOutOfBounds("pickupItems"))?;
            write_word(&mut overlay, table + 2 * index, item.item);
        }
        self.write_overlay(number, &overlay)
    }
}
