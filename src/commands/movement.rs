use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use crate::{World, Room, TapError};

pub async fn handle_move(username: &str, direction: &str, world: &Arc<Mutex<World>>) -> String {
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
    format!("OK room={}\n", new_room_id)

}