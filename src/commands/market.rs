use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
use crate::{World, MarketListing, TapError};

pub async fn handle_market(world: &Arc<Mutex<World>>) -> String {
    let w = world.lock().await;
    let listings: Vec<serde_json::Value> = w.market.iter().enumerate()
        .map(|(i, listing)| {
            let item = w.items.get(&listing.item_id);
            json!({
                "index": i,
                "item_id": listing.item_id,
                "name": item.map(|it| it.name.as_str()).unwrap_or("Unknown"),
                "seller": listing.seller,
                "price": listing.price,
                "damage": item.and_then(|it| it.damage),
                "armor": item.and_then(|it| it.armor),
                "heal": item.and_then(|it| it.heal),
            })
        })
        .collect();
    match serde_json::to_string(&listings) {
        Ok(j) => format!("OK {}\n", j),
        Err(_) => TapError::SendFailed.message(),
    }
}

pub async fn handle_sell(username: &str, item_id: &str, world: &Arc<Mutex<World>>) -> String {
    let mut w = world.lock().await;

    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };

    let item_full_id = match player.inventory.iter().find(|id| id.contains(item_id)) {
        Some(id) => id.clone(),
        None => return TapError::ItemNotInInventory.message(),
    };

    let price = match w.items.get(&item_full_id) {
        Some(item) => item.value,
        None => return TapError::ItemNotFound.message(),
    };

    if let Some(p) = w.get_mut_player(username) {
        if let Some(pos) = p.inventory.iter().position(|i| i == &item_full_id) {
            p.inventory.remove(pos);
        }
    }

    w.market.push(MarketListing {
        item_id: item_full_id.clone(),
        seller: username.to_string(),
        price,
    });

    tracing::info!(event = "market_sell", player = %username, item = %item_full_id, price = price, "item listed on market");

    format!("OK listed={} price={}\n", item_full_id, price)
}

pub async fn handle_market_buy(username: &str, index_str: &str, world: &Arc<Mutex<World>>) -> String {
    let mut w = world.lock().await;

    let index: usize = match index_str.parse() {
        Ok(i) => i,
        Err(_) => return TapError::ItemNotFound.message(),
    };

    if index >= w.market.len() {
        return TapError::ItemNotFound.message();
    }

    let listing = &w.market[index];
    let price = listing.price;
    let item_id = listing.item_id.clone();
    let seller = listing.seller.clone();

    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };

    if player.gold < price {
        return "ERR 412 NOT_ENOUGH_GOLD\n".to_string();
    }

    if let Some(p) = w.get_mut_player(username) {
        p.gold -= price;
        p.inventory.push(item_id.clone());
    }

    if let Some(s) = w.get_mut_player(&seller) {
        s.gold += price;
    }

    w.market.remove(index);

    tracing::info!(event = "market_buy", buyer = %username, seller = %seller, item = %item_id, price = price, "item purchased from market");

    let buyer_gold = w.get_player(username).map(|p| p.gold).unwrap_or(0);
    format!("OK bought={} price={} seller={} gold={}\n", item_id, price, seller, buyer_gold)
}
