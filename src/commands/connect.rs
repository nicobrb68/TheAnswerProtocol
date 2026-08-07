use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{Player, TapError, World, utils::is_valid_username};

pub async fn handle_connect(username: &str, world: &Arc<Mutex<World>>) -> String {
    if !is_valid_username(username) {
        return TapError::InvalidUsername.message();
    }
    
    let mut w = world.lock().await;

    if w.has_player(username) {
        return TapError::NameInUse.message();
    }
    let spawn = w.spawn.clone();
    let user = Player::new(username.to_string(), spawn.clone());
    w.add_player(user);
    w.get_mut_room(&spawn).unwrap().players.push(username.to_string());

    "OK Connected\n".to_string()
}