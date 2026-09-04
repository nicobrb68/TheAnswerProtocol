use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World, TapError};
use crate::events::room::notify_room;

pub async fn handle_drop(
    username: &str,
    item_id: &str,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
) -> String {
    let mut w = world.lock().await;
    let player = match w.get_player(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };
    let current_room = player.current_room.clone();
    let inventory = player.inventory.clone();
    let item_full_id = match inventory.iter().find(|id| id.contains(item_id)) {
        Some(id) => id.clone(),
        None => return TapError::ItemNotInInventory.message(),
    };
    if let Some(p) = w.get_mut_player(username) {
        // Remove only ONE matching copy, not every copy — inventories can
        // legitimately hold duplicate item ids (e.g. a quest reward given
        // in multiple units), and DROP should shed a single unit at a time.
        if let Some(pos) = p.inventory.iter().position(|i| i == &item_full_id) {
            p.inventory.remove(pos);
        }
    }
    if let Some(r) = w.get_mut_room(&current_room) {
        r.items.push(item_full_id.clone());
    }
    tracing::info!(event = "item_drop", player = %username, item = %item_full_id, room = %current_room, "item dropped");
    drop(w);

    notify_room(
        &current_room,
        &format!("EVT ROOM ITEM DROPPED {} {}\n", item_full_id, username),
        Some(username),
        world,
        registry,
    ).await;

    format!("OK dropped={}\n", item_full_id)
}