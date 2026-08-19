use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, PlayerState};

pub async fn handle_status(username: &str, world: &Arc<Mutex<World>>) -> String {
	let w = world.lock().await;
	let player = w.get_player(username).unwrap();
	let status_str = match player.status {
		PlayerState::Alive => "alive",
		PlayerState::Dead => "dead",
	};
	format!("OK {{\"hp\": {}, \"max_hp\": {}, \"status\": \"{}\"}}\n", player.hp, player.max_hp, status_str)
}
