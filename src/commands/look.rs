use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
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
    let mut value = match serde_json::to_value(room) {
        Ok(v) => v,
        Err(_) => return TapError::SendFailed.message(),
    };
    if player.current_room == w.sleep_room {
        value.as_object_mut().map(|o| o.insert("can_sleep".to_string(), json!(true)));
    }
    if w.merchant_room.as_deref() == Some(player.current_room.as_str()) {
        value.as_object_mut().map(|o| o.insert("can_trade".to_string(), json!(true)));
    }
    format!("OK {}\n", value)
}
