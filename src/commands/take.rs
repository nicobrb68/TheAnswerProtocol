use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use crate::{World, TapError};

const ITEM_RESPAWN_SECS: u64 = 30;

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
    if let Some(r) = w.get_mut_room(&current_room) {
        r.items.retain(|i| i != &item_full_id);
    }
    if let Some(p) = w.get_mut_player(username) {
        p.inventory.push(item_full_id.clone());
    }

    tracing::info!(event = "item_take", player = %username, item = %item_full_id, room = %current_room, "item taken");

    let world_clone = Arc::clone(world);
    let room_clone = current_room.clone();
    let item_clone = item_full_id.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(ITEM_RESPAWN_SECS)).await;
        let mut w = world_clone.lock().await;
        if let Some(r) = w.get_mut_room(&room_clone) {
            if !r.items.contains(&item_clone) {
                r.items.push(item_clone.clone());
                tracing::info!(event = "item_respawn", item = %item_clone, room = %room_clone, "item respawned");
            }
        }
    });

    format!("OK taken={}\n", item_full_id)
}