use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::{World, events::room::notify_room};

pub async fn handle_disconnect(
    username: &Option<String>,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) -> Result<(), String> {
    match username {
        Some(name) => {
            let player_room_id = {
                let w = world.lock().await;
                match w.get_player(name) {
                    Some(player) => player.current_room.clone(),
                    None => return Err("Failed to fetch user".to_string()),
                }
            };
            notify_room(&player_room_id, &format!("EVT ROOM PRESENCE LEAVE {}\n", name), Some(name), world, registry).await;
            let mut w = world.lock().await;
			let mut reg = registry.lock().await;
			reg.remove(name);
            match w.get_mut_room(&player_room_id) {
                Some(room) => room.players.retain(|p| p != name),
                None => return Err("Failed to fetch user's room".to_string()),
            };
            w.players.remove(name);
        },
        None => {}
    }
    Ok(())
}