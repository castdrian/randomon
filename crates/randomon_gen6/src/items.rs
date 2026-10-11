use std::collections::{BTreeMap, HashMap};

use randomon_core::{Compatibility, RandomizationError, Result};
use randomon_ctr::bytes::{
    byte_from_flags, find, find_after_prefix, read_byte_into_flags, read_word,
    type_tm_palette_number, write_word,
};
use randomon_data::constants::items;
use randomon_data::{IngameTrade, MoveId, PickupItem, Shop};
use randomon_formats::ctr::{Amx, Decompression, GarcArchive};

use crate::constants;
use crate::handler::{Gen6RomHandler, io};
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

impl Gen6RomHandler {
    fn tm_data_offset(&self) -> i32 {
        find(&self.code, constants::TM_DATA_PREFIX)
    }

    pub(crate) fn read_tm_moves(&self) -> Vec<i32> {
        let offset = self.tm_data_offset();
        if offset == 0 {
            return Vec::new();
        }
        let mut offset = (offset + constants::TM_DATA_PREFIX.len() as i32 / 2) as usize;
        let mut tms: Vec<i32> = (0..constants::TM_BLOCK_ONE_COUNT as usize)
            .map(|index| read_word(&self.code, offset + index * 2))
            .collect();
        offset += (constants::tm_block_two_starting_offset(self.rom_type) * 2) as usize;
        let remaining = (constants::TM_COUNT - constants::TM_BLOCK_ONE_COUNT) as usize;
        tms.extend((0..remaining).map(|index| read_word(&self.code, offset + index * 2)));
        tms
    }

    pub(crate) fn read_hm_moves(&self) -> Vec<i32> {
        let offset = self.tm_data_offset();
        if offset == 0 {
            return Vec::new();
        }
        let offset = (offset
            + constants::TM_DATA_PREFIX.len() as i32 / 2
            + constants::TM_BLOCK_ONE_COUNT * 2) as usize;
        let mut hms: Vec<i32> = (0..constants::HM_BLOCK_ONE_COUNT as usize)
            .map(|index| read_word(&self.code, offset + index * 2))
            .collect();
        if self.is_oras() {
            hms.push(read_word(
                &self.code,
                offset + constants::ROCK_SMASH_OFFSET_ORAS as usize,
            ));
            hms.push(read_word(
                &self.code,
                offset + constants::DIVE_OFFSET_ORAS as usize,
            ));
        }
        hms
    }

    pub(crate) fn write_tm_moves(&mut self, move_indexes: &[i32]) -> Result<()> {
        let Some(mut offset) = find_after_prefix(&self.code, constants::TM_DATA_PREFIX) else {
            return Ok(());
        };
        let block_one = constants::TM_BLOCK_ONE_COUNT as usize;
        for (index, &move_number) in move_indexes.iter().take(block_one).enumerate() {
            write_word(&mut self.code, offset + index * 2, move_number);
        }
        offset += (constants::tm_block_two_starting_offset(self.rom_type) * 2) as usize;
        let remaining = constants::TM_COUNT as usize - block_one;
        for (index, &move_number) in move_indexes
            .iter()
            .skip(block_one)
            .take(remaining)
            .enumerate()
        {
            write_word(&mut self.code, offset + index * 2, move_number);
        }
        self.update_tm_descriptions(move_indexes)?;
        self.update_tm_palettes(move_indexes);
        Ok(())
    }

    fn tm_item_blocks(&self) -> [(i32, i32, usize); 3] {
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

    fn update_tm_descriptions(&mut self, move_indexes: &[i32]) -> Result<()> {
        let mut item_descriptions =
            self.strings(false, self.entry.int("ItemDescriptionsTextOffset"))?;
        let move_descriptions =
            self.strings(false, self.entry.int("MoveDescriptionsTextOffset"))?;
        for (item_offset, count, move_offset) in self.tm_item_blocks() {
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
        )
    }

    fn update_tm_palettes(&mut self, move_indexes: &[i32]) {
        let Some(palettes) = find_after_prefix(&self.code, constants::ITEM_PALETTES_PREFIX) else {
            return;
        };
        for (item_offset, count, move_offset) in self.tm_item_blocks() {
            for index in 0..count as usize {
                let item_number = item_offset as usize + index;
                let move_number = move_indexes[index + move_offset];
                let kind = self
                    .core
                    .data
                    .mv(MoveId::from_index(move_number as usize))
                    .kind;
                let palette = type_tm_palette_number(kind, false);
                write_word(&mut self.code, palettes + item_number * 4, palette);
            }
        }
    }

    pub(crate) fn read_tmhm_compatibility(&self) -> Compatibility {
        let mut compatibility = BTreeMap::new();
        let flag_count = (constants::TM_COUNT + constants::hm_count(self.rom_type) + 1) as usize;
        for number in 1..=self.total_pokemon() {
            let data = self
                .poke_garc
                .file(number as usize)
                .expect("pokemon stats exist");
            let mut flags = vec![false; flag_count];
            for byte in 0..14 {
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
            for byte in 0..14 {
                data[constants::BS_TMHM_COMPAT_OFFSET + byte] =
                    byte_from_flags(flags, byte * 8 + 1);
            }
        }
    }

    fn move_tutor_moves_offset(&self) -> i32 {
        let mut offset = self.move_tutor_moves_offset.get();
        if offset == 0 {
            offset = find(&self.code, constants::TUTORS_LOCATOR);
            self.move_tutor_moves_offset.set(offset);
        }
        offset
    }

    pub(crate) fn read_move_tutor_moves(&self) -> Vec<i32> {
        let mut moves = Vec::new();
        let offset = self.move_tutor_moves_offset();
        if offset <= 0 {
            return moves;
        }
        let mut offset = offset as usize;
        let mut value = 0;
        while value != 0xFFFF {
            value = read_word(&self.code, offset);
            offset += 2;
            if value == 0x26E || value == 0xFFFF {
                continue;
            }
            moves.push(value);
        }
        moves
    }

    pub(crate) fn write_move_tutor_moves(&mut self, moves: &[i32]) -> Result<()> {
        if let Some(offset) = find_after_prefix(&self.code, constants::TUTORS_SHOP_PREFIX) {
            for index in 0..constants::TUTOR_MOVE_COUNT as usize {
                let move_number = *moves
                    .get(index)
                    .ok_or(RandomizationError::IndexOutOfBounds("moves"))?;
                write_word(&mut self.code, offset + index * 8, move_number);
            }
        }
        let offset = self.move_tutor_moves_offset();
        if offset > 0 {
            let mut offset = offset as usize;
            for &move_number in moves {
                if read_word(&self.code, offset) == 0x26E {
                    offset += 2;
                }
                write_word(&mut self.code, offset, move_number);
                offset += 2;
            }
        }
        Ok(())
    }

    pub(crate) fn read_move_tutor_compatibility(&self) -> Compatibility {
        let sizes = constants::TUTOR_SIZE;
        let total: usize = sizes.iter().sum();
        let mut compatibility = BTreeMap::new();
        for number in 1..=self.total_pokemon() {
            let data = self
                .poke_garc
                .file(number as usize)
                .expect("pokemon stats exist");
            let mut flags = vec![false; total + 1];
            let mut offset = 0;
            for (tutor, size) in sizes.iter().enumerate() {
                for byte in 0..4 {
                    read_byte_into_flags(
                        data,
                        &mut flags,
                        offset + byte * 8 + 1,
                        constants::BS_MT_COMPAT_OFFSET + tutor * 4 + byte,
                    );
                }
                offset += size;
            }
            compatibility.insert(self.pk(number), flags);
        }
        compatibility
    }

    pub(crate) fn write_move_tutor_compatibility(&mut self, compatibility: &Compatibility) {
        if !self.is_oras() {
            return;
        }
        let sizes = constants::TUTOR_SIZE;
        for number in 1..=self.total_pokemon() {
            let pokemon = self.pk(number);
            let flags = compatibility
                .get(&pokemon)
                .expect("compatibility for every pokemon");
            let data =
                garc_file_mut(&mut self.poke_garc, number as usize).expect("pokemon stats exist");
            let mut offset = 0;
            for (tutor, &size) in sizes.iter().enumerate() {
                let mut tutor_flags = vec![false; size + 1];
                tutor_flags[1..=size].copy_from_slice(&flags[offset + 1..offset + 1 + size]);
                for byte in 0..4 {
                    data[constants::BS_MT_COMPAT_OFFSET + tutor * 4 + byte] =
                        byte_from_flags(&tutor_flags, byte * 8 + 1);
                }
                offset += size;
            }
        }
    }

    fn scripts_garc(&self) -> Result<GarcArchive> {
        self.read_garc(self.file("Scripts"), Decompression::Skip)
    }

    fn script_items(
        &self,
        garc: &GarcArchive,
        file_key: &str,
        offset_key: &str,
    ) -> Result<Vec<i32>> {
        let file = self.entry.int(file_key) as usize;
        let amx = Amx::parse(garc_file(garc, file)?).map_err(io)?;
        let start = self.entry.int(offset_key) as usize;
        Ok((start..amx.dec_data.len())
            .step_by(12)
            .map(|offset| read_word(&amx.dec_data, offset))
            .collect())
    }

    fn mega_stone_items(&self, garc: &GarcArchive) -> Result<Vec<i32>> {
        let file = self.entry.int("MegaStoneItemScriptNumber") as usize;
        let amx = Amx::parse(garc_file(garc, file)?).map_err(io)?;
        Ok((0..constants::MEGASTONE_TABLE_LENGTH_ORAS)
            .map(|index| {
                let offset = constants::MEGASTONE_TABLE_STARTING_OFFSET_ORAS
                    + index * constants::MEGASTONE_TABLE_ENTRY_SIZE_ORAS;
                read_word(&amx.dec_data, offset)
            })
            .collect())
    }

    pub(crate) fn field_items(&self) -> Result<Vec<i32>> {
        let garc = self.scripts_garc()?;
        let mut items = self.script_items(&garc, "FieldItemsScriptNumber", "FieldItemsOffset")?;
        if self.is_xy() {
            items.extend(self.script_items(
                &garc,
                "HiddenItemsScriptNumber",
                "HiddenItemsOffset",
            )?);
        } else if let Some(hidden) =
            find_after_prefix(&self.code, constants::HIDDEN_ITEMS_PREFIX_ORAS)
        {
            for index in 0..constants::HIDDEN_ITEM_COUNT_ORAS as usize {
                items.push(read_word(&self.code, hidden + index * 0xE + 2));
            }
        }
        if self.is_oras() {
            items.extend(self.mega_stone_items(&garc)?);
        }
        Ok(items)
    }

    fn write_script_items<'a>(
        &self,
        garc: &mut GarcArchive,
        file_key: &str,
        offset_key: &str,
        iterator: &mut impl Iterator<Item = &'a i32>,
    ) -> Result<()> {
        let file = self.entry.int(file_key) as usize;
        let mut amx = Amx::parse(garc_file(garc, file)?).map_err(io)?;
        let start = self.entry.int(offset_key) as usize;
        for offset in (start..amx.dec_data.len()).step_by(12) {
            let item = *iterator.next().ok_or_else(no_such_element)?;
            write_word(&mut amx.dec_data, offset, item);
        }
        garc.set_file(file, amx.to_bytes().map_err(io)?).map_err(io)
    }

    fn write_mega_stone_items<'a>(
        &self,
        garc: &mut GarcArchive,
        iterator: &mut impl Iterator<Item = &'a i32>,
    ) -> Result<()> {
        let old_items = self.mega_stone_items(garc)?;
        let file = self.entry.int("MegaStoneItemScriptNumber") as usize;
        let mut amx = Amx::parse(garc_file(garc, file)?).map_err(io)?;
        let mut replacements: HashMap<i32, i32> = HashMap::new();
        for (index, old_item) in old_items.into_iter().enumerate() {
            let offset = constants::MEGASTONE_TABLE_STARTING_OFFSET_ORAS
                + index * constants::MEGASTONE_TABLE_ENTRY_SIZE_ORAS;
            let new_item = *iterator.next().ok_or_else(no_such_element)?;
            let written = *replacements.entry(old_item).or_insert(new_item);
            write_word(&mut amx.dec_data, offset, written);
        }
        garc.set_file(file, amx.to_bytes().map_err(io)?).map_err(io)
    }

    pub(crate) fn write_field_items(&mut self, field_items: &[i32]) -> Result<()> {
        let mut iterator = field_items.iter();
        let mut garc = self.scripts_garc()?;
        self.write_script_items(
            &mut garc,
            "FieldItemsScriptNumber",
            "FieldItemsOffset",
            &mut iterator,
        )?;
        if self.is_xy() {
            self.write_script_items(
                &mut garc,
                "HiddenItemsScriptNumber",
                "HiddenItemsOffset",
                &mut iterator,
            )?;
        } else if let Some(hidden) =
            find_after_prefix(&self.code, constants::HIDDEN_ITEMS_PREFIX_ORAS)
        {
            for index in 0..constants::HIDDEN_ITEM_COUNT_ORAS as usize {
                let item = *iterator.next().ok_or_else(no_such_element)?;
                write_word(&mut self.code, hidden + index * 0xE + 2, item);
            }
        }
        if self.is_oras() {
            self.write_mega_stone_items(&mut garc, &mut iterator)?;
        }
        self.write_garc(self.file("Scripts"), &garc)
    }

    pub(crate) fn current_field_tms(&self) -> Result<Vec<i32>> {
        let allowed = constants::allowed_items(self.rom_type);
        Ok(self
            .field_items()?
            .into_iter()
            .filter(|&item| allowed.is_tm(item))
            .map(tm_from_index)
            .collect())
    }

    pub(crate) fn write_field_tms(&mut self, field_tms: &[i32]) -> Result<()> {
        let mut items = self.field_items()?;
        let allowed = constants::allowed_items(self.rom_type);
        let mut iterator = field_tms.iter();
        for item in items.iter_mut() {
            if allowed.is_tm(*item) {
                *item = index_from_tm(*iterator.next().ok_or_else(no_such_element)?);
            }
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
            if !allowed.is_tm(*item) && allowed.is_allowed(*item) && *item != items::MASTER_BALL {
                *item = *iterator.next().ok_or_else(no_such_element)?;
            }
        }
        self.write_field_items(&items)
    }

    fn shop_items_offset(&self) -> i32 {
        let mut offset = self.shop_items_offset.get();
        if offset == 0 {
            offset = find(&self.code, constants::shop_items_locator(self.rom_type));
            self.shop_items_offset.set(offset);
        }
        offset
    }

    fn is_skipped_shop(&self, shop: i32) -> bool {
        self.entry.array("TMShops").contains(&shop)
            || self.entry.array("RegularShops").contains(&shop)
    }

    pub(crate) fn read_shop_items(&self) -> BTreeMap<i32, Shop> {
        let sizes = self.entry.array("ShopItemSizes");
        let mut shops = BTreeMap::new();
        let offset = self.shop_items_offset();
        if offset <= 0 {
            return shops;
        }
        let mut offset = offset as usize;
        for shop in 0..self.entry.int("ShopCount") {
            let size = sizes[shop as usize] as usize;
            if self.is_skipped_shop(shop) {
                offset += size * 2;
                continue;
            }
            let items = (0..size)
                .map(|index| read_word(&self.code, offset + index * 2))
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
        shops
    }

    pub(crate) fn write_shop_items(&mut self, shops: &BTreeMap<i32, Shop>) -> Result<()> {
        let sizes = self.entry.array("ShopItemSizes");
        let offset = self.shop_items_offset();
        if offset <= 0 {
            return Ok(());
        }
        let mut offset = offset as usize;
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
                write_word(&mut self.code, offset, item);
                offset += 2;
            }
        }
        Ok(())
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

    fn pickup_table_offset(&self) -> i32 {
        if self.pickup_items_table_offset.get() == 0 {
            let offset = find(&self.code, constants::PICKUP_TABLE_LOCATOR);
            if offset > 0 {
                self.pickup_items_table_offset.set(offset);
            }
        }
        self.pickup_items_table_offset.get()
    }

    pub(crate) fn read_pickup_items(&self) -> Vec<PickupItem> {
        let offset = self.pickup_table_offset();
        if offset <= 0 {
            return Vec::new();
        }
        let mut pickup: Vec<PickupItem> = (0..constants::NUMBER_OF_PICKUP_ITEMS)
            .map(|index| PickupItem::new(read_word(&self.code, offset as usize + 2 * index)))
            .collect();
        for level_range in 0..10 {
            let rare = 18 + level_range;
            pickup[level_range].probabilities[level_range] = 30;
            for index in 1..7 {
                pickup[level_range + index].probabilities[level_range] = 10;
            }
            pickup[level_range + 7].probabilities[level_range] = 4;
            pickup[level_range + 8].probabilities[level_range] = 4;
            pickup[rare].probabilities[level_range] = 1;
            pickup[rare + 1].probabilities[level_range] = 1;
        }
        pickup
    }

    pub(crate) fn write_pickup_items(&mut self, pickup: &[PickupItem]) {
        let offset = self.pickup_items_table_offset.get();
        if offset <= 0 {
            return;
        }
        for (index, item) in pickup
            .iter()
            .take(constants::NUMBER_OF_PICKUP_ITEMS)
            .enumerate()
        {
            write_word(&mut self.code, offset as usize + 2 * index, item.item);
        }
    }

    pub(crate) fn read_ingame_trades(&self) -> Result<Vec<IngameTrade>> {
        let count = self.entry.int("IngameTradeCount") as usize;
        let strings = self.strings(false, self.entry.int("IngameTradesTextOffset"))?;
        let text_offset = self.entry.int("IngameTradesTextExtraOffset") as usize;
        let mut trades = Vec::new();
        let Some(mut offset) =
            find_after_prefix(&self.code, constants::ingame_trades_prefix(self.rom_type))
        else {
            return Ok(trades);
        };
        for index in 0..count {
            let text = |position: usize| {
                strings
                    .get(position)
                    .cloned()
                    .ok_or(RandomizationError::IndexOutOfBounds("tradeStrings"))
            };
            let requested = read_word(&self.code, offset + 0x20);
            trades.push(IngameTrade {
                id: 0,
                nickname: text(text_offset + index)?,
                given_pokemon: self.pk(read_word(&self.code, offset)),
                ivs: (0..6)
                    .map(|iv| i32::from(self.code[offset + 5 + iv] as i8))
                    .collect(),
                ot_id: read_word(&self.code, offset + 0xE),
                item: read_word(&self.code, offset + 0x10),
                ot_name: text(text_offset + count + index)?,
                requested_pokemon: self.optional_pokemon(requested),
            });
            offset += constants::INGAME_TRADE_SIZE as usize;
        }
        Ok(trades)
    }

    fn optional_pokemon(&self, number: i32) -> Option<randomon_data::PokemonId> {
        let id = self.pk(number);
        self.core.data.exists(id).then_some(id)
    }

    pub(crate) fn write_ingame_trades(&mut self, trades: &[IngameTrade]) -> Result<()> {
        let old_trades = self.read_ingame_trades()?;
        let count = self.entry.int("IngameTradeCount") as usize;
        let mut strings = self.strings(false, self.entry.int("IngameTradesTextOffset"))?;
        let text_offset = self.entry.int("IngameTradesTextExtraOffset") as usize;
        let Some(mut offset) =
            find_after_prefix(&self.code, constants::ingame_trades_prefix(self.rom_type))
        else {
            return Ok(());
        };
        for index in 0..count {
            let trade = trades
                .get(index)
                .ok_or(RandomizationError::IndexOutOfBounds("trades"))?;
            strings[text_offset + index] = trade.nickname.clone();
            write_word(&mut self.code, offset, trade.given_pokemon.number());
            for iv in 0..6 {
                self.code[offset + 5 + iv] = trade.ivs[iv] as u8;
            }
            write_word(&mut self.code, offset + 0xE, trade.ot_id);
            write_word(&mut self.code, offset + 0x10, trade.item);
            strings[text_offset + count + index] = trade.ot_name.clone();
            let requested = trade
                .requested_pokemon
                .map_or(0, |pokemon| pokemon.number());
            write_word(&mut self.code, offset + 0x20, requested);
            offset += constants::INGAME_TRADE_SIZE as usize;
            if self.is_xy()
                && let Some(position) = constants::XY_HARDCODED_TRADE_OFFSETS
                    .iter()
                    .position(|&hardcoded| hardcoded == index)
            {
                self.update_hardcoded_trade_text(
                    &old_trades[index],
                    trade,
                    constants::XY_HARDCODED_TRADE_TEXTS[position],
                )?;
            }
        }
        self.set_strings(false, self.entry.int("IngameTradesTextOffset"), &strings)
    }

    fn update_hardcoded_trade_text(
        &mut self,
        old_trade: &IngameTrade,
        new_trade: &IngameTrade,
        text_file: usize,
    ) -> Result<()> {
        let mut strings = self.strings(true, text_file as i32)?;
        let name_of = |pokemon: Option<randomon_data::PokemonId>| {
            pokemon.map(|pokemon| self.core.data.pk(pokemon).name.clone())
        };
        let old_requested = name_of(old_trade.requested_pokemon);
        let new_requested = name_of(new_trade.requested_pokemon);
        let old_given = self.core.data.pk(old_trade.given_pokemon).name.clone();
        let new_given = self.core.data.pk(new_trade.given_pokemon).name.clone();
        for text in strings.iter_mut() {
            if let (Some(old), Some(new)) = (&old_requested, &new_requested)
                && text.contains(old.as_str())
            {
                *text = text.replace(old.as_str(), new);
            }
            if text.contains(old_given.as_str()) {
                *text = text.replace(old_given.as_str(), &new_given);
            }
        }
        self.set_strings(true, text_file as i32, &strings)
    }
}
