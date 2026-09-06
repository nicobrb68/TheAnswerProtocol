use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, PlayerState, TapError};

pub async fn handle_use(username: &str, item_id: &str, world: &Arc<Mutex<World>>) -> String {
    let mut w = world.lock().await;

    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };

    if matches!(player.status, PlayerState::Dead) {
        return TapError::PlayerDead.message();
    }

    let item_full_id = match player.inventory.iter().find(|id| id.contains(item_id)) {
        Some(id) => id.clone(),
        None => return TapError::ItemNotInInventory.message(),
    };

    let heal = match w.get_item(&item_full_id).and_then(|i| i.heal) {
        Some(h) => h,
        None => return TapError::ItemNotUsable.message(),
    };

    let player = match w.get_mut_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };

    if let Some(pos) = player.inventory.iter().position(|i| i == &item_full_id) {
        player.inventory.remove(pos);
    }

    player.hp = (player.hp + heal).min(player.max_hp);
    let new_hp = player.hp;
    let max_hp = player.max_hp;

    tracing::info!(event = "item_use", player = %username, item = %item_full_id, heal = heal, hp = new_hp, "item used");

    format!("OK {{\"used\": \"{}\", \"heal\": {}, \"hp\": {}, \"max_hp\": {}}}\n", item_full_id, heal, new_hp, max_hp)
}
