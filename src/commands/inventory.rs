use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};

pub async fn handle_inventory(username: &str, world: &Arc<Mutex<World>>) -> String {
	let w = world.lock().await;
	let player = match w.get_player(username) {
		Some(p) => p,
		None => return TapError::PlayerNotFound.message(),
	};
	match serde_json::to_string(&player.inventory) {
		Ok(json) => format!("OK {}\n", json),
		Err(_) => TapError::SendFailed.message(),
	}
}
