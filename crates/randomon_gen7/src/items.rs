use std::collections::{BTreeMap, HashMap};

use randomon_core::{Compatibility, RandomizationError, Result};
use randomon_ctr::bytes::{
    byte_from_flags, find, find_after_prefix, read_byte_into_flags, read_word,
    type_tm_palette_number, write_word,
};
use randomon_data::{MoveId, PickupItem, Shop};
use randomon_formats::ctr::Decompression;

use crate::constants;
use crate::handler::Gen7RomHandler;
use crate::pokemon::{garc_file, garc_file_mut};

fn no_such_element() -> RandomizationError {
    RandomizationError::randomization("java.util.NoSuchElementException")
}

fn tm_from_index(index: i32) -> i32 {
    if (constants::TM_BLOCK_ONE_OFFSET
        ..constants::TM_BLOCK_ONE_OFFSET + constants::TM_BLOCK_ONE_COUNT)
        .contains(&index)
    {
        index - (constants::TM_BLOCK_ONE_OFFSET - 1)
    } else if (constants::TM_BLOCK_TWO_OFFSET
        ..constants::TM_BLOCK_TWO_OFFSET + constants::TM_BLOCK_TWO_COUNT)
        .contains(&index)
    {
        (index + constants::TM_BLOCK_ONE_COUNT) - (constants::TM_BLOCK_TWO_OFFSET - 1)
    } else {
        (index + constants::TM_BLOCK_ONE_COUNT + constants::TM_BLOCK_TWO_COUNT)
            - (constants::TM_BLOCK_THREE_OFFSET - 1)
    }
}

fn index_from_tm(tm: i32) -> i32 {
    if (1..=constants::TM_BLOCK_ONE_COUNT).contains(&tm) {
        tm + (constants::TM_BLOCK_ONE_OFFSET - 1)
    } else if tm > constants::TM_BLOCK_ONE_COUNT
        && tm <= constants::TM_BLOCK_ONE_COUNT + constants::TM_BLOCK_TWO_COUNT
    {
        tm + (constants::TM_BLOCK_TWO_OFFSET - 1 - constants::TM_BLOCK_ONE_COUNT)
    } else {
        tm + (constants::TM_BLOCK_THREE_OFFSET
            - 1
            - (constants::TM_BLOCK_ONE_COUNT + constants::TM_BLOCK_TWO_COUNT))
    }
}

fn tm_item_blocks() -> [(i32, i32, usize); 3] {
    let one = constants::TM_BLOCK_ONE_COUNT as usize;
    let two = constants::TM_BLOCK_TWO_COUNT as usize;
    [
        (
            constants::TM_BLOCK_ONE_OFFSET,
            constants::TM_BLOCK_ONE_COUNT,
            0,
        ),
        (
            constants::TM_BLOCK_TWO_OFFSET,
            constants::TM_BLOCK_TWO_COUNT,
            one,
        ),
        (
            constants::TM_BLOCK_THREE_OFFSET,
            constants::TM_BLOCK_THREE_COUNT,
            one + two,
        ),
    ]
}

impl Gen7RomHandler {
    pub(crate) fn read_tm_moves(&self) -> Vec<i32> {
        let prefix = constants::tm_data_prefix(self.rom_type);
        let offset = find(&self.code, prefix);
        if offset == 0 {
            return Vec::new();
        }
        let offset = (offset + prefix.len() as i32 / 2) as usize;
        (0..constants::TM_COUNT as usize)
            .map(|index| read_word(&self.code, offset + index * 2))
            .collect()
    }

    pub(crate) fn write_tm_moves(&mut self, move_indexes: &[i32]) -> Result<()> {
        let prefix = constants::tm_data_prefix(self.rom_type);
        let Some(offset) = find_after_prefix(&self.code, prefix) else {
            return Ok(());
        };
        for (index, &move_number) in move_indexes
            .iter()
            .take(constants::TM_COUNT as usize)
            .enumerate()
        {
            write_word(&mut self.code, offset + index * 2, move_number);
        }
        let mut item_descriptions =
            self.strings(false, self.entry.int("ItemDescriptionsTextOffset"))?;
        let move_descriptions =
            self.strings(false, self.entry.int("MoveDescriptionsTextOffset"))?;
        for (item_offset, count, move_offset) in tm_item_blocks() {
            for index in 0..count as usize {
                let description = move_descriptions
                    .get(move_indexes[index + move_offset] as usize)
                    .cloned()
                    .ok_or(RandomizationError::IndexOutOfBounds("moveDescriptions"))?;
                *item_descriptions
                    .get_mut(index + item_offset as usize)
                    .ok_or(RandomizationError::IndexOutOfBounds("itemDescriptions"))? = description;
            }
        }
        self.set_strings(
            false,
            self.entry.int("ItemDescriptionsTextOffset"),
            &item_descriptions,
        )?;
        if let Some(palettes) = find_after_prefix(&self.code, constants::ITEM_PALETTES_PREFIX) {
            for (item_offset, count, move_offset) in tm_item_blocks() {
                for index in 0..count as usize {
                    let item_number = item_offset as usize + index;
                    let move_number = move_indexes[index + move_offset];
                    let kind = self
                        .core
                        .data
                        .mv(MoveId::from_index(move_number as usize))
                        .kind;
                    let palette = type_tm_palette_number(kind, true);
                    write_word(&mut self.code, palettes + item_number * 4, palette);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn read_tmhm_compatibility(&self) -> Compatibility {
        let mut compatibility = BTreeMap::new();
        for number in 1..=self.total_pokemon() {
            let data = self
                .poke_garc
                .file(number as usize)
                .expect("pokemon stats exist");
            let mut flags = vec![false; constants::TM_COUNT as usize + 1];
            for byte in 0..13 {
                read_byte_into_flags(
                    data,
                    &mut flags,
                    byte * 8 + 1,
                    constants::BS_TMHM_COMPAT_OFFSET + byte,
                );
            }
            compatibility.insert(self.pk(number), flags);
        }
        compatibility
    }

    pub(crate) fn write_tmhm_compatibility(&mut self, compatibility: &Compatibility) {
        for (pokemon, flags) in compatibility {
            let data =
                garc_file_mut(&mut self.poke_garc, pokemon.index()).expect("pokemon stats exist");
            for byte in 0..13 {
                data[constants::BS_TMHM_COMPAT_OFFSET + byte] =
                    byte_from_flags(flags, byte * 8 + 1);
            }
        }
    }

    pub(crate) fn read_move_tutor_moves(&self) -> Vec<i32> {
        let mut moves = Vec::new();
        let Some(mut offset) = find_after_prefix(&self.code, constants::TUTORS_PREFIX) else {
            return moves;
        };
        let mut value = 0;
        while value != 0xFFFF {
            value = read_word(&self.code, offset);
            offset += 2;
            if value != 0xFFFF {
                moves.push(value);
            }
        }
        moves
    }

    pub(crate) fn write_move_tutor_moves(&mut self, moves: &[i32]) -> Result<()> {
        if let Some(mut offset) = find_after_prefix(&self.code, constants::TUTORS_PREFIX) {
            for &move_number in moves {
                write_word(&mut self.code, offset, move_number);
                offset += 2;
            }
        }
        let path = self.file("ShopsAndTutors");
        let mut tutor_cro = self.read_file(path)?;
        for (index, &move_number) in moves.iter().enumerate() {
            write_word(
                &mut tutor_cro,
                constants::TUTORS_OFFSET + index * 4,
                move_number,
            );
        }
        self.write_file(path, &tutor_cro)
    }

    pub(crate) fn read_move_tutor_compatibility(&self) -> Compatibility {
        let mut compatibility = BTreeMap::new();
        for number in 1..=self.total_pokemon() {
            let data = self
                .poke_garc
                .file(number as usize)
                .expect("pokemon stats exist");
            let mut flags = vec![false; constants::TUTOR_MOVE_COUNT + 1];
            for byte in 0..10 {
                read_byte_into_flags(
                    data,
                    &mut flags,
                    byte * 8 + 1,
                    constants::BS_MT_COMPAT_OFFSET + byte,
                );
            }
            compatibility.insert(self.pk(number), flags);
        }
        compatibility
    }

    pub(crate) fn write_move_tutor_compatibility(&mut self, compatibility: &Compatibility) {
        if !self.is_usum() {
            return;
        }
        for number in 1..=self.total_pokemon() {
            let pokemon = self.pk(number);
            let flags = compatibility
                .get(&pokemon)
                .expect("compatibility for every pokemon");
            let data =
                garc_file_mut(&mut self.poke_garc, number as usize).expect("pokemon stats exist");
            for byte in 0..10 {
                data[constants::BS_MT_COMPAT_OFFSET + byte] = byte_from_flags(flags, byte * 8 + 1);
            }
        }
    }

    pub(crate) fn current_field_tms(&self) -> Result<Vec<i32>> {
        let allowed = constants::allowed_items(self.rom_type);
        let mut tms = Vec::new();
        for item in self.field_items()? {
            if allowed.is_tm(item) {
                let tm = tm_from_index(item);
                if !tms.contains(&tm) {
                    tms.push(tm);
                }
            }
        }
        Ok(tms)
    }

    pub(crate) fn write_field_tms(&mut self, field_tms: &[i32]) -> Result<()> {
        let mut items = self.field_items()?;
        let allowed = constants::allowed_items(self.rom_type);
        let mut iterator = field_tms.iter();
        let mut replacements: HashMap<i32, i32> = HashMap::new();
        for item in items.iter_mut() {
            if !allowed.is_tm(*item) {
                continue;
            }
            if let Some(&replacement) = replacements.get(item) {
                *item = replacement;
                continue;
            }
            let new_item = index_from_tm(*iterator.next().ok_or_else(no_such_element)?);
            replacements.insert(*item, new_item);
            *item = new_item;
        }
        self.write_field_items(&items)
    }

    pub(crate) fn regular_field_items(&self) -> Result<Vec<i32>> {
        let allowed = constants::allowed_items(self.rom_type);
        Ok(self
            .field_items()?
            .into_iter()
            .filter(|&item| allowed.is_allowed(item) && !allowed.is_tm(item))
            .collect())
    }

    pub(crate) fn write_regular_field_items(&mut self, new_items: &[i32]) -> Result<()> {
        let mut items = self.field_items()?;
        let allowed = constants::allowed_items(self.rom_type);
        let mut iterator = new_items.iter();
        for item in items.iter_mut() {
            if !allowed.is_tm(*item) && allowed.is_allowed(*item) {
                *item = *iterator.next().ok_or_else(no_such_element)?;
            }
        }
        self.write_field_items(&items)
    }

    fn is_skipped_shop(&self, shop: i32) -> bool {
        self.entry.array("TMShops").contains(&shop)
            || self.entry.array("RegularShops").contains(&shop)
    }

    pub(crate) fn read_shop_items(&self) -> Result<BTreeMap<i32, Shop>> {
        let sizes = self.entry.array("ShopItemSizes");
        let shops_cro = self.read_file(self.file("ShopsAndTutors"))?;
        let mut offset = constants::shop_items_offset(self.rom_type);
        let mut shops = BTreeMap::new();
        for shop in 0..self.entry.int("ShopCount") {
            let size = sizes[shop as usize] as usize;
            if self.is_skipped_shop(shop) {
                offset += size * 2;
                continue;
            }
            let items = (0..size)
                .map(|index| read_word(&shops_cro, offset + index * 2))
                .collect();
            offset += size * 2;
            shops.insert(
                shop,
                Shop {
                    items,
                    name: self.shop_names[shop as usize].clone(),
                    is_main_game: constants::main_game_shops(self.rom_type).contains(&shop),
                },
            );
        }
        Ok(shops)
    }

    pub(crate) fn write_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()> {
        let sizes = self.entry.array("ShopItemSizes");
        let path = self.file("ShopsAndTutors");
        let mut shops_cro = self.read_file(path)?;
        let mut offset = constants::shop_items_offset(self.rom_type);
        for shop in 0..self.entry.int("ShopCount") {
            let size = sizes[shop as usize] as usize;
            if self.is_skipped_shop(shop) {
                offset += size * 2;
                continue;
            }
            let contents = &shops
                .get(&shop)
                .ok_or(RandomizationError::NullPointer("shopItems.get"))?
                .items;
            let mut iterator = contents.iter();
            for _ in 0..size {
                let item = *iterator.next().ok_or_else(no_such_element)?;
                write_word(&mut shops_cro, offset, item);
                offset += 2;
            }
        }
        self.write_file(path, &shops_cro)
    }

    pub(crate) fn write_shop_prices(&self) -> Result<()> {
        let path = self.file("ItemData");
        let mut garc = self.read_garc(path, Decompression::Skip)?;
        for item in 1..garc.files.len() {
            let price = constants::balanced_item_price(item as i32)
                .ok_or(RandomizationError::NullPointer("balancedItemPrices.get"))?;
            write_word(garc_file_mut(&mut garc, item)?, 0, price);
        }
        self.write_garc(path, &garc)
    }

    pub(crate) fn read_pickup_items(&self) -> Result<Vec<PickupItem>> {
        let garc = self.read_garc(self.file("PickupData"), Decompression::All)?;
        let data = garc_file(&garc, 0)?;
        let count = randomon_ctr::bytes::read_long(data, 0) - 1;
        let mut pickup = Vec::new();
        for index in 0..count.max(0) as usize {
            let offset = 4 + index * 0xC;
            let mut item = PickupItem::new(read_word(data, offset));
            for level_range in 0..10 {
                item.probabilities[level_range] = i32::from(data[offset + level_range + 2] as i8);
            }
            pickup.push(item);
        }
        Ok(pickup)
    }

    pub(crate) fn write_pickup_items(&self, pickup: &[PickupItem]) -> Result<()> {
        let path = self.file("PickupData");
        let mut garc = self.read_garc(path, Decompression::All)?;
        let data = garc_file_mut(&mut garc, 0)?;
        for (index, item) in pickup.iter().enumerate() {
            write_word(data, 4 + index * 0xC, item.item);
        }
        self.write_garc(path, &garc)
    }
}
