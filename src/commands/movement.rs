use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use crate::{World, TapError};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;
use crate::events::room::notify_room;

pub async fn handle_move(username: &str, direction: &str, world: &Arc<Mutex<World>>, registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>) -> String {
    let mut w: MutexGuard<World> = world.lock().await;
    let current_room = match w.get_player(username) {
        Some(p) => p.current_room.clone(),
        None => return TapError::PlayerNotFound.message(),
    };
    let room = match w.get_room(&current_room) {
        Some(r) => r,
        None => return TapError::NoExit.message(),
    };
    let new_room_id = match room.exits.get(direction) {
        Some(id) => id.clone(),
        None => return TapError::NoExit.message(),
    };

    // A guarded room only lets you back out the way you came in, until every
    // hostile in it has been put down.
    if room.guarded {
        let still_hostile = room.npcs.iter()
            .any(|id| w.get_npc(id).map(|n| n.hostile).unwrap_or(false));
        let came_from = w.get_player(username).and_then(|p| p.entered_from.clone());
        if still_hostile && came_from.as_deref() != Some(new_room_id.as_str()) {
            return TapError::RoomGuarded.message();
        }
    }

    if let Some(p) = w.get_mut_player(username) {
        p.current_room = new_room_id.clone();
        p.entered_from = Some(current_room.clone());
        // Walking away ends the fight.
        p.in_combat_with = None;
        p.defending = false;
    }
    if let Some(r) = w.get_mut_room(&current_room) { r.players.retain(|p| p != username); }
    if let Some(r) = w.get_mut_room(&new_room_id) { r.players.push(username.to_string()); }
    drop(w);
    notify_room(&current_room, &format!("EVT ROOM PRESENCE LEAVE {}\n", username), Some(username), &world, &registry).await;
    notify_room(&new_room_id, &format!("EVT ROOM PRESENCE ENTER {}\n", username), Some(username), &world, &registry).await;
    format!("OK room={}\n", new_room_id)
}
