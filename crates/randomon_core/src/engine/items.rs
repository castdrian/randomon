use std::collections::BTreeMap;

use randomon_data::{ItemList, PickupItem, Shop};
use randomon_java::JavaRng;
use randomon_settings::{FieldItemsMod, Settings};

use crate::engine::pool::all_pokemon_incl_formes_without_null;
use crate::engine::util::random_index;
use crate::error::{RandomizationError, Result};
use crate::handler::RomHandler;

fn possible_items<H: RomHandler + ?Sized>(handler: &mut H, ban_bad_items: bool) -> ItemList {
    if ban_bad_items {
        handler.non_bad_items_mut().clone()
    } else {
        handler.allowed_items_mut().clone()
    }
}

fn distinct_rare(items: &ItemList, common: i32, random: &mut impl JavaRng) -> i32 {
    let mut rare = items.random_item(random);
    while rare == common {
        rare = items.random_item(random);
    }
    rare
}

pub fn randomize_wild_held_items<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let pokemon = all_pokemon_incl_formes_without_null(handler);
    let items = possible_items(handler, settings.ban_bad_random_wild_pokemon_held_items);
    let mut random = handler.core().engine.random.clone();
    for id in pokemon {
        let pk = handler.core_mut().data.pk_mut(id);
        if pk.guaranteed_held_item == -1
            && pk.common_held_item == -1
            && pk.rare_held_item == -1
            && pk.dark_grass_held_item == -1
        {
            return Ok(());
        }
        let mut can_have_dark_grass = pk.dark_grass_held_item != -1;
        if pk.guaranteed_held_item != -1 {
            if pk.guaranteed_held_item > 0 {
                if random.next_double() < 0.9 {
                    can_have_dark_grass = false;
                    pk.guaranteed_held_item = items.random_item(&mut random);
                } else {
                    pk.guaranteed_held_item = 0;
                    pk.common_held_item = items.random_item(&mut random);
                    pk.rare_held_item = distinct_rare(&items, pk.common_held_item, &mut random);
                }
            } else {
                let decision = random.next_double();
                if decision < 0.5 {
                    pk.common_held_item = 0;
                    pk.rare_held_item = 0;
                } else if decision < 0.65 {
                    pk.common_held_item = 0;
                    pk.rare_held_item = items.random_item(&mut random);
                } else if decision < 0.8 {
                    pk.common_held_item = items.random_item(&mut random);
                    pk.rare_held_item = 0;
                } else if decision < 0.95 {
                    pk.common_held_item = items.random_item(&mut random);
                    pk.rare_held_item = distinct_rare(&items, pk.common_held_item, &mut random);
                } else {
                    can_have_dark_grass = false;
                    pk.guaranteed_held_item = items.random_item(&mut random);
                    pk.common_held_item = 0;
                    pk.rare_held_item = 0;
                }
            }
        } else {
            let decision = random.next_double();
            if decision < 0.5 {
                pk.common_held_item = 0;
                pk.rare_held_item = 0;
            } else if decision < 0.65 {
                pk.common_held_item = 0;
                pk.rare_held_item = items.random_item(&mut random);
            } else if decision < 0.8 {
                pk.common_held_item = items.random_item(&mut random);
                pk.rare_held_item = 0;
            } else {
                pk.common_held_item = items.random_item(&mut random);
                pk.rare_held_item = distinct_rare(&items, pk.common_held_item, &mut random);
            }
        }
        if can_have_dark_grass {
            pk.dark_grass_held_item = if random.next_double() < 0.5 {
                items.random_item(&mut random)
            } else {
                0
            };
        } else if pk.dark_grass_held_item != -1 {
            pk.dark_grass_held_item = 0;
        }
    }
    Ok(())
}

pub fn randomize_starter_held_items<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let old = handler.get_starter_held_items();
    let items = possible_items(handler, settings.ban_bad_random_starter_held_items);
    let mut random = handler.core().engine.random.clone();
    let new_items: Vec<i32> = old.iter().map(|_| items.random_item(&mut random)).collect();
    handler.set_starter_held_items(&new_items);
    Ok(())
}

pub fn shuffle_field_items<H: RomHandler + ?Sized>(handler: &mut H) -> Result<()> {
    let mut items = handler.get_regular_field_items();
    let mut tms = handler.get_current_field_tms();
    let mut random = handler.core().engine.random.clone();
    randomon_java::shuffle(&mut items, &mut random);
    randomon_java::shuffle(&mut tms, &mut random);
    handler.set_regular_field_items(&items)?;
    handler.set_field_tms(&tms)
}

fn item_placement_history<H: RomHandler + ?Sized>(handler: &H, item: i32) -> i32 {
    handler
        .core()
        .engine
        .item_placement_history
        .get(&item)
        .copied()
        .unwrap_or(0)
}

fn item_placement_average<H: RomHandler + ?Sized>(handler: &H) -> f32 {
    let history = &handler.core().engine.item_placement_history;
    let sum: i32 = history.values().sum();
    sum as f32 / history.len() as f32
}

pub fn randomize_field_items<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let distribute = settings.field_items_mod == FieldItemsMod::RandomEven;
    let unique_items = !settings.balance_shop_prices;
    let mut items = possible_items(handler, settings.ban_bad_random_field_items);
    let current_items = handler.get_regular_field_items();
    let current_tms = handler.get_current_field_tms();
    let required_tms = handler.get_required_field_tms();
    let unique_no_sell = handler.get_unique_no_sell_items();
    let total_tm_count = handler.get_tm_count();
    let mut random = handler.core().engine.random.clone();
    let mut new_items: Vec<i32> = Vec::new();
    let mut new_tms: Vec<i32> = required_tms.clone();
    for _ in 0..current_items.len() {
        let mut chosen = items.random_non_tm(&mut random);
        if distribute {
            let mut iterations = 0;
            while f64::from(item_placement_history(handler, chosen))
                > f64::from(item_placement_average(handler))
                && iterations < 100
            {
                chosen = items.random_non_tm(&mut random);
                iterations += 1;
            }
        }
        new_items.push(chosen);
        if unique_items && unique_no_sell.contains(&chosen) {
            items.ban_singles(&[chosen]);
        } else if distribute {
            *handler
                .core_mut()
                .engine
                .item_placement_history
                .entry(chosen)
                .or_insert(0) += 1;
        }
    }
    for _ in required_tms.len()..current_tms.len() {
        loop {
            let tm = random_index(&mut random, total_tm_count.max(0) as usize)? as i32 + 1;
            if !new_tms.contains(&tm) {
                new_tms.push(tm);
                break;
            }
        }
    }
    randomon_java::shuffle(&mut new_items, &mut random);
    randomon_java::shuffle(&mut new_tms, &mut random);
    handler.set_regular_field_items(&new_items)?;
    handler.set_field_tms(&new_tms)
}

pub fn shuffle_shop_items<H: RomHandler + ?Sized>(handler: &mut H) -> Result<()> {
    let Some(mut shops) = handler.get_shop_items() else {
        return Ok(());
    };
    let mut all_items: Vec<i32> = shops
        .values()
        .flat_map(|shop| shop.items.iter().copied())
        .collect();
    let mut random = handler.core().engine.random.clone();
    randomon_java::shuffle(&mut all_items, &mut random);
    let mut iterator = all_items.into_iter();
    for shop in shops.values_mut() {
        for slot in shop.items.iter_mut() {
            *slot = iterator
                .next()
                .ok_or(RandomizationError::IndexOutOfBounds("shop items"))?;
        }
    }
    handler.set_shop_items(&shops)
}

fn unique_random_non_tm(items: &ItemList, existing: &[i32], random: &mut impl JavaRng) -> i32 {
    loop {
        let item = items.random_non_tm(random);
        if !existing.contains(&item) {
            return item;
        }
    }
}

pub fn randomize_shop_items<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    if handler.get_shop_items().is_none() {
        return Ok(());
    }
    let regular = if settings.ban_regular_shop_items {
        Some(
            handler
                .get_regular_shop_items()
                .ok_or(RandomizationError::NullPointer("getRegularShopItems"))?,
        )
    } else {
        None
    };
    let op_items = if settings.ban_op_shop_items {
        Some(
            handler
                .get_op_shop_items()
                .ok_or(RandomizationError::NullPointer("getOPShopItems"))?,
        )
    } else {
        None
    };
    let possible = {
        let list = if settings.ban_bad_random_shop_items {
            handler.non_bad_items_mut()
        } else {
            handler.allowed_items_mut()
        };
        if let Some(regular) = &regular {
            list.ban_singles(regular);
        }
        if let Some(op_items) = &op_items {
            list.ban_singles(op_items);
        }
        list.clone()
    };
    let current = handler.get_shop_items().unwrap_or_default();
    let mut shop_item_count: usize = current.values().map(|shop| shop.items.len()).sum();
    let mut new_items: Vec<i32> = Vec::new();
    let mut new_shops: BTreeMap<i32, Shop> = BTreeMap::new();
    let mut guaranteed: Vec<i32> = Vec::new();
    if settings.guarantee_evolution_items {
        guaranteed.extend(
            handler
                .get_evolution_items()
                .ok_or(RandomizationError::NullPointer("getEvolutionItems"))?,
        );
    }
    if settings.guarantee_x_items {
        guaranteed.extend(handler.get_x_items());
    }
    let mut random = handler.core().engine.random.clone();
    if settings.guarantee_evolution_items || settings.guarantee_x_items {
        new_items.extend(guaranteed.iter().copied());
        shop_item_count = shop_item_count.saturating_sub(new_items.len());
        for _ in 0..shop_item_count {
            let item = unique_random_non_tm(&possible, &new_items, &mut random);
            new_items.push(item);
        }
        let main_game: Vec<i32> = current
            .iter()
            .filter(|(_, shop)| shop.is_main_game)
            .map(|(&id, _)| id)
            .collect();
        let non_main_game: Vec<i32> = current
            .iter()
            .filter(|(_, shop)| !shop.is_main_game)
            .map(|(&id, _)| id)
            .collect();
        randomon_java::shuffle(&mut new_items, &mut random);
        for id in non_main_game {
            let old = &current[&id];
            let mut shop_items = Vec::new();
            for _ in &old.items {
                let mut position = 0;
                let mut item = *new_items
                    .get(position)
                    .ok_or(RandomizationError::IndexOutOfBounds("newItems"))?;
                while guaranteed.contains(&item) {
                    position += 1;
                    item = *new_items
                        .get(position)
                        .ok_or(RandomizationError::IndexOutOfBounds("newItems"))?;
                }
                shop_items.push(item);
                crate::engine::util::remove_first(&mut new_items, &item);
            }
            new_shops.insert(
                id,
                Shop {
                    items: shop_items,
                    name: old.name.clone(),
                    is_main_game: old.is_main_game,
                },
            );
        }
        randomon_java::shuffle(&mut new_items, &mut random);
        for id in main_game {
            let old = &current[&id];
            let mut shop_items = Vec::new();
            for _ in &old.items {
                if new_items.is_empty() {
                    return Err(RandomizationError::IndexOutOfBounds("newItems"));
                }
                shop_items.push(new_items.remove(0));
            }
            new_shops.insert(
                id,
                Shop {
                    items: shop_items,
                    name: old.name.clone(),
                    is_main_game: old.is_main_game,
                },
            );
        }
    } else {
        for _ in 0..shop_item_count {
            let item = unique_random_non_tm(&possible, &new_items, &mut random);
            new_items.push(item);
        }
        let mut iterator = new_items.into_iter();
        for (&id, old) in &current {
            let mut shop_items = Vec::new();
            for _ in &old.items {
                shop_items.push(
                    iterator
                        .next()
                        .ok_or(RandomizationError::IndexOutOfBounds("newItems"))?,
                );
            }
            new_shops.insert(
                id,
                Shop {
                    items: shop_items,
                    name: old.name.clone(),
                    is_main_game: old.is_main_game,
                },
            );
        }
    }
    handler.set_shop_items(&new_shops)?;
    if settings.balance_shop_prices {
        handler.set_shop_prices();
    }
    Ok(())
}

pub fn randomize_pickup_items<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let items = possible_items(handler, settings.ban_bad_random_pickup_items);
    let current = handler.get_pickup_items();
    let generation = handler.generation_of_pokemon();
    let mut random = handler.core().engine.random.clone();
    let mut new_items: Vec<PickupItem> = Vec::new();
    for old in &current {
        let item = if generation == 3 || generation == 4 {
            items.random_item(&mut random)
        } else {
            items.random_non_tm(&mut random)
        };
        let mut pickup = PickupItem::new(item);
        let mut probabilities = old.probabilities.clone();
        probabilities.resize(current.len(), 0);
        pickup.probabilities = probabilities;
        new_items.push(pickup);
    }
    handler.set_pickup_items(&new_items);
    Ok(())
}
