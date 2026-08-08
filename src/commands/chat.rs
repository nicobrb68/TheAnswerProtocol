use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World, TapError};
use crate::events::room::notify_room;
use crate::events::group::notify_group;


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
        match w.get_player(username) {
            Some(player) => player.current_room.clone(),
            None => return TapError::PlayerNotFound.message(),
        }
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
pub async fn handle_chat_group(
    username: &str,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
    message: &str,
    world: &Arc<Mutex<World>>
) -> String {
    let group_id = {
        let w = world.lock().await;
        match w.get_player(username) {
            Some(player) => match &player.group_id {
                Some(id) => id.clone(),
                None => return TapError::NotInGroup.message(),
            },
            None => return TapError::PlayerNotFound.message(),
        }
    };
    notify_group(
        &group_id,
        &format!("(GROUP) {}: {}\n", username, message),
        None,
        world,
        registry
    ).await;
    "OK\n".to_string()
}
