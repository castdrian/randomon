use randomon_data::text::java_equals_ignore_case;
use randomon_java::JavaRng;
use randomon_settings::{CustomNames, InGameTradesMod, Settings};

use crate::engine::pool::{check_pokemon_restrictions, random_pokemon};
use crate::engine::util::random_index;
use crate::error::Result;
use crate::handler::RomHandler;

fn filtered_names<H: RomHandler + ?Sized>(
    handler: &H,
    names: &[String],
    max_length: i32,
) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for name in names {
        if handler.internal_string_length(name) <= max_length && !result.contains(name) {
            result.push(name.clone());
        }
    }
    result
}

fn pick_unused(names: &[String], used: &[String], random: &mut impl JavaRng) -> Result<String> {
    let mut name = &names[random_index(random, names.len())?];
    while used.contains(name) {
        name = &names[random_index(random, names.len())?];
    }
    Ok(name.clone())
}

pub fn randomize_ingame_trades<H: RomHandler + ?Sized>(
    handler: &mut H,
    settings: &Settings,
) -> Result<()> {
    let randomize_request =
        settings.in_game_trades_mod == InGameTradesMod::RandomizeGivenAndRequested;
    let random_nickname = settings.randomize_in_game_trades_nicknames;
    let random_ot = settings.randomize_in_game_trades_ots;
    let random_stats = settings.randomize_in_game_trades_ivs;
    let random_item = settings.randomize_in_game_trades_items;
    let custom = settings
        .custom_names
        .clone()
        .unwrap_or_else(CustomNames::default_names);
    check_pokemon_restrictions(handler);
    let trainer_names = if random_ot {
        filtered_names(
            handler,
            &custom.trainer_names,
            handler.max_trade_ot_name_length(),
        )
    } else {
        Vec::new()
    };
    let nicknames = if random_nickname {
        filtered_names(
            handler,
            &custom.pokemon_nicknames,
            handler.max_trade_nickname_length(),
        )
    } else {
        Vec::new()
    };
    let mut trades = handler.get_ingame_trades();
    let mut used_requests = Vec::new();
    let mut used_givens = Vec::new();
    let mut used_ots: Vec<String> = Vec::new();
    let mut used_nicknames: Vec<String> = Vec::new();
    let items = handler.allowed_items_mut().clone();
    let has_dvs = handler.has_dvs();
    let mut random = handler.core().engine.random.clone();
    for trade in &mut trades {
        let old_given = trade.given_pokemon;
        let mut given = random_pokemon(handler)?;
        while used_givens.contains(&given) {
            given = random_pokemon(handler)?;
        }
        used_givens.push(given);
        trade.given_pokemon = given;
        if Some(old_given) == trade.requested_pokemon {
            trade.requested_pokemon = Some(given);
        } else if randomize_request && trade.requested_pokemon.is_some() {
            let mut request = random_pokemon(handler)?;
            while used_requests.contains(&request) || request == given {
                request = random_pokemon(handler)?;
            }
            used_requests.push(request);
            trade.requested_pokemon = Some(request);
        }
        if random_nickname && nicknames.len() > used_nicknames.len() {
            let nickname = pick_unused(&nicknames, &used_nicknames, &mut random)?;
            used_nicknames.push(nickname.clone());
            trade.nickname = nickname;
        } else if java_equals_ignore_case(&trade.nickname, &handler.core().data.pk(old_given).name)
        {
            trade.nickname = handler.core().data.pk(trade.given_pokemon).name.clone();
        }
        if random_ot && trainer_names.len() > used_ots.len() {
            let ot = pick_unused(&trainer_names, &used_ots, &mut random)?;
            used_ots.push(ot.clone());
            trade.ot_name = ot;
            trade.ot_id = random.next_int_bounded(65536);
        }
        if random_stats {
            let max_iv = if has_dvs { 16 } else { 32 };
            for iv in &mut trade.ivs {
                *iv = random.next_int_bounded(max_iv);
            }
        }
        if random_item {
            trade.item = items.random_item(&mut random);
        }
    }
    handler.set_ingame_trades(&trades)
}
