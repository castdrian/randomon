use std::collections::BTreeMap;

use randomon_data::{IngameTrade, PickupItem, Shop};

use crate::constants as c;
use crate::handler::Gen3RomHandler;
use crate::rom_entry::RomType;

const TRADE_NICKNAME_LENGTH: i32 = 12;
const TRADE_OT_NAME_LENGTH: i32 = 11;
const SHOP_TERMINATOR: i32 = 0;
const PICKUP_LEVEL_RANGES: usize = 10;
const GIVEN_TM_CACHE: usize = 512;

impl Gen3RomHandler {
    fn field_item_values(&self) -> Vec<(i32, i32)> {
        self.item_offsets
            .iter()
            .map(|&offset| (offset, self.rom.word(offset)))
            .collect()
    }

    pub(crate) fn current_field_tms(&self) -> Vec<i32> {
        let allowed = c::allowed_items();
        let mut tms = Vec::new();
        for (_, item) in self.field_item_values() {
            if allowed.is_tm(item) {
                let tm = item - c::TM_ITEM_OFFSET + 1;
                if !tms.contains(&tm) {
                    tms.push(tm);
                }
            }
        }
        tms
    }

    pub(crate) fn write_field_tms(&mut self, tms: &[i32]) {
        let allowed = c::allowed_items();
        let mut new_tms = tms.iter();
        let mut given = vec![0i32; GIVEN_TM_CACHE];
        for (offset, item) in self.field_item_values() {
            if !allowed.is_tm(item) {
                continue;
            }
            let Some(cached) = given.get_mut(item as usize) else {
                continue;
            };
            if *cached != 0 {
                self.rom.set_byte(offset, *cached as u8);
                continue;
            }
            let Some(&tm) = new_tms.next() else {
                return;
            };
            let replacement = tm + c::TM_ITEM_OFFSET - 1;
            *cached = replacement;
            self.rom.set_word(offset, replacement);
        }
    }

    pub(crate) fn regular_field_items(&self) -> Vec<i32> {
        let allowed = c::allowed_items();
        self.field_item_values()
            .into_iter()
            .map(|(_, item)| item)
            .filter(|&item| allowed.is_allowed(item) && !allowed.is_tm(item))
            .collect()
    }

    pub(crate) fn write_regular_field_items(&mut self, items: &[i32]) {
        let allowed = c::allowed_items();
        let mut replacements = items.iter();
        for (offset, item) in self.field_item_values() {
            if allowed.is_allowed(item) && !allowed.is_tm(item) {
                let Some(&replacement) = replacements.next() else {
                    return;
                };
                self.rom.set_word(offset, replacement);
            }
        }
    }

    fn shop_item_count(&self, mut offset: i32) -> usize {
        let mut count = 0;
        while self.rom.contains_range(offset, 2) && self.rom.word(offset) != SHOP_TERMINATOR {
            count += 1;
            offset += 2;
        }
        count
    }

    pub(crate) fn shop_items(&self) -> BTreeMap<i32, Shop> {
        let names = c::shop_names(self.rom_type());
        let main_game = self.array("MainGameShops");
        let skip = self.array("SkipShops");
        let mut shops = BTreeMap::new();
        for (index, offset) in self.array("ShopItemOffsets").into_iter().enumerate() {
            let index = index as i32;
            if skip.contains(&index) {
                continue;
            }
            let items = (0..self.shop_item_count(offset) as i32)
                .map(|slot| self.rom.word(offset + slot * 2))
                .collect();
            let shop = Shop {
                items,
                name: names
                    .get(index as usize)
                    .map(|name| (*name).to_owned())
                    .unwrap_or_default(),
                is_main_game: main_game.contains(&index),
            };
            shops.insert(index, shop);
        }
        shops
    }

    pub(crate) fn write_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) {
        for (index, offset) in self.array("ShopItemOffsets").into_iter().enumerate() {
            let Some(shop) = shops.get(&(index as i32)) else {
                continue;
            };
            let capacity = self.shop_item_count(offset);
            for (slot, &item) in shop.items.iter().take(capacity).enumerate() {
                self.rom.set_word(offset + slot as i32 * 2, item);
            }
            if shop.items.len() < capacity {
                self.rom
                    .set_word(offset + shop.items.len() as i32 * 2, SHOP_TERMINATOR);
            }
        }
    }

    pub(crate) fn write_shop_prices(&mut self) {
        let item_data = self.value("ItemData");
        let entry_size = self.value("ItemEntrySize");
        for item in 1..self.value("ItemCount") {
            let Some(price) = c::balanced_item_price(item) else {
                continue;
            };
            let offset = item_data + item * entry_size + c::ITEM_DATA_PRICE_OFFSET;
            self.rom.set_word(offset, price * 10);
        }
    }

    fn pickup_entry_size(&self) -> i32 {
        if self.rom_type().is_emerald() { 2 } else { 4 }
    }

    pub(crate) fn locate_pickup_table(&mut self) {
        if self.pickup_items_table_offset != 0 {
            return;
        }
        let Some(locator) = self.entry.string("PickupTableStartLocator") else {
            return;
        };
        let offset = self.rom.find(locator);
        if offset > 0 {
            self.pickup_items_table_offset = offset;
        }
    }

    pub(crate) fn pickup_items(&self) -> Vec<PickupItem> {
        if self.pickup_items_table_offset <= 0 {
            return Vec::new();
        }
        let size = self.pickup_entry_size();
        let mut items: Vec<PickupItem> = (0..self.value("PickupItemCount"))
            .map(|index| {
                PickupItem::new(self.rom.word(self.pickup_items_table_offset + size * index))
            })
            .collect();
        if !items.is_empty() {
            assign_pickup_probabilities(&mut items, self.rom_type());
        }
        items
    }

    pub(crate) fn write_pickup_items(&mut self, items: &[PickupItem]) {
        if self.pickup_items_table_offset <= 0 {
            return;
        }
        let size = self.pickup_entry_size();
        for (index, item) in items.iter().enumerate() {
            let offset = self.pickup_items_table_offset + size * index as i32;
            self.rom.set_word(offset, item.item);
        }
    }

    fn trade_entries(&self) -> Vec<i32> {
        let table = self.value("TradeTableOffset");
        let unused = self.array("TradesUnused");
        let mut unused_index = 0;
        let mut entries = Vec::new();
        for entry in 0..self.value("TradeTableSize") {
            if unused.get(unused_index) == Some(&entry) {
                unused_index += 1;
                continue;
            }
            entries.push(table + entry * c::TRADE_ENTRY_LENGTH);
        }
        entries
    }

    pub(crate) fn ingame_trades(&self) -> Vec<IngameTrade> {
        self.trade_entries()
            .into_iter()
            .map(|offset| IngameTrade {
                nickname: self.read_variable_string(offset),
                given_pokemon: self.pokemon_or_fallback(self.rom.word(offset + 12)),
                ivs: (0..6)
                    .map(|index| self.rom.int(offset + 14 + index))
                    .collect(),
                ot_id: self.rom.word(offset + 24),
                item: self.rom.word(offset + 40),
                ot_name: self.read_variable_string(offset + 43),
                requested_pokemon: Some(self.pokemon_or_fallback(self.rom.word(offset + 56))),
                ..IngameTrade::default()
            })
            .collect()
    }

    pub(crate) fn write_ingame_trades(&mut self, trades: &[IngameTrade]) {
        for (offset, trade) in self.trade_entries().into_iter().zip(trades) {
            self.write_fixed_string(&trade.nickname, offset, TRADE_NICKNAME_LENGTH);
            let given = self.internal_of(trade.given_pokemon);
            self.rom.set_word(offset + 12, given);
            for (index, &iv) in trade.ivs.iter().take(6).enumerate() {
                self.rom.set_byte(offset + 14 + index as i32, iv as u8);
            }
            self.rom.set_word(offset + 24, trade.ot_id);
            self.rom.set_word(offset + 40, trade.item);
            self.write_fixed_string(&trade.ot_name, offset + 43, TRADE_OT_NAME_LENGTH);
            if let Some(requested) = trade.requested_pokemon {
                let requested = self.internal_of(requested);
                self.rom.set_word(offset + 56, requested);
            }
        }
    }
}

fn set_probability(items: &mut [PickupItem], index: usize, level_range: usize, value: i32) {
    if let Some(item) = items.get_mut(index) {
        item.probabilities[level_range] = value;
    }
}

fn assign_pickup_probabilities(items: &mut [PickupItem], rom_type: RomType) {
    for level_range in 0..PICKUP_LEVEL_RANGES {
        match rom_type {
            RomType::Ruby | RomType::Sapphire => {
                set_probability(items, 0, level_range, 30);
                set_probability(items, 7, level_range, 5);
                set_probability(items, 8, level_range, 4);
                set_probability(items, 9, level_range, 1);
                for index in 1..7 {
                    set_probability(items, index, level_range, 10);
                }
            }
            RomType::FireRedLeafGreen => {
                set_probability(items, 0, level_range, 15);
                for index in 1..7 {
                    set_probability(items, index, level_range, 10);
                }
                for index in 7..11 {
                    set_probability(items, index, level_range, 5);
                }
                for index in 11..16 {
                    set_probability(items, index, level_range, 1);
                }
            }
            RomType::Emerald => {
                let common = level_range;
                let rare = 18 + level_range;
                set_probability(items, common, level_range, 30);
                for index in 1..7 {
                    set_probability(items, common + index, level_range, 10);
                }
                set_probability(items, common + 7, level_range, 4);
                set_probability(items, common + 8, level_range, 4);
                set_probability(items, rare, level_range, 1);
                set_probability(items, rare + 1, level_range, 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emerald_pickup_probabilities_slide_with_the_level_range() {
        let mut items: Vec<PickupItem> = (0..29).map(PickupItem::new).collect();
        assign_pickup_probabilities(&mut items, RomType::Emerald);
        assert_eq!(items[0].probabilities[0], 30);
        assert_eq!(items[9].probabilities[9], 30);
        assert_eq!(items[17].probabilities[9], 4);
        assert_eq!(items[28].probabilities[9], 1);
        assert_eq!(items[0].probabilities[1], 0);
        for range in 0..PICKUP_LEVEL_RANGES {
            let total: i32 = items.iter().map(|item| item.probabilities[range]).sum();
            assert_eq!(total, 100);
        }
    }

    #[test]
    fn short_pickup_tables_never_panic() {
        let mut items: Vec<PickupItem> = (0..3).map(PickupItem::new).collect();
        assign_pickup_probabilities(&mut items, RomType::FireRedLeafGreen);
        assert_eq!(items[0].probabilities[0], 15);
    }
}
