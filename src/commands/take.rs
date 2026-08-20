use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};

pub async fn handle_take(username: &str, item_id: &str, world: &Arc<Mutex<World>>) -> String {
    let mut w = world.lock().await;
    let current_room = match w.get_player(username) {
        Some(p) => p.current_room.clone(),
        None => return TapError::PlayerNotFound.message(),
    };
    let room = match w.get_room(&current_room) {
        Some(r) => r,
        None => return TapError::ItemNotFound.message(),
    };
    let item_full_id = match room.items.iter().find(|id| id.contains(item_id)) {
        Some(id) => id.clone(),
        None => return TapError::ItemNotFound.message(),
    };
    if let Some(r) = w.get_mut_room(&current_room) { r.items.retain(|i| i != &item_full_id); }
    if let Some(p) = w.get_mut_player(username) { p.inventory.push(item_full_id.clone()); }
    tracing::info!(event = "item_take", player = %username, item = %item_full_id, room = %current_room, "item taken");
    format!("OK taken={}\n", item_full_id)
}
