use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{World, TapError};

pub async fn handle_take(username: &str, item_id: &str, world: &Arc<Mutex<World>>) -> String {
    let mut w = world.lock().await;
    let current_room = w.get_player(username).unwrap().current_room.clone();
    let item_full_id = match w.get_room(&current_room).unwrap().items.iter().find(|id| id.contains(item_id)) {
        Some(id) => id.clone(),
        None => return TapError::ItemNotFound.message(),
    };
    w.get_mut_room(&current_room).unwrap().items.retain(|i| i != &item_full_id);
    w.get_mut_player(username).unwrap().inventory.push(item_full_id.clone());
    format!("OK taken={}\n", item_full_id)
}