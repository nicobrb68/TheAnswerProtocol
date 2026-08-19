use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};

pub async fn handle_drop(username: &str, item_id: &str, world: &Arc<Mutex<World>>) -> String {
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
	if let Some(p) = w.get_mut_player(username) { p.inventory.retain(|i| i != &item_full_id); }
	if let Some(r) = w.get_mut_room(&current_room) { r.items.push(item_full_id.clone()); }
	format!("OK dropped={}\n", item_full_id)
}
