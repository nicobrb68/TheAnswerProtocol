use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use crate::World;

pub async fn notify_room(
    room_id: &str,
    message: &str,
    exclude: Option<&str>,
    world: &Arc<Mutex<World>>,
    registry: &Arc<Mutex<HashMap<String, UnboundedSender<String>>>>
) {
    let w = world.lock().await;
    let players = match w.get_room(room_id) {
        Some(room) => room.players.clone(),
        None => return,
    };
    let reg = registry.lock().await;
    for player in &players {
        if exclude.map_or(true, |ex| player != ex) {
            if let Some(tx) = reg.get(player) {
                let _ = tx.send(format!("{}", message));
            }
        }
    }
}