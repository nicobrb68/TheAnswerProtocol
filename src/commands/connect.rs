use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{Player, World, TapError};

pub async fn handle_connect(username: &str, world: &Arc<Mutex<World>>) -> String {
    let mut w = world.lock().await;

    if w.has_player(&username) {
        return TapError::NameInUse.message();
    }
    let user = Player::new(username.to_string(), "room.square".to_string());
    w.add_player(user);

    "OK Connected\n".to_string()
}