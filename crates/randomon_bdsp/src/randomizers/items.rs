use std::collections::BTreeMap;

use randomon_unity::Value;

use crate::assets::{BundleId, path_ids};
use crate::context::Context;
use crate::data::GameData;
use crate::error::{Result, data_error};

const ITEM_COMMANDS: [i64; 3] = [41, 187, 194];
const FLOAT_ARGUMENT: i64 = 1;
const MASTER_BALL: u16 = 1;
const SAFARI_BALL: u16 = 5;
const RANDOMIZABLE_POCKETS: [u8; 6] = [0, 1, 2, 3, 4, 6];
const COMMON_MART_ZONE: i64 = -1;

pub fn is_randomizable_item(data: &GameData, item: u16) -> bool {
    data.items.get(&item).is_some_and(|info| {
        RANDOMIZABLE_POCKETS.contains(&info.pocket)
            && info.price > 0
            && item != MASTER_BALL
            && item != SAFARI_BALL
            && data
                .names
                .items
                .get(&item)
                .is_none_or(|name| !name.is_empty() && name != "???")
    })
}

pub fn item_pool(data: &GameData) -> Vec<u16> {
    data.items
        .keys()
        .copied()
        .filter(|&item| is_randomizable_item(data, item))
        .collect()
}

pub fn decode_item_argument(raw: i64) -> Option<u16> {
    let value = f32::from_bits(raw as u32);
    (value.fract() == 0.0 && value >= 0.0 && value <= f32::from(u16::MAX)).then_some(value as u16)
}

pub fn encode_item_argument(item: u16) -> i128 {
    i128::from(f32::from(item).to_bits() as i32)
}

fn field_item_argument(script: &mut Value) -> Result<Option<&mut Value>> {
    let Ok(commands) = script.field_mut("Commands") else {
        return Ok(None);
    };
    let Some(first) = commands
        .as_array_mut()
        .and_then(|commands| commands.first_mut())
    else {
        return Ok(None);
    };
    let arguments = first
        .field_mut("Arg")?
        .as_array_mut()
        .ok_or_else(|| data_error("Arg is not an array"))?;
    if arguments.len() < 2 || !ITEM_COMMANDS.contains(&arguments[0].int_field("data")?) {
        return Ok(None);
    }
    if arguments[1].int_field("argType")? != FLOAT_ARGUMENT {
        return Ok(None);
    }
    Ok(Some(arguments[1].field_mut("data")?))
}

pub fn randomize_field_items(context: &mut Context<'_>) -> Result<()> {
    if !context.settings.field_items {
        return Ok(());
    }
    let pool = item_pool(&context.data);
    let mut table = context
        .bundles
        .read(BundleId::EvScript, path_ids::FLD_ITEM)?;
    let scripts = table
        .field_mut("Scripts")?
        .as_array_mut()
        .ok_or_else(|| data_error("Scripts is not an array"))?;
    let mut changed = 0usize;
    for script in scripts.iter_mut() {
        let Some(argument) = field_item_argument(script)? else {
            continue;
        };
        let Some(item) = decode_item_argument(argument.int()?) else {
            continue;
        };
        if !is_randomizable_item(&context.data, item) {
            continue;
        }
        let replacement = context.pick(&pool, "field item")?;
        argument.set_int(encode_item_argument(replacement))?;
        changed += 1;
    }
    context
        .bundles
        .write(BundleId::EvScript, path_ids::FLD_ITEM, &table)?;
    context.log.line(
        "Field items",
        format!("{changed} field items randomized; TMs and key items were kept."),
    );
    Ok(())
}

fn randomize_group(
    context: &mut Context<'_>,
    pool: &[u16],
    entries: &mut [Value],
    group_field: &str,
    skip: impl Fn(&Value) -> bool,
) -> Result<BTreeMap<i64, Vec<u16>>> {
    let mut groups: BTreeMap<i64, Vec<u16>> = BTreeMap::new();
    for entry in entries.iter_mut() {
        if skip(entry) {
            continue;
        }
        let group = entry.int_field(group_field)?;
        let used = groups.entry(group).or_default();
        let fresh: Vec<u16> = pool
            .iter()
            .copied()
            .filter(|item| !used.contains(item))
            .collect();
        let choices = if fresh.is_empty() {
            pool.to_vec()
        } else {
            fresh
        };
        let item = context.pick(&choices, "shop item")?;
        groups.entry(group).or_default().push(item);
        entry.set_int_field("ItemNo", i128::from(item))?;
    }
    Ok(groups)
}

pub fn randomize_shops(context: &mut Context<'_>) -> Result<()> {
    if !context.settings.shops {
        return Ok(());
    }
    let pool = item_pool(&context.data);
    let keep_basics = context.settings.shops_keep_basics;
    let mut table = context
        .bundles
        .read(BundleId::Masterdatas, path_ids::SHOP_TABLE)?;
    let marts = table
        .field_mut("FS")?
        .as_array_mut()
        .ok_or_else(|| data_error("FS is not an array"))?;
    let mart_groups = randomize_group(context, &pool, marts, "ZoneID", |entry| {
        keep_basics
            && entry
                .int_field("ZoneID")
                .is_ok_and(|zone| zone == COMMON_MART_ZONE)
    })?;
    let fixed = table
        .field_mut("FixedShop")?
        .as_array_mut()
        .ok_or_else(|| data_error("FixedShop is not an array"))?;
    let fixed_groups = randomize_group(context, &pool, fixed, "ShopID", |_| false)?;
    context
        .bundles
        .write(BundleId::Masterdatas, path_ids::SHOP_TABLE, &table)?;
    let names = &context.data.names;
    let mut lines = Vec::new();
    for (zone, items) in &mart_groups {
        let list: Vec<String> = items.iter().map(|&item| names.item(item)).collect();
        lines.push(format!("Mart in zone {zone}: {}", list.join(", ")));
    }
    for (shop, items) in &fixed_groups {
        let list: Vec<String> = items.iter().map(|&item| names.item(item)).collect();
        lines.push(format!("Shop {shop}: {}", list.join(", ")));
    }
    for line in lines {
        context.log.line("Shops", line);
    }
    Ok(())
}
