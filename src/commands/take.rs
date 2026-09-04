use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World, TapError};
use crate::events::room::notify_room;

const ITEM_RESPAWN_SECS: u64 = 30;

pub async fn handle_take(
    username: &str,
    item_id: &str,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
) -> String {
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
    drop(w);

    // Let everyone else still in the room know the item is gone, so their
    // client-side room state doesn't go stale until their next manual LOOK.
    notify_room(
        &current_room,
        &format!("EVT ROOM ITEM TAKEN {} {}\n", item_full_id, username),
        Some(username),
        world,
        registry,
    ).await;

    // Item respawns on its own after a while — broadcast that too, so
    // clients see it reappear instead of it silently coming back only on
    // their next LOOK.
    let world_clone = Arc::clone(world);
    let registry_clone = Arc::clone(registry);
    let room_clone = current_room.clone();
    let item_clone = item_full_id.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(ITEM_RESPAWN_SECS)).await;
        let mut w = world_clone.lock().await;
        let spawned = if let Some(r) = w.get_mut_room(&room_clone) {
            if !r.items.contains(&item_clone) {
                r.items.push(item_clone.clone());
                tracing::info!(event = "item_respawn", item = %item_clone, room = %room_clone, "item respawned");
                true
            } else {
                false
            }
        } else {
            false
        };
        drop(w);
        if spawned {
            notify_room(
                &room_clone,
                &format!("EVT ROOM ITEM RESPAWN {}\n", item_clone),
                None,
                &world_clone,
                &registry_clone,
            ).await;
        }
    });

    format!("OK taken={}\n", item_full_id)
}