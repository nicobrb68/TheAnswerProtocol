use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};

pub async fn handle_look(username: &str, world: &Arc<Mutex<World>>) -> String {
    let w = world.lock().await;
    let player = match w.players.get(username) {
        Some(p) => p,
        None => return TapError::PlayerNotFound.message(),
    };
    let room = match w.rooms.get(player.current_room.as_str()) {
        Some(r) => r,
        None => return TapError::NoExit.message(),
    };
    let json = match serde_json::to_string(room) {
        Ok(j) => j,
        Err(_) => return TapError::SendFailed.message(),
    };
    format!("OK {}\n", json)
}
