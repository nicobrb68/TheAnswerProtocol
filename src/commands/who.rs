use std::sync::Arc;
use tokio::sync::Mutex;
use crate::World;

pub async fn handle_who(world: &Arc<Mutex<World>>) -> String {
	let w = world.lock().await;
	
	format!("OK players={}\n", w.players.len())
}
