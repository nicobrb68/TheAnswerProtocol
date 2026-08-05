use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use crate::{World, Room, TapError};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

pub async fn handle_move(username: &str, direction: &str, world: &Arc<Mutex<World>>, registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>) -> String {
    let mut w: MutexGuard<World> = world.lock().await;
    let current_room: String = w.get_mut_player(username).unwrap().current_room.clone();
    let new_room_id: String = {
        let room: &Room = w.get_room(&current_room).unwrap();
        if !room.exits.contains_key(direction) {
            return TapError::NoExit.message();
        }
        room.exits.get(direction).unwrap().clone()
    };
    w.get_mut_player(username).unwrap().current_room = new_room_id.clone();
    w.get_mut_room(&current_room).unwrap().players.retain(|p| p != username);
    w.get_mut_room(&new_room_id).unwrap().players.push(username.to_string());
    let reg = registry.lock().await;
    for player in &w.get_room(&current_room).unwrap().players {
        if let Some(tx) = reg.get(player) {
            if player != username {
                let _ = tx.send(format!("EVT ROOM PRESENCE LEAVE {}\n", username));
            }
        }
    }
    for player in &w.get_room(&new_room_id).unwrap().players {
        if let Some(tx) = reg.get(player) {
            if player != username {
                let _ = tx.send(format!("EVT ROOM PRESENCE ENTER {}\n", username));
            }
        }
    }
    format!("OK room={}\n", new_room_id)
}