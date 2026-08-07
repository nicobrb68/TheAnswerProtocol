use std::sync::Arc;
use tokio::sync::Mutex;
use crate::World;

pub async fn handle_inventory(username: &str, world: &Arc<Mutex<World>>) -> String {
	let w = world.lock().await;
	let inventory = w.get_player(username).unwrap().inventory.clone();
	format!("OK {}\n", serde_json::to_string(&inventory).unwrap())
}