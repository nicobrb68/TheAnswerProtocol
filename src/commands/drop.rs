use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};


pub async fn handle_drop(username: &str, item_id: &str, world: &Arc<Mutex<World>>) -> String {
    let mut w = world.lock().await;
	let current_room = w.get_player(username).unwrap().current_room.clone();
	let inventory = w.get_player(username).unwrap().inventory.clone();
	let item_full_id = match inventory.iter().find(|id| id.contains(item_id)) {
		Some(id) => id.clone(),
		None => return TapError::ItemNotInInventory.message(),
	};
	w.get_mut_player(username).unwrap().inventory.retain(|i| i != &item_full_id);
	w.get_mut_room(&current_room).unwrap().items.push(item_full_id.clone());
	format!("OK dropped={}\n", item_full_id)
}