use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
use crate::{World, TapError};

pub async fn handle_shop(world: &Arc<Mutex<World>>) -> String {
    let w = world.lock().await;
    let catalog: Vec<serde_json::Value> = w.shop.iter()
        .filter_map(|id| w.items.get(id))
        .map(|item| json!({
            "id": item.id,
            "name": item.name,
            "price": item.value,
            "damage": item.damage,
            "armor": item.armor,
            "heal": item.heal,
        }))
        .collect();
    match serde_json::to_string(&catalog) {
        Ok(j) => format!("OK {}\n", j),
        Err(_) => TapError::SendFailed.message(),
    }
}

pub async fn handle_shop_buy(username: &str, item_id: &str, world: &Arc<Mutex<World>>) -> String {
    let mut w = world.lock().await;

    let item_full_id = match w.shop.iter().find(|id| id.contains(item_id)) {
        Some(id) => id.clone(),
        None => return TapError::ItemNotFound.message(),
    };

    let price = match w.items.get(&item_full_id) {
        Some(item) => item.value,
        None => return TapError::ItemNotFound.message(),
    };

    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };

    if player.gold < price {
        return "ERR 412 NOT_ENOUGH_GOLD\n".to_string();
    }

    if let Some(p) = w.get_mut_player(username) {
        p.gold -= price;
        p.inventory.push(item_full_id.clone());
    }

    tracing::info!(event = "shop_buy", player = %username, item = %item_full_id, price = price, "item purchased from shop");

    format!("OK bought={} price={} gold={}\n", item_full_id, price,
        w.get_player(username).map(|p| p.gold).unwrap_or(0))
}
