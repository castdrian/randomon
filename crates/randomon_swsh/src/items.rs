use std::collections::{BTreeMap, HashMap, VecDeque};

use randomon_data::{ItemList, Shop};

use crate::constants;
use crate::formats::items::{TM_COUNT, pouch};
use crate::handler::SwshRomHandler;

const RANDOMIZABLE_POUCHES: &[u8] = &[
    pouch::MEDICINE,
    pouch::BALLS,
    pouch::BATTLE,
    pouch::BERRIES,
    pouch::ITEMS,
    pouch::TREASURES,
];
const SHOP_NAME_PREFIX: &str = "Shop";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldSlot {
    Visible {
        area: usize,
        zone: usize,
        item: usize,
    },
    Hidden {
        area: usize,
        zone: usize,
        item: usize,
        chance: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShopSlot {
    Single { shop: usize, index: usize },
    Multi { shop: usize, item: i32 },
}

fn is_named(name: Option<&String>) -> bool {
    name.is_some_and(|name| {
        let trimmed = name.trim();
        !trimmed.is_empty() && trimmed != "???" && !trimmed.starts_with('(')
    })
}

impl SwshRomHandler {
    fn item_is_valid(&self, item: i32) -> bool {
        let data = &self.files.data;
        let Ok(number) = u16::try_from(item) else {
            return false;
        };
        let Some(record) = data.items.record(number) else {
            return false;
        };
        item > 0
            && RANDOMIZABLE_POUCHES.contains(&record.pouch)
            && record.price > 0
            && data.item_hashes.hash_of(u32::from(number)).is_some()
            && is_named(self.item_names.get(item as usize))
    }

    pub(crate) fn load_items(&mut self) {
        let mut allowed = ItemList::new(constants::HIGHEST_ITEM_INDEX);
        for item in 0..=constants::HIGHEST_ITEM_INDEX as i32 {
            if !self.item_is_valid(item) || constants::DYNAMAX_CRYSTALS.contains(&item) {
                allowed.ban_singles(&[item]);
            }
        }
        let machines = &self.files.data.items.machines;
        self.tm_items = machines
            .iter()
            .take(TM_COUNT)
            .map(|machine| i32::from(machine.item))
            .collect();
        for machine in machines {
            allowed.tm_range(i32::from(machine.item), 1);
            allowed.ban_singles(&[i32::from(machine.item)]);
        }
        let mut non_bad = allowed.clone();
        constants::non_bad_bans(&mut non_bad);
        self.allowed_items = allowed;
        self.non_bad_items = non_bad;
    }

    pub(crate) fn read_tm_moves(&self) -> Vec<i32> {
        self.files
            .data
            .items
            .tm_moves()
            .into_iter()
            .map(i32::from)
            .collect()
    }

    pub(crate) fn read_tr_moves(&self) -> Vec<i32> {
        self.files
            .data
            .items
            .tr_moves()
            .into_iter()
            .map(i32::from)
            .collect()
    }

    fn encode_moves(moves: &[i32], current: Vec<u16>) -> Vec<u16> {
        current
            .iter()
            .enumerate()
            .map(|(index, &original)| {
                moves
                    .get(index)
                    .and_then(|&move_number| u16::try_from(move_number).ok())
                    .filter(|&move_number| move_number > 0)
                    .unwrap_or(original)
            })
            .collect()
    }

    pub(crate) fn write_tm_moves(&mut self, moves: &[i32]) -> randomon_core::Result<()> {
        let items = &mut self.files.data.items;
        let encoded = Self::encode_moves(moves, items.tm_moves());
        if encoded != items.tm_moves() {
            items
                .set_tm_moves(&encoded)
                .map_err(crate::handler::swsh_error)?;
        }
        Ok(())
    }

    pub(crate) fn write_tr_moves(&mut self, moves: &[i32]) -> randomon_core::Result<()> {
        let items = &mut self.files.data.items;
        let encoded = Self::encode_moves(moves, items.tr_moves());
        if encoded != items.tr_moves() {
            items
                .set_tr_moves(&encoded)
                .map_err(crate::handler::swsh_error)?;
        }
        Ok(())
    }

    fn field_slots(&self) -> Vec<(FieldSlot, u64)> {
        let mut slots = Vec::new();
        for (area, placement) in self.files.data.placement.iter().enumerate() {
            for (zone, contents) in placement.archive.zones.iter().enumerate() {
                for (item, field_item) in contents.field_items.iter().enumerate() {
                    if let Some(&hash) = field_item.item_hashes.first() {
                        slots.push((FieldSlot::Visible { area, zone, item }, hash));
                    }
                }
                for (item, hidden) in contents.hidden_items.iter().enumerate() {
                    for (chance, entry) in hidden.chances.iter().enumerate() {
                        slots.push((
                            FieldSlot::Hidden {
                                area,
                                zone,
                                item,
                                chance,
                            },
                            entry.hash,
                        ));
                    }
                }
            }
        }
        slots
    }

    fn field_items(&self) -> Vec<(FieldSlot, i32)> {
        let hashes = &self.files.data.item_hashes;
        self.field_slots()
            .into_iter()
            .filter_map(|(slot, hash)| {
                hashes
                    .item_of(hash)
                    .filter(|&item| item > 0)
                    .map(|item| (slot, item as i32))
            })
            .collect()
    }

    fn set_field_item(&mut self, slot: FieldSlot, item: i32) {
        let Some(hash) = u32::try_from(item)
            .ok()
            .and_then(|item| self.files.data.item_hashes.hash_of(item))
        else {
            return;
        };
        let placement = &mut self.files.data.placement;
        match slot {
            FieldSlot::Visible { area, zone, item } => {
                let target =
                    &mut placement[area].archive.zones[zone].field_items[item].item_hashes[0];
                if *target != hash {
                    *target = hash;
                }
            }
            FieldSlot::Hidden {
                area,
                zone,
                item,
                chance,
            } => {
                let target = &mut placement[area].archive.zones[zone].hidden_items[item].chances
                    [chance]
                    .hash;
                if *target != hash {
                    *target = hash;
                }
            }
        }
    }

    fn is_randomizable_field_item(&self, item: i32) -> bool {
        self.allowed_items.is_allowed(item) && !self.allowed_items.is_tm(item)
    }

    fn tm_number(&self, item: i32) -> Option<i32> {
        self.tm_items
            .iter()
            .position(|&candidate| candidate == item)
            .map(|index| index as i32 + 1)
    }

    pub(crate) fn current_field_tms(&self) -> Vec<i32> {
        let mut tms = Vec::new();
        for (_, item) in self.field_items() {
            if let Some(tm) = self.tm_number(item)
                && !tms.contains(&tm)
            {
                tms.push(tm);
            }
        }
        tms
    }

    pub(crate) fn write_field_tms(&mut self, field_tms: &[i32]) {
        let mut iterator = field_tms.iter();
        let mut replacements: HashMap<i32, i32> = HashMap::new();
        for (slot, item) in self.field_items() {
            if self.tm_number(item).is_none() {
                continue;
            }
            let replacement = match replacements.get(&item) {
                Some(&known) => known,
                None => {
                    let Some(&tm) = iterator.next() else {
                        continue;
                    };
                    let Some(&replacement) = self.tm_items.get(tm as usize - 1) else {
                        continue;
                    };
                    replacements.insert(item, replacement);
                    replacement
                }
            };
            self.set_field_item(slot, replacement);
        }
    }

    pub(crate) fn regular_field_items(&self) -> Vec<i32> {
        self.field_items()
            .into_iter()
            .map(|(_, item)| item)
            .filter(|&item| self.is_randomizable_field_item(item))
            .collect()
    }

    pub(crate) fn write_regular_field_items(&mut self, new_items: &[i32]) {
        let mut iterator = new_items.iter();
        for (slot, item) in self.field_items() {
            if !self.is_randomizable_field_item(item) {
                continue;
            }
            let Some(&replacement) = iterator.next() else {
                return;
            };
            if self.is_randomizable_field_item(replacement) {
                self.set_field_item(slot, replacement);
            }
        }
    }

    fn pouch_of(&self, item: i32) -> Option<u8> {
        self.files.data.items.pouch(u16::try_from(item).ok()?)
    }

    fn is_shop_item(&self, item: i32) -> bool {
        self.is_randomizable_field_item(item)
            && self
                .pouch_of(item)
                .is_some_and(|pouch| pouch != pouch::KEY && pouch != pouch::MACHINES)
    }

    fn shop_slots(&self) -> Vec<(String, Vec<(ShopSlot, i32)>)> {
        let shops = &self.files.data.shops;
        let mut groups = Vec::new();
        for (shop, single) in shops.single.iter().enumerate() {
            let slots: Vec<(ShopSlot, i32)> = single
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| self.is_shop_item(**item))
                .map(|(index, &item)| (ShopSlot::Single { shop, index }, item))
                .collect();
            if !slots.is_empty() {
                groups.push((format!("{SHOP_NAME_PREFIX} {:016X}", single.hash), slots));
            }
        }
        for (shop, multi) in shops.multi.iter().enumerate() {
            let mut distinct: Vec<i32> = Vec::new();
            for &item in multi.inventories.iter().flatten() {
                if self.is_shop_item(item) && !distinct.contains(&item) {
                    distinct.push(item);
                }
            }
            if !distinct.is_empty() {
                groups.push((
                    format!("Poké Mart {:016X}", multi.hash),
                    distinct
                        .into_iter()
                        .map(|item| (ShopSlot::Multi { shop, item }, item))
                        .collect(),
                ));
            }
        }
        groups
    }

    pub(crate) fn read_shop_items(&self) -> BTreeMap<i32, Shop> {
        self.shop_slots()
            .into_iter()
            .enumerate()
            .map(|(index, (name, slots))| {
                (
                    index as i32,
                    Shop {
                        items: slots.into_iter().map(|(_, item)| item).collect(),
                        name,
                        is_main_game: true,
                    },
                )
            })
            .collect()
    }

    fn proposals_by_pouch(&self, shops: &BTreeMap<i32, Shop>) -> BTreeMap<u8, VecDeque<i32>> {
        let mut queues: BTreeMap<u8, VecDeque<i32>> = BTreeMap::new();
        for &item in shops.values().flat_map(|shop| shop.items.iter()) {
            if !self.is_shop_item(item) {
                continue;
            }
            if let Some(pouch) = self.pouch_of(item) {
                queues.entry(pouch).or_default().push_back(item);
            }
        }
        queues
    }

    pub(crate) fn write_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) {
        let mut queues = self.proposals_by_pouch(shops);
        let groups = self.shop_slots();
        let mut multi_replacements: BTreeMap<(usize, i32), i32> = BTreeMap::new();
        for (_, slots) in groups {
            for (slot, original) in slots {
                let Some(pouch) = self.pouch_of(original) else {
                    continue;
                };
                let Some(replacement) = queues.get_mut(&pouch).and_then(VecDeque::pop_front) else {
                    continue;
                };
                match slot {
                    ShopSlot::Single { shop, index } => {
                        let target = &mut self.files.data.shops.single[shop].items[index];
                        if *target != replacement {
                            *target = replacement;
                        }
                    }
                    ShopSlot::Multi { shop, item } => {
                        multi_replacements.insert((shop, item), replacement);
                    }
                }
            }
        }
        for (shop, multi) in self.files.data.shops.multi.iter_mut().enumerate() {
            for inventory in multi.inventories.iter_mut() {
                for item in inventory.iter_mut() {
                    if let Some(&replacement) = multi_replacements.get(&(shop, *item)) {
                        *item = replacement;
                    }
                }
            }
        }
    }

    pub(crate) fn write_shop_prices(&mut self) {
        let items = &mut self.files.data.items;
        for number in 1..items.item_count() as u16 {
            let Some(price) = randomon_gen7::constants::balanced_item_price(i32::from(number))
            else {
                continue;
            };
            if !items.has_own_record(number) || items.price(number).unwrap_or(0) == 0 {
                continue;
            }
            let price = price.max(1) as u32;
            if items.price(number) != Some(price) {
                let _ = items.set_price(number, price);
            }
        }
    }
}
