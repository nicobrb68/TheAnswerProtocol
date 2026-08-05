use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World};

pub async fn handle_disconnect(username: &Option<String>, world: &Arc<Mutex<World>>) -> Result<(), String> {
	
	match username {
		Some(name) => {
			let mut w = world.lock().await;
		
			let player_room_id = match w.get_player(name) {
				Some(player) => player.current_room.clone(),
				_ => return Err("Failed to fetch user: {}".to_string()),
			};
			match w.get_mut_room(&player_room_id){
				Some(room) => room.players.retain(|p| p != name),
				_ => return Err("Failed to fetch user's room".to_string()),
			};
			w.players.remove(name);
    	},
    	_ => {}
	}
	Ok(())
}
