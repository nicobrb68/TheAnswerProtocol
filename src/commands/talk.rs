use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};

pub async fn handle_talk(username: &str, npc_id: &str, world: &Arc<Mutex<World>>) -> String {
    let w = world.lock().await;
    let player = w.get_player(username).unwrap();
    let room = w.get_room(&player.current_room).unwrap();
    let npc_full_id = match room.npcs.iter().find(|id| id.contains(npc_id)) {
        Some(id) => id.clone(),
        None => return TapError::NpcNotFound.message(),
    };
    let npc = w.get_npc(&npc_full_id).unwrap();
    let dialogue = npc.dialogue.first().unwrap_or(&"...".to_string()).clone();
    format!("OK {{\"npc\": \"{}\", \"dialogue\": \"{}\"}}\n", npc.name, dialogue)
}