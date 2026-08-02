use std::sync::Arc;
use tokio::sync::Mutex;
use crate::World;

pub async fn handle_look(username: &str, world: &Arc<Mutex<World>>) -> String {
    let w = world.lock().await;
    let player = w.players.get(username).unwrap();
    let room = w.rooms.get(player.current_room.as_str()).unwrap();
    let json = serde_json::to_string(room).unwrap();
    format!("OK {}\n", json)
}