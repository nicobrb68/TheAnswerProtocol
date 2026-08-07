use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::World;


pub async fn handle_chat_global(
	username: &str,
	registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
	message: &str
) -> String {
	let r = registry.lock().await; 
	for (_player, tx) in r.iter() {
		let _ = tx.send(format!("(GLOBAL) {}: {}\n", username, message));
	}
	"OK\n".to_string()	
}


pub async fn handle_chat_room(
	username: &str,
	registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
	message: &str,
	world:	&Arc<Mutex<World>> 
) -> String {

	let w = world.lock().await;
	let room_id = w.get_player(username).unwrap().current_room.clone();
	let room_players = w.get_room(&room_id).unwrap().players.clone();
	drop(w);
	let r = registry.lock().await;
	for player in &room_players {
			if let Some(tx) = r.get(player) {
				let _ = tx.send(format!("(ROOM) {}: {}\n", username, message));
		}
	}
	"OK\n".to_string()	
}
