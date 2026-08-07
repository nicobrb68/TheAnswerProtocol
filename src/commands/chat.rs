use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World};
use crate::events::room::notify_room;


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
    world: &Arc<Mutex<World>>
) -> String {
    let room_id = {
        let w = world.lock().await;
        w.get_player(username).unwrap().current_room.clone()
    };
    notify_room(
        &room_id,
        &format!("(ROOM) {}: {}\n", username, message),
        None,
        world,
        registry
    ).await;
    "OK\n".to_string()
}
