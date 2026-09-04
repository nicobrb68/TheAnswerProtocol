use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};

pub async fn handle_examine(username: &str, item_id: &str, world: &Arc<Mutex<World>>) -> String {
    let w = world.lock().await;
    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };

    // Look in the current room first, then in the player's own inventory —
    // matches the same "here or in your bag" reach TAKE/DROP already use.
    let room_match = w.get_room(&player.current_room)
        .and_then(|r| r.items.iter().find(|id| id.contains(item_id)).cloned());
    let inventory_match = player.inventory.iter().find(|id| id.contains(item_id)).cloned();

    let item_full_id = match room_match.or(inventory_match) {
        Some(id) => id,
        None => return TapError::ItemNotFound.message(),
    };

    let item = match w.get_item(&item_full_id) {
        Some(i) => i,
        None => return TapError::ItemNotFound.message(),
    };

    match serde_json::to_string(item) {
        Ok(json) => format!("OK {}\n", json),
        Err(_) => TapError::SendFailed.message(),
    }
}